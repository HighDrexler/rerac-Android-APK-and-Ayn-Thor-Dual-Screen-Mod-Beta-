#!/usr/bin/env python3
"""Package a genuine source-built Android core with the existing editable launcher.
Both architecture artifacts must come from one native source revision. Version
metadata is read from compiled ELF stamps and checked against archived source;
there is deliberately no --core-version argument. Original importer/Java payloads
and signing identity are retained. No game data or private key is packaged.
"""
from __future__ import annotations
import argparse, copy, hashlib, io, json, re, subprocess, sys, tarfile, tempfile, tomllib, zipfile
from pathlib import Path
ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT/'android/launcher/tools/android'))
from build_launcher import build as build_launcher
from apk_format import file_sha256, native_alignment
from native_abi_gate import inspect
STAMP = b'RERAC_ANDROID_CORE_JSON:'
TARGETS = {'arm64-v8a':'aarch64-linux-android','x86_64':'x86_64-linux-android'}
SHADER_PREFIX = 'crates/rc-engine/assets/shaders/'
V16 = {'arm64-v8a':'67a8859411c621edb834e7689ff520010349161b13d0ff31a97f1ea2ee2d037f', 'x86_64':'4297a53c0fc783046fe0517789d9e52fddb78281bf3ad7cbce85d859d03e00d5'}

def require(value, message):
    if not value: raise ValueError(message)

def digest(data): return hashlib.sha256(data).hexdigest()

def core_stamp(data: bytes) -> dict:
    stamps=[]
    for m in re.finditer(re.escape(STAMP), data):
        start=m.end(); end=data.find(b'\0', start, start+2048)
        if end<0: continue
        try: value=json.loads(data[start:end])
        except (UnicodeDecodeError,ValueError): continue
        if isinstance(value,dict) and value.get('schema')==1 and value not in stamps: stamps.append(value)
    require(len(stamps)==1,'Missing or ambiguous compiled native-core provenance')
    info=stamps[0]
    require(set(info)=={'schema','core_version','upstream_sha','source_sha','target','min_sdk','adapter_api'},'Unknown native stamp schema')
    require(isinstance(info['core_version'],str) and re.fullmatch(r'[0-9]+\.[0-9]+\.[0-9]+(?:[-+][A-Za-z0-9.-]+)?',info['core_version']),'Invalid compiled core version')
    for field in ('upstream_sha','source_sha'):
        require(isinstance(info[field],str) and re.fullmatch('[0-9a-f]{40}',info[field]),'Invalid '+field)
    require(info['min_sdk']==26 and info['adapter_api']==1,'Unsupported native platform contract')
    return info

def read_member(z,name,maximum):
    entries=[e for e in z.infolist() if e.filename==name]
    require(len(entries)==1 and 0<entries[0].file_size<=maximum,'Missing, duplicate or oversized artifact member: '+name)
    return z.read(entries[0])

def native_artifact(path:Path,abi:str,work:Path):
    with zipfile.ZipFile(path) as z:
        data=read_member(z,f'jniLibs/{abi}/librc_engine.so',350*1024*1024)
        source=read_member(z,'source-commit.txt',128).decode().strip()
        archive=read_member(z,'source.tar.gz',80*1024*1024)
        info=core_stamp(data)
        require(info['source_sha']==source and info['target']==TARGETS[abi],'Native artifact source/ABI does not match compiled stamp')
        require(digest(data)!=V16[abi],'Refusing to relabel the retained v16 engine')
        library=work/(abi+'.so');library.write_bytes(data)
        gate=inspect(library,abi); require(gate['ok'],'Original Android native interface is missing')
        needed=subprocess.check_output(['readelf','-d',str(library)],text=True)
        dependencies=set(re.findall(r'\(NEEDED\).*\[([^\]]+)\]',needed))
        require(dependencies <= {'libdl.so','liblog.so','libandroid.so','libaaudio.so','libm.so','libc.so'},'Unpackaged native runtime dependency: '+str(dependencies))
        symbols=subprocess.check_output(['readelf','--dyn-syms','--wide',str(library)],text=True)
        require(' rerac_android_core_info_v1' in symbols,'Missing callable native version API')
    shaders={}
    with tarfile.open(fileobj=io.BytesIO(archive),mode='r:gz') as tar:
        members=tar.getmembers()
        require(len(members)<10000,'Unexpected source archive size')
        require(len({m.name for m in members})==len(members),'Duplicate source archive paths')
        def source_file(name):
            member=tar.getmember(name)
            require(member.isfile() and member.size<=2*1024*1024,'Invalid source file: '+name)
            return tar.extractfile(member).read()
        cargo=tomllib.loads(source_file('Cargo.toml').decode())
        require(cargo['workspace']['package']['version']==info['core_version'],'Archived source version differs from compiled engine')
        pin=dict(line.split('=',1) for line in source_file('android/native/upstream.lock').decode().splitlines() if '=' in line and not line.startswith('#'))
        require(pin=={'version':info['core_version'],'commit':info['upstream_sha']},'Archived upstream pin differs from compiled engine')
        for member in members:
            if member.name.startswith(SHADER_PREFIX) and member.isfile():
                suffix=member.name[len(SHADER_PREFIX):]
                require(re.fullmatch('[A-Za-z0-9_./-]+',suffix) and '..' not in suffix,'Invalid shader path')
                shaders['assets/shaders/'+suffix]=source_file(member.name)
        require(len(shaders)>=20,'Native source archive lacks its matching shader set')
    return data,info,shaders,gate

def package(baseline:Path,tools:Path,arm64:Path,x86:Path,output:Path,version:str,code:int)->dict:
    require(code>19,'Native core update must have a versionCode newer than beta.3')
    output.parent.mkdir(parents=True,exist_ok=True)
    require(not output.exists(),'Refusing to overwrite an existing candidate APK')
    with tempfile.TemporaryDirectory(prefix='rerac-core-') as td:
        work=Path(td); libraries={}; infos={}; shader_sets={}; gates={}
        for abi,path in [('arm64-v8a',arm64),('x86_64',x86)]:
            libraries[abi],infos[abi],shader_sets[abi],gates[abi]=native_artifact(path,abi,work)
        first=infos['arm64-v8a'];second=infos['x86_64']
        require({k:v for k,v in first.items() if k!='target'}=={k:v for k,v in second.items() if k!='target'},'Architectures were not built from the same source/core')
        require(shader_sets['arm64-v8a']==shader_sets['x86_64'],'Architectures have different shader source')
        shaders=shader_sets['arm64-v8a'];core=first['core_version']
        launcher=work/'launcher.apk';build_launcher(baseline,tools,launcher,version,code)
        replacements={f'lib/{abi}/librc_engine.so':data for abi,data in libraries.items()}
        replacements.update(shaders)
        with zipfile.ZipFile(launcher) as z:
            metadata=json.loads(z.read('assets/rerac-build.json'))
            metadata.update(core_version=core,core_provenance=f'Source-built Android core {core}; upstream {first["upstream_sha"][:12]}',native_source_rebuild=True,native_source_sha=first['source_sha'],upstream_sha=first['upstream_sha'],adapter_api=first['adapter_api'])
            for abi,data in libraries.items():metadata['native_sha256'][f'lib/{abi}/librc_engine.so']=digest(data)
            metadata['shader_sha256']={name:digest(data) for name,data in shaders.items()}
            replacements['assets/rerac-build.json']=(json.dumps(metadata,sort_keys=True,separators=(',',':'))+'\n').encode()
            require(len(replacements['assets/rerac-build.json'])<=16384,'Launcher metadata size limit exceeded')
            mod=json.loads(z.read('assets/mods/dual-screen/mod.json'));mod['core_versions']=[core];mod['version']='1.1'
            mod['description']='Companion maps, menus, inventory and quick selection, backed by the source-built Android engine. Changes apply on the next game launch.'
            replacements['assets/mods/dual-screen/mod.json']=(json.dumps(mod,sort_keys=True,separators=(',',':'))+'\n').encode()
            raw=work/'core-unaligned.apk'
            with zipfile.ZipFile(raw,'w') as new:
                for entry in z.infolist():
                    if entry.filename.startswith('assets/shaders/') and entry.filename not in shaders:continue
                    data=replacements.pop(entry.filename,z.read(entry))
                    clone=copy.copy(entry);clone.extra=b'';clone.date_time=(2026,10,19,0,0,0)
                    new.writestr(clone,data)
                for name,data in sorted(replacements.items()):
                    zi=zipfile.ZipInfo(name,(2026,10,19,0,0,0));zi.compress_type=zipfile.ZIP_DEFLATED;new.writestr(zi,data)
        subprocess.run([str(tools/'build-tools/zipalign'),'-P','16','-f','4',str(raw),str(output)],check=True)
        aligned=native_alignment(output)
        preserved=[]
        with zipfile.ZipFile(baseline) as old,zipfile.ZipFile(output) as new:
            require(len(new.namelist())==len(set(new.namelist())),'Duplicate APK entries')
            for entry in old.infolist():
                name=entry.filename
                if name in ('AndroidManifest.xml','classes6.dex') or name.startswith(('META-INF/','assets/shaders/')) or name in {f'lib/{abi}/librc_engine.so' for abi in TARGETS}:continue
                require(old.read(name)==new.read(name),'Unrelated original payload changed: '+name);preserved.append(name)
            for abi in TARGETS:require(core_stamp(new.read(f'lib/{abi}/librc_engine.so'))==infos[abi],'Final native bytes changed')
            require(json.loads(new.read('assets/rerac-build.json'))==metadata,'Final metadata changed')
    report=dict(schema=1,launcher_version=version,version_code=code,core_version=core,unsigned_sha256=file_sha256(output),bytes=output.stat().st_size,native_source_rebuild=True,native_source_sha=first['source_sha'],upstream_sha=first['upstream_sha'],native_libraries=aligned,native_abi=gates,shader_sha256=metadata['shader_sha256'],preserved_entries=preserved,device_gameplay_tested=False,physical_dual_screen_tested=False)
    output.with_suffix('.build-report.json').write_text(json.dumps(report,indent=2)+'\n')
    return report

if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__)
    for name in ('baseline','tools','arm64','x86','output'):p.add_argument('--'+name,type=Path,required=True)
    p.add_argument('--version',required=True);p.add_argument('--code',type=int,required=True)
    args=p.parse_args();print(json.dumps(package(**vars(args)),indent=2))

#!/usr/bin/env python3
"""Build launcher 1.1 with source-built native selection presentation."""
import argparse, copy, hashlib, json, struct, subprocess, zipfile
from pathlib import Path
from launcher_dex import source_built_dex, EXPECTED_SIGNER
import sys
sys.path.insert(0,str(Path(__file__).resolve().parents[1]/'launcher/tools/android'))
sys.path.insert(0,str(Path(__file__).resolve().parents[1]/'native'))
from apk_format import axml_chunks, string_pool, make_string_pool, manifest_values

BASE_SHA='1fc723d70a20da5dff97d789edcdaeeecbdbbbbdf56aa5ce1e8f835e6d669003'
from package_core import core_stamp

def sha(data):return hashlib.sha256(data).hexdigest()

def manifest(data,version,code):
    parts=axml_chunks(data)
    index=next(i for i,p in enumerate(parts) if struct.unpack_from('<H',p)[0]==1)
    strings,flags=string_pool(parts[index]);assert strings.count('1.1')==1
    label=next(e['attrs']['label'] for e in manifest_values(data) if e['element']=='application')
    assert isinstance(label,str) and label in strings
    strings=[version if s=='1.1' else 'ReRAC 1.1' if s==label else s for s in strings]
    parts[index]=bytearray(make_string_pool(strings,flags));changed=0
    for node in parts:
        if struct.unpack_from('<H',node)[0]!=0x102:continue
        if strings[struct.unpack_from('<I',node,20)[0]]!='manifest':continue
        start,size,count=struct.unpack_from('<HHH',node,24)
        for j in range(count):
            pos=16+start+j*size
            if strings[struct.unpack_from('<I',node,pos+4)[0]]=='versionCode':
                assert struct.unpack_from('<I',node,pos+16)[0]==38
                struct.pack_into('<I',node,pos+16,code);changed+=1
    assert changed==1
    result=struct.pack('<HHI',3,8,8+sum(map(len,parts)))+b''.join(parts)
    expected=copy.deepcopy(manifest_values(data));expected[0]['attrs'].update(versionName=version,versionCode=code)
    next(e for e in expected if e['element']=='application')['attrs']['label']='ReRAC 1.1'
    assert manifest_values(result)==expected
    return result

def build(baseline,sdk,output,arm64,x86,source,packaging):
    assert sha(baseline.read_bytes())==BASE_SHA
    tools=sdk/'build-tools/36.0.0';zipalign=tools/('zipalign.exe' if (tools/'zipalign.exe').exists() else 'zipalign')
    dex=source_built_dex(EXPECTED_SIGNER,zipalign)
    assert b'update-channel3/builds/' in dex and b'UpdateArchive' in dex
    output.mkdir(parents=True,exist_ok=True)
    with zipfile.ZipFile(baseline) as old:
        original=json.loads(old.read('assets/rerac-build.json'))
        assert original['native_source_sha']=='7ece329abb56f20253dfa3e6303eee9ed01b663b' and original['signer_sha256']==EXPECTED_SIGNER
        assert original['package_id']=='org.rerac.android.ui035test' and original['core_version']=='0.3.6'
        assert original['version_code']==38 and not original.get('upgrade_test_fixture',False)
        native={n:sha(old.read(n)) for n in old.namelist() if n.startswith('lib/') and n.endswith('.so')}
        assert len(native)==4 and native==original['native_sha256']
        libraries={'lib/arm64-v8a/librc_engine.so':arm64.read_bytes(),'lib/x86_64/librc_engine.so':x86.read_bytes()}
        for name,data in libraries.items():
            stamp=core_stamp(data);assert stamp['source_sha']==source and stamp['core_version']=='0.3.6'
            assert stamp['target']==('aarch64-linux-android' if 'arm64' in name else 'x86_64-linux-android')
        native.update({name:sha(data) for name,data in libraries.items()})
        name='launcher-unsigned.apk'
        metadata=dict(original)
        metadata.update(update_feed_branch='update-channel3',build_delivery='branch-archive-v1',baseline_apk_sha256=BASE_SHA,companion_world_visibility='independent-of-weapon-lock-v1',companion_map_image_cache='exact-mask-and-palette-v1',companion_wheel_copy='per-publication-v1')
        version,code='1.1',39
        metadata.update(launcher_version=version,version_code=code,companion_input='contact-once-v1',companion_presentation='rac1-native-selection-v1',source_sha=source,native_source_sha=source,packaging_source_sha=packaging,native_sha256=native,core_provenance='Source-built ReRAC 0.3.6 / '+source[:12])
        metadata.pop('upgrade_test_fixture',None)
        raw=output/(name+'.unaligned');target=output/name
        assert not target.exists() and not raw.exists()
        with zipfile.ZipFile(raw,'w') as new:
            for entry in old.infolist():
                body=old.read(entry)
                if entry.filename in libraries:body=libraries[entry.filename]
                elif entry.filename=='classes6.dex':body=dex
                elif entry.filename=='assets/rerac-build.json':body=(json.dumps(metadata,sort_keys=True,separators=(',',':'))+'\n').encode()
                elif entry.filename=='AndroidManifest.xml':body=manifest(body,version,code)
                info=zipfile.ZipInfo(entry.filename,(2026,10,21,0,0,0))
                info.compress_type=zipfile.ZIP_STORED;info.create_system=0;info.external_attr=0o600<<16
                new.writestr(info,body)
        subprocess.run([str(zipalign),'-P','16','-f','4',str(raw),str(target)],check=True)
        raw.unlink()
        with zipfile.ZipFile(target) as check:
            assert check.namelist()==old.namelist() and check.testzip() is None
            allowed={'classes6.dex','assets/rerac-build.json','AndroidManifest.xml',*libraries}
            for n in old.namelist():
                if n not in allowed:assert sha(old.read(n))==sha(check.read(n)),n
        report=dict(schema=1,apk=name,sha256=sha(target.read_bytes()),bytes=target.stat().st_size,
                    native_source_sha=source,native_sha256=native,changed_entries=sorted(allowed))
        target.with_suffix('.json').write_text(json.dumps(report,indent=2)+'\n')
        print(json.dumps(report))

if __name__=='__main__':
    p=argparse.ArgumentParser()
    for n in ('baseline','sdk','output','arm64','x86'):p.add_argument('--'+n,type=Path,required=True)
    p.add_argument('--source',required=True)
    p.add_argument('--packaging-source',required=True)
    a=p.parse_args();build(a.baseline,a.sdk,a.output,a.arm64,a.x86,a.source,a.packaging_source)

#!/usr/bin/env python3
"""Build separate-install ReRAC Android Dev APK from a *real* source-built native candidate.

Changes only the application package, update-signature trust, feed branch,
app label, and launcher metadata. The signed candidate must retain the exact
same immutable Android/engine payload after this transformation.
Never store an app-signing private key in the repository.
"""
from __future__ import annotations
import argparse,copy,hashlib,json,re,struct,subprocess,sys,tempfile,zipfile
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]
sys.path.insert(0,str(ROOT/'android/launcher/tools/android'))
from apk_format import axml_chunks,string_pool,make_string_pool,manifest_values,native_alignment

OLD_PACKAGE='org.rerac.android.uipreview'
DEV_PACKAGE='org.rerac.android.ui035test'
OLD_CERT='25c5eebb05a013bf3fb84706f9ad87904b8d4d8a99ba7857d62e665d2b94e24a'
OLD_BRANCH='update-channels'
DEV_BRANCH='update-channel2'
EXPECTED_SIGNER='e2253eb7370c495ef0265efcdbe6bfbe09832070558fe9ed7047e7a2361ad75e'
assert len(OLD_PACKAGE)==len(DEV_PACKAGE) and len(OLD_BRANCH)==len(DEV_BRANCH)

def manifest(data,core,version,code,old_version,old_code):
    parts=axml_chunks(data)
    index=next(i for i,p in enumerate(parts) if struct.unpack_from('<H',p)[0]==1)
    strings,flags=string_pool(parts[index]);before=manifest_values(data)
    assert strings.count(OLD_PACKAGE)==1 and strings.count(OLD_PACKAGE+'.updates')==1
    strings=[s.replace(OLD_PACKAGE,DEV_PACKAGE) if s in (OLD_PACKAGE,OLD_PACKAGE+'.updates') else s for s in strings]
    assert strings.count(old_version)==1
    strings=[version if s==old_version else s for s in strings]
    label='ReRAC '+core+' Test'
    assert label not in strings
    label_id=len(strings);strings.append(label)
    parts[index]=bytearray(make_string_pool(strings,flags))
    found=0;version_found=0
    for node in parts:
        if struct.unpack_from('<H',node)[0]!=0x102:continue
        element=strings[struct.unpack_from('<I',node,20)[0]]
        if element not in ('application','manifest'):continue
        start,size,count=struct.unpack_from('<HHH',node,24)
        for j in range(count):
            pos=16+start+j*size
            key=strings[struct.unpack_from('<I',node,pos+4)[0]]
            if element=='manifest' and key=='versionCode':
                assert struct.unpack_from('<I',node,pos+16)[0]==old_code
                struct.pack_into('<I',node,pos+16,code)
                version_found+=1
            if element=='application' and key=='label':
                struct.pack_into('<I',node,pos+8,label_id)
                node[pos+15]=3
                struct.pack_into('<I',node,pos+16,label_id)
                found+=1
    assert found==1 and version_found==1
    result=struct.pack('<HHI',3,8,8+sum(map(len,parts)))+b''.join(parts)
    expected=copy.deepcopy(before)
    for entry in expected:
        if entry['element']=='manifest':entry['attrs'].update(package=DEV_PACKAGE,versionName=version,versionCode=code)
        if entry['element']=='application':entry['attrs']['label']=label
        if entry['element']=='provider' and entry['attrs'].get('name')=='org.rerac.android.UpdateApkProvider':
            entry['attrs']['authorities']=DEV_PACKAGE+'.updates'
    assert manifest_values(result)==expected,'Unrelated Android manifest attribute changed'
    return result

def source_built_dex(signer,zipalign):
    """Compile Java sources with proper DEX string_ids ordering, never byte-patch DEX.

    Prior sidecar corrupted the DEX sorted string pool, preventing the provider
    from loading and causing a crash before the launcher could open.
    """
    assert re.fullmatch(r'[0-9a-f]{64}',signer)
    source=ROOT/'android/launcher/android/src'
    android_jar=zipalign.parents[2]/'platforms/android-36/android.jar'
    d8=zipalign.parent/'d8'
    assert android_jar.is_file() and d8.is_file(), 'Android SDK 36 build tools required'
    with tempfile.TemporaryDirectory(prefix='rerac-recompile-') as td:
        base=Path(td);sources=base/'sources';classes=base/'classes';dexdir=base/'dex'
        classes.mkdir();dexdir.mkdir();paths=[]
        for original in sorted(source.rglob('*.java')):
            relative=original.relative_to(source)
            target=sources/relative;target.parent.mkdir(parents=True,exist_ok=True)
            text=original.read_text()
            def single(a,b):
                nonlocal text
                assert text.count(a)==1, f'Unexpected constant in {relative}: {a}'
                text=text.replace(a,b)
            if original.name=='LauncherBuild.java':
                single(OLD_PACKAGE,DEV_PACKAGE)
                single(OLD_CERT,signer)
            if original.name=='UpdatePolicy.java':
                single('"/'+OLD_BRANCH+'/"','"/'+DEV_BRANCH+'/"')
            target.write_text(text);paths.append(str(target))
        assert len(paths)>=10, 'Missing Android launcher sources'
        subprocess.run(['javac','-encoding','UTF-8','--release','8','-classpath',str(android_jar),'-d',str(classes),*paths],check=True)
        jar=base/'launcher.jar'
        subprocess.run(['jar','cf',str(jar),'-C',str(classes),'.'],check=True)
        subprocess.run([str(d8),'--release','--min-api','26','--lib',str(android_jar),'--output',str(dexdir),str(jar)],check=True)
        found=sorted(dexdir.glob('*.dex'))
        assert [x.name for x in found]==['classes.dex'],'Unexpected compiled multidex layout'
        data=found[0].read_bytes()
        assert b'UpdateApkProvider' in data and signer.encode() in data and DEV_PACKAGE.encode() in data
        assert DEV_BRANCH.encode() in data and OLD_CERT.encode() not in data
        return data

def transform(candidate,zipalign,output,signer,version=None,code=None):
    assert signer==EXPECTED_SIGNER,'Unrecognized release certificate'
    assert not output.exists(),'Refusing to overwrite an APK'
    assert zipalign.is_file(),'Android SDK zipalign is required'
    raw=output.with_suffix('.unaligned.tmp.apk')
    assert not raw.exists()
    compiled_dex=source_built_dex(signer,zipalign)
    with zipfile.ZipFile(candidate) as old:
        meta=json.loads(old.read('assets/rerac-build.json'))
        assert meta['package_id']==OLD_PACKAGE
        assert meta['native_source_rebuild'] is True
        assert int(meta['version_code'])>=20
        assert meta['core_version'].count('.')==2 and meta['core_version']!='0.3.4'
        old_version=meta['launcher_version'];old_code=meta['version_code']
        version=version or old_version; code=code if code is not None else old_code
        assert isinstance(version,str) and re.fullmatch(r'1\.1\.0-beta\.[0-9]+',version)
        assert isinstance(code,int) and code>=old_code and code>=20
        native={n:hashlib.sha256(old.read(n)).hexdigest() for n in old.namelist() if n.startswith('lib/') and n.endswith('.so')}
        assert len(native)==4
        meta.update(package_id=DEV_PACKAGE,signer_sha256=signer,update_feed_branch=DEV_BRANCH,parallel_install=True,launcher_version=version,version_code=code)
        with zipfile.ZipFile(raw,'w',allowZip64=True) as new:
            names=set()
            for entry in old.infolist():
                name=entry.filename
                if name.startswith('META-INF/') and name.upper().endswith(('.RSA','.SF','.MF','.DSA','.EC')):continue
                assert name not in names
                names.add(name)
                body=old.read(entry)
                if name=='AndroidManifest.xml':body=manifest(body,meta['core_version'],version,code,old_version,old_code)
                elif name=='classes6.dex':body=compiled_dex
                elif name=='assets/rerac-build.json':body=(json.dumps(meta,sort_keys=True,separators=(',',':'))+'\n').encode()
                clone=copy.copy(entry)
                clone.extra=b''
                new.writestr(clone,body)
    try:
        subprocess.run([str(zipalign),'-P','16','-f','4',str(raw),str(output)],check=True)
    finally:
        raw.unlink(missing_ok=True)
    checked=native_alignment(output)
    assert {n:v['sha256'] for n,v in checked.items()}==native,'Native libraries changed unexpectedly'
    with zipfile.ZipFile(output) as z:
        assert z.testzip() is None
        assert manifest_values(z.read('AndroidManifest.xml'))[0]['attrs']['package']==DEV_PACKAGE
        assert z.read('classes6.dex')==compiled_dex
        assert signer.encode() in z.read('classes6.dex')
        assert OLD_CERT.encode() not in z.read('classes6.dex')
        assert DEV_BRANCH.encode() in z.read('classes6.dex')
    return dict(schema=1,core_version=meta['core_version'],launcher_version=meta['launcher_version'],
                version_code=meta['version_code'],app_id=DEV_PACKAGE,signer_sha256=signer,
                update_channel_branch=DEV_BRANCH,sha256=hashlib.sha256(output.read_bytes()).hexdigest(),
                bytes=output.stat().st_size,native_sha256=native)
if __name__=='__main__':
    p=argparse.ArgumentParser()
    p.add_argument('--candidate',type=Path,required=True)
    p.add_argument('--zipalign',type=Path,required=True)
    p.add_argument('--output',type=Path,required=True)
    p.add_argument('--signer',default=EXPECTED_SIGNER)
    p.add_argument('--launcher-version')
    p.add_argument('--version-code',type=int)
    a=p.parse_args()
    print(json.dumps(transform(a.candidate,a.zipalign,a.output,a.signer,version=a.launcher_version,code=a.version_code),indent=2))

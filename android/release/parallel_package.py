#!/usr/bin/env python3
"""Build separate-install ReRAC Android Dev APK from a *real* source-built native candidate.

Changes only the application package, update-signature trust, feed branch,
app label, and launcher metadata. The signed candidate must retain the exact
same immutable Android/engine payload after this transformation.
Never store an app-signing private key in the repository.
"""
from __future__ import annotations
import argparse,copy,hashlib,json,struct,subprocess,sys,zipfile,zlib
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

def manifest(data,core):
    parts=axml_chunks(data)
    index=next(i for i,p in enumerate(parts) if struct.unpack_from('<H',p)[0]==1)
    strings,flags=string_pool(parts[index]);before=manifest_values(data)
    assert strings.count(OLD_PACKAGE)==1 and strings.count(OLD_PACKAGE+'.updates')==1
    strings=[s.replace(OLD_PACKAGE,DEV_PACKAGE) if s in (OLD_PACKAGE,OLD_PACKAGE+'.updates') else s for s in strings]
    label='ReRAC '+core+' Test'
    assert label not in strings
    label_id=len(strings);strings.append(label)
    parts[index]=bytearray(make_string_pool(strings,flags))
    found=0
    for node in parts:
        if struct.unpack_from('<H',node)[0]!=0x102:continue
        if strings[struct.unpack_from('<I',node,20)[0]]!='application':continue
        start,size,count=struct.unpack_from('<HHH',node,24)
        for j in range(count):
            pos=16+start+j*size
            if strings[struct.unpack_from('<I',node,pos+4)[0]]=='label':
                struct.pack_into('<I',node,pos+8,label_id)
                node[pos+15]=3
                struct.pack_into('<I',node,pos+16,label_id)
                found+=1
    assert found==1
    result=struct.pack('<HHI',3,8,8+sum(map(len,parts)))+b''.join(parts)
    expected=copy.deepcopy(before)
    for entry in expected:
        if entry['element']=='manifest':entry['attrs']['package']=DEV_PACKAGE
        if entry['element']=='application':entry['attrs']['label']=label
        if entry['element']=='provider' and entry['attrs'].get('name')=='org.rerac.android.UpdateApkProvider':
            entry['attrs']['authorities']=DEV_PACKAGE+'.updates'
    assert manifest_values(result)==expected,'Unrelated Android manifest attribute changed'
    return result

def dex(data,cert):
    assert data.startswith(b'dex\n')
    data=bytearray(data)
    for original,new in ((OLD_PACKAGE,DEV_PACKAGE),(OLD_CERT,cert),(OLD_BRANCH,DEV_BRANCH)):
        old=original.encode();replacement=new.encode()
        assert len(old)==len(replacement) and data.count(old)==1,original
        data=data.replace(old,replacement)
    data[12:32]=hashlib.sha1(data[32:]).digest()
    struct.pack_into('<I',data,8,zlib.adler32(data[12:])&0xffffffff)
    return bytes(data)

def transform(candidate,zipalign,output,signer):
    assert signer==EXPECTED_SIGNER,'Unrecognized release certificate'
    assert not output.exists(),'Refusing to overwrite an APK'
    assert zipalign.is_file(),'Android SDK zipalign is required'
    raw=output.with_suffix('.unaligned.tmp.apk')
    assert not raw.exists()
    with zipfile.ZipFile(candidate) as old:
        meta=json.loads(old.read('assets/rerac-build.json'))
        assert meta['package_id']==OLD_PACKAGE
        assert meta['native_source_rebuild'] is True
        assert int(meta['version_code'])>=20
        assert meta['core_version'].count('.')==2 and meta['core_version']!='0.3.4'
        native={n:hashlib.sha256(old.read(n)).hexdigest() for n in old.namelist() if n.startswith('lib/') and n.endswith('.so')}
        assert len(native)==4
        meta.update(package_id=DEV_PACKAGE,signer_sha256=signer,update_feed_branch=DEV_BRANCH,parallel_install=True)
        with zipfile.ZipFile(raw,'w',allowZip64=True) as new:
            names=set()
            for entry in old.infolist():
                name=entry.filename
                if name.startswith('META-INF/') and name.upper().endswith(('.RSA','.SF','.MF','.DSA','.EC')):continue
                assert name not in names
                names.add(name)
                body=old.read(entry)
                if name=='AndroidManifest.xml':body=manifest(body,meta['core_version'])
                elif name=='classes6.dex':body=dex(body,signer)
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
    a=p.parse_args()
    print(json.dumps(transform(a.candidate,a.zipalign,a.output,a.signer),indent=2))

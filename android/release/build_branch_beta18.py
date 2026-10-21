#!/usr/bin/env python3
"""Updater-only rebuild from the verified same-key Actions upgrade fixture.

The input fixture has the same native engine and release signing certificate.
Its checksum, package, native provenance and signature must match exactly.
"""
import argparse, copy, hashlib, json, struct, subprocess, zipfile
from pathlib import Path
from parallel_package import source_built_dex, EXPECTED_SIGNER
from apk_format import axml_chunks, string_pool, make_string_pool, manifest_values

BASE_SHA='c26544740a309ea54a36e129f503080e266047a3a62ce265670bae06b85935c2'
NATIVE_SHA='0ab6c9ca4be8d2f8f671a457c069a5f181e6d9da'

def sha(data):return hashlib.sha256(data).hexdigest()

def manifest(data,version,code):
    parts=axml_chunks(data)
    index=next(i for i,p in enumerate(parts) if struct.unpack_from('<H',p)[0]==1)
    strings,flags=string_pool(parts[index]);assert strings.count('1.1.0-beta.16')==1
    strings=[version if s=='1.1.0-beta.16' else s for s in strings]
    parts[index]=bytearray(make_string_pool(strings,flags));changed=0
    for node in parts:
        if struct.unpack_from('<H',node)[0]!=0x102:continue
        if strings[struct.unpack_from('<I',node,20)[0]]!='manifest':continue
        start,size,count=struct.unpack_from('<HHH',node,24)
        for j in range(count):
            pos=16+start+j*size
            if strings[struct.unpack_from('<I',node,pos+4)[0]]=='versionCode':
                assert struct.unpack_from('<I',node,pos+16)[0]==32
                struct.pack_into('<I',node,pos+16,code);changed+=1
    assert changed==1
    result=struct.pack('<HHI',3,8,8+sum(map(len,parts)))+b''.join(parts)
    expected=copy.deepcopy(manifest_values(data));expected[0]['attrs'].update(versionName=version,versionCode=code)
    assert manifest_values(result)==expected
    return result

def build(baseline,sdk,output):
    assert sha(baseline.read_bytes())==BASE_SHA
    tools=sdk/'build-tools/36.0.0';zipalign=tools/('zipalign.exe' if (tools/'zipalign.exe').exists() else 'zipalign')
    dex=source_built_dex(EXPECTED_SIGNER,zipalign)
    assert b'update-channel3/builds/' in dex and b'UpdateArchive' in dex
    output.mkdir(parents=True,exist_ok=True)
    with zipfile.ZipFile(baseline) as old:
        original=json.loads(old.read('assets/rerac-build.json'))
        assert original['native_source_sha']==NATIVE_SHA and original['signer_sha256']==EXPECTED_SIGNER
        assert original['package_id']=='org.rerac.android.ui035test' and original['core_version']=='0.3.6'
        assert original['version_code']==32 and original['upgrade_test_fixture'] is True
        native={n:sha(old.read(n)) for n in old.namelist() if n.startswith('lib/') and n.endswith('.so')}
        assert len(native)==4 and native==original['native_sha256']
        for fixture in (False,True):
            name='branch-upgrade-fixture-unsigned.apk' if fixture else 'beta18-unsigned.apk'
            metadata=dict(original)
            metadata.update(update_feed_branch='update-channel3',build_delivery='branch-archive-v1')
            version,code=('1.1.0-beta.17',33) if fixture else ('1.1.0-beta.18',34)
            metadata.update(launcher_version=version,version_code=code)
            metadata.pop('upgrade_test_fixture',None)
            if fixture:metadata['upgrade_test_fixture']=True
            raw=output/(name+'.unaligned');target=output/name
            assert not target.exists() and not raw.exists()
            with zipfile.ZipFile(raw,'w') as new:
                for entry in old.infolist():
                    body=old.read(entry)
                    if entry.filename=='classes6.dex':body=dex
                    elif entry.filename=='assets/rerac-build.json':body=(json.dumps(metadata,sort_keys=True,separators=(',',':'))+'\n').encode()
                    elif entry.filename=='AndroidManifest.xml':body=manifest(body,version,code)
                    info=zipfile.ZipInfo(entry.filename,(2026,10,21,0,0,0))
                    info.compress_type=zipfile.ZIP_STORED;info.create_system=0;info.external_attr=0o600<<16
                    new.writestr(info,body)
            subprocess.run([str(zipalign),'-P','16','-f','4',str(raw),str(target)],check=True)
            raw.unlink()
            with zipfile.ZipFile(target) as check:
                assert check.namelist()==old.namelist() and check.testzip() is None
                allowed={'classes6.dex','assets/rerac-build.json','AndroidManifest.xml'}
                for n in old.namelist():
                    if n not in allowed:assert sha(old.read(n))==sha(check.read(n)),n
            report=dict(schema=1,apk=name,sha256=sha(target.read_bytes()),bytes=target.stat().st_size,
                        native_source_sha=NATIVE_SHA,native_sha256=native,fixture=fixture,changed_entries=sorted(allowed))
            target.with_suffix('.json').write_text(json.dumps(report,indent=2)+'\n')
            print(json.dumps(report))

if __name__=='__main__':
    p=argparse.ArgumentParser()
    for n in ('baseline','sdk','output'):p.add_argument('--'+n,type=Path,required=True)
    a=p.parse_args();build(a.baseline,a.sdk,a.output)

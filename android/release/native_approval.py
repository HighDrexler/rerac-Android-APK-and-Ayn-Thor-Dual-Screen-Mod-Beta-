#!/usr/bin/env python3
"""Public signing receipts only; no private release key is stored or used in CI."""
import argparse, json, re, subprocess, zipfile
from pathlib import Path
from public_signature import require, reconstruct, verify

def apply(approval:Path,artifacts:Path,output:Path,tools:Path):
    doc=json.loads(approval.read_text());require(doc.get('schema')==1 and len(doc.get('apks',[]))==2,'Invalid native signature approval')
    require(type(doc.get('candidate_run')) is int and doc['candidate_run']>0,'Missing successful candidate run')
    require(re.fullmatch('[0-9a-f]{40}',doc.get('native_source_sha','')) is not None,'Missing native source revision')
    expected={doc['launcher_unsigned'],'ReRAC-Native-Probe-unsigned.apk'}
    require(re.fullmatch(r'ReRAC-Launcher-1\.1\.0-beta\.[1-9][0-9]*-unsigned\.apk',doc['launcher_unsigned']) is not None,'Unsupported launcher candidate name')
    require({a['unsigned_name'] for a in doc['apks']}==expected,'Approval must contain the exact launcher and separate probe')
    output.mkdir(parents=True,exist_ok=True)
    for a in doc['apks']:
        require(a['signed_name']==a['unsigned_name'].replace('-unsigned.apk','.apk'),'Unsafe signed filename')
        matches=list(artifacts.rglob(a['unsigned_name']));require(len(matches)==1,'Exact candidate artifact required')
        target=output/a['signed_name'];require(not target.exists(),'Refusing to replace an existing signed APK')
        target.write_bytes(reconstruct(matches[0].read_bytes(),a))
        try:
            verify(target,tools)
            with zipfile.ZipFile(target) as z:
                if a['unsigned_name']==doc['launcher_unsigned']:
                    b=json.loads(z.read('assets/rerac-build.json'))
                    require(b['native_source_rebuild'] is True and b['native_source_sha']==doc['native_source_sha'],'Signed engine source differs from approval')
                    require(b['core_version']==doc['core_version'] and b['version_code']==doc['version_code'],'Signed engine version differs from approval')
        except Exception:target.unlink();raise
        print(a['signed_sha256'],target.name)
    (output/'SHA256SUMS.txt').write_text(''.join(a['signed_sha256']+'  '+a['signed_name']+'\n' for a in doc['apks']))
if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__)
    for name in ('approval','artifacts','output','tools'):p.add_argument('--'+name,type=Path,required=True)
    apply(**vars(p.parse_args()))

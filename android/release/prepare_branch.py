"""Apply a public signing receipt, verify it, and prepare a branch-only update."""
import argparse,hashlib,json,re,subprocess,zipfile
from pathlib import Path
from public_signature import reconstruct
from launcher_dex import EXPECTED_SIGNER

def prepare(candidate,output,tools):
    receipt=json.loads(Path('android/release/branch-signatures.json').read_text())
    report=json.loads(Path('builds/v1_1_r2/signing-candidate.json').read_text())
    assert receipt['schema']==1 and receipt['signer_sha256']==EXPECTED_SIGNER and receipt['feed_branch']=='update-channel3'
    assert len(receipt['apks'])==1
    approval=receipt['apks'][0]
    assert (approval['unsigned_name'],approval['signed_name'])==('launcher-unsigned.apk','ReRAC-Android-1.1.apk')
    assert hashlib.sha256(candidate.read_bytes()).hexdigest()==report['archive_sha256']
    with zipfile.ZipFile(candidate) as z:
        assert z.namelist()==['unsigned.apk']
        unsigned=z.read('unsigned.apk')
    assert hashlib.sha256(unsigned).hexdigest()==report['sha256']==approval['unsigned_sha256']
    output.mkdir(parents=True,exist_ok=True);target=output/approval['signed_name']
    target.write_bytes(reconstruct(unsigned,approval))
    result=subprocess.check_output([str(tools/'apksigner'),'verify','--verbose','--print-certs',str(target)],text=True)
    assert re.findall(r'^Signer #\d+ certificate SHA-256 digest: ([0-9a-f]+)$',result,re.M)==[EXPECTED_SIGNER]
    assert 'Verified using v2 scheme (APK Signature Scheme v2): true' in result
    subprocess.run([str(tools/'zipalign'),'-c','-P','16','4',str(target)],check=True)
    manifest=subprocess.check_output([str(tools/'aapt'),'dump','badging',str(target)],text=True)
    assert "package: name='org.rerac.android.ui035test' versionCode='38' versionName='1.1'" in manifest
    assert "application-label:'ReRAC 1.1'" in manifest
    with zipfile.ZipFile(target) as apk:
        meta=json.loads(apk.read('assets/rerac-build.json'))
        assert meta==approval['build'] and meta['version_code']==38 and meta['launcher_version']=='1.1'
        assert meta['signer_sha256']==EXPECTED_SIGNER and meta['update_feed_branch']=='update-channel3'
        assert meta['native_source_sha']==report['native_source_sha']
        assert meta['companion_presentation']=='rac1-native-selection-v1'
        assert {n:hashlib.sha256(apk.read(n)).hexdigest() for n in meta['native_sha256']}==meta['native_sha256']==report['native_sha256']
    (output/'signature.txt').write_text(result)
    folder=Path('builds/v1_1_r2');archive=folder/'ReRAC-Android-1.1.apk.zip'
    with zipfile.ZipFile(archive,'w',compression=zipfile.ZIP_DEFLATED,compresslevel=9) as z:
        info=zipfile.ZipInfo('ReRAC.apk',(2026,10,21,0,0,0));info.compress_type=zipfile.ZIP_DEFLATED
        info.create_system=0;info.external_attr=0o600<<16
        z.writestr(info,target.read_bytes(),compresslevel=9)
    assert archive.stat().st_size<100*1024*1024
    final=dict(schema=1,launcher_version='1.1',version_code=38,core_version='0.3.6',package_id=meta['package_id'],signer_sha256=EXPECTED_SIGNER,
        sha256=approval['signed_sha256'],bytes=approval['signed_bytes'],archive_sha256=hashlib.sha256(archive.read_bytes()).hexdigest(),archive_bytes=archive.stat().st_size,
        apk_url='https://raw.githubusercontent.com/HighDrexler/rerac-Android-APK-and-Ayn-Thor-Dual-Screen-Mod-Beta-/update-channel3/builds/v1_1_r2/'+archive.name,
        native_source_sha=meta['native_source_sha'],native_sha256=meta['native_sha256'])
    (folder/'build.json').write_text(json.dumps(final,indent=2)+'\n')
    (folder/'SHA256SUMS.txt').write_text(final['sha256']+'  ReRAC.apk\n'+final['archive_sha256']+'  '+archive.name+'\n')
    print(json.dumps(final,indent=2))

if __name__=='__main__':
    p=argparse.ArgumentParser()
    for name in ('candidate','output','tools'):p.add_argument('--'+name,type=Path,required=True)
    a=p.parse_args();prepare(a.candidate,a.output,a.tools)

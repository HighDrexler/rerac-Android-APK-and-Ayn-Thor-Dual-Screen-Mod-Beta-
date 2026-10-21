"""Apply public signatures, validate the APKs, and prepare branch-hosted data."""
import hashlib,json,re,subprocess,zipfile
from pathlib import Path
from public_signature import reconstruct

def prepare(unsigned,output,tools):
    receipt=json.loads(Path('android/release/beta19-branch-signatures.json').read_text())
    pin='57188113b5796632a7d4db37622abf7c4d2fb3d9b0bccdc9830b01560ee0f81e'
    assert receipt['schema']==1 and receipt['signer_sha256']==pin and receipt['feed_branch']=='update-channel3'
    assert [(a['unsigned_name'],a['signed_name']) for a in receipt['apks']]==[
        ('beta19-unsigned.apk','ReRAC-Android-beta19.apk')]
    output.mkdir(parents=True,exist_ok=True)
    for i,approval in enumerate(receipt['apks']):
        source=unsigned/approval['unsigned_name'];target=output/approval['signed_name']
        target.write_bytes(reconstruct(source.read_bytes(),approval))
        result=subprocess.check_output([str(tools/'apksigner'),'verify','--verbose','--print-certs',str(target)],text=True)
        assert re.findall(r'^Signer #\d+ certificate SHA-256 digest: ([0-9a-f]+)$',result,re.M)==[pin]
        assert 'Verified using v2 scheme (APK Signature Scheme v2): true' in result
        subprocess.run([str(tools/'zipalign'),'-c','-P','16','4',str(target)],check=True)
        manifest=subprocess.check_output([str(tools/'aapt'),'dump','badging',str(target)],text=True)
        version,code=('1.1.0-beta.19',36)
        assert f"package: name='org.rerac.android.ui035test' versionCode='{code}' versionName='{version}'" in manifest
        with zipfile.ZipFile(target) as apk:
            meta=json.loads(apk.read('assets/rerac-build.json'))
            assert meta==approval['build'] and meta['version_code']==code
            assert meta['signer_sha256']==pin and meta['update_feed_branch']=='update-channel3'
            assert meta['build_delivery']=='branch-archive-v1'
            assert meta['native_source_sha']=='0ab6c9ca4be8d2f8f671a457c069a5f181e6d9da'
            assert {n:hashlib.sha256(apk.read(n)).hexdigest() for n in meta['native_sha256']}==meta['native_sha256']
        (output/(target.name+'.signature.txt')).write_text(result)
    folder=Path('builds/beta19r2');folder.mkdir(parents=True,exist_ok=True)
    archive=folder/'ReRAC-Android-beta19.apk.zip'
    with zipfile.ZipFile(archive,'w',compression=zipfile.ZIP_DEFLATED,compresslevel=9) as z:
        info=zipfile.ZipInfo('ReRAC.apk',(2026,10,21,0,0,0));info.compress_type=zipfile.ZIP_DEFLATED
        info.create_system=0;info.external_attr=0o600<<16
        z.writestr(info,(output/'ReRAC-Android-beta19.apk').read_bytes(),compresslevel=9)
    assert archive.stat().st_size<100*1024*1024
    main=receipt['apks'][0]
    report=dict(schema=1,launcher_version='1.1.0-beta.19',version_code=36,core_version='0.3.6',
        package_id='org.rerac.android.ui035test',signer_sha256=pin,sha256=main['signed_sha256'],bytes=main['signed_bytes'],
        archive_sha256=hashlib.sha256(archive.read_bytes()).hexdigest(),archive_bytes=archive.stat().st_size,
        apk_url='https://raw.githubusercontent.com/HighDrexler/rerac-Android-APK-and-Ayn-Thor-Dual-Screen-Mod-Beta-/update-channel3/builds/beta19r2/'+archive.name,
        native_source_sha=main['build']['native_source_sha'],native_sha256=main['build']['native_sha256'])
    (folder/'build.json').write_text(json.dumps(report,indent=2)+'\n')
    (folder/'SHA256SUMS.txt').write_text(main['signed_sha256']+'  ReRAC.apk\n'+report['archive_sha256']+'  '+archive.name+'\n')
    print(json.dumps(report,indent=2))

if __name__=='__main__':
    import argparse
    p=argparse.ArgumentParser()
    for n in ('unsigned','output','tools'):p.add_argument('--'+n,type=Path,required=True)
    a=p.parse_args();prepare(a.unsigned,a.output,a.tools)

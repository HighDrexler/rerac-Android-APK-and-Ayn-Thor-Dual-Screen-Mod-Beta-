#!/usr/bin/env python3
"""Reconstruct offline-signed public bytes and verify the new update feed."""
import base64, hashlib, json, re, subprocess, zipfile
from pathlib import Path
from cryptography import x509
from cryptography.hazmat.primitives import hashes, serialization
from cryptography.hazmat.primitives.asymmetric import padding
from public_signature import reconstruct

def verify(inputs, output, tools):
    config = json.loads(Path('android/release/release-identity-2026.json').read_text())
    receipt = json.loads(Path('android/release/beta17-new-baseline-receipt.json').read_text())
    assert receipt['schema'] == 1 and receipt['identity'] == config
    assert receipt['stage_commit'] == '87ae8378da9a5394631c397dca2d543c2f0d6818'
    assert receipt['stage_run'] == 39666025011
    assert [a['unsigned_name'] for a in receipt['apks']] == ['beta17-new-baseline-unsigned.apk','upgrade-fixture-unsigned.apk']
    output.mkdir(parents=True,exist_ok=True)
    for number,approval in enumerate(receipt['apks']):
        proof = json.loads((inputs/approval['unsigned_name']).with_suffix('.json').read_text())
        assert proof['identity'] == config and proof['sha256'] == approval['unsigned_sha256']
        target = output/approval['signed_name']
        assert target.name == (config['apk_name'] if number == 0 else 'upgrade-fixture-signed.apk')
        target.write_bytes(reconstruct((inputs/approval['unsigned_name']).read_bytes(),approval))
        signature = subprocess.check_output([str(tools/'apksigner'),'verify','--verbose','--print-certs','--print-certs-pem',str(target)],text=True)
        assert re.findall(r'^Signer #\d+ certificate SHA-256 digest: ([0-9a-f]+)$',signature,re.M) == [config['signer_sha256']]
        assert 'Verified using v2 scheme (APK Signature Scheme v2): true' in signature
        subprocess.run([str(tools/'zipalign'),'-c','-P','16','4',str(target)],check=True)
        manifest = subprocess.check_output([str(tools/'aapt'),'dump','badging',str(target)],text=True)
        code,version = (33,'1.1.0-beta.17') if number == 0 else (32,'1.1.0-beta.16')
        assert f"package: name='{config['package_id']}' versionCode='{code}' versionName='{version}'" in manifest
        with zipfile.ZipFile(target) as apk:
            assert len(apk.namelist()) == len(set(apk.namelist())) and apk.testzip() is None
            meta = json.loads(apk.read('assets/rerac-build.json'))
            assert meta == approval['build']
            assert meta['signer_sha256'] == config['signer_sha256'] and meta['update_feed_branch'] == config['feed_branch']
            assert meta['native_source_sha'] == config['native_source_sha']
            assert meta['upstream_sha'] == '89c87e98f9d8296187df2a6310c67b32144090d3'
            assert meta['core_version'] == '0.3.6' and meta['native_source_rebuild'] is True
            assert {n:hashlib.sha256(apk.read(n)).hexdigest() for n in proof['native_sha256']} == proof['native_sha256'] == meta['native_sha256']
            dex = apk.read('classes6.dex')
            assert config['signer_sha256'].encode() in dex and config['feed_branch'].encode() in dex
            assert b'e2253eb7370c495ef0265efcdbe6bfbe09832070558fe9ed7047e7a2361ad75e' not in dex
        cert = x509.load_pem_x509_certificate(re.search(r'-----BEGIN CERTIFICATE-----.*?-----END CERTIFICATE-----',signature,re.S)[0].encode())
        assert hashlib.sha256(cert.public_bytes(serialization.Encoding.DER)).hexdigest() == config['signer_sha256']
        (output/(target.name+'.signature.txt')).write_text(signature)
    for channel in ('development','stable'):
        text = receipt['signed_'+channel+'_feed_text']; env = json.loads(text)
        assert env['algorithm'] == 'SHA256withRSA'
        payload = base64.b64decode(env['payload'],validate=True)
        cert.public_key().verify(base64.b64decode(env['signature'],validate=True),payload,padding.PKCS1v15(),hashes.SHA256())
        feed = json.loads(payload)
        assert feed['schema'] == 1 and feed['channel'] == channel
        if channel == 'stable': assert feed['release'] is None
        else:
            release = feed['release']; signed = receipt['apks'][0]
            assert release['signer_sha256'] == config['signer_sha256']
            assert release['package_id'] == config['package_id'] and release['version_code'] == 33
            assert release['launcher_version'] == config['launcher_version'] and release['core_version'] == '0.3.6'
            assert release['sha256'] == signed['signed_sha256'] and release['bytes'] == signed['signed_bytes']
            assert release['apk_url'] == 'https://github.com/HighDrexler/rerac-Android-APK-and-Ayn-Thor-Dual-Screen-Mod-Beta-/releases/download/'+config['release_tag']+'/'+config['apk_name']
        (output/(channel+'.json')).write_text(text)
    (output/'SHA256SUMS.txt').write_text(receipt['apks'][0]['signed_sha256']+'  '+config['apk_name']+'\n')
    print('PASS: exact APK signatures, native payload, manifest and signed feeds')

if __name__ == '__main__':
    import argparse
    p=argparse.ArgumentParser()
    for n in ('inputs','output','tools'):p.add_argument('--'+n,type=Path,required=True)
    a=p.parse_args();verify(a.inputs,a.output,a.tools)

#!/usr/bin/env python3
"""Sign one exact-hash ReRAC 0.3.5+ Development update manifest.
The key is loaded only at signing time, never committed or uploaded as an artifact.
"""
import argparse,base64,hashlib,json,os,re,zipfile
from pathlib import Path
from cryptography.hazmat.primitives import hashes,serialization
from cryptography.hazmat.primitives.asymmetric import padding
from cryptography.hazmat.primitives.serialization import pkcs12

EXPECTED_CERT='e2253eb7370c495ef0265efcdbe6bfbe09832070558fe9ed7047e7a2361ad75e'
APP='org.rerac.android.ui035test'
REPO='HighDrexler/rerac-Android-APK-and-Ayn-Thor-Dual-Screen-Mod-Beta-'

def emit(apk:Path,keystore:Path,password:str,tag:str,output:Path):
    if not re.fullmatch(r'rerac-android-[0-9]+\.[0-9]+\.[0-9]+-dev\.[0-9]+',tag):raise ValueError('Invalid unique Development release tag')
    if not re.fullmatch(r'[A-Za-z0-9][A-Za-z0-9._-]{0,127}\.apk',apk.name):raise ValueError('Invalid release APK name')
    private,cert,_=pkcs12.load_key_and_certificates(keystore.read_bytes(),password.encode())
    if private is None or cert is None:raise ValueError('The signing key is missing')
    digest=hashlib.sha256(cert.public_bytes(serialization.Encoding.DER)).hexdigest()
    if digest!=EXPECTED_CERT:raise ValueError('Wrong Android certificate; refusing to stage an incompatible update')
    with zipfile.ZipFile(apk) as archive:
        meta=json.loads(archive.read('assets/rerac-build.json'))
        if meta['package_id']!=APP or meta['signer_sha256']!=digest or meta['native_source_rebuild'] is not True:
            raise ValueError('The APK is not an approved source-built 0.3.5+ Android package')
        if int(meta['version_code'])<20:raise ValueError('Version code is not newer than initial beta.4')
        if meta['core_version']=='0.3.4':raise ValueError('Old v16 core')
    version=tag.split('-')[2]
    if version!=meta['core_version']:raise ValueError('Tag does not match the actual native core')
    release=dict(package_id=APP,signer_sha256=digest,
        launcher_version=meta['launcher_version'],core_version=meta['core_version'],
        version_code=meta['version_code'],bytes=apk.stat().st_size,min_sdk=26,
        abis=['arm64-v8a','x86_64'],sha256=hashlib.sha256(apk.read_bytes()).hexdigest(),
        apk_url=f'https://github.com/{REPO}/releases/download/{tag}/{apk.name}',
        notes='Source-built ReRAC Android '+meta['core_version']+' Development update. Saves and settings are retained when installed over the same app.')
    payload=json.dumps({'schema':1,'channel':'development','release':release},separators=(',',':')).encode('utf-8')
    signature=private.sign(payload,padding.PKCS1v15(),hashes.SHA256())
    cert.public_key().verify(signature,payload,padding.PKCS1v15(),hashes.SHA256())
    envelope={'algorithm':'SHA256withRSA','payload':base64.b64encode(payload).decode(),
        'signature':base64.b64encode(signature).decode()}
    output.write_text(json.dumps(envelope,indent=2)+'\n')
    return {'core_version':meta['core_version'],'launcher_version':meta['launcher_version'],
        'version_code':meta['version_code'],'sha256':release['sha256'],'bytes':release['bytes'],
        'feed':str(output),'apk_url':release['apk_url']}
if __name__=='__main__':
    parser=argparse.ArgumentParser()
    for name in ('apk','keystore','output'):parser.add_argument('--'+name,type=Path,required=True)
    parser.add_argument('--password-env',default='RERAC_KEY_PASSWORD')
    parser.add_argument('--release-tag',required=True)
    args=parser.parse_args()
    pw=os.environ.get(args.password_env)
    if not pw:parser.error('Missing private keystore password environment variable')
    print(json.dumps(emit(args.apk,args.keystore,pw,args.release_tag,args.output),indent=2))

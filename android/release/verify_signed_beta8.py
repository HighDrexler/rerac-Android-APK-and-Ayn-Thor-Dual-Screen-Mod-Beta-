#!/usr/bin/env python3
"""Reconstruct and verify the exact offline-signed beta.8; no signing keys."""
import argparse,base64,hashlib,json,re,subprocess,zipfile
from pathlib import Path
from public_signature import reconstruct,require
from cryptography import x509
from cryptography.hazmat.primitives import hashes,serialization
from cryptography.hazmat.primitives.asymmetric import padding,rsa

SOURCE_SHA="28b4905bb5f9d3f59ab81db15ac009a82b8b9b83d04c99a47adbfa670865d754"
SOURCE_BYTES=253566507
NATIVE_SOURCE="9fe2fd193a498e6103723d06ae2c4694e6b5326c"
SIGNER="e2253eb7370c495ef0265efcdbe6bfbe09832070558fe9ed7047e7a2361ad75e"
PACKAGE="org.rerac.android.ui035test"
VERSION="1.1.0-beta.8"
CODE=24
TAG="rerac-android-0.3.5-dev.3"
APK="ReRAC-Android-0.3.5-Launcher-1.1-beta8-DEV.apk"
URL="https://github.com/HighDrexler/rerac-Android-APK-and-Ayn-Thor-Dual-Screen-Mod-Beta-/releases/download/"+TAG+"/"+APK
NATIVE={
 "lib/arm64-v8a/librc_engine.so":"fef794f0d697b739ea8235817e46bcf333a548fa1ad682c4c3e1eddfe6cb81ed",
 "lib/arm64-v8a/librc_extract.so":"2c2d849308dfcb167349246d50d4a8526500e678df02ae8a8b6d431a21e39a79",
 "lib/x86_64/librc_engine.so":"6dc04d17b8e734222041f8089adcc4f1ea5be05678ae3c8fbba5c30f3d927812",
 "lib/x86_64/librc_extract.so":"b3ea84cd0ebe49879b542cb3d0a0a1c18a090c36ca318975cb5001615b2feecf"
}
def digest(data):return hashlib.sha256(data).hexdigest()
def sdk_certificate(text):
    blocks=re.findall(r"-----BEGIN CERTIFICATE-----[\\s\\S]*?-----END CERTIFICATE-----",text)
    require(len(blocks)==1,"Expected one SDK-verified signing certificate")
    cert=x509.load_pem_x509_certificate(blocks[0].encode()).public_bytes(serialization.Encoding.DER)
    require(digest(cert)==SIGNER,"SDK certificate pin mismatch")
    public=x509.load_der_x509_certificate(cert).public_key().public_bytes(serialization.Encoding.DER,serialization.PublicFormat.SubjectPublicKeyInfo)
    return cert,public
def feed_check(text,certificate,public_key,signed_sha,signed_bytes):
    envelope=json.loads(text)
    require(envelope.get("algorithm")=="SHA256withRSA","Unexpected feed algorithm")
    body=base64.b64decode(envelope["payload"],validate=True)
    payload=json.loads(body)
    require(payload.get("schema")==1 and payload.get("channel")=="development","Wrong feed channel/schema")
    release=payload.get("release")
    require(isinstance(release,dict),"Missing feed release")
    expected=dict(package_id=PACKAGE,signer_sha256=SIGNER,launcher_version=VERSION,core_version="0.3.5",
        version_code=CODE,bytes=signed_bytes,sha256=signed_sha,min_sdk=26,abis=["arm64-v8a","x86_64"],apk_url=URL)
    for key,value in expected.items():require(release.get(key)==value,"Feed mismatch: "+key)
    require(digest(certificate)==SIGNER,"Feed signer certificate mismatch")
    public=x509.load_der_x509_certificate(certificate).public_key()
    require(isinstance(public,rsa.RSAPublicKey),"Expected RSA signer")
    require(public.public_bytes(serialization.Encoding.DER,serialization.PublicFormat.SubjectPublicKeyInfo)==public_key,"APK certificate/public-key mismatch")
    public.verify(base64.b64decode(envelope["signature"],validate=True),body,padding.PKCS1v15(),hashes.SHA256())
    return payload
def main():
    p=argparse.ArgumentParser(description=__doc__)
    for name in ("unsigned","approval","output","tools"):p.add_argument("--"+name,type=Path,required=True)
    p.add_argument("--approval-sha256",required=True)
    args=p.parse_args()
    raw=args.approval.read_bytes()
    require(re.fullmatch("[0-9a-f]{64}",args.approval_sha256) is not None,"Invalid receipt pin")
    require(digest(raw)==args.approval_sha256,"Public signing receipt hash changed")
    doc=json.loads(raw)
    require(doc.get("schema")==1,"Unexpected public receipt schema")
    a=doc
    require(doc.get("release_tag")==TAG,"Wrong release tag")
    require(a.get("signed_sha256")=="90f188976db46a1b989ffec0ae49f109e70f15dc862ca5e4f82c85820a0ebac8" and a.get("signed_bytes")==253574057,"Wrong signed candidate")
    require(a.get("unsigned_sha256")==SOURCE_SHA,"Wrong unsigned candidate")
    require(a.get("unsigned_name")=="ReRAC-1.1.0-beta.8-production-unsigned.apk","Wrong unsigned name")
    require(a.get("signed_name")==APK,"Wrong release asset name")
    unsigned=args.unsigned.read_bytes()
    require(len(unsigned)==SOURCE_BYTES and digest(unsigned)==SOURCE_SHA,"Unsigned APK provenance mismatch")
    require(re.fullmatch("[0-9a-f]{64}",a.get("signed_sha256","")) is not None,"Invalid signed digest")
    args.output.mkdir(parents=True,exist_ok=True)
    output=args.output/APK
    require(not output.exists(),"Refusing to overwrite APK")
    output.write_bytes(reconstruct(unsigned,a))
    with zipfile.ZipFile(output) as z:
        require(len(z.namelist())==len(set(z.namelist())) and z.testzip() is None,"ZIP integrity failure")
        m=json.loads(z.read("assets/rerac-build.json"))
        expected=dict(package_id=PACKAGE,launcher_version=VERSION,version_code=CODE,core_version="0.3.5",
            signer_sha256=SIGNER,update_feed_branch="update-channel2",native_source_rebuild=True,native_source_sha=NATIVE_SOURCE)
        for key,value in expected.items():require(m.get(key)==value,"APK metadata mismatch: "+key)
        require({n:digest(z.read(n)) for n in z.namelist() if n.startswith("lib/") and n.endswith(".so")}==NATIVE,"Native payload mismatch")
        require(SIGNER.encode() in z.read("classes6.dex") and b"update-channel2" in z.read("classes6.dex"),"Launcher trust pins changed")
    verified=subprocess.check_output([str(args.tools/"apksigner"),"verify","--verbose","--print-certs","--print-certs-pem",str(output)],text=True)
    require("Signer #1 certificate SHA-256 digest: "+SIGNER in verified,"Wrong APK signer")
    require("Verified using v2 scheme (APK Signature Scheme v2): true" in verified,"Missing APK v2 signature")
    (args.output/"apksigner.txt").write_text(verified)
    subprocess.run([str(args.tools/"zipalign"),"-c","-P","16","4",str(output)],check=True)
    badging=subprocess.check_output([str(args.tools/"aapt"),"dump","badging",str(output)],text=True)
    match=re.search(r"^package: name='([^']+)' versionCode='([^']+)' versionName='([^']+)'",badging,re.M)
    require(match is not None and match.groups()==(PACKAGE,str(CODE),VERSION),"Android manifest mismatch")
    (args.output/"aapt.txt").write_text(badging)
    cert,pub=sdk_certificate(verified)
    require(digest(cert)==SIGNER,"Wrong certificate")
    feed=doc["signed_feed_text"]
    feed_check(feed,cert,pub,a["signed_sha256"],a["signed_bytes"])
    (args.output/"beta8-development-signed-feed.json").write_bytes(feed.encode("utf-8"))
    (args.output/"SHA256SUMS.txt").write_text(a["signed_sha256"]+"  "+APK+"\n")
    report=dict(status="SIGNED_APK_AND_FEED_VERIFIED",unsigned_sha256=SOURCE_SHA,
       signed_sha256=a["signed_sha256"],signed_bytes=a["signed_bytes"],receipt_sha256=args.approval_sha256,
       native_source_sha=NATIVE_SOURCE,native_sha256=NATIVE,package=PACKAGE,launcher=VERSION,version_code=CODE,
       signer=SIGNER,sdk_checks=dict(apk_signature="PASSED",zip_alignment_16k="PASSED",apk_manifest="PASSED"),
       physical_gameplay="NOT_RUN",live_upgrade_test="NOT_RUN")
    (args.output/"reconstruction.json").write_text(json.dumps(report,indent=2)+"\n")
    print(json.dumps(report,indent=2))
if __name__=="__main__":main()

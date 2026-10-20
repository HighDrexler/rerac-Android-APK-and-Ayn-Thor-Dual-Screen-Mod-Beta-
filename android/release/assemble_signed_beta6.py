#!/usr/bin/env python3
"""Reconstruct ONE already-signed ReRAC beta.6 APK from a pinned GitHub CI artifact.

The detached recipe contains public release data (modified ZIP records and
an existing APK v2 signature block), NOT a private signing key. This cannot
sign a changed APK. Refuse all mismatched payloads and hashes.
"""
import argparse
import base64
import hashlib
import json
import re
import struct
import zipfile
import zlib
from pathlib import Path

PATCH_B64_SHA256 = "0cf0d1b556ef5c0f65edaa3e9d93c58100c9a0afb0347a115f8b265dcc893560"
SOURCE_SHA256 = "523e52a1ccc6e8104f5413c77063f89d0494ca9cc982f797fcb84ee8a8f89fae"
EXPECTED_SHA256 = "c24280174fe216018c76f2353b441c8465ded1d021d7bb5c22547f154a0e6835"
EXPECTED_BYTES = 253519331
PACKAGE_ID = "org.rerac.android.ui035test"
CERT_DIGEST = "e2253eb7370c495ef0265efcdbe6bfbe09832070558fe9ed7047e7a2361ad75e"
RELEASE_TAG = "rerac-android-0.3.5-dev.1"
ASSET_NAME = "ReRAC-Android-0.3.5-Launcher-1.1-beta6-DEV.apk"

def digest(path):
    with path.open("rb") as f:
        return hashlib.file_digest(f, "sha256").hexdigest()

def source_record(inp,info):
    inp.seek(info.header_offset)
    header = inp.read(30)
    assert header[:4] == b"PK\x03\x04"
    name_len, extra_len = struct.unpack_from("<HH", header, 26)
    head = header + inp.read(name_len + extra_len)
    return head, inp.read(info.compress_size)

def verified_recipe(parts_dir):
    parts = sorted(parts_dir.glob("part-*.txt"))
    assert len(parts) == 5, "Expected exactly five detached recipe chunks"
    encoded = "".join(p.read_text().strip() for p in parts)
    assert re.fullmatch("[A-Za-z0-9+/=]+",encoded)
    assert hashlib.sha256((encoded + "\n").encode()).hexdigest() == PATCH_B64_SHA256, "Recipe hash changed"
    import brotli
    recipe = json.loads(brotli.decompress(base64.b64decode(encoded)))
    assert recipe["schema"] == 2
    assert recipe["src_sha"] == SOURCE_SHA256
    assert recipe["dst_sha"] == EXPECTED_SHA256
    assert recipe["sha256_expected"] == EXPECTED_SHA256
    assert recipe["bytes"] == EXPECTED_BYTES
    assert recipe["release_tag"] == RELEASE_TAG
    assert recipe["asset_name"] == ASSET_NAME
    return recipe

def rebuild(source, output, recipe):
    assert not output.exists(), "Refusing to overwrite an APK"
    assert digest(source) == SOURCE_SHA256, "Not the pinned CI unsigned APK"
    with zipfile.ZipFile(source) as archive, source.open("rb") as src, output.open("wb") as dest:
        infos = sorted(archive.infolist(), key=lambda x: x.header_offset)
        names = [info.filename for info in infos]
        assert names == recipe["order"], "ZIP contents or order changed"
        assert len(names) == len(set(names))
        assert set(recipe["headers"]).issubset(names)
        assert set(recipe["delta"]) == {"AndroidManifest.xml","classes6.dex","assets/rerac-build.json"}
        for info in infos:
            name = info.filename
            header, payload = source_record(src, info)
            if name in recipe["headers"]:
                header = base64.b64decode(recipe["headers"][name])
            if name in recipe["delta"]:
                before = archive.read(name)
                after = bytearray()
                for op in recipe["delta"][name]:
                    if op[0] == "c":
                        start, size = op[1:]
                        assert 0 <= start <= len(before) and 0 <= size <= len(before)-start
                        after.extend(before[start:start+size])
                    elif op[0] == "l":
                        after.extend(base64.b64decode(op[1]))
                    else:
                        raise AssertionError("Unexpected patch opcode")
                # All three original ZIP members used Python/zlib level 6 raw DEFLATE.
                payload = zlib.compress(bytes(after), 6)[2:-4]
            dest.write(header)
            dest.write(payload)
        dest.write(base64.b64decode(recipe["suffix"]))
    assert output.stat().st_size == EXPECTED_BYTES
    assert digest(output) == EXPECTED_SHA256, "Signed APK reconstruction differs"
    with zipfile.ZipFile(output) as archive:
        assert archive.testzip() is None
        metadata=json.loads(archive.read("assets/rerac-build.json"))
        assert metadata["package_id"] == PACKAGE_ID
        assert metadata["core_version"] == "0.3.5"
        assert metadata["launcher_version"] == "1.1.0-beta.6"
        assert metadata["version_code"] == 22
        assert metadata["signer_sha256"] == CERT_DIGEST
        assert metadata["native_source_rebuild"] is True
        for abi in ("arm64-v8a","x86_64"):
            assert "lib/"+abi+"/librc_engine.so" in archive.namelist()
    return metadata

def verify_feed(recipe, output):
    env_text = recipe["signed_feed_text"]
    envelope=json.loads(env_text)
    assert envelope["algorithm"] == "SHA256withRSA"
    payload=base64.b64decode(envelope["payload"])
    data=json.loads(payload)
    assert data["schema"] == 1 and data["channel"] == "development"
    release=data["release"]
    assert release["package_id"] == PACKAGE_ID
    assert release["version_code"] == 22
    assert release["launcher_version"] == "1.1.0-beta.6"
    assert release["core_version"] == "0.3.5"
    assert release["bytes"] == EXPECTED_BYTES
    assert release["sha256"] == EXPECTED_SHA256
    assert release["signer_sha256"] == CERT_DIGEST
    assert release["apk_url"] == (
        "https://github.com/HighDrexler/"
        "rerac-Android-APK-and-Ayn-Thor-Dual-Screen-Mod-Beta-/"
        "releases/download/"+RELEASE_TAG+"/"+ASSET_NAME
    )
    # Extract the X.509 certificate from the already-signed APK v2 block.
    from cryptography import x509
    from cryptography.hazmat.primitives import hashes, serialization
    from cryptography.hazmat.primitives.asymmetric import padding
    def read_lp(buf, at):
        size=struct.unpack_from("<I",buf,at)[0]
        assert at+4+size <= len(buf)
        return buf[at+4:at+4+size],at+4+size
    with output.open("rb") as f:
        f.seek(-22,2)
        eocd=f.read(22)
        assert eocd[:4] == b"PK\x05\x06"
        cd=struct.unpack_from("<I",eocd,16)[0]
        f.seek(cd-24)
        footer=f.read(24)
        assert footer[8:] == b"APK Sig Block 42"
        block_size=struct.unpack_from("<Q",footer)[0]
        f.seek(cd-block_size-8)
        block=f.read(block_size+8)
    assert struct.unpack_from("<Q",block,0)[0] == block_size
    pos=8
    v2=None
    while pos < len(block)-24:
        length=struct.unpack_from("<Q",block,pos)[0]
        pos+=8
        id=struct.unpack_from("<I",block,pos)[0]
        if id==0x7109871a: v2=block[pos+4:pos+length]
        pos+=length
    assert v2 is not None and pos==len(block)-24
    signer_list,_=read_lp(v2,0)
    signer,_=read_lp(signer_list,0)
    signed,off=read_lp(signer,0)
    sigs,off=read_lp(signer,off)
    spki,off=read_lp(signer,off)
    cert_records,j=read_lp(signed,4+struct.unpack_from("<I",signed,0)[0])
    cert_der,_=read_lp(cert_records,0)
    assert hashlib.sha256(cert_der).hexdigest()==CERT_DIGEST
    certificate=x509.load_der_x509_certificate(cert_der)
    public=certificate.public_key()
    signature=base64.b64decode(envelope["signature"])
    public.verify(signature,payload,padding.PKCS1v15(),hashes.SHA256())
    output_feed=output.parent/"beta6-development-signed-feed.json"
    output_feed.write_text(env_text)
    return output_feed

if __name__=="__main__":
    parser=argparse.ArgumentParser()
    parser.add_argument("--source",type=Path,required=True)
    parser.add_argument("--parts",type=Path,required=True)
    parser.add_argument("--output",type=Path,required=True)
    args=parser.parse_args()
    recipe=verified_recipe(args.parts)
    meta=rebuild(args.source,args.output,recipe)
    verified_feed=verify_feed(recipe,args.output)
    print(json.dumps({"status":"VERIFIED","signed_apk_sha256":EXPECTED_SHA256,
          "apk":str(args.output),"feed":str(verified_feed),
          "launcher_version":meta["launcher_version"],"core_version":meta["core_version"]},indent=2))

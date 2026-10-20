#!/usr/bin/env python3
"""Reconstruct the exact already-signed beta.7 APK; never create a signature.

Requires the pinned unsigned beta.7 candidate, published beta.6 APK and five
public schema-3 fragments. The beta.6 assembler and release inputs are immutable.
Use --apksigner and --zipalign for Android SDK verification. Their absence is
reported as NOT_RUN, never as a successful APK signature/alignment check.
"""
import argparse
import base64
import hashlib
import json
import re
import struct
import subprocess
import zipfile
import zlib
from pathlib import Path

PATCH_B64_SHA256 = "097c6b5ddc329da23af7b216957ceadd28d98e8d06cd7b05f62359d9f272b653"
SOURCE_SHA256 = "28af286d44c28a3a73b45b1a6be15cfa0c91b003aa995f1c54dd5409ed775beb"
PREVIOUS_SHA256 = "c24280174fe216018c76f2353b441c8465ded1d021d7bb5c22547f154a0e6835"
EXPECTED_SHA256 = "e64cc4bde9b2d65a2684de868282903e9b1f71de7c4f2daef1219ceddc04bfb6"
EXPECTED_BYTES = 253519332
PACKAGE_ID = "org.rerac.android.ui035test"
CERT_DIGEST = "e2253eb7370c495ef0265efcdbe6bfbe09832070558fe9ed7047e7a2361ad75e"
LAUNCHER_VERSION = "1.1.0-beta.7"
VERSION_CODE = 23
RELEASE_TAG = "rerac-android-0.3.5-dev.2"
ASSET_NAME = "ReRAC-Android-0.3.5-Launcher-1.1-beta7-DEV.apk"
REPO_URL = "https://github.com/HighDrexler/rerac-Android-APK-and-Ayn-Thor-Dual-Screen-Mod-Beta-"
DELTA_MEMBERS = {"AndroidManifest.xml", "assets/rerac-build.json"}
REUSE_MEMBERS = ["classes6.dex"]
NATIVE_MEMBERS = {
    "lib/" + abi + "/" + name
    for abi in ("arm64-v8a", "x86_64")
    for name in ("librc_engine.so", "librc_extract.so")
}


class VerificationError(ValueError):
    pass


def require(condition, message):
    # These checks must remain active under python -O.
    if not condition:
        raise VerificationError(message)


def digest(path):
    value = hashlib.sha256()
    with Path(path).open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            value.update(chunk)
    return value.hexdigest()


def b64(value):
    try:
        return base64.b64decode(value, validate=True)
    except (ValueError, TypeError) as exc:
        raise VerificationError("Invalid base64 in detached recipe") from exc


def validate_recipe(recipe):
    expected = {
        "schema": 3,
        "src_sha": SOURCE_SHA256,
        "previous_sha": PREVIOUS_SHA256,
        "dst_sha": EXPECTED_SHA256,
        "sha256_expected": EXPECTED_SHA256,
        "bytes": EXPECTED_BYTES,
        "release_tag": RELEASE_TAG,
        "asset_name": ASSET_NAME,
    }
    require(isinstance(recipe, dict), "Recipe must be an object")
    for key, value in expected.items():
        require(recipe.get(key) == value, "Recipe mismatch: " + key)
    order = recipe.get("order")
    require(isinstance(order, list) and all(isinstance(n, str) for n in order), "Invalid ZIP order")
    require(len(order) == len(set(order)), "Duplicate ZIP members in recipe")
    require(isinstance(recipe.get("headers"), dict), "Invalid replacement headers")
    require(isinstance(recipe.get("delta"), dict), "Invalid delta map")
    require(set(recipe["delta"]) == DELTA_MEMBERS, "Unexpected delta members")
    require(recipe.get("reuse_previous") == REUSE_MEMBERS, "Unexpected previous-APK reuse")
    require(set(recipe["headers"]) == DELTA_MEMBERS | set(REUSE_MEMBERS) | NATIVE_MEMBERS,
            "Unexpected replacement header members")
    require(set(recipe["headers"]).issubset(order), "Replacement member absent from ZIP order")


def verified_recipe(parts_dir):
    parts_dir = Path(parts_dir)
    names = ["part-%02d.txt" % n for n in range(5)]
    require(sorted(p.name for p in parts_dir.glob("part-*.txt")) == names,
            "Expected exactly part-00.txt through part-04.txt")
    encoded = "".join((parts_dir / name).read_text(encoding="ascii").strip() for name in names)
    require(hashlib.sha256((encoded + "\n").encode("ascii")).hexdigest() == PATCH_B64_SHA256,
            "Recipe hash changed")
    compressed = b64(encoded)
    try:
        import brotli
    except ImportError as exc:
        raise VerificationError("Brotli is required to read the pinned recipe") from exc
    try:
        recipe = json.loads(brotli.decompress(compressed))
    except Exception as exc:
        raise VerificationError("Invalid compressed recipe") from exc
    validate_recipe(recipe)
    return recipe


def local_header(header, name):
    require(len(header) >= 30 and header[:4] == b"PK\x03\x04", "Invalid local ZIP header")
    flags, method = struct.unpack_from("<HH", header, 6)
    crc, compressed, plain = struct.unpack_from("<III", header, 14)
    name_len, extra_len = struct.unpack_from("<HH", header, 26)
    require(len(header) == 30 + name_len + extra_len, "Local ZIP header length changed")
    require(flags & 9 == 0, "Encrypted/descriptor ZIP records are unsupported")
    require(method in (zipfile.ZIP_STORED, zipfile.ZIP_DEFLATED), "Unexpected ZIP compression")
    raw_name = header[30:30 + name_len]
    require(raw_name.decode("utf-8" if flags & 0x800 else "cp437") == name, "ZIP member name changed")
    return method, crc, compressed, plain


def source_record(stream, info):
    stream.seek(info.header_offset)
    fixed = stream.read(30)
    require(len(fixed) == 30, "Truncated ZIP header")
    name_len, extra_len = struct.unpack_from("<HH", fixed, 26)
    header = fixed + stream.read(name_len + extra_len)
    method, crc, compressed, plain = local_header(header, info.filename)
    require((method, crc, compressed, plain) ==
            (info.compress_type, info.CRC, info.compress_size, info.file_size),
            "Local/central ZIP record mismatch")
    payload = stream.read(compressed)
    require(len(payload) == compressed, "Truncated compressed payload")
    return header, payload


def apply_delta(before, operations):
    after = bytearray()
    require(isinstance(operations, list), "Invalid delta operations")
    for op in operations:
        require(isinstance(op, list) and op, "Invalid delta operation")
        if op[0] == "c":
            require(len(op) == 3 and type(op[1]) is int and type(op[2]) is int,
                    "Invalid copy operation")
            start, size = op[1:]
            require(0 <= start <= len(before) and 0 <= size <= len(before) - start,
                    "Copy operation outside source member")
            after.extend(before[start:start + size])
        elif op[0] == "l":
            require(len(op) == 2 and isinstance(op[1], str), "Invalid literal operation")
            after.extend(b64(op[1]))
        else:
            raise VerificationError("Unexpected patch opcode")
    return bytes(after)


def verify_output(output, recipe):
    validate_recipe(recipe)
    require(output.stat().st_size == EXPECTED_BYTES, "Signed APK size differs")
    require(digest(output) == EXPECTED_SHA256, "Signed APK reconstruction differs")
    with zipfile.ZipFile(output) as archive:
        require(archive.namelist() == recipe["order"], "Output ZIP order/content changed")
        require(archive.testzip() is None, "Output ZIP CRC check failed")
        metadata = json.loads(archive.read("assets/rerac-build.json"))
        expected = {
            "package_id": PACKAGE_ID, "core_version": "0.3.5",
            "launcher_version": LAUNCHER_VERSION, "version_code": VERSION_CODE,
            "signer_sha256": CERT_DIGEST, "native_source_rebuild": True,
        }
        for key, value in expected.items():
            require(metadata.get(key) == value, "APK metadata mismatch: " + key)
        require(NATIVE_MEMBERS.issubset(archive.namelist()), "Native ABI library missing")
    return metadata


def rebuild(source, previous, output, recipe):
    source, previous, output = Path(source), Path(previous), Path(output)
    validate_recipe(recipe)
    require(not output.exists(), "Refusing to overwrite an APK")
    require(digest(source) == SOURCE_SHA256, "Not the pinned beta.7 unsigned APK")
    require(digest(previous) == PREVIOUS_SHA256, "Not the pinned published beta.6 APK")
    created = False
    try:
        with zipfile.ZipFile(source) as archive, zipfile.ZipFile(previous) as old:
            infos = sorted(archive.infolist(), key=lambda info: info.header_offset)
            names = [info.filename for info in infos]
            require(names == recipe["order"] and len(names) == len(set(names)),
                    "Source ZIP contents or order changed")
            old_names = old.namelist()
            require(len(old_names) == len(set(old_names)), "Duplicate previous-APK member")
            require(set(REUSE_MEMBERS).issubset(old_names), "Previous APK lacks reused member")
            with source.open("rb") as src, previous.open("rb") as prev, output.open("xb") as dest:
                created = True
                for info in infos:
                    name = info.filename
                    header, payload = source_record(src, info)
                    if name in REUSE_MEMBERS:
                        # Preserve the published compressed bytes, not a recompression.
                        old_header, payload = source_record(prev, old.getinfo(name))
                        old_description = local_header(old_header, name)
                    if name in recipe["headers"]:
                        header = b64(recipe["headers"][name])
                    method, crc, compressed, plain = local_header(header, name)
                    if name in REUSE_MEMBERS:
                        require((method, crc, compressed, plain) == old_description,
                                "Reused member disagrees with approved header")
                    if name in recipe["delta"]:
                        after = apply_delta(archive.read(name), recipe["delta"][name])
                        require(method == zipfile.ZIP_DEFLATED, "Delta member must use DEFLATE")
                        encoder = zlib.compressobj(6, zlib.DEFLATED, -15)
                        payload = encoder.compress(after) + encoder.flush()
                        require((zlib.crc32(after) & 0xffffffff, len(after)) == (crc, plain),
                                "Delta member CRC/size differs from approved header")
                    require(len(payload) == compressed, "Compressed member length changed: " + name)
                    dest.write(header)
                    dest.write(payload)
                dest.write(b64(recipe["suffix"]))
        return verify_output(output, recipe)
    except BaseException:
        if created:
            output.unlink()
        raise


def read_lp(buffer, offset):
    require(0 <= offset <= len(buffer) - 4, "Truncated length-prefixed field")
    size = struct.unpack_from("<I", buffer, offset)[0]
    end = offset + 4 + size
    require(end <= len(buffer), "Length-prefixed field exceeds its parent")
    return buffer[offset + 4:end], end


def certificate_from_block(block):
    require(len(block) >= 32 and block[-16:] == b"APK Sig Block 42", "Invalid APK signing block")
    size = len(block) - 8
    require(struct.unpack_from("<Q", block, 0)[0] == size and
            struct.unpack_from("<Q", block, len(block) - 24)[0] == size,
            "Signing block sizes differ")
    position, limit, v2 = 8, len(block) - 24, None
    while position < limit:
        require(position + 8 <= limit, "Truncated signing block record")
        length = struct.unpack_from("<Q", block, position)[0]
        position += 8
        require(4 <= length <= limit - position, "Invalid signing block record length")
        ident = struct.unpack_from("<I", block, position)[0]
        if ident == 0x7109871a:
            require(v2 is None, "Duplicate v2 signature block")
            v2 = block[position + 4:position + length]
        position += length
    require(position == limit and v2 is not None, "Missing v2 signature block")
    signers, end = read_lp(v2, 0)
    require(end == len(v2), "Unexpected data after signer list")
    signer, end = read_lp(signers, 0)
    require(end == len(signers), "Expected one APK signer")
    signed, offset = read_lp(signer, 0)
    _, offset = read_lp(signer, offset)  # APK signatures are checked by apksigner.
    public_key, offset = read_lp(signer, offset)
    require(offset == len(signer), "Unexpected data after APK signer")
    _, offset = read_lp(signed, 0)  # Content digests.
    certificates, offset = read_lp(signed, offset)
    _, offset = read_lp(signed, offset)  # Additional attributes.
    require(offset == len(signed), "Unexpected signed APK fields")
    certificate, end = read_lp(certificates, 0)
    require(end == len(certificates), "Expected one signing certificate")
    require(hashlib.sha256(certificate).hexdigest() == CERT_DIGEST, "APK signer changed")
    return certificate, public_key


def apk_certificate(output):
    with output.open("rb") as stream:
        size = output.stat().st_size
        require(size >= 46, "Truncated APK")
        stream.seek(size - 22)
        eocd = stream.read(22)
        require(eocd[:4] == b"PK\x05\x06" and eocd[20:] == b"\0\0", "Unexpected APK EOCD")
        cd_size, cd = struct.unpack_from("<II", eocd, 12)
        require(cd >= 24 and cd + cd_size == size - 22, "Invalid central directory")
        stream.seek(cd - 24)
        footer = stream.read(24)
        require(footer[8:] == b"APK Sig Block 42", "No APK signing block")
        block_size = struct.unpack_from("<Q", footer)[0]
        require(24 <= block_size <= min(cd - 8, 16 * 1024 * 1024), "Invalid signing block size")
        stream.seek(cd - block_size - 8)
        return certificate_from_block(stream.read(block_size + 8))


def verify_signed_feed(env_text, certificate, public_key):
    envelope = json.loads(env_text)
    require(envelope.get("algorithm") == "SHA256withRSA", "Unexpected feed algorithm")
    payload = b64(envelope["payload"])
    data = json.loads(payload)
    require(data.get("schema") == 1 and data.get("channel") == "development", "Wrong feed channel/schema")
    release = data.get("release")
    require(isinstance(release, dict), "Missing feed release")
    expected = {
        "package_id": PACKAGE_ID, "signer_sha256": CERT_DIGEST,
        "launcher_version": LAUNCHER_VERSION, "core_version": "0.3.5",
        "version_code": VERSION_CODE, "bytes": EXPECTED_BYTES,
        "sha256": EXPECTED_SHA256, "min_sdk": 26, "abis": ["arm64-v8a", "x86_64"],
        "apk_url": REPO_URL + "/releases/download/" + RELEASE_TAG + "/" + ASSET_NAME,
    }
    for key, value in expected.items():
        require(release.get(key) == value, "Feed release mismatch: " + key)
    require(hashlib.sha256(certificate).hexdigest() == CERT_DIGEST, "Feed verification signer changed")
    try:
        from cryptography import x509
        from cryptography.hazmat.primitives import hashes, serialization
        from cryptography.hazmat.primitives.asymmetric import padding, rsa
    except ImportError as exc:
        raise VerificationError("cryptography is required to verify the signed feed") from exc
    public = x509.load_der_x509_certificate(certificate).public_key()
    require(isinstance(public, rsa.RSAPublicKey), "Expected RSA release certificate")
    require(public.public_bytes(serialization.Encoding.DER, serialization.PublicFormat.SubjectPublicKeyInfo)
            == public_key, "APK public key/certificate mismatch")
    try:
        public.verify(b64(envelope["signature"]), payload, padding.PKCS1v15(), hashes.SHA256())
    except Exception as exc:
        raise VerificationError("Feed signature verification failed") from exc
    return data


def verify_sdk(output, apksigner=None, zipalign=None, aapt=None):
    result = {"apk_signature": "NOT_RUN", "zip_alignment_16k": "NOT_RUN", "apk_manifest": "NOT_RUN"}
    if apksigner:
        checked = subprocess.run([str(apksigner), "verify", "--verbose", "--print-certs", str(output)],
                                 check=True, capture_output=True, text=True)
        require("Signer #1 certificate SHA-256 digest: " + CERT_DIGEST in checked.stdout,
                "apksigner certificate digest mismatch")
        result["apk_signature"] = "PASSED"
    if zipalign:
        subprocess.run([str(zipalign), "-c", "-P", "16", "4", str(output)], check=True)
        result["zip_alignment_16k"] = "PASSED"
    if aapt:
        checked = subprocess.run([str(aapt), "dump", "badging", str(output)],
                                 check=True, capture_output=True, text=True)
        package = re.search(r"^package: name='([^']+)' versionCode='([^']+)' versionName='([^']+)'",
                            checked.stdout, re.MULTILINE)
        require(package is not None and package.groups() ==
                (PACKAGE_ID, str(VERSION_CODE), LAUNCHER_VERSION), "Binary APK manifest mismatch")
        result["apk_manifest"] = "PASSED"
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", type=Path, required=True)
    parser.add_argument("--previous", type=Path, required=True)
    parser.add_argument("--parts", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--apksigner", type=Path)
    parser.add_argument("--zipalign", type=Path)
    parser.add_argument("--aapt", type=Path)
    args = parser.parse_args()
    recipe = verified_recipe(args.parts)
    metadata = rebuild(args.source, args.previous, args.output, recipe)
    certificate, public_key = apk_certificate(args.output)
    verify_signed_feed(recipe["signed_feed_text"], certificate, public_key)
    checks = verify_sdk(args.output, args.apksigner, args.zipalign, args.aapt)
    feed = args.output.parent / "beta7-development-signed-feed.json"
    # Preserve the approved UTF-8 bytes; refuse replacement of an existing feed.
    with feed.open("xb") as stream:
        stream.write(recipe["signed_feed_text"].encode("utf-8"))
    print(json.dumps({
        "status": "RECONSTRUCTED_HASH_AND_FEED_VERIFIED",
        "apk": str(args.output), "feed": str(feed), "sha256": EXPECTED_SHA256,
        "bytes": EXPECTED_BYTES, "launcher_version": metadata["launcher_version"],
        "core_version": metadata["core_version"], "sdk_checks": checks,
        "upgrade_test": "NOT_RUN", "physical_gameplay": "NOT_RUN",
    }, indent=2))


if __name__ == "__main__":
    main()

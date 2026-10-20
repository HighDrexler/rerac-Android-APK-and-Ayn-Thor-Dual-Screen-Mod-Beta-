"""Host-only reconstruction regressions. No APK installation or private keys.

Run from a complete checkout:
python3 -m unittest discover -s android/release/tests -p test_assemble_signed_beta7.py -v

The small ZIP fixtures exercise reconstruction mechanics with test-local hash
pins. Real-input integration remains a separate check against the pinned APKs.
The public recipe/feed tests require existing brotli/cryptography dependencies;
missing dependencies are explicit skips, not evidence of signature verification.
"""
import base64
import copy
import hashlib
import importlib.util
import json
import shutil
import struct
import subprocess
import tempfile
import unittest
import zipfile
from pathlib import Path
from unittest import mock

RELEASE = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location("assemble_signed_beta7", RELEASE / "assemble_signed_beta7.py")
assembler = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(assembler)


def encoded(data):
    return base64.b64encode(data).decode("ascii")


def write_zip(path, members):
    with zipfile.ZipFile(path, "w") as archive:
        for name, content in members.items():
            info = zipfile.ZipInfo(name, (2026, 10, 20, 0, 0, 0))
            info.compress_type = zipfile.ZIP_STORED if name.startswith("lib/") else zipfile.ZIP_DEFLATED
            archive.writestr(info, content, compresslevel=6)


def raw_record(path, name):
    # Inspect the fixture's local record independently of source_record().
    with zipfile.ZipFile(path) as archive, path.open("rb") as stream:
        info = archive.getinfo(name)
        stream.seek(info.header_offset)
        header = stream.read(30)
        name_len, extra_len = struct.unpack_from("<HH", header, 26)
        return header + stream.read(name_len + extra_len), stream.read(info.compress_size)


class ReconstructionTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        root = Path(self.temporary.name)
        self.source, self.previous, self.golden, self.output = (
            root / name for name in ("unsigned.apk", "previous.apk", "golden.apk", "output.apk")
        )
        self.metadata = {
            "package_id": assembler.PACKAGE_ID, "core_version": "0.3.5",
            "launcher_version": assembler.LAUNCHER_VERSION, "version_code": 23,
            "signer_sha256": assembler.CERT_DIGEST, "native_source_rebuild": True,
        }
        before = {name: b"fixture native " + name.encode() for name in sorted(assembler.NATIVE_MEMBERS)}
        before.update({
            "AndroidManifest.xml": b"unsigned manifest",
            "classes6.dex": b"unsigned launcher DEX, deliberately different",
            "assets/rerac-build.json": b'{"unsigned": true}',
        })
        after = dict(before)
        after["AndroidManifest.xml"] = b"approved fixture manifest"
        after["classes6.dex"] = b"published beta.6 compressed DEX fixture" * 30
        after["assets/rerac-build.json"] = json.dumps(self.metadata).encode()
        write_zip(self.source, before)
        write_zip(self.previous, {"classes6.dex": after["classes6.dex"]})
        write_zip(self.golden, after)
        with zipfile.ZipFile(self.golden) as archive:
            suffix = self.golden.read_bytes()[archive.start_dir:]
        self.pins = {
            "SOURCE_SHA256": assembler.digest(self.source),
            "PREVIOUS_SHA256": assembler.digest(self.previous),
            "EXPECTED_SHA256": assembler.digest(self.golden),
            "EXPECTED_BYTES": self.golden.stat().st_size,
        }
        self.addCleanup(mock.patch.stopall)
        mock.patch.multiple(assembler, **self.pins).start()
        self.recipe = {
            "schema": 3, "src_sha": self.pins["SOURCE_SHA256"],
            "previous_sha": self.pins["PREVIOUS_SHA256"],
            "dst_sha": self.pins["EXPECTED_SHA256"], "sha256_expected": self.pins["EXPECTED_SHA256"],
            "bytes": self.pins["EXPECTED_BYTES"], "release_tag": assembler.RELEASE_TAG,
            "asset_name": assembler.ASSET_NAME, "order": list(before),
            "headers": {name: encoded(raw_record(self.golden, name)[0]) for name in before},
            "delta": {name: [["l", encoded(after[name])]] for name in assembler.DELTA_MEMBERS},
            "reuse_previous": ["classes6.dex"], "suffix": encoded(suffix),
        }

    def rebuild(self):
        return assembler.rebuild(self.source, self.previous, self.output, self.recipe)

    def test_schema3_reconstructs_exact_bytes_and_reuses_previous_compressed_dex(self):
        self.assertEqual(self.rebuild(), self.metadata)
        self.assertEqual(self.output.read_bytes(), self.golden.read_bytes())
        self.assertEqual(raw_record(self.output, "classes6.dex")[1],
                         raw_record(self.previous, "classes6.dex")[1])
        self.assertNotEqual(raw_record(self.source, "classes6.dex")[1],
                            raw_record(self.previous, "classes6.dex")[1])

    def test_wrong_unsigned_source_is_rejected_before_output_creation(self):
        self.source.write_bytes(b"wrong unsigned APK")
        with self.assertRaisesRegex(assembler.VerificationError, "pinned beta.7"):
            self.rebuild()
        self.assertFalse(self.output.exists())

    def test_tampered_previous_apk_is_rejected_before_output_creation(self):
        self.previous.write_bytes(self.previous.read_bytes() + b"tampered")
        with self.assertRaisesRegex(assembler.VerificationError, "pinned published beta.6"):
            self.rebuild()
        self.assertFalse(self.output.exists())

    def test_existing_output_is_never_overwritten(self):
        self.output.write_bytes(b"keep existing file")
        with self.assertRaisesRegex(assembler.VerificationError, "overwrite"):
            self.rebuild()
        self.assertEqual(self.output.read_bytes(), b"keep existing file")

    def test_altered_output_is_detected_by_full_hash(self):
        self.rebuild()
        changed = bytearray(self.output.read_bytes())
        changed[40] ^= 1
        self.output.write_bytes(changed)
        with self.assertRaisesRegex(assembler.VerificationError, "reconstruction differs"):
            assembler.verify_output(self.output, self.recipe)

    def test_wrong_final_hash_removes_only_newly_created_output(self):
        with mock.patch.object(assembler, "EXPECTED_SHA256", "0" * 64):
            self.recipe["dst_sha"] = self.recipe["sha256_expected"] = "0" * 64
            with self.assertRaisesRegex(assembler.VerificationError, "reconstruction differs"):
                self.rebuild()
        self.assertFalse(self.output.exists())
        self.assertEqual(assembler.digest(self.source), self.pins["SOURCE_SHA256"])
        self.assertEqual(assembler.digest(self.previous), self.pins["PREVIOUS_SHA256"])

    def test_schema2_and_other_reused_members_are_rejected(self):
        for field, value in (("schema", 2), ("reuse_previous", ["classes.dex"])):
            recipe = copy.deepcopy(self.recipe)
            recipe[field] = value
            with self.subTest(field=field), self.assertRaises(assembler.VerificationError):
                assembler.rebuild(self.source, self.previous, self.output, recipe)
        self.assertFalse(self.output.exists())

    def test_reused_member_must_match_approved_crc_and_size(self):
        header = bytearray(base64.b64decode(self.recipe["headers"]["classes6.dex"]))
        header[14] ^= 1
        self.recipe["headers"]["classes6.dex"] = encoded(header)
        with self.assertRaisesRegex(assembler.VerificationError, "Reused member"):
            self.rebuild()
        self.assertFalse(self.output.exists())

    def test_delta_header_must_match_actual_reconstructed_member(self):
        self.recipe["delta"]["AndroidManifest.xml"] = [["l", encoded(b"unapproved content")]]
        with self.assertRaisesRegex(assembler.VerificationError, "Delta member CRC/size"):
            self.rebuild()
        self.assertFalse(self.output.exists())


class BoundaryTests(unittest.TestCase):
    def test_copy_and_literal_operations(self):
        self.assertEqual(assembler.apply_delta(b"abcdef", [["c", 1, 3], ["l", encoded(b"!")]]), b"bcd!")

    def test_invalid_copy_ranges_and_unknown_opcodes_are_rejected(self):
        for op in (["c", -1, 1], ["c", 0, 7], ["c", 6, 1], ["c", True, 1], ["x"], ["l", "!!"]):
            with self.subTest(op=op), self.assertRaises(assembler.VerificationError):
                assembler.apply_delta(b"abcdef", [op])

    def test_truncated_signing_block_is_rejected(self):
        for block in (b"", b"APK Sig Block 42", b"\0" * 100):
            with self.subTest(length=len(block)), self.assertRaises(assembler.VerificationError):
                assembler.certificate_from_block(block)

    def test_unexecuted_sdk_checks_are_explicit(self):
        with mock.patch.object(assembler.subprocess, "run") as run:
            self.assertEqual(assembler.verify_sdk(Path("not-opened.apk")),
                             {"apk_signature": "NOT_RUN", "zip_alignment_16k": "NOT_RUN",
                              "apk_manifest": "NOT_RUN"})
            run.assert_not_called()

    def test_sdk_failure_cannot_be_reported_as_success(self):
        with mock.patch.object(assembler.subprocess, "run",
                               side_effect=subprocess.CalledProcessError(1, "apksigner")):
            with self.assertRaises(subprocess.CalledProcessError):
                assembler.verify_sdk(Path("candidate.apk"), apksigner=Path("apksigner"))

    def test_successful_sdk_exit_with_wrong_certificate_is_rejected(self):
        result = subprocess.CompletedProcess([], 0, stdout="Signer #1 certificate SHA-256 digest: wrong\n")
        with mock.patch.object(assembler.subprocess, "run", return_value=result):
            with self.assertRaisesRegex(assembler.VerificationError, "certificate digest"):
                assembler.verify_sdk(Path("candidate.apk"), apksigner=Path("apksigner"))

    def test_binary_manifest_cannot_disagree_with_metadata_identity(self):
        result = subprocess.CompletedProcess(
            [], 0, stdout="package: name='org.rerac.android.uipreview' versionCode='23' versionName='1.1.0-beta.7'\n"
        )
        with mock.patch.object(assembler.subprocess, "run", return_value=result):
            with self.assertRaisesRegex(assembler.VerificationError, "Binary APK manifest mismatch"):
                assembler.verify_sdk(Path("candidate.apk"), aapt=Path("aapt"))


class PublicRecipeTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.parts = Path(self.temporary.name) / "parts"
        shutil.copytree(RELEASE / "beta7-detached", self.parts)

    def test_omitted_fragment_is_rejected(self):
        (self.parts / "part-02.txt").unlink()
        with self.assertRaisesRegex(assembler.VerificationError, "exactly part-00"):
            assembler.verified_recipe(self.parts)

    def test_corrupted_fragment_is_rejected_before_decompression(self):
        part = self.parts / "part-02.txt"
        part.write_text(part.read_text().strip() + "A", encoding="ascii")
        with self.assertRaisesRegex(assembler.VerificationError, "Recipe hash changed"):
            assembler.verified_recipe(self.parts)

    def test_misnamed_fragment_is_rejected(self):
        (self.parts / "part-04.txt").rename(self.parts / "part-05.txt")
        with self.assertRaisesRegex(assembler.VerificationError, "exactly part-00"):
            assembler.verified_recipe(self.parts)

    def actual_recipe(self):
        if importlib.util.find_spec("brotli") is None:
            self.skipTest("brotli unavailable: real recipe decoding NOT_RUN")
        return assembler.verified_recipe(self.parts)

    def test_pinned_real_recipe_and_public_certificate(self):
        recipe = self.actual_recipe()
        self.assertEqual(recipe["reuse_previous"], ["classes6.dex"])
        suffix = base64.b64decode(recipe["suffix"])
        length = struct.unpack_from("<Q", suffix)[0] + 8
        certificate, public_key = assembler.certificate_from_block(suffix[:length])
        self.assertEqual(hashlib.sha256(certificate).hexdigest(), assembler.CERT_DIGEST)
        self.assertTrue(public_key)

    def test_real_feed_signature_and_tampering(self):
        recipe = self.actual_recipe()
        if importlib.util.find_spec("cryptography") is None:
            self.skipTest("cryptography unavailable: real feed signature NOT_RUN")
        suffix = base64.b64decode(recipe["suffix"])
        length = struct.unpack_from("<Q", suffix)[0] + 8
        certificate, public_key = assembler.certificate_from_block(suffix[:length])
        text = recipe["signed_feed_text"]
        self.assertEqual(assembler.verify_signed_feed(text, certificate, public_key)["release"]["version_code"], 23)
        envelope = json.loads(text)
        signature = bytearray(base64.b64decode(envelope["signature"]))
        signature[0] ^= 1
        envelope["signature"] = encoded(signature)
        with self.assertRaisesRegex(assembler.VerificationError, "Feed signature"):
            assembler.verify_signed_feed(json.dumps(envelope), certificate, public_key)
        envelope = json.loads(text)
        payload = json.loads(base64.b64decode(envelope["payload"]))
        payload["release"]["version_code"] = 22
        envelope["payload"] = encoded(json.dumps(payload).encode())
        with self.assertRaisesRegex(assembler.VerificationError, "Feed release mismatch: version_code"):
            assembler.verify_signed_feed(json.dumps(envelope), certificate, public_key)


if __name__ == "__main__":
    unittest.main()

#!/usr/bin/env python3
"""Regression and fail-closed security tests for one-instruction v16 touch bridge."""
import hashlib
import importlib.util
import struct
import unittest
from pathlib import Path
from unittest.mock import patch
import zlib

MODULE = Path(__file__).resolve().parents[1] / "touch_dex.py"
spec = importlib.util.spec_from_file_location("touch_dex", MODULE)
touch = importlib.util.module_from_spec(spec)
spec.loader.exec_module(touch)


def dex_with_instruction(insn):
    src = bytearray(touch.OFFSET + 256)
    src[:8] = b"dex\n035\x00"
    src[touch.OFFSET:touch.OFFSET+4] = insn
    src[12:32] = hashlib.sha1(src[32:]).digest()
    struct.pack_into("<I", src, 8, zlib.adler32(src[12:]) & 0xffffffff)
    return bytes(src)


def approved_patch(src):
    result=bytearray(src)
    result[touch.OFFSET:touch.OFFSET+4]=touch.AFTER
    result[12:32]=hashlib.sha1(result[32:]).digest()
    struct.pack_into("<I",result,8,zlib.adler32(result[12:]) & 0xffffffff)
    return bytes(result)


class TouchWheelDexTests(unittest.TestCase):
    def test_exact_original_rewrite_is_bounded_and_still_valid_dex(self):
        source=dex_with_instruction(touch.BEFORE)
        expected=approved_patch(source)
        with patch.object(touch,"ORIGINAL_SHA256",touch.sha256(source)),patch.object(
            touch,"PATCHED_SHA256",touch.sha256(expected)):
            result=touch.patch_touch_wheel_triangle(source)
            self.assertEqual(result,expected)
            self.assertEqual(touch.patch_touch_wheel_triangle(result),result)
        changed=[i for i,(a,b) in enumerate(zip(source,result)) if a!=b]
        self.assertEqual(len(result),len(source))
        self.assertTrue(set(changed).issubset(set(range(8,32)) | set(
            range(touch.OFFSET,touch.OFFSET+4))))
        self.assertEqual(result[touch.OFFSET:touch.OFFSET+4],bytes.fromhex("29000600"))
        self.assertEqual(hashlib.sha1(result[32:]).digest(),result[12:32])
        self.assertEqual(zlib.adler32(result[12:]) & 0xffffffff,struct.unpack_from("<I",result,8)[0])

    def test_wrong_source_not_accepted(self):
        with self.assertRaisesRegex(ValueError,"Unrecognized"):
            touch.patch_touch_wheel_triangle(dex_with_instruction(touch.AFTER))

    def test_corrupt_four_byte_branch_is_rejected_even_with_a_pinned_hash(self):
        source=dex_with_instruction(b"\0\0\0\0")
        with patch.object(touch,"ORIGINAL_SHA256",touch.sha256(source)):
            with self.assertRaisesRegex(ValueError,"branch changed"):
                touch.patch_touch_wheel_triangle(source)

    def test_fail_closed_on_different_verified_output(self):
        original=dex_with_instruction(touch.BEFORE)
        with patch.object(touch,"ORIGINAL_SHA256",touch.sha256(original)), patch.object(
            touch,"PATCHED_SHA256","0"*64):
            with self.assertRaisesRegex(ValueError,"reconstruction"):
                touch.patch_touch_wheel_triangle(original)

    def test_pinned_original_and_patched_hashes_are_distinct(self):
        self.assertEqual(len(touch.ORIGINAL_SHA256),64)
        self.assertEqual(len(touch.PATCHED_SHA256),64)
        self.assertNotEqual(touch.ORIGINAL_SHA256,touch.PATCHED_SHA256)
        self.assertEqual(touch.OFFSET,0x12086)
        self.assertEqual(touch.BEFORE,bytes.fromhex("33460600"))
        self.assertEqual(touch.AFTER,bytes.fromhex("29000600"))


if __name__=="__main__":
    unittest.main()

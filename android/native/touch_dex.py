#!/usr/bin/env python3
"""Fail-closed, four-byte v16 touch-WEAPONS compatibility fix.

The bundled v16 TouchLayout sends the WEAPONS button as the Triangle bit
(0x10), but classifies it as a weapon-wheel gesture. TouchInputModel.snapshot
drops the input before nativeTouchInput. Allow that wheel finger's own Triangle
bit through the existing JNI bridge; native Android gates it to an actual
in-game interaction prompt. No Java UI, weapon-wheel selection, or JNI
signatures are replaced.

A single DEX branch, at the pinned original classes2.dex instruction:
    if-ne v6, v4, +6    =>    goto/16 +6
Its surrounding role==3 / kind==1 checks and the rest of the binary remain
untouched. The DEX SHA1 signature and Adler32 checksum are regenerated.
"""
import hashlib
import struct
import zlib

ORIGINAL_SHA256 = "0d5073fcab9869b69ca4b02d0d480ab7512f049de5f190dd42c84df51f36a45f"
PATCHED_SHA256 = "d13bbd9c4365ea0a229d5fa47c8356fbe575af61bee155c7d9620b4aa9e4c110"
OFFSET = 0x12086
BEFORE = bytes.fromhex("33460600")
AFTER = bytes.fromhex("29000600")


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def patch_touch_wheel_triangle(data: bytes) -> bytes:
    if sha256(data) == PATCHED_SHA256:
        return data
    if sha256(data) != ORIGINAL_SHA256:
        raise ValueError("Unrecognized classes2.dex: refusing to change touch controls")
    if data[:4] != b"dex\\n" or data[OFFSET:OFFSET + 4] != BEFORE:
        raise ValueError("Original TouchInputModel snapshot branch changed")
    result = bytearray(data)
    result[OFFSET:OFFSET + 4] = AFTER
    result[12:32] = hashlib.sha1(result[32:]).digest()
    struct.pack_into("<I", result, 8, zlib.adler32(result[12:]) & 0xFFFFFFFF)
    result = bytes(result)
    if sha256(result) != PATCHED_SHA256:
        raise ValueError("Failed byte-exact DEX reconstruction")
    return result

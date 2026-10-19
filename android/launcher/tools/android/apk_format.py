#!/usr/bin/env python3
"""Read-only binary XML and 16 KiB alignment helpers recovered from v16."""
from __future__ import annotations
import hashlib
from pathlib import Path
import struct
import zipfile

PACKAGE_ID = "org.rerac.android.uipreview"
APP_LABEL = "ReRAC"

def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def file_sha256(path: Path) -> str:
    with path.open("rb") as stream:
        digest = hashlib.sha256()
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def decode_length(data: bytes, offset: int, utf8: bool) -> tuple[int, int]:
    if utf8:
        value = data[offset]
        offset += 1
        if value & 0x80:
            value = ((value & 0x7f) << 8) | data[offset]
            offset += 1
    else:
        value = struct.unpack_from("<H", data, offset)[0]
        offset += 2
        if value & 0x8000:
            value = ((value & 0x7fff) << 16) | struct.unpack_from("<H", data, offset)[0]
            offset += 2
    return value, offset


def encode_length(value: int, utf8: bool) -> bytes:
    if utf8:
        if value >= 0x8000:
            raise ValueError("String too long for Android UTF-8 pool")
        return bytes([value]) if value < 0x80 else bytes([(value >> 8) | 0x80, value & 0xff])
    return struct.pack("<H", value) if value < 0x8000 else struct.pack("<HH", (value >> 16) | 0x8000, value & 0xffff)


def string_pool(chunk: bytes) -> tuple[list[str], int]:
    _, hs, _ = struct.unpack_from("<HHI", chunk)
    count, styles, flags, strings_start, styles_start = struct.unpack_from("<IIIII", chunk, 8)
    if styles or styles_start:
        raise ValueError("Unexpected styled manifest strings")
    utf8 = bool(flags & 0x100)
    strings = []
    for index in range(count):
        offset = strings_start + struct.unpack_from("<I", chunk, hs + index * 4)[0]
        units, offset = decode_length(chunk, offset, utf8)
        if utf8:
            size, offset = decode_length(chunk, offset, True)
            value = chunk[offset:offset + size].decode("utf-8")
        else:
            value = chunk[offset:offset + units * 2].decode("utf-16le")
        strings.append(value)
    return strings, flags


def make_string_pool(strings: list[str], flags: int) -> bytes:
    utf8 = bool(flags & 0x100)
    offsets = []
    data = bytearray()
    for value in strings:
        offsets.append(len(data))
        units = len(value.encode("utf-16le")) // 2
        if utf8:
            encoded = value.encode("utf-8")
            data += encode_length(units, True) + encode_length(len(encoded), True) + encoded + b"\0"
        else:
            data += encode_length(units, False) + value.encode("utf-16le") + b"\0\0"
    data += bytes((-len(data)) % 4)
    start = 28 + 4 * len(strings)
    # Clear the sorted flag: replacing strings can change lexical ordering.
    return (struct.pack("<HHIIIIII", 1, 28, start + len(data), len(strings), 0,
                        flags & ~1, start, 0)
            + b"".join(struct.pack("<I", offset) for offset in offsets) + data)


def axml_chunks(data: bytes) -> list[bytearray]:
    kind, hs, total = struct.unpack_from("<HHI", data)
    if kind != 3 or hs != 8 or total != len(data):
        raise ValueError("Unexpected binary AndroidManifest.xml header")
    result = []
    offset = hs
    while offset < total:
        _, _, size = struct.unpack_from("<HHI", data, offset)
        if size < 8 or offset + size > total:
            raise ValueError("Invalid binary XML chunk")
        result.append(bytearray(data[offset:offset + size]))
        offset += size
    return result


def manifest_values(data: bytes) -> list[dict]:
    chunks = axml_chunks(data)
    strings, _ = string_pool(next(bytes(c) for c in chunks if struct.unpack_from("<H", c)[0] == 1))
    output = []
    for chunk in chunks:
        if struct.unpack_from("<H", chunk)[0] != 0x102:
            continue
        name = strings[struct.unpack_from("<I", chunk, 20)[0]]
        start, size, count = struct.unpack_from("<HHH", chunk, 24)
        attrs = {}
        for i in range(count):
            offset = 16 + start + i * size
            _, key, raw = struct.unpack_from("<III", chunk, offset)
            _, _, typ, value = struct.unpack_from("<HBBI", chunk, offset + 12)
            attrs[strings[key]] = (strings[raw] if raw != 0xffffffff else
                                  strings[value] if typ == 3 else
                                  bool(value) if typ == 0x12 else value)
        output.append({"element": name, "attrs": attrs})
    return output


def native_alignment(path: Path) -> dict:
    result = {}
    with zipfile.ZipFile(path) as archive, path.open("rb") as raw:
        for entry in archive.infolist():
            if not entry.filename.startswith("lib/") or not entry.filename.endswith(".so"):
                continue
            if entry.compress_type != zipfile.ZIP_STORED:
                raise ValueError("Compressed native library: " + entry.filename)
            raw.seek(entry.header_offset + 26)
            name_size, extra_size = struct.unpack("<HH", raw.read(4))
            offset = entry.header_offset + 30 + name_size + extra_size
            if offset % 16384:
                raise ValueError("Native ZIP entry not 16 KiB aligned: " + entry.filename)
            data = archive.read(entry)
            if data[:6] != b"\x7fELF\x02\x01":
                raise ValueError("Unexpected native ELF class/endianness")
            program_offset = struct.unpack_from("<Q", data, 32)[0]
            program_size, program_count = struct.unpack_from("<HH", data, 54)
            segments = []
            for index in range(program_count):
                segment = struct.unpack_from("<IIQQQQQQ", data, program_offset + index * program_size)
                if segment[0] != 1:
                    continue
                file_offset, address, alignment = segment[2], segment[3], segment[7]
                if alignment < 16384 or (address - file_offset) % 16384:
                    raise ValueError("Native ELF load segment not 16 KiB compatible")
                segments.append({"offset": file_offset, "address": address, "alignment": alignment})
            if not segments:
                raise ValueError("Native ELF has no load segments")
            result[entry.filename] = {"bytes": len(data), "sha256": sha256(data),
                                      "zip_offset": offset, "elf_load_segments": segments}
    if len(result) != 4:
        raise ValueError("Expected all four original native libraries")
    return result

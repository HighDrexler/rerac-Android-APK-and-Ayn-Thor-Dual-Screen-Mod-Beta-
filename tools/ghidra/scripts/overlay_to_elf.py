#!/usr/bin/env python3
"""Turn Insomniac "Ratchet executable" level overlays into ELFs Ghidra can load.

The overlay (docs/formats/wad_layouts_rac1.md §4) is a bare run of `[16-byte header][data]` sections with no
names. This writes the same ELF the retired C++ extractor wrote for Ratchet 1 (`work/ghidra-import/levelNN.elf`):
one PT_LOAD per PROGBITS section, the seven donor section names, e_flags 0x20924001 so the Emotion Engine module
picks r5900.

Usage:
  overlay_to_elf.py --overlay extracted/levels/01/overlay.bin --out level01.elf
  overlay_to_elf.py --rc2-dir "<extracted RC2 disc folder with G/LEVELn.WAD>" --out-dir work/ghidra-import/rc2

The RC2 path reads `G/LEVELn.WAD` directly: the level WAD header (0x60 bytes: data SectorRange at 0x10; 0x68 bytes:
at 0x0c), then the level data WAD whose header starts with the overlay ByteRange. It also copies the boot ELF and
writes `overlay.bin` next to each ELF.
"""
import argparse
import os
import shutil
import struct
import sys

SECTOR = 0x800
DONOR = [  # (name, flags, align) for the seven overlay sections, in order
    (".lit", 0x10000003, 64),
    (".bss", 0x10000003, 64),
    (".data", 0x3, 64),
    ("lvl.vtbl", 0x2, 1),
    ("lvl.camvtbl", 0x2, 1),
    ("lvl.sndvtbl", 0x2, 1),
    (".text", 0x6, 64),
]
DONOR_TYPES = [1, 8, 1, 1, 1, 1, 1]
SHT_NOBITS = 8
E_FLAGS = 0x20924001
DATA_START = 0x100


def parse_overlay(blob):
    """[(dest, type, data)], entry. Stops when a header's entry differs from the first (how the game walks it)."""
    secs, pos, entry = [], 0, None
    while pos + 16 <= len(blob):
        dest, size, kind, ent = struct.unpack_from("<IIII", blob, pos)
        if entry is None:
            entry = ent
        elif ent != entry:
            break
        if size > len(blob) - pos - 16:
            raise ValueError(f"section at {pos:#x} runs past the overlay ({size:#x} bytes)")
        secs.append((dest, kind, blob[pos + 16: pos + 16 + size]))
        pos += 16 + size
    if not secs:
        raise ValueError("overlay has no sections")
    return secs, entry, pos


def align(n, a):
    return (n + a - 1) // a * a


def build_elf(secs, entry):
    if [k for _, k, _ in secs] != DONOR_TYPES:
        raise ValueError(f"section types {[k for _, k, _ in secs]} do not match the donor {DONOR_TYPES}")
    progbits = [i for i, (_, k, _) in enumerate(secs) if k != SHT_NOBITS]
    phnum = len(progbits)
    # Lay the PROGBITS data out contiguously from 0x100, 16-byte aligned; NOBITS takes the next offset, no bytes.
    offsets, pos = {}, DATA_START
    for i, (_, k, data) in enumerate(secs):
        pos = align(pos, 16)
        offsets[i] = pos
        if k != SHT_NOBITS:
            pos += len(data)
    shstr = b"\0" + b"".join(n.encode() + b"\0" for n, _, _ in DONOR) + b".shstrtab\0"
    shstr_off = pos
    shoff = align(shstr_off + len(shstr), 4)
    shnum = len(secs) + 2

    out = bytearray()
    out += struct.pack("<16sHHIIIIIHHHHHH", b"\x7fELF\x01\x01\x01" + b"\0" * 9, 2, 8, 1, entry, 52, shoff,
                       E_FLAGS, 52, 32, phnum, 40, shnum, shnum - 1)
    for i in progbits:
        dest, _, data = secs[i]
        out += struct.pack("<8I", 1, offsets[i], dest, dest, len(data), len(data), 7, 0x10)
    for i, (_, k, data) in enumerate(secs):
        if k != SHT_NOBITS:
            out += b"\0" * (offsets[i] - len(out))
            out += data
    out += b"\0" * (shstr_off - len(out))
    out += shstr
    out += b"\0" * (shoff - len(out))
    out += struct.pack("<10I", *([0] * 10))
    name_off = 1
    for i, (dest, k, data) in enumerate(secs):
        name, flags, al = DONOR[i]
        out += struct.pack("<10I", name_off, k, flags, dest, offsets[i], len(data), 0, 0, al, 0)
        name_off += len(name) + 1
    out += struct.pack("<10I", name_off, 3, 0, 0, shstr_off, len(shstr), 0, 0, 1, 0)
    return bytes(out)


def rc2_level_overlay(wad_path):
    """The overlay blob inside G/LEVELn.WAD (header at offset 0, ranges relative to the file)."""
    with open(wad_path, "rb") as f:
        head = f.read(0x80)
        header_size = struct.unpack_from("<i", head, 0)[0]
        if header_size == 0x60:
            data_off, data_size = struct.unpack_from("<ii", head, 0x10)
        elif header_size == 0x68:
            data_off, data_size = struct.unpack_from("<ii", head, 0x0c)
        else:
            raise ValueError(f"{wad_path}: unknown level WAD header size {header_size:#x}")
        f.seek(data_off * SECTOR)
        ov_off, ov_size = struct.unpack("<ii", f.read(8))
        f.seek(data_off * SECTOR + ov_off)
        return f.read(ov_size)


def convert(blob, out_path, label):
    secs, entry, used = parse_overlay(blob)
    if used != len(blob):
        print(f"  note: {label}: {len(blob) - used} trailing bytes after the last section", file=sys.stderr)
    elf = build_elf(secs, entry)
    with open(out_path, "wb") as f:
        f.write(elf)
    text = secs[6][2]
    print(f"{label}: entry {entry:#x}, .text {secs[6][0]:#x}+{len(text):#x}, {len(elf)} bytes -> {out_path}")


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--overlay", help="one overlay.bin to convert")
    ap.add_argument("--out", help="output ELF for --overlay")
    ap.add_argument("--rc2-dir", help="extracted Ratchet 2 disc folder (holds G/LEVELn.WAD and the boot ELF)")
    ap.add_argument("--out-dir", help="output folder for --rc2-dir")
    a = ap.parse_args()
    if a.overlay:
        convert(open(a.overlay, "rb").read(), a.out or os.path.splitext(a.overlay)[0] + ".elf", a.overlay)
    elif a.rc2_dir:
        out_dir = a.out_dir or "work/ghidra-import/rc2"
        os.makedirs(out_dir, exist_ok=True)
        boots = [n for n in os.listdir(a.rc2_dir) if n.startswith("SCUS_") or n.startswith("SCES_")]
        for b in boots:
            shutil.copyfile(os.path.join(a.rc2_dir, b), os.path.join(out_dir, b))
            print(f"boot ELF {b} copied")
        n = 0
        while os.path.exists(os.path.join(a.rc2_dir, "G", f"LEVEL{n}.WAD")):
            blob = rc2_level_overlay(os.path.join(a.rc2_dir, "G", f"LEVEL{n}.WAD"))
            with open(os.path.join(out_dir, f"level{n:02d}.overlay.bin"), "wb") as f:
                f.write(blob)
            convert(blob, os.path.join(out_dir, f"level{n:02d}.elf"), f"LEVEL{n}")
            n += 1
        print(f"{n} levels")
    else:
        ap.error("give --overlay or --rc2-dir")


if __name__ == "__main__":
    main()

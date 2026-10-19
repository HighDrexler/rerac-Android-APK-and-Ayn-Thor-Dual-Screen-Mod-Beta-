#!/usr/bin/env python3
"""Which Ratchet 1 functions survive unchanged in Ratchet 2?

Compares one Ratchet 1 level overlay against one Ratchet 2 level overlay function by function, with the
relocation mask `rc_formats::level_overlay::mask` uses between levels (j/jal targets, lui of main-RAM addresses
and the %lo that completes them, $gp-relative offsets), plus two looser tiers:

    =  identical after the relocation mask and the stack-frame and scheduling normalisations (frame_norm, sched_norm):
       the port's function is already right for Ratchet 2
    k  identical after masking every 16-bit immediate too (same code, other constants / struct offsets)
    s  same instruction shape (opcodes and functs, registers and immediates ignored): edited or re-allocated
    ?  no counterpart found (changed enough, or gone)

`--similar NAME...` finds the closest Ratchet 2 function for a changed one and says how much of it survives.
A match needs a unique counterpart; ambiguous hashes (tiny functions) count as `?` unless the names say otherwise.

    crossgame_diff.py [--rc1 extracted/levels/01/overlay.bin] [--rc1-index work/decomp/level01.elf/index.tsv]
                      [--rc2 work/ghidra-import/rc2/level01.overlay.bin] [--rc2-funcs work/ghidra-import/rc2/level01_functions.json]
                      [--out work/crossgame/level01.tsv] [--cite crates/rc-game/src/hero]

Writes a TSV (rc1 address, name, status, rc2 address, words) and prints the totals, plus the status of every
function the Rust sources under --cite mention by address.
"""
import argparse, glob, json, os, re, struct, sys
from collections import Counter, defaultdict

MAX_EXTENT = 0x8000
TEXT = 6


def sections(blob):
    out, pos, entry = [], 0, None
    while pos + 16 <= len(blob):
        dest, size, kind, ent = struct.unpack_from("<IIII", blob, pos)
        if entry is None: entry = ent
        elif ent != entry: break
        out.append((dest, kind, blob[pos + 16: pos + 16 + size])); pos += 16 + size
    return out


def op(w): return w >> 26
def rs(w): return (w >> 21) & 31
def rt(w): return (w >> 16) & 31
def rd(w): return (w >> 11) & 31
def sext(w): return struct.unpack("<i", struct.pack("<I", (w & 0xffff) << 16))[0] >> 16
def is_branch(o): return o == 1 or 4 <= o <= 7 or 0x14 <= o <= 0x17
def lui_is_address(w): return 0x0010 <= (w & 0xffff) < 0x0060
WRITES_RT = set(range(8, 16)) | set(range(0x20, 0x28)) | {0x30, 0x37, 0x1a, 0x1b, 0x1e}


def mask(words):
    """Relocation mask, as the Rust walk (branch-target state propagation included)."""
    st = [None] * 32
    pending, branch_to, out = {}, None, []
    for i, w in enumerate(words):
        p = pending.pop(i, None)
        if p:
            for r in range(32):
                if st[r] is None: st[r] = p[r]
        o = op(w)
        if o in (2, 3):
            m = w & 0xfc000000
        elif o == 15:
            r = rt(w)
            if r == 0: m = w
            elif lui_is_address(w): st[r] = w & 0xffff; m = w & 0xffff0000
            else: st[r] = None; m = w
        elif o == 0:
            f = w & 63; s, t, d = rs(w), rt(w), rd(w)
            moved = (st[s] if t == 0 else st[t] if s == 0 else None) if f in (0x21, 0x2d, 0x25) else None
            if d: st[d] = moved
            m = w
        elif is_branch(o):
            m = w
        elif 0x10 <= o <= 0x13:
            if rs(w) <= 2 and rt(w): st[rt(w)] = None
            m = w
        elif o == 0x1c:
            if rd(w): st[rd(w)] = None
            m = w
        else:
            s = rs(w)
            full = s == 28 or st[s] is not None
            m = (w & 0xffff0000) if full else w
            if o in WRITES_RT and rt(w): st[rt(w)] = None
        out.append(m)
        if branch_to is not None:
            e = pending.setdefault(branch_to, [None] * 32)
            for r in range(32):
                if e[r] is None: e[r] = st[r]
            branch_to = None
        if is_branch(o) or (o == 0x11 and rs(w) == 8):
            t = i + 1 + sext(w)
            if t > i + 1 and t < len(words): branch_to = t
    return out


SP = 29
SP_OPS = set(range(0x20, 0x30)) | {0x30, 0x37, 0x38, 0x3f, 0x1a, 0x1b, 0x1e, 0x1f, 0x31, 0x35, 0x39, 0x3d, 9, 0x19}


def frame_norm(masked):
    """Stack-frame normalisation: Ratchet 2 is built with 64-bit callee saves (sd/ld) and tighter frames where
    Ratchet 1 uses 128-bit sq/lq, so stack offsets, the frame size and the save width differ in every function.
    sq/lq with base $sp become sd/ld, and every $sp-relative immediate (including addiu sp,sp,N) is masked."""
    out = []
    for w in masked:
        o = op(w)
        if rs(w) == SP and o in SP_OPS:
            if o == 0x1f: w = (w & 0x03ffffff) | (0x3f << 26)
            elif o == 0x1e: w = (w & 0x03ffffff) | (0x37 << 26)
            w &= 0xffff0000
        out.append(w)
    return out


def sched_norm(masked):
    """Scheduling normalisation: the Ratchet 2 compiler pads with extra nops after branches, so nops are dropped
    and branch offsets (relative, shifted by the padding) are masked."""
    return [w & 0xffff0000 if (is_branch(op(w)) or (op(w) == 0x11 and rs(w) == 8)) else w for w in masked if w != 0]


def mask_imm(masked):
    """Tier k: every I-type immediate zeroed as well (j/jal/R-type/cop untouched)."""
    return [w & 0xffff0000 if (op(w) not in (0, 2, 3) and not (0x10 <= op(w) <= 0x13)) else w for w in masked]


def shape(words):
    """Tier s: opcode + funct only (after the frame normalisation)."""
    words = sched_norm(frame_norm(words))
    return [((w >> 26) << 6) | (w & 63 if op(w) in (0, 0x1c) else (rs(w) if 0x10 <= op(w) <= 0x13 else 0)) for w in words]


def functions(text_start, text, starts):
    starts = sorted(a for a in starts if text_start <= a < text_start + 4 * len(text))
    out = {}
    for i, a in enumerate(starts):
        end = starts[i + 1] if i + 1 < len(starts) else text_start + 4 * len(text)
        n = min(end - a, MAX_EXTENT) // 4
        out[a] = text[(a - text_start) // 4:(a - text_start) // 4 + n]
    return out


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--rc1", default="extracted/levels/01/overlay.bin")
    ap.add_argument("--rc1-index", default="work/decomp/level01.elf/index.tsv")
    ap.add_argument("--rc2", default="work/ghidra-import/rc2/level01.overlay.bin")
    ap.add_argument("--rc2-funcs", default="work/ghidra-import/rc2/level01_functions.json")
    ap.add_argument("--out", default="work/crossgame/level01.tsv")
    ap.add_argument("--cite", action="append", default=[], help="Rust source folder whose 0x addresses to report")
    ap.add_argument("--similar", nargs="*", default=[], help="Ratchet 1 function names: print the closest Ratchet 2 function by masked-sequence similarity (slow: a few seconds each)")
    a = ap.parse_args()

    s1, s2 = sections(open(a.rc1, "rb").read()), sections(open(a.rc2, "rb").read())
    t1s, t1 = s1[TEXT][0], list(struct.unpack(f"<{len(s1[TEXT][2]) // 4}I", s1[TEXT][2][:len(s1[TEXT][2]) // 4 * 4]))
    t2s, t2 = s2[TEXT][0], list(struct.unpack(f"<{len(s2[TEXT][2]) // 4}I", s2[TEXT][2][:len(s2[TEXT][2]) // 4 * 4]))
    names1 = {}
    for line in open(a.rc1_index):
        f = line.rstrip("\n").split("\t")
        if len(f) >= 2: names1[int(f[0], 16)] = f[1]
    f2 = {int(f["address"], 16): f["name"] for f in json.load(open(a.rc2_funcs))["functions"]}
    fn1, fn2 = functions(t1s, t1, names1), functions(t2s, t2, f2)

    idx = [defaultdict(list), defaultdict(list), defaultdict(list)]
    for addr, words in fn2.items():
        m = sched_norm(frame_norm(mask(words)))
        idx[0][tuple(m)].append(addr); idx[1][tuple(mask_imm(m))].append(addr); idx[2][tuple(shape(words))].append(addr)

    rows, used = [], set()
    for addr in sorted(fn1):
        words = fn1[addr]; m = sched_norm(frame_norm(mask(words)))
        keys = [tuple(m), tuple(mask_imm(m)), tuple(shape(words))]
        status, other = "?", None
        for tier, (code, key) in enumerate(zip("=ks", keys)):
            cands = [c for c in idx[tier].get(key, []) if c not in used]
            if len(cands) == 1 and (tier == 0 or len(words) >= 8):
                status, other = code, cands[0]; break
            if len(cands) > 1 and len(words) >= 8:
                status, other = code + "*", None; break  # ambiguous: several identical copies
        if other is not None: used.add(other)
        rows.append((addr, names1.get(addr, ""), status, other, len(words)))

    os.makedirs(os.path.dirname(a.out), exist_ok=True)
    with open(a.out, "w") as f:
        f.write("rc1_addr\tname\tstatus\trc2_addr\twords\n")
        for addr, name, status, other, n in rows:
            f.write(f"{addr:08x}\t{name}\t{status}\t{other and f'{other:08x}' or ''}\t{n}\n")

    c = Counter(r[2] for r in rows)
    print(f"Ratchet 1 level01: {len(rows)} functions; Ratchet 2 level01: {len(fn2)} functions")
    for k in ("=", "k", "s", "=*", "k*", "s*", "?"):
        if c[k]: print(f"  {k:2s} {c[k]:5d}  ({100 * c[k] / len(rows):.1f}%)")
    print(f"  Ratchet 2 functions with no Ratchet 1 counterpart: {len(fn2) - len(used)}")
    print(f"TSV: {a.out}")

    if a.similar:
        import difflib
        by = {n: addr for addr, n in names1.items()}
        norm2 = {addr: sched_norm(frame_norm(mask(w))) for addr, w in fn2.items()}
        print(f"\n{'function':28s} {'RC1 words':>9s} {'best RC2':>9s} {'words':>6s} {'same':>5s}  verdict")
        for name in a.similar:
            addr = by.get(name) or int(name, 16)
            m = sched_norm(frame_norm(mask(fn1[addr])))
            best = (0.0, None)
            for b, mm in norm2.items():
                if not 0.6 <= len(mm) / max(1, len(m)) <= 1.7: continue
                sm = difflib.SequenceMatcher(None, m, mm, autojunk=False)
                if sm.real_quick_ratio() < best[0] or sm.quick_ratio() < best[0]: continue
                r = sm.ratio()
                if r > best[0]: best = (r, b)
            r, b = best
            verdict = "same with edits" if r >= 0.8 else "reworked" if r >= 0.5 else "rewritten, or the counterpart is split / sized differently"
            print(f"{name:28s} {len(fn1[addr]):9d} {f'{b:08x}' if b else '-':>9s} {len(fn2[b]) if b else 0:6d} {r:5.2f}  {verdict}")

    for folder in a.cite:
        cited = set()
        for p in glob.glob(os.path.join(folder, "**/*.rs"), recursive=True):
            for h in re.findall(r"0x([0-9a-fA-F]{5,6})\b", open(p).read()):
                v = int(h, 16)
                if v in fn1: cited.add(v)
        sub = [r for r in rows if r[0] in cited]
        cc = Counter(r[2] for r in sub)
        print(f"\n{folder}: {len(sub)} cited level01 functions: " + ", ".join(f"{k} {v}" for k, v in sorted(cc.items())))
        for addr, name, status, other, n in sub:
            if status not in ("=",):
                print(f"  {status:2s} {addr:08x} {name:40s} -> {other and f'{other:08x}' or '-':8s} ({n} words)")


if __name__ == "__main__":
    main()

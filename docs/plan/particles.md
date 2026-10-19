# Particles and sprite effects (RAC1, from the decomp)

Addresses are **level01.elf** (Novalis overlay) unless marked *boot*. The particle engine is shared code:
95 of its ~130 functions hash-match in all 19 level overlays and most others in several
(`tools/ghidra/names/clusters.tsv` maps the addresses per overlay). The boot ELF only carries the allocator,
`UpdateParts` and `PartProc` (it has no particle types). Confidence tags: **high** = read from
disassembly/decomp and constants re-read from memory; **med** = inferred from usage; **low** = guess.

## 1. Functions

| level01 | boot | Name (ours / Lombyte) | Role |
|---|---|---|---|
| 0x27c498 | 0x217a30 | `CreatePart(u8 type)` (Lombyte `CreatePart`) | stub `a1 = 0; j 0x27c4a0`; returns record ptr or 0 when full |
| 0x27c4a0 | – | `CreatePart2(type, a1)` | real allocator; `a1` is dead (its bitmap-2 code is jumped over) |
| 0x27c5e8 | – | `KillPart(rec)` | free record |
| 0x27c7e8 | 0x217b88 | `UpdateParts` | per-tick dispatch through the type table |
| 0x27c878 | 0x217c18 | `PartProc` | per-frame cull, depth sort, VU1 chain build (program 0x101088) |
| 0x27d46c | 0x21880c | `_part_load_tex` | queue a 32×32 texture + CLUT upload, build TEX0 |
| 0x27d4e0 | – | `RegisterPartTypes` | fills the 81-entry update table 0x1b2300 (called once from 0x251c30) |
| 0x253648 | 0x2026c8 | `ParseParticleTexs` | level load: part_defs + part_textures → 0x1b2500 / 0x1b2700 / 0x1b1f00 |
| 0x21ae98 | 0x1f44b8 | `GetEffectTex(i)` | FX textures (fonts, lens flare); shares the upload queue, not used by particles |
| 0x26c930.. | 0x213260.. | random helpers | §5 |
| – | 0x22bba0 / 0x22ae70 | `SkySpriteProc` / `update_sky_effects` | sky stars, same VU1 program (§8) |

## 2. Pool and record — confidence high

* **Pool**: 2048 records × 0x40 bytes (128 KiB), pointer at gp−0x6a94 (0x16016c), placed 0x40-aligned
  after other level data by level init 0x255958. Allocation bitmap 0x1b1d00 (256 bytes), second bitmap
  0x1b1e00 (never set; see flag 0x40). Globals: gp−0x6a90 (0x160170) lowest-free hint, gp−0x6a8c
  (0x160174) highest index ever live ("hw", −1 when empty), gp−0x6a88 live count. Level init sets
  hint 0, hw −1, count 0 and zeroes both bitmaps; the records themselves are **not** cleared.
* **CreatePart**: if hint ≥ 0x800 → 0. Else i = hint; set bit i; count++; hw = max(hw, i); new hint =
  i+1 if i > old hw, else first clear bit scanning from the start of the byte holding i+1. Record i:
  `sq zero` at +0x00 and **three times at +0x10** (0x27c5c8..d4): bytes 0x20..0x3f keep the previous
  occupant's data (a real bug every spawner must cover). byte0 = type. No other init.
* **KillPart**: count−−; clear bit (and bitmap-2 bit if byte1 & 0x40); if i == hw, hw = next lower set
  bit (−1 if none); byte1 = 0x80; hint = min(hint, i).
* **Order of use**: lowest free slot first; `UpdateParts` and `PartProc` walk 0..hw.

Record layout (common part; +0x0a.. and +0x20.. are per-type, see §4):

| Off | Type | Meaning |
|---|---|---|
| 0x00 | u8 | type 0..80 (index into update table 0x1b2300) |
| 0x01 | u8 | flags: 0x80 dead; 0x40 bitmap-2 (unused); 0x20 drawn last frame (set/cleared by `PartProc`); bits 0–1 **render kind**: 0 camera-facing sprite, 1 flat quad in the world XY plane, 2 untextured line, 3 textured ribbon |
| 0x02 | u8 | particle-texture index (0x1b1f00 table, §6) |
| 0x03 | u8 | GS ALPHA_1 low byte: 0x44 = (Cs−Cd)·As+Cd, 0x48 = Cs·As+Cd (additive); FIX = 0 |
| 0x04 | u32 | RGBA colour 1 (R low byte; 0x80 = 1.0) |
| 0x08 | u8 | rotation, 256 steps per turn |
| 0x09 | u8 | low nibble n: near = n/4 units; high nibble m: far = 32·m units (capped by the global far) |
| 0x0a | s16 | usually the life timer (ticks) |
| 0x0c | f32 / u32 | size (kinds 0/1, units of 1/210000 world unit) / colour 2 (kinds 2/3) |
| 0x10 | vec4 | position (world, game units); +0x1c = ribbon half-width at end 1 (kind 3) |
| 0x20 | vec4 | velocity for most types / end-2 position (kinds 2/3); +0x2c = end-2 width factor (kind 3) |
| 0x30.. | | per-type |

## 3. Types and spawn API — confidence high (table), med (labels)

`RegisterPartTypes` fills 81 update pointers (type: update/spawner, level01):
0:27d928, 1:27dbc8/27daf0, 2:27de70/27dc98, 3:27e0d8, 4:27e650/27e538, 5:27e850/27e750,
6:27eb08/27e900, 7:27f200, 8:27f3a0/27f2b0, 9:27f4d8, 10:27f788/27f660, 11:27fb80/27f8f8,
12:280408/280138, 13:280818/280698, 14:280ac8/280970, 15:280d08/280bd0, 16:2810e8/280f30, 17:281428,
18:2815b8/281430, 19:281850/281748, 20:2818c0, 21:281d10/281c10, 22:27c6f0/281f30, 23:282210/282060,
24:2824d0, 25:282760/2825f8, 26:282c40/282b00, 27:282e60/282d80, 28:2831b0/282ef0, 29:283408,
30:2836d0, 31:283be8, 32:283ea8/283d88, 33:2840a8, 34:2842e0/2840e0, 35:2846c0/2845a8, 36:2848f8,
37:284a60, 38:284b60, 39:284c98, 40:284f70/284d88, 41:2858a8/285768, 42:285ae0/2858f0, 43:286238,
44:2865e8/286450, 45:2868e0/286780, 46:286b88/286a68, 47:286d80/286cb0, 48:286e20, 49:286eb8,
50:286f90, 51:287060, 52:287288/287158, 53:2874b8/287328, 54:287700, 55:2879a0/2878a0,
56:287bd8/287b00, 57:287d90/287c80, 58:288068/287e70, 59:288530/288410, 60:288740/288588, 61:2888f0,
62:288ca8/288bb0, 63:288d58, 64:288f10/288d90, 65:289388, 66:2897b0/289648, 67:2899c0/289850,
68:289cc0, 69:289df0, 70:28a078, 71:28a0e0, 72:28a578/28a3f0, 73:28a5b8, 74:28a908/28a7a8, 75:28aae8,
76:28ac08, 77:28ac58, 78:28ae98/28ad08, 79:28b160, 80:28b2a8.

* **API shape.** Game code never calls `CreatePart` directly: each type has one *spawner*
  (47 in level01, all in 0x27d4e0..0x28b300, each the only `CreatePart` call site for its type) that
  fills the record from arguments. 34 types have no spawner in level01 (other levels' code or unused).
  Kinds set by spawners: flat quad = types 45, 46, 52, 57, 66; ribbon = 19, 55; the rest sprites.
  Typical spawner (type 59, 0x288410): `(f32 size, vec4* pos, u32 rgba, u8 rot, int def, bool normal_blend,
  s16 ticks, int attach)` → size·210000 at +0xc, byte9 = 0x21, byte3 = normal ? 0x44 : 0x48,
  byte2 = `*part_def[def]` (first frame), +0x30 = attach. Returns the record (callers may patch fields).
* **Attach.** There is no moby link in the common record. "Attached" types store an offset from the hero
  position 0x13f3d0 and re-add it every tick (types 59, 60 with `attach == 1`); moby-driven types keep a
  pointer to their owner's pvars at +0x2c (type 6). Effects that must follow a moby are re-spawned by it.
* **Expire.** Each update calls `KillPart(self)` itself: timer (`FastDecTimer` 0x220ea8: returns 1 if
  t == 0, else t = max(t,1)−1 and returns 2 when t ≤ 0 → kill on the tick t reaches 0), alpha ≤ 0,
  bounds boxes (e.g. type 60: any coord outside [2, 1021]), or a colour byte running out.

## 4. Simulation — confidence high unless noted

* **Tick order.** Game-state update 0x2a4080 (substates 0/8/3): `0x2ab920 → 0x2793d8` (moby updates) →
  `0x2a1a18 → 0x228870 → UpdateParts`. `UpdateParts` snapshots end = pool + (hw+1)·0x40 at entry and
  calls `table[byte0](rec)` for every record whose byte1 bit 7 is clear, in index order. Records created
  during the pass above the old hw wait a tick; ones created in a hole ahead of the cursor run now.
* **Time base** (`fun_00214970`, NTSC / PAL): 0x15ed60 speed 1.0 / 1.2; 0x15ed68 timer scale 1.0 / 0.8333;
  0x15ed6c dt 1/60 / 1/50; 0x15ed70 dt² 1/3600 / 1/2500. Timers: `ticks(n) = (int)(n·[0x15ed68] + 0.5)`
  (0x220e30). Velocities are units per tick; gravity where used = 9.8·dt² per tick (type 0: `vz -= 9.8·dt²`).
* **Float rules.** EE FPU/VU0 (no denormals, truncating) as elsewhere in the port; colour packing uses
  `truncate_float_to_s32(v·255)`; `FastTweenColor(f, a, b)` (0x2221a8) = per byte
  `ftoi0(itof(a)·(1−f) + itof(b)·f)` on VU0.
* **Common pattern** (types 56, 57, 59, 60 read in full): pos += vel (vector add), optional damping,
  size += constant, rotation byte from a float angle, colour/alpha from the timer, kill test.
  - type 56 (mist puff): vel *= 0.975 (0x3f79999a); pos += vel; size += 6300; grey g = R−4 →
    RGBA = (g, g, g, 0x60); kill when g < 1. Spawn: RGB = 0x60 + 2·randi(16), A 0x40, byte9 0x24, 0x48,
    byte2 = def[56][0], rot = randi(256).
  - type 57 (flat ripple, kind 1): pos += vel; size += 5880; spin *= 0.98; angle += spin; byte8 =
    trunc(angle); t = --timer (starts 0x80); A = t ≤ 64 ? t : 128 − t; RGB 0x80; kill at t ≤ 0.
  - type 59 (glow): if attach, pos += hero delta; timer only. Type 60: pos += vel (attach variant keeps
    a hero offset), kill outside [2,1021]³ or timer, then colour = FastTweenColor((t−1)/t, rgb|0, rgba)
    (alpha falls linearly to 0).
* **Type 6 = data-driven emitter particle** (spawner 0x27e900, update 0x27eb08; owner = class 27
  emitter moby, update 0x2bd100). All behaviour comes from the owner's pvars P (0xe0 bytes):
  - Emitter tick (0x2bd100): on level 1 only, skip unless `FastBSphereCheck(240, {pos, r 20})` passes.
    Counter P+0xc8: if < 1 → reload with P[0xc5] and emit P[0xc4] particles, else −1. Per particle
    (rand order matters): offset = randf(−P.cc, ..), randf(−P.d0, ..), randf(−P.d4, ..) (upper args garbled by the decompiler, med;
    rotated by the moby with flag 0x2000); velocity = flag 0x1000 ? P+0x80 rotated by Euler(randf(±P.90..98)·π/180)
    (flag 0x4000 adds the moby rotation) : randf(P.80..90, P.84..94, P.88..98)·speed; size = randf(P.a8, P.ac);
    angle = randf(P.a0, P.a4) (turns); colour = P.b0..b8, alpha = P.bc; ticks(P.c0).
  - Spawn record: byte9 = (P.c6 >> 5)·16 + 4 and byte3 = P.c6 & 1 ? 0x48 : 0x44; byte2 = def[P.c7][0];
    +0x20 vel, +0x2c P, +0x30 alpha (f32), +0x34 angle (f32 turns), byte8 = trunc(angle·255), +0x38..3a
    low bytes of trunc(c·65535), +4 = packed trunc(c·255) with A = trunc(alpha·255), +0x3b = 0.
  - Update, in order: (1) p' = pos + vel; if P.70 & 1, `CollLine_Fix(pos, p')` and on hit reflect vel
    (0x271170, factor P.3c) and place at hit + 0.055·normal; else pos = p'. (2) flag 2: each channel as a
    16-bit value (hi byte from +4, lo from +0x38+k)/65535 ± P.(4c,50,54)·speed (sign = +0x3b bits
    0x10/0x20/0x40), clamp to [P.(58,5c,60), P.(64,68,6c)], flip direction at the clamp when flags
    0x10.. / 0x100.. allow; store lo = trunc(v·65535), +4 = trunc(v·255) with A from the **old** alpha.
    Flag 4 without 2: A = trunc(alpha·255). (3) flag 4: alpha ± P.38·speed, clamp [P.44, P.48]
    (flip on 0x80/0x800). (4) byte8 = trunc(angle·255) (old angle). (5) flag 8: vel += P+0 (rotated by the
    owner if 0x8000), then clamp |vel| ≤ P.40 (`fun_001f9c90` rescales, it is not drag). (6) size +=
    P.30·speed, clamp [P.74, P.78]. (7) angle += P.34·speed. (8) kill if alpha ≤ 0, timer, or pos outside
    [P.10..18, P.20..28].
* **RNG users**: 17 functions of the module call `rand` directly, more through the §5 helpers.

## 5. RNG — confidence high

`rand` (boot 0x1160d8, newlib): `s = s·0x41c64e6d + 0x3039 (mod 2³²); return s & 0x7fffffff`, state u32 at
`*(0x12f76c) + 0x58` = 0x12f4d8, initial 1 (static). `srand` = boot 0x1160c8 (Lombyte
`set_global_state_slot`). Seeds: **srand(1234)** in level render init 0x255958 on every level load;
srand(12345) on the first frame of the boot star generator. One global stream shared by all game code
(61 call sites in level01), so particle randomness only matches if every consumer runs in game order.
Helpers (level01; *boot* in brackets), r = (rand() >> 16) & 0x7fff:
`randi(n)` 0x26c930 [0x213260] = r % n; `rand_range(a,b)` 0x26c970 = r % (b−a+1) + a;
`randf(a,b)` 0x26c9c8 [0x2132a8] = a + ((f32)r·(b−a))·2⁻¹⁵ (exact op order: cvt, mul, mul, add);
`randf_sym(a,b)` 0x26ca28: k = (rand()>>16) & 0xfff, v = a + ((f32)k·(b−a))·2⁻¹², negated if bit 16 of
rand() is set; `rand_angle` 0x26ca90 [0x213308] = (f32)(((rand()>>16) & 0xfff) − 0x800)·π·2⁻¹¹;
`rand_vec(len)` 0x26cb58: l = randf(?, len) (args lost by the decompiler), a = rand_angle, b = rand_angle → (cos a·sin b·l, sin a·sin b·l, cos b·l).

**Seeding order (pinned 2026-09-26, sky stars).** `srand(12345)` exists only in the boot executable's
`update_sky_effects` 0x22ae70 (boot sky pointer 0x16045c, reached from `transition_draw_sky` in the boot program).
The level overlays' star code (the eight star levels' dispatches and generators, level00 0x288ec0 and every own-overlay
copy) calls no `srand` (no `jal 0x1160c8`), and level init (level00 0x241940, level06 0x250600, level01 0x255958)
calls `srand(0x4d2 = 1234)`. So in a level the stream is: `srand(1234)` at load → load-time moby pass → per main-loop
iteration: gameplay tick consumers (mobys, particles, …) → frame render, where the sky dispatch generates the stars
on the first frame (levels 00: 244·9 + 12·2 = 2220 draws; 05/07/13/15: 1096; 02: 2234; 06: 2160; 17: 2208) and then
draws one `rand()` per twinkling star (plus the level-06 fixed stars' 1–2) **every rendered frame**. Star levels
therefore advance the shared stream by 120–246 values per frame, after that frame's tick; the port does the same
(`rc_engine::sky_stars`, after `particle_render::tick`).

## 6. Textures — confidence high

* **Source**: core index header +0x50 count / +0x54 offset `part_textures` (Novalis 41 entries,
  `{palette, unk4, texture, side}`), +0x64 `part_bank_offset` (core_data), +0x6c `part_defs`.
  Every Novalis entry: side 32, unk4 0, palette/texture offsets into the part bank (0xe800 bytes).
  Format = level textures: 32×32 PSMT8, 256×RGBA32 CLUT, CSM1-swizzled, alpha 0x80 = 1.0
  (textures_rac1.md §8.1). Not in gs_ram; uploaded from EE memory on demand every frame.
* **part_defs** (header `{81, texture count, data_off, data_size}` + 81 offsets): one frame list per
  **type** (u8 texture indices). `ParseParticleTexs` copies the index blob to 0x1b2700 and stores
  pointers in 0x1b2500[type]; offset 0 points at the blob start (Novalis blob starts 00 01 02 … 0a, which
  is exactly the 9-frame run type 1 animates through with a null def). Frame lengths are implicit:
  each update function knows its own frame count (type 1: `def[(T − t)·9 / T]`).
* **0x1b1f00** (128 × u64): `lo = (bank + palette)·16 | unk4`, `hi = (bank + texture)·16 | log2 side`.
* **Upload** (`_part_load_tex`, called by `PartProc` when the per-frame SPR copy of the entry has bit 63
  clear): appends to the GIF paging list 0x16d200 (shared with `GetEffectTex`, count 0x15f418) and
  allocates GS memory from 0x15ee74 (byte address, reset each frame by `SetupGifPaging`): CLUT
  0x400 − unk4·0x100 bytes, then image 0x400 bytes. TEX0 = template 0x1607d0 (TBW 1, PSMT8, TW=TH=5,
  TCC 1, TFX MODULATE, CPSM CT32, CSM1, CLD 4) | TBP0 = image>>8 | CBP = clut>>8 | CSA = unk4. Each
  texture is uploaded at most once per frame, and only if a visible particle uses it.

## 7. Rendering (`PartProc`, VU1 program 221571 at 0x101088) — confidence high

Frame position: after mobys, the registered draw callbacks `0x21b030` (list filled by `RegisterDrawCallback` 0x21afe0; the second list, filled by
0x21b198, is drained by 0x21b1e8 after the particles) and the lens-flare pass
(`0x20f480`, FX textures, ALPHA 0x8000000048), gated by `0x15f3f4 & 0x20` and `0x16a4a8`; then
`CLAMP_1 = 5`, a 3-qw DIRECT (0x1c2970): **TEST_1 = 0x5380b** (alpha test A ≥ 0x80, fail → RGB only,
no Z/alpha write; ZTST GEQUAL), **ALPHA_1 = 0x44**; then `PartProc`; resident VU1 id = 8.

**Pass 1 (cull, EE + VU0)**: SPR double-buffered copies of 32 records. For each live record, camera
space c = R·(pos − cam) (R = rotation-only view 0x167100, cam = 0x167240; x right, y down, z forward).
Kinds 0/1: z12 = ftoi12(c.z); cull if z12 ≤ near12 (n<<10) or z12 ≥ min(far12 (m<<17), global far
gp−0x6a84 = 0x1f4000 = 500 units, or 0x40000 = 64 units when the alternate/underwater fog 0x167494 is
on); kind 1 also culls when z12 − ftoi12(r) < 0x100; frustum: |c.x| − r·√(1+tx²) ≤ tx·c.z and same for y
(0x16d0a0, 0x16cf70), with r = size/420000. Kinds 2/3: both ends through the guard-band clip matrix
0x1671c0 (×1024) with xy × 0.9375·(4,4); cull when both ends are outside the same plane; depth tests
use the midpoint. Survivors go into 1024 depth buckets, `min(z12 >> 10, 1023)` (0.25-unit bins),
head-inserted linked lists. Records that were visible (0x20) and are now culled get 0x20 cleared.

**Order**: buckets 1023 → 0 (**back to front**); inside a bucket, the later-processed (higher index)
record first. Visible records get flag 0x20.

**Pass 2 (packets)**: chain = REF to the program (qwc at 0x101070), CNT 6 qw → VU 0..5
(view·proj 0x167140 rows 0–2 ×1024, 0x16d0d0 = (cf10,…), 0x16d060 = (2048, 2048, 8388112, fog
intercept)), REF 14 qw at 0x1c31b0 (VU 6..15 constants, then `MSCALF 0; BASE 0; OFFSET 0x200` and a
DIRECT TEX1_1 = 0xff9000000120 (bilinear, no mips), CLAMP_1 = 5). Then batches of ≤ ~27 items
(UNPACK with FLG to the double buffer, first item's +0x2c = batch size in qw, `MSCALF 0x14`).
Per item (EE side), p = pos − cam, fade m = min((far12 − z12) >> 4, z12 − near12, 0x1000):

| kind | qw sent | VU output (one GIF packet per item, PACKED, last gets EOP) |
|---|---|---|
| 0 sprite | p; RGBA with A = byte7·m >> 12; (cos θ, sin θ)·Q, byte3; TEX0 | tag 0x1c3200: PRIM 0x54 (tri-strip, TME, ABE, flat, no fog); ALPHA_1 = byte3; TEX0; RGBAQ; 4 × (ST, XYZ2) |
| 1 flat | p (marker 1); RGBA faded; (cos, sin)·size/420000; TEX0 | same tag; corners in world XY, each projected |
| 2 line | p1 (−1); RGBA1 raw; p2; RGBA2 (+0xc) raw | tag 0x1c3210: PRIM 0x49 (line, Gouraud, ABE); ALPHA 0x44; RGBAQ, XYZ2 × 2 |
| 3 ribbon | p1 (−2), w1 = +0x1c; p2, k = +0x2c; n̂·w1; RGBA1/RGBA2 with A = byte7/byte15 faded; TEX0, byte3 | tag 0x1c3220: PRIM 0x5c (tri-strip, Gouraud, TME, ABE) |

θ = byte8·2π/256 (cos table 0x166d00, 256 floats; sin = entry (b+192) & 255). Sprite size:
Q = size·[0x1607ec] / (c.z + 0.5) with 0x1607ec = (W/2)/(tx·210000) = 256/(0.63·210000), i.e. half-diagonal
**in frame-buffer pixels** = 406.35·(size/210000)/(z + 0.5). VU (L1): centre S = VP·(p,1); Q' = cf10/S.w
(= n/z); XYZ = S.xyz·Q' + (2048, 2048, 8388112); a = (c, 1.0625·s), b = (−s, 1.0625·c) (qw6 = (1, 1.0625));
vertices S+a (ST 0,0), S+b (1,0), S−b (0,1), S−a (1,1), all at the centre's Z, ST Q = 1 (affine).
Flat quad (L2): world corners p + (c, s), p + (−s, c), p + (s, −c), p − (c, s), same STs. Ribbon (L4):
n̂ = normalize(cross(p2 − p1, (p1 + p2)/2)) (`vopmula/vopmsub`, sign med), vertices p1 + n̂w1, p1 − n̂w1,
p2 + n̂w1k, p2 − n̂w1k with STs (0,0), (1,0), (0,1), (1,1) and colours 1, 1, 2, 2. Line (L3): two
projected points, 1 px. Texture: TCC 1, MODULATE (C = Ct·Cv >> 7, A = At·Av >> 7), bilinear, clamp.
**GS state summary**: blend per record (0x44 normal / 0x48 additive, lines always 0x44); depth test
GEQUAL on; depth **and alpha** written only where the modulated A ≥ 0x80, colour always; no fog;
ZBUF inherited (ZMSK 0 in the normal frame, med).

## 8. Sky stars (same renderer) — confidence high for the boot template, med per level

Boot `update_sky_effects` 0x22ae70 (template; the star levels 00, 02, 05, 06, 07, 13, 15, 17 have their
own copies in the overlay, counts in sky_render_notes.md §6): on the first frame, sprite count =
0x100, `srand(12345)`, 0x20-byte records at sky header +0x1c: records < 0xf6 are fixed twinklers
(byte0 1, byte2 1, byte3 0x48, rot = rand_angle, size = (32 + randi(24))/256, position on a radius-50
sphere, azimuth −3 + 0.2·rand_angle, polar 1.2 + 0.09·rand_angle, base colour 0x30505050 + randi(32)<<24
+ randi(24)<<8 or <<16), the rest move (byte0 0, two u16 angle counters +1 per frame, size 0.16). Each
frame twinklers get RGB = base + random (rand()>>16 bit fields) − 0x20 per channel. `SkySpriteProc`
0x22bba0 then emits the kind-0 packet (same constants block, same program, 30 per batch, no depth
sort, no near/far fade) with rotation-only view (stars stay at infinity), Q = size·[0x18cf0c]/z (boot, med),
sin/cos via VU0 `vcallms 0x190/0x192`, TEX0 from the sky's texture table (+0x10), loaded with its own
`_load_tex` 0x22bec4. Drawn between sky shells 1 and 2 (after it `ALPHA_1 = 0x8000000044`). Novalis has none.

## 9. Novalis effects at level start (no input)

From the vtbl at 0x20bb00 (12-byte `{o_class, update, …}`) × gameplay instances × call graph:
1. **Class 27 emitters ×10 (type 6), high** — the only unconditional source. All ten share one pvar
   set: 1 particle/tick, life 150 ticks, additive, texture def[4][0] = part texture 0, size 200000–310000
   (half-diagonal 0.95–1.48 u) +500/tick, alpha 1.0 −0.0067/tick, grey 0.15 rising by (0.00119, 0.0011,
   0.001)/tick, spin −0.002 turns/tick, velocity (0.537, 0.832, −0.139) ±0.3° then clamped to 0.015/tick
   after the first move, near 1 u / far 224 u, kill below (0,0,0) or above 1000. At (213.3, 380.3, 79.9),
   (210.0, 390.2, 79.9), (212.8, 385.3, 80.8), (196.6, 163.4, 66.3), (119.9, 325.0, 85.3), (149.0, 171.8,
   56.8), (149.5, 166.5, 56.8), (51.9, 109.8, 53.7), (56.8, 110.7, 107.3), (242.8, 227.4, 61.8); each only
   emits while its r = 20 sphere passes `FastBSphereCheck(240)` and the moby is within update distance.
2. **Class 1134 ×3 (types 59, 60), med** — hovering, spinning collectible (state 1; state 2 attaches it to
   Ratchet, sets a per-level flag and memcard-saves: likely the gold bolts); sparkles every tick.
3. **Class 751 (types 56, 57), med** — Novalis water system (21 patches); only while the camera is inside
   zone 5: mist puffs and flat ripples around (177, 165–176, 39) (a waterfall foot).
Not at start: bolts (13, type 53 on pickup), crates (500–511, types 11/13 on break), class 700 fires
(type 16; pvar[0] = 0 until triggered), 704/729/778 (type 22 bursts), 760 (the fire / smoke fields, drawn by their
own draw callback, not particles; +0x42 = 0 → no type-23 particles on Novalis: creatures.md §7),
hero dust/landing (0x2370b8 → types 34, 35, 45, 46, 47), impact sparks (type 1 via 0x2780b0).
The lens flare (0x20f480, FX textures 6+) is not a particle.

## 10. Unknowns

Semantic labels of most of the 81 types; which type spawners other overlays call; the ribbon normal's
sign; whether class 1134 is the gold bolt; ZBUF_1 state at particle time (inherited); the unreachable
bitmap-2 path of `CreatePart2` (dead in retail); frame-exact behaviour of `CollLine_Fix` for bouncing
types (collision_queries.md); nothing checked against PCSX2 yet.

## 11. Port plan

* **Formats** (`rc-formats`, new `particles.rs`): parse `part_textures` (+0x50/+0x54), `part_defs`
  (+0x6c: count 81, per-type byte lists with the "null = blob start" rule) and decode the 32×32 PSMT8
  images + CSM1 palettes from `part_bank` with the existing texture decoder; FX textures likewise from
  +0x58/+0x5c/`fx_bank`. Golden (done, see "In the port"; the C++ side has since been retired): add `rc_extract part-textures` to the C++ extractor (same decode
  routine as level textures) and compare RGBA bytes for all 19 levels; assert Novalis = 41 × 32² and the
  0x1b1f00 packing. Test the def lists against the table printed in §6.
* **Game** (`crates/rc-game`, `particles` + `rng`): newlib LCG with srand(1234) at level load as one
  shared stream; the 2048 × 64-byte pool as raw bytes with the exact allocator (hint, hw, bitmap,
  half-zeroing bug) and `UpdateParts` snapshot semantics; the type table as `fn(&mut [u8; 64], &mut Ctx)`
  starting with types 6, 56, 57, 59, 60, 1 on the PS2 float model; the class-27 emitter reading its pvars.
  Unit tests: allocator/kill sequences, RNG first values (1 → 1103527590, …), a type-6 particle's
  150-tick trajectory computed by hand from the pvars above.
* **Engine** (`rc-engine`, `particle_render.rs`): each frame replicate pass 1 on the CPU (near/far/
  frustum, fade, 1024-bucket back-to-front order) and draw instanced quads in that order with
  per-instance kind/TEX0 layer (a 2D array of the level's 32×32 textures), sprite corners built in pixel
  space from the §7 formulas (1.0625 y factor, z + 0.5), world quads/ribbons/lines as in L2–L4.
  Blend: 0x44 → `Cs·As + Cd·(1−As)`, 0x48 → `Cs·As + Cd` with As = A/128 (clamp like the sky pass).
  Z: test ≥ (reverse-Z) against the world; emulate "colour always, depth only if A ≥ 0x80" with two
  pipelines per item (colour, depth write off; then depth-only with discard A < 0x80). Draw after mobys.

## In the port (2026-09-27)

**Files.** `crates/rc-formats/src/particle_tex.rs` (first verified against the C++ oracle's `rc_extract particles`
dump, retired 2026-09-27; now snapshot-tested);
`crates/rc-game/src/rng.rs`, `crates/rc-game/src/particles.rs`, `crates/rc-game/src/particles/type06.rs`;
`crates/rc-engine/src/particle_render.rs`, `crates/rc-engine/assets/shaders/particle.wgsl`.

**Textures (verified, golden).** `part_textures` entry `{palette, unk4, texture, side}` (offsets into the part bank,
core +0x64), `part_defs` `{81, texture count, data_off = 352, data_size}` + 81 offsets relative to `part_defs`
(pointer = blob + offset − data_off, 0 → blob start, `ParseParticleTexs`), FX entries `{palette, texture, width,
height}` in the FX bank (+0x68). All 19 levels: 948 part textures (all 32×32, unk4 0, CLUT alpha ≤ 0x80) and
1057 FX textures (16..256 px, none absent), byte-identical between Rust and C++ when the snapshot hashes were generated (`particle_textures_for_every_level`;
it fails on a CLUT offset change or a changed null-offset rule). The 0x1b1f00 words are
`((bank + palette) << 4 | unk4, (bank + texture) << 4 | log2 side)` (`PartTextureEntry::runtime_words`).
Novalis texture 0 (the class-27 frame) is a soft grey smoke puff.

**RNG (verified against the disassembly).** `rand`/`srand` exactly as §5. Pinned from the code: `randf` computes
`b − a` first, then `cvt, mul, mul(2⁻¹⁵), add`; `rand_range` wraps (`subu/addiu`); `rand_vec` (0x26cb58) takes the
caller's `(lo, hi)` straight through to `randf`, then two `rand_angle`, with `fast_cos`/`fast_sin` = VU0 `vcallms
0x190/0x192`, the same range fold + x⁹ polynomial as `moby_light::vu0_sin_cos`; `ticks(n)` (0x220e30) =
`cvt.w.s(0.25 + 0.25 + n·[0x15ed68])`; π in the helpers is 0x40490fdb. The helpers use π · 2⁻¹¹ etc. on the PS2
FPU model (`rc_game::ps2v`).

**Pool / scheduler (verified).** As §2/§4, from the disassembly of 0x27c4a0 (the new hint scans from bit 0 of the
byte holding i + 1; 0x800 when full) and 0x27c5e8. The pool starts as zero bytes (on the PS2 its previous
contents are unknown). Types without a port kill themselves on their first update and are counted
(`PartStats::unported_kills`, printed by `RC_PART_STATS=1`).

**Type 6 + class 27 (verified from disassembly unless noted).**
* Emitter random arguments (the decompiler's garbled ones): offset `randf(−P.cc, P.cc)`, **`randf(−P.d0, P.d4)`**
  (y pairs the negated y range with the z range, a game quirk), `randf(−P.d4, P.d4)`; flag 0x1000: Euler
  `randf(−P.9k, P.9k)·π/180` (mul then div), with 0x4000 `ex = add_rot(ex, rx); ey = add_rot(ex, ry); ez = add_rot(ex, rz)`
  (every call gets the new ex); flag clear: `randf(P.8k, P.9k)·speed`. Then size `randf(P.a8, P.ac)`, angle
  `randf(P.a0, P.a4)`: 8 `rand()` per particle. Novalis flags 0x170e: colour, alpha, accelerate, flip at the colour
  max, random Euler velocity; no collision, no moby rotation; P.c4 = 1, P.c5 = 0 → one particle per tick.
* **In-view rule** (`FastBSphereCheck` 0x2221f0, returns −1 = skip): sphere (pos, 20) and far 240 in integer
  units (× 1024) against the block 0x16d140 written during the previous frame's render (rotation-only view rows
  0x167100, camera × 1024, (tan x, tan y), sec = 1/cos(atan tan)): culled if `c.z − r > 240`, if `c.z + r ≤ 0`
  (sign bit of `0 − (r + c.z)` clear), or if `tan·c.z − (|c.x or y| − r·sec) < 0`. Depth is along the view axis,
  not radial. The 0/1 (intersecting / fully inside) result is not modelled. Level test: `[0x15ed84] == 1` is the
  current level number (it indexes per-level tables elsewhere), so only Novalis tests visibility. The port takes
  sec = √(1 + tan²) (the PS2 value comes from `FastArcTan` + `fast_cos`, inferred to agree to the last bits only).
* Colour channel quirk: each tick rebuilds the 16-bit value as `trunc(v·255) << 8 | (trunc(v·65535) & 0xff)`,
  which is up to 256 short of `trunc(v·65535)`; with the Novalis step (0.00119/tick) the grey climbs only
  ~0.0004/tick and stalls near 0.20 instead of reaching 0.33 (§9's "rising by …" is the nominal step). Follows
  from the instructions; not yet checked on PCSX2.
* Particle lifetime 150 updates exactly (alpha reaches ≤ 0 and the timer on the same update); speed after the first
  move is clamped to 0.015/tick (`fun_001f9c90` = VU `sqrt` of `(x² + y²) + 1·z²`, rescale by `max/len` only when
  `max − len` is not positive). Unit tests cover the spawn record bytes, a full 150-tick trajectory and the stream use.
* Not ported: the collision branch (flag 1: `CollLine_Fix`, reflect by P.3c, re-place at hit + 0.055·n; counted),
  particle +0x1c (the game stores an uninitialised stack word; kept), moby+0x1c (copied into the spawn position's w;
  0 here). The load-time moby pass (every update once before the first frame, `FUN_002792d0`) runs the emitters once
  with the initial camera (the PS2's 0x16d140 contents at that moment are unknown: inferred).

**Tick placement.** `rc-engine` runs, per 60 Hz `FixedUpdate` tick: the ten emitters in instance order (activity
gate: update distance 0xff on all ten), then `UpdateParts` — i.e. after the moby updates and before the camera, as
in 0x2a4080. Nothing else consumes `rand` yet, so the stream only matches the PS2 while no other ported consumer
exists (on the PS2 other mobys draw from it every frame).

**Renderer.** `PartProc` pass 1 on the CPU per frame (near/far from byte9 with the 500-unit global far,
frustum with r = size/420000, fade `min((far12 − z12) >> 4, z12 − near12, 0x1000)`, 1024 buckets back to front,
higher index first in a bucket, flag 0x20 written back); kind-0 sprites with `Q = size·256/(0.63·210000)/(z + 0.5)`
pixels, the 1.0625 y factor and 256-step rotation, corners added in clip space at the centre's depth; MODULATE,
bilinear, clamp, no fog. TEST_1 0x5380b as two draws per blend group; ALPHA 0x44/0x48 as one premultiplied blend
(`One, OneMinusSrcAlpha`, alpha As or 0). The additive group draws B (A < 0x80, no Z) before A (Z write) so that a
particle's opaque core does not hide the soft edges of those behind it (in the GS order a B pixel is only rejected
by A pixels drawn earlier); the normal group draws A then B. Deviations: linear-light blending on the sRGB target (since
2026-09-27 display bytes, see "Display-space blending" below), As > 0x80 clamps in the 0x44 destination factor, 0x44 and
0x48 particles are not interleaved, no underwater far, kinds 1–3 counted and not drawn (kind 1 since 2026-09-28, kinds 2
/ 3 since 2026-09-27), textures uploaded once (no paging).

**Checked on screen** (Novalis, `RC_CAM=141,160,61,150.5,170,57.5`, frames 120 and 300): the pair of emitters over
the crater (149, 171.8, 56.8)/(149.5, 166.5, 56.8) show as two soft, glowing white steam puffs about 1 unit across;
7 emitters pass the view test there, `RC_PART_STATS=1` reports 427 alive at tick 60, 847 at 120 and a steady 1043
(= 7 × 149) from tick 150 on, 894 drawn; two frame-300 captures are byte-identical; ~60 fps (vsync). The three
emitters at (210–213, 380–390, 80) sit inside the mountain mesh at the north edge (their particles are hidden by it).

## In the port (2026-09-28): crate-break dust, TNT sparks, bolt sparkles (types 13, 11, 53)

**Which types.** From the ported spawners (`moby_update::services::World::part13/part11/part53`, each re-checked
against its spawner's disassembly: 0x280698, 0x27f8f8, 0x287328) and their callers: `CrateBreakFx` 0x2eb918 spawns
six **type 13** dust puffs for every broken crate and, for TNT, up to ten **type 11** sparks (the flashes are class-0x70
*mobys*, `FlashSpawn` 0x2c20e0, and the debris are mobys of the 0x2c5218 group, whose update spawns no particle);
`CollectBolt` 0x2bc4f0 spawns two **type 53** sparkles. All three are kind-0 sprites (byte1 = 0), so no new render
kind was needed. None animates its texture: the spawner's `def[type][0]` stays.

**Files.** `crates/rc-game/src/particles/{type11,type13,type53}.rs`; `particles.rs` registers them, and `UpdateFn` /
`Particles::update_parts` now take the shared `&mut Rng` (only type 11 draws). New helpers: `fast_dec_timer_u8`
(`FUN_00220ed8`), `tween_color` (`FastTweenColor` 0x2221a8 on the VU0 model: `vsubx.w` 1 − f, `vmulaw`, `vmaddx`,
`vftoi0`, low byte per lane), `TimeBase::dt2` (0x15ed70), `Particles::frame_load` (0x15f5d0/0x15f5d4, 0 = no throttle)
and `Particles::camera` (0x167240).

* **Type 13** (update 0x280818, no RNG). Every tick: byte8 += byte 0x28 (the spawner's ±1 spin), z += +0x30 (g =
  0.05), size ×= +0x2c (`randf(1.01, 1.075)`). Phase 0 (+0x24 = 0): A = `trunc((ticks(10) − t)·(96/ticks(10))) + 0x20`;
  when the timer fires: phase 1, timer `ticks(30)`, A = 0x7f. Phase 1: A = `trunc(t·(127/ticks(30)))`, killed when
  the timer fires. RGB from +0x34 each tick. 40 updates. Note 96/10 truncates to 9.599999: at t = 5 the fade-in gives
  0x20 + 47, not + 48.
* **Type 53** (update 0x2874b8, no RNG). byte8 += byte 0x2a; with d = life − t: d < 6 → f = d/5, size = b + ((a+b)/2
  − b)·f, A = N + (N/2 − N)·f; else f = t/(life − 6), size = a + ((a+b)/2 − a)·f, A = (N >> 1)·f (a = +0x20, b = +0x24,
  N = +0x2b); pos += vel (+0x2c..0x34), then vel.z −= +0x38; killed outside [2, 1021]³ (tested first) or on the timer.
  The first sparkle of a bolt (a = 0.2g, b = g, ×210000) starts at size b, shrinks to the mean 0.6g over five
  updates and then linearly to 0.2g while A falls from N/2 to 0 (life `ticks(25)`).
* **Type 11** (update 0x27fb80; spawner 0x27f8f8 ported again as `type11::spawn`, since the update's split calls it).
  Spark phase: byte8 += 1; pos += vel; flag 4: vel.z += 9.8·dt²; vel ·= 0.6 (flag 2) or 0.8. On the tick the timer
  equals `trunc(N·0.85)` a spark of generation (+0x3a) 0 or 1 without flag 4 spawns five children (per child
  `randf(0.9, 1.1)` for the life, `randf(0.85, 1)` for the speed, then the spawner's 5 draws: 35 draws; the children
  inherit position, velocity as the base, colours, t1 = +0x38, flags, generation + 1). Without flag 4 the size is the
  larger root of A·x² + B·x + (N − t) = 0 with B = (8N − 3NS)/(8S² + 8S), A = (3N + 8BS)/(−8S) (S = +0x3c, VU `vsqrt`),
  times `0.075·d + 0.25` when the camera is closer than d = 10. Colour: `FastTweenColor(t/N, c2, c1)` (flag 2:
  `(t/N, c1, 0x0fffffff)`). Byte countdown +0x38. When the timer fires: flag 2 → a new phase of 3N ticks with flag 4
  (gravity, no size curve) and speed `+0x2c·randf(1.6, 2)`; else smoke (flag 1): colours (c2, 0x0f2f3f3f, or 0 when
  additive), life `rand_range(2N, 3N)` (×5 with flag 8) as a byte. Smoke phase: vel.z += 3·dt², pos += vel, vel ·= 0.98;
  while t > s0 = `trunc(0.75N) + 1` the colour tweens `(t − s0)/(trunc(0.25N) + 1)`; after that additive smoke dies at
  once and normal smoke fades A = 15t/s0 (integer division); an extra timer step when 0x15f5d0 > 1.
* **Not modelled.** The frame-load globals (always 0: no throttle draws, no extra smoke step).

**Tests** (`cargo xtask test-quick rc-game --filter particles`): per type a multi-tick run from a synthetic record (positions,
colours, sizes, lifetimes against independent f64 / hand formulas) and the RNG use per update (13 and 53: none;
11: 5 per spawn, 35 on the split, 1 at the phase change, 0 otherwise); `curve_size` against an f64 reference.

**Engine wiring.** `particle_render::update_parts(sim, rng)` takes `Option<&mut Rng>`; the game tick's particle hook
(`rc_game::tick::TickHooks::particles`, called as `(hooks.particles)(&self.hero, &self.camera.out, &mut self.rng,
self.counter)`) passes the game's stream (`update_parts(sim, Some(rng))` in `rc-engine/src/gameplay.rs`), so the type-11
draws are on the one stream between the hero and the camera, and sets `sim.sys.camera` (0x167240) from the hook's
`CameraView` (the previous tick's camera update). `ParticleSim::rng` is only the `RC_PLAY=0` loop's stream.

**Checked on screen** (Novalis, `RC_SCENE=0`, `RC_PLAY_SCRIPT='0-5:rstick 1 0,20-226:stick 0 -1,250-310:rstick 1
0,330-331:press SQUARE'`): the wrench breaks crates on ticks 341 and 343 (6 + 6 type-13 records, `RC_PART_STATS`
at tick 360: `6:389 13:12`); frames 350 and 370 show the soft grey puffs around the broken crates, rising and fading
(frame 342 has only the just-spawned, 0x20-alpha puffs, mostly inside the crate stack). Walking on
(`…,360-440:stick 0 -1`) collects the dropped bolts on ticks 377, 380, 384, …; frame 387 shows the type-53 star
sparkles at Ratchet's shoulders. Two runs to frame 450 give byte-identical captures and identical `RC_PLAY_TRACE`
lines (game RNG included) and stats lines. Type 11 is covered by the unit tests only: Novalis's one TNT crate
(instance 549 at (64.1, 101.7, 35.1)) is not on the scripted route.

## Blarg flyer trails (2026-09-28): what draws them, and the display-byte additive fix

**What the game draws (from the savestates' RAM, `work/trace/novalis_{spawn,idle}_ee.bin`).** All ten class-27
emitters (mobys 285–294, pvars 0x1ef4920 + 0xe0·k) are the flyers' exhausts: each flyer 619–628 (class 660, pvar
+0x120 = 2: flag bit 1 only, link +0x140 = one emitter) carries one, and the emitters sit at the flyers' joints, not
at their placed positions (§9's "steam over the crater" is only what the emitters do before the flyers move them).
The trail is nothing but their **type-6 particles**: the pool holds no line or ribbon (kinds 2/3) record at all
(spawn state: 483 type 6 + 56 type 62; idle state: 370 type 6 among explosion types 4/11/15/16), and neither
`BlargFlyerUpdate` 0x2f4428 nor `FlyerPathDriver` 0x2f5168 registers a draw callback or spawns another type. Record,
as the port creates it: byte1 = 0 (sprite), byte2 = 0 (part texture 0 = `part_defs[4][0]`), byte3 = 0x48
(additive), byte9 = 0x74, RGBA (0x26 → ~0x34 grey, A 0xff → 0 over 150 ticks), size 200000–310000 (+500/tick),
+0x2c = the emitter's pvar. Pvars in RAM = the port's (P+0x70 = 0x170e: 0x1000 is already set in the level data; it
only selects "velocity = P+0x80 turned by ±0.3°"; P+0x80 = the flyer's unit joint direction, w 1.0). A trail is one
puff per tick at the flyer's speed (1.25 u apart at 1.235 u/tick, flyer 622 / emitter 292), 150 ticks long, i.e.
100–185 units; after the first 1-unit move each puff drifts at 0.015 u/tick. So the port's particles were right.

**Why they vanished in the port: blending.** Per puff the GS adds `Cs·As >> 7` with Cs = Ct·Cv >> 7 ≤ 106·0x26 >> 7
= 31 and As = At·Av >> 7 ≤ 0x50·0xff >> 7 = 159: up to +38 display levels (≈ 0.15), 1.5–2 puffs overlapping → a
light-grey streak. The port added `lin(Cs)·As` in linear light on the sRGB target: lin(31/255) = 0.014, +0.02 linear
over a sky at display 0.6 ≈ +0.02 display: a tenth of the GS's step.

**Fix (renderer only; superseded 2026-09-27 by the general mechanism of "Display-space blending" below).** Additive
(0x48) fragments read the scene under them from Bevy's screen-space transmission snapshot
(`view_transmission_texture`: a hidden, fragment-less "opaque, reads transmission" trigger item puts one item in
Transmissive3d, whose pass copies the frame after the sky/opaque/alpha-mask passes) and output
`lin(min(Cd + (Cs·As >> 7), 255)) − lin(Cd)` with alpha 0 into the unchanged One/OneMinusSrcAlpha blend. Exact for one
additive layer over the scene; overlapping additive sprites each add their increment on the same snapshot Cd
(slightly darker than the GS's sum, by the sRGB curve's convexity), and translucent items drawn before the particles
are not in Cd. 0x44 particles unchanged (linear mix). Simulation, RNG draws and particle counts unchanged
(`RC_PART_STATS` lines identical before/after).

**Checked on screen** (`RC_SCENE=0 RC_PLAY_FLY=1 RC_SCREENSHOT_FRAME=300`, `RC_CAM=150,100,80,60,210,90` and
`120,137,92,60,210,95`): the flyers' trails show as long light-grey streaks across the sky (a formation's loop and
its climb over the hills, the flyers over the city trailing them); before the fix only a faint line was visible.
Two runs give byte-identical captures.

## In the port (2026-09-26): the hero's particles (types 25, 34, 47, 60) — hero polish

**Types** (level01 code; standard `f32`): **25** grind / cable spark (`PartType25Spawn` 0x2825f8 / update 0x282760:
additive, byte9 0x44, RGBA from channels 1.0 cooling R −0.01 / G −0.03 / B −0.05 a tick at alpha 0.6, size
`randf(5000, 30000)` −100 a tick, life 80, gravity from vel.w; each move is a world line `CollLine_Fix(pos, new, 2)`: on
a hit the spark sits at the hit point, stops, doubles in size), **34** bubble (0x2840e0 / 0x2842e0: normal blend,
rises toward 1.3 u/s, wobbles sideways to the camera yaw 0x167258, pops at its level; one `randi(20)` a tick near
the surface), **47** dust / sand puff (0x286cb0 / 0x286d80: alpha 0x28 + `randi(8)` −2 a tick, +14700 size a tick,
vel ×0.975, spin from the stale +0x30), **60** glint (0x288588 / 0x288740: refused outside [2, 1021]³, alpha falls
linearly over the life; the attached variant follows 0x13f3d0). `Particles` gained `coll` (the level mesh for
type 25; the engine clones the level collision at setup), `hero` and `cam_yaw` (set by the particle hook). Type 53's
record fill moved to `type53::fill` / `spawn` (used by `World::part53` and the hero), unchanged.

**The hero's spawns** (`rc-game/src/hero/fx.rs`): the hero code makes the spawners' draws at the game's point
(`fx::spark`, `fx::dust`, `fx::bubbles` = `0x22b140(n, 0)`, `fx::sparkle_burst` = L00 `0x2a7e20`) and queues the
record (`HeroFx::parts`); the engine's particle hook creates the queue in order right before `UpdateParts`
(`fx::create_particles`), which gives the game's pool slots (nothing else creates a particle in between). Callers: the
grind 0x28 / 0x2b and cable 0x74 sparks (`hero::boots`), the cable grab's burst, the sinking floor's sand puff
(`hero::surface`), the hurt-under-water 0x76 bubbles (`hero::damage`). The swing targets (803) spawn their glints
through `World::part60` (`moby_update::classes::swing_target`).

**Not ported (then; the water ones are wired since 2026-09-28, "The hero's water effects" below):** `0x22b140` modes
1 / 2, 0x82's / 0x6a's bubble at joint list 4, the swim's splash-countdown bubbles and splashes, the surface wake
`0x22ac40`, the burn fire `0x209ec8` (type-25 sparks plus 57 type-4 fire particles every 3..7 ticks; type 4
unported), the Hydro-Pack's jets. The spawner's own draws are made even when the pool is full (the game skips them
then; the pool never fills in play).

**Checked on screen** (`RC_SCENE=0 RC_LEVEL=14 RC_GIVE_ITEMS=29 RC_HERO_AT=157.83,262.53,51.4,-1.5565`, frame 60):
white-hot sparks at the feet on the rail cooling to orange and falling (`RC_PART_STATS`: 55 type-25 alive at tick
60); Aridia (`RC_LEVEL=2 RC_GIVE_ITEMS=12 RC_HERO_AT=71.00,271.08,47.00,1.571`, frame 50): the swing target's glint
streaking across it (14–17 type-60 alive). Two runs each: identical PNGs, traces and WAVs.

## The Bomb Glove explosion and the effect particles (2026-09-28): types 2, 4, 8, 15, 52, render kind 1

**The call tree** (level01; every call below was read in the disassembly of its caller, `0x2c3300` in full; draws =
`rand` calls in game order). The explosion is effect **mobys** plus **particles**; the translucent shells are mobys.

* **Flight** (bomb 121 state 1, `0x2c3300`): glow fade; on ticks with `0x15f5cc & 7 == 0` and not in water, the
  trail: `TRAIL_COUNT` (gp 0x16142c = 1) × `PartType02Spawn` 0x27dc98 — velocity 1 = 0.5·bomb velocity + `rand_vec(0,
  1·dt)`, velocity 2 = 0 minus 4·dt²·B, sizes `randf(0.25, 0.15)` / `randf(0.15, 0.05)`, colours
  `FastTweenColor(randf(0,1), 0x804080ff, 0x8040ffff)` / `(…, 0x80104040, 0x80002080)`, phases A = `1 + randf(0,1)`, B =
  `30·(1 + randf(±0.5))`, C = `10·(1 + randf(±0.5))` ticks, texture `def[25]`, additive (11 draws). Spin, move,
  gravity 11 u/s² (1.1 in water), `CollLine_Fix` on the path; water entry: `0x2b82a8` ripple, `0x2ff768` splash moby,
  15 × `PartType35Spawn` 0x2845a8, bubbles `0x2840e0` while sinking (**not ported**).
* **Explosion** (+0xbc = 1): sphere 0.5 hit (`0x26f8f8`); 10 low fireballs + 4 high + 1 toward the camera
  (`0x2c4c20` → class 122, `+0xbc` = 0 / 1 / 1); 1..4 smoke rings = **type 11** (`PartType11Spawn` 0x27f8f8,
  400000 size, 8–10 u/s, colours `0x20a980` / `0x20a998`); flashes = **class 1192 mobys** (`0x309a68`): with the camera
  farther than 9: size 4 (0x7f,0x7f,0x7f, α 0x20, 15 ticks) and 4 (0x7f,0x20,0, α 0x20, 24); always 4 (0x7f,0x3f,0,
  α 0x30, 20), 3.5 (0x60,0x10,0, α 0x40, 27), 3 (0x20,0,0, α 0x20, 29); class sound 0; in water only: `0x2840e0` debris,
  the scorch `PartType64Spawn` 0x288d90 ×100 and `PartType15Spawn` ×20 (+0x68 ≠ 0 and the ground within 1.2 of the
  water) (**not ported**); camera shake `0.4 − 0.0175·d` for 25 ticks; the explosion light `0x2f3570(0x20a930)` (class
  639). State 2: the growing damage sphere (0.5 → 2.5 over 15 ticks), then `DeleteMoby`.
* **Fireball** (class 122, `0x2c4d88`): with +0xbc bit 0 (the 5 high ones), every tick one **type-4** puff
  (`PartType04Spawn` 0x27e538): `dt`-long random direction, colour `0x161438[randi(4)]` (0x2fffffff white, 0x2f00ffff
  yellow, 0x2f007fff orange, 0x2f004fff deep orange) fading to 0x4fff over `ticks(45)`, size 50 → 170 (×1000), additive
  (5 draws). These puffs are the **bright fire trails that arc away from the blast**: the fireballs fly at 3.5–10 u/s
  with gravity 14.6 u/s² and leave a puff every tick. Then spin, move, fall, shrink over the last quarter of 60..120
  ticks.
* **Flash** (class 1192, `FlashUpdate` 0x2c22a8): scale `(T − t)·full/T` (0 → size × class scale over T ticks), all
  three Euler angles + π·dt a tick, alpha `t·α0/(T/2)` over the second half. Drawn by the moby renderer as a
  translucent moby (moby+0x23 < 0x80; `moby_render::MobyBlend`, moby_render_notes.md §8). The nested shells of the
  PCSX2 frames are these flashes: the biggest (size 4, α 0x30, orange) is the faint outer ball, the 3.5 / 3 ones (α 0x40
  / 0x20, red) grow slower inside it (at 10 ticks: radius 2.0 / 1.3 / 1.0 units); the fire inside is the type-11 rings
  (additive) and the type-4 puffs. Class 1192 (and 0x70, the crates' / creatures' flash) is **one** 320-triangle
  sphere with one texture (Novalis moby texture 211; 0x70: 23), class scale 0.0416 and header sphere 24576 packed units,
  i.e. a radius of about 1 unit per unit of size; mode bits 0 (alpha-mix, not additive); `FlashUpdate` has no texture
  scroll: the moving fire pattern is the three-axis spin. So the two visible shells are **two flash mobys of the same
  class**, each with its own size, colour (ambient), alpha and timer; the game draws them in moby-array order (the port:
  back to front by position, equal positions by entity; the result is the same for these faint shells).
* **Fireball shards** (class 122): a 16-triangle untextured solid (texture −1: the lit vertex colour, ambient
  0x7f7f7f), scale 0.0033: the small grey-white shards flying out of the blast in the PCSX2 frames, each high one
  leaving its type-4 fire trail.
* **Light** (class 639, `0x2f3748`, template 0x20a930): offset z + 1, radius 20, red/green/blue up by 1.28 a tick to
  2.55 / 2.55 / 1.0, then down by 0.09 / 0.12 / 0.13 a tick, life 70, a 3-tick delay; the radius is drawn with
  `randf(20, 20)` every tick (flag 0x80000: 1 draw). It owns a point-light slot (crate `rc_game::point_lights`).

**Types** (`crates/rc-game/src/particles/type{02,04,08,15,52}.rs`, each module doc has the record and the update;
standard `f32`; spawners in the modules, `creature::fx::part02/04/08/15/52` wrap them for the moby code):

| type | spawner / update | who | draws in the update | kind |
|---|---|---|---|---|
| 2 | 0x27dc98 / 0x27de70 | bomb trail, amoeboid goo bursts (`fx::goo_burst` = 0x2ef770) | none | 0 |
| 4 | 0x27e538 / 0x27e650 | fireball smoke / fire trails | none | 0 |
| 8 | 0x27f2b0 / 0x27f3a0 | creature explosion puffs (`SpawnBeamExplosion`) | none | 0 |
| 15 | 0x280bd0 / 0x280d08 | creature explosion streaks (with split: a child streak on odd ticks) | 6 per child + the child's 1 | 0 |
| 52 | 0x287158 / 0x287288 | amoeboid goo drips (`fx::goo_drips` = 0x2ef560) | none | **1** |

Type 2 keeps its two velocities **packed** in one word each (`0x276380`: 8 bits per axis around 127 on a shared scale
`clamp(trunc(max|v|·10000/63), 1, 255)·0.0001`; unpacked by `0x2764a8`); the port stores and reads the same words, so
the blobs move in the game's quantised steps. Types 2 and 15 read the tick counter's parity (`Particles::counter`, set
by the particle hook). Type 8 animates its texture through 11 frames of `def[8]`.

**Render kind 1** (flat quad, type 52): `particle_render.rs` draws it with the world XY corners of L2 (§7), projected per
corner, same blend and alpha split as the sprites; the extra cull `z12 − ftoi12(r) < 0x100`. Kinds 2/3 were not
drawn then (drawn since 2026-09-27, see "Display-space blending").

**RNG.** With these spawners and the light's slot, a dry bomb explosion makes exactly the game's draws: `bomb.rs`
`tests::dry_explosion_rand_stream_matches_the_game_ledger` checks every tick of a flight and explosion against a ledger
of the per-call draw counts above (11 per trail tick, 80 + 28 + 6 + 10·rings + 3·flashes at the explosion, 5 per high
fireball update, 1 per light update), with nothing left in `FxStats::unported`. Before this pass the trail, smoke and
light draws were skipped, so the stream diverged after every explosion. **Still diverging**: the creature knockback's burn sparks (`0x271258`) and water splash (`0x2ff768`), the beam explosion's debris burst (`0x2c4c20` from `SpawnBeamExplosion`).

**Checked on screen**: `RC_SCENE=0 RC_GIVE_ITEMS=10 RC_HERO_AT=143.42,125.81,57.0,0 RC_PLAY_SCRIPT='20-21:press CIRCLE'`,
frames 30–70 (docs/plan/moby_render_notes.md §8 has the before/after).

## In the port (2026-09-27): the waterfall foam (types 56, 57) and what 760 is

**The waterfall foam is particles, 760 is not.** The foam at the foot of Novalis's fall is the ripple manager 751's
zone-5 spray (world_animation.md §2, `0x2fd750..0x2fd944`): every tick while the camera is in zone 5, one flat foam
ring (type 57) at (177 ± 0.6, 176 − 11·row/20 ± 0.1, 39), `row` = `0x1fa6a0[counter % 20]`, size 1.0, spin
`randf(0.4, 0.5)`, drifting +0.02 x a tick; then 20 steps of 0.05 down the fall, each with odds 1/32 of a mist puff
(type 56) at (178 ± 0.6, 176 − 11·t ± 0.1, 39 − `randf(0, 0.2)`), size `randf(4, 10)`, still. Class 760, which the
survey called "waterfall foam", is the **fire and smoke** on the bombed buildings: its own draw callback, not the
particle system (creatures.md §7, `moby_update::classes::fire_field`).

**Types** (level01 code, read in full; standard `f32`):
* **56** mist puff (`PartType56Spawn` 0x287b00 / update 0x287bd8): sprite, additive, byte9 0x24, texture `def[56][0]`,
  grey `0x60 + 2·randi(16)` with alpha 0x40, rotation `randi(0x100)` (the spawner's two draws, only with a record).
  Update: vel ×0.975 (xyz), pos += vel, size + 6300, grey − 4 a tick with alpha 0x60; killed below 1. `particles/type56.rs`.
* **57** flat foam ring (`PartType57Spawn` 0x287c80 / update 0x287d90): **kind 1** (flat quad in world XY; the renderer
  draws kind 1), additive, byte9 0x2c, texture `def[57][0]`, alpha `2·randi(16)`, timer 0x80, size `size·210000`,
  angle `randf(0, 256)` (+0x30, byte8), spin +0x34 (the spawner's two draws). Update: pos += vel, size + 5880, spin
  ×0.98, angle += spin, timer − 1 (killed at ≤ 0), alpha = t ≤ 64 ? t : 128 − t. `particles/type57.rs`.

**Spawns** (`rc_game::water::RippleSim::tick_with`, the counter and the particle system passed by the 751 external
update in `rc-engine` gameplay.rs / `tools/trace` port_sim): the caller's draws, then the spawner's; without a particle
system the spawners' draws are still made (the stream stays right). The old code made only the caller's draws (4 fewer
per puff, 2 per ring).

**Checked:** unit tests of both types (lifetimes, sizes, alpha shapes); `novalis_world.rs`
`novalis_waterfall_foam_spawns_rings_and_mist` (600 ticks in zone 5: 127 rings alive, ~400 puffs, deterministic);
engine `RC_HERO_AT=177,184,41,-1.5708 RC_PLAY_FLY=1 RC_CAM=168,171,44,178,170,39` frame 200: the white foam on the pool at
the foot of the fall (`RC_PART_STATS`: 56:11–15, 57:120–127 alive).

## Display-space blending, the effect particles and the draw callbacks (2026-09-27)

User decision 2026-09-27: "Let's go with the PS2-style blending glow on effects" (hardware_fidelity_layers.md, Open
decisions → decided; the layer is a row of "Result-level reproductions").

**Why the effects were dim (root causes).**
1. *Linear-light blending of stacked effects.* The GS adds `Cs·As >> 7` to display bytes and clamps each draw at 255.
   The arrival crash's orb trail (type 23, `FUN_00278810`) puts, per trail point, two orange glow puffs (texture 0: grey
   0x50 at a GS alpha ≤ 0x29, so +25 display levels each at alpha 0x6e) and three white cores (vertex colour 0xff: Cs =
   2·Ct, +58 each) on top of each other; the GS sums them to white. The earlier fix blended each additive sprite
   against one snapshot of the frame, so overlapping sprites added *linear* increments: eight +25 layers on grass (30)
   gave 139 instead of 255 (and the orange hue washed out to grey-green). Measured on the crash frame: the orbs rose
   +74 / +53 / +32 over the grass (30, 41, 16); PCSX2 shows them saturated (255, 255, ~215).
2. *The particles ignored the view's field of view.* `0x1607ec` (the sprite size in pixels) and the cull's tan_x are
   recomputed by `UpdateViewContext` 0x219580 from 0x16cf70 whenever the FOV changes; the port used the default 0.63.
   The scenes narrow the FOV, so every scene particle was drawn smaller than the game's (less overlap, dimmer). Now
   `particle_render::view_tans` reads the camera's `GameProjection` (sizes, frustum cull, the emitters'
   `FastBSphereCheck` view, and the star cull `sky_stars.rs`).
3. *Vertex colours above 0x80* were **not** clamped anywhere in the chain (checked: `particle.wgsl` MODULATE is
   `min(⌊Ct·Cv/128⌋, 255)`, `moby.wgsl` `min(t·c, 1)` with c = byte/128, the callback shader likewise), so the
   handoff's candidate (b) was not a cause.

**The mechanism (`rc-engine/src/display_blend.rs`).** Every effect entity carries `DisplayEffect` and its pipeline
targets `Rgba8Unorm` (`display_blend::specialize`): the particle draws, the translucent / additive effect mobys
(`GsPass::EffectMix` / `AdditiveNoZ`, `moby.wgsl` `DISPLAY_BLEND_*`), the draw-callback primitives (`fx_draw.rs`
`FxPrimMaterial`, `fx_prim.wgsl`, which the fire fields now share). In the render world, before Bevy's transparent pass
their `Transparent3d` items are taken out of the view's phase (`split_effects`); after it, `effect_pass` converts the
frame into an `Rgba8Unorm` target holding the display bytes (`effect_blit.wgsl` `to_display`; MSAA: a multisampled copy
resolved into it), draws the items there in their sorted order with ordinary hardware blending (`One, OneMinusSrcAlpha`
on the premultiplied GS terms of `display_blend.wgsl` `gs_add` / `gs_mix`) against the view's depth buffer, and
converts back (`to_linear`); the items are then put back into Bevy's retained phase. Both conversions round-trip every
byte. Order: all effects after all world translucent draws (effect mobys by depth, list-1 callbacks band 9e5,
particles 1e6, list-2 callbacks 2e6 / 3e6). **Cost**: nothing without effect items; with them two full-frame blits and
the MSAA resolve of the effect target. Free-running (`RC_NOVSYNC=1`, 2048×1664, MSAA 4): flyer view 118 fps before and
after; nanotech close-up 91 fps after vs 76–87 before (the "after" build also carries the concurrent world-light work),
i.e. below the noise of these runs.

**The nanotech glow (`0x301c00`, `fx_draw.rs` `nanotech_prims`).** Registered by the cluster (`pickup.rs`, list 1)
every tick it is in view; drawn from overlay tables found through the function's relocations (the same code on all 19
levels: ordinals of its `lui/addiu`, gp and absolute references, `NanotechTables::parse`): a camera-facing hemisphere of
radius 0.3 (290 vertices, three strips, FX 21 × 0x50804040, ALPHA 0x44: the dark translucent ball), a 32-quad halo
ring 0.3..0.44 (FX 11 radial glow, colour 0x802020 with alpha `bob/0.06·64 + 128`: the pulsing blue-violet glow), and on
the crate the glass's sphere-mapped sheen (4 quads, 0x107f7f7f). The dotted rings are the orbs' type-62 trails (life 40
when the game camera is within 10 units, 5 otherwise).

**New particle types** (`crates/rc-game/src/particles/`, standard `f32`, the game's draws at its points; module docs have
the records): 16 (smoke; spawner throttle draws in `type16::throttled`), 19 / 55 (ribbons, kind 3), 22 (rising puffs;
the VU0 update read from the disassembly), 26 (glow riding a moby), 35 (drops), 45 / 66 (flat rings), 46 (water rings),
64 (bursting scorch). Types 26 / 55 follow mobys through `Particles::joint_anchors`, which the moby loop fills after its
pass (`World::refresh_particle_anchors`). Callers wired with their real arguments: the fires 700 (16), the breakables
704 / 729 / 778 / 779 (22), the gunship's shells and embers (26, 22), the path enemies' shots and jet exhaust (26, 22),
the fire fields' smoke (23, level 00), the cutscene driver's scene-1 rings / drops (46, 35) and the splash class 775
(`classes/splash.rs`, `FUN_002ff768` / update `0x2ff810`), the infobot's thrusters `0x278450` (23, from its root joint:
`SceneActorState::joint_matrix`). The bomb's water entry and underwater burst are wired since 2026-09-28 (`bomb_water.rs`, docs/plan/breakables.md
§5; `0x2bfe40` is the Mine Glove's mine, class 74, with the same calls). **Not wired**: the Comet-Strike's hit ribbons (`0x2bdd20`, type 19) and `0x2c72c8`, the joint ribbon's caller
`0x2e8dd8` (type 55).

**Renderer kinds 2 / 3**: see "In the port (2026-09-27)" § Pass 1 / `particle_render.rs` module doc; checked with a
temporary test spawn (a line and type-19 streaks over the nanotech crate), then removed; no ported caller spawns them yet.

**The ship glass (`0x2a70a8`)**: drawn since 2026-09-28 in the frame of the ship's **joint list 0** (the canopy
joint; `0x264508`'s argument is a list index), docs/plan/breakables.md §4.

**Checked** (engine, frame-exact, scratch `effects/shots/`): `cmp_trail.png` (the crash frame 132: before / after /
PCSX2 crop), `cmp_nanotech.png` (before / after / `images/37.webp`), `cmp_flyer.png`, `cmp_bomb.png` (frames 40 / 50 / 65
before and after, PCSX2 frames 20–23); scene 1 at frames 490 / 500 (the Plumber's splash, rings and drops; stats: types
35 and 46 alive, no unported kills), scene 4 at 900. Two runs of the crash frame and of the bomb frame 50 give identical
PNGs. `cargo test --workspace` green (the bomb rand ledger and `novalis_hero_digest` included).


## The hero's water effects (2026-09-28): splash, wake, bow rings, spray, bubbles, breath, Hydro-Pack jets

**Root cause of "no splashes".** The particle types (34, 35, 45, 46) and the splash moby 775 were ported, but the hero
never called them: `hero::swim` recorded `SwimEvent::Splash` and the engine only applied the `Ripple` events; the
wake / ring / spray helpers, the joint-point bubbles and the breath bubbles had no port, and the walk / wade case,
the jumps and the landing had no water calls at all.

**The helpers** (level01, read from the disassembly; `rc_game::hero::swim::effects`, `hero::fx::bubbles_at`):

| helper | what (all positions on the hero 0x13f3d0, the level 0x13f640) | draws, in order |
|---|---|---|
| `0x22b3a8(rings, drops, big)` splash | big: splash moby 775 `FUN_002ff768(2.25, {x, y, W})`; rings: type 45 at ±0.3, size `randf(0.3, 0.6)`, growth 5250, level pointer &0x13f640, RGBA −1; drops: type 35, `rand_angle`, out `randf(0, 3·dt)`, up `randf(3·dt, 8·dt)`, at `(x + 8·vx, y + 8·vx, W − randf(0, 0.2))` (**the game adds vx to both x and y**, kept), kind `randi(2)`, life `rand_range(90, 120)` | moby 1; ring 3 + 4; drop 6 + 2 |
| `0x22ac40(max, after)` wake (counter 0x13fc54) | past `after`: one type-45 ring at ±0.05, size `randf(0.4, 0.6)`; counter = `randi(max)` | 3 + 4 + 1 |
| `0x22ad38(max, after)` bow rings (0x13fc58) | past `after`: two type-46 at yaw ± 45°, vel `(cos, sin)·0.6·dt` placed 18× ahead, then minus the facing's 1.5·dt; size `randf(0.4, 0.5)`, spin ±2, level pointer &0x13f640; counter = 0 (max 0) or `randi(max)` per ring | (1 + 2) × 2 |
| `0x22af48(max, after)` spray (0x13fc56) | past `after`: one type-35 drop at hero ± 0.15 + 8 × disp 0x13f450, z = W + 0.1, out `randf(0, 3·dt)` + 0.75·disp, up `randf(3·dt, 6.5·dt)`, life `rand_range(90, 120)`, kind 3 below 0.25 depth else 1; counter = `randi(max)` | 6 + 2 + 1 |
| `0x22b140(n, mode)` bubbles | type 34 at the feet (mode 0: ±0.1, −0.5..0.1), Ratchet's joint lists 0 / 0xe (1) or 0x17 / 0x16 (2), alternating; drift 0.7 × disp (1.15 in 0x34) ± 0.4·dt, down `randf(−dt, 0)`; size `randf(4200, 7350)`; level −1 | mode 0: 13, modes 1 / 2: 10 |
| breath (inline, 0x2381bc / 0x238934) | type 34 size 25200 at joint list 4 + 0.4 along Ratchet's x row, vel = disp + 1.5·dt along that row + 2·dt up | 6 (+ the timer's draws) |
| Hydro-Pack jets (inline, 0x238244) | four type 34 at the pack moby's (class 0x261) lists 0 / 1, − 0.15 along Ratchet's x row, + `randf(0, 0.03)` per axis, vel = disp + `sph(−0.75·dt, yaw, pitch)`, size `randf(6300, 9450)`; timer 0x13fc48 = `trunc((7·dt − speed)/(7·dt)·ticks(5))` | 10 each |

The spawners' own draws are made at the call (`hero::fx::reserve` keeps the stream at the spawner's first draw for the
queued record, which the particle hook creates right before `UpdateParts`, as before); the splash moby is created by
the tick right after the hero update through the hit sink's `create_moby` (`hero::fx::create_mobys`, `splash::fill`),
before anything else can take a slot. Types 45 / 46 gained the water-level pointer the hero passes
(`type45::HERO_WATER_LEVEL`: the update reads `Particles::water_z` = 0x13f640 through it, so a ring within 0.2 of the
live level sits 0.02 above it, as the game's pointer does). The joint points are `hero::fx::joint_point` (Ratchet's
pose now, his moby as the last write-back left it: the formula of `World::joint_point`, shared as
`fx::joint_world_point`) and `hero::fx::pack_point` (the back pack placed by `HeroItemsAttach`: `fx::end`).

**Callers** (every call site of the helpers in level01; `xrefs` of 0x22ac40 / 0x22ad38 / 0x22af48 / 0x22b140 /
0x22b3a8 / 0x2840e0): hero_states.md "Swim effects". Tests: `hero::swim::tests` (entry ledger, wake, bow rings, water
jump, wading and ankle-deep water with the jump and landing), `hero::swim::effects::tests`,
`particles::type45::tests::rings_ride_the_hero_water_level`. Engine, frame-exact, scratch `hero_water/`: jumping in
(rings, drops, the splash shell, bubbles), the water jump's splash at its tick 20, wading bubbles; two runs identical
(PNGs, WAV, traces).

## Update types, 2026-09-29 (G-PRT-001, G-PRT-007)

**System or not.** Evidence from the table and the census: every particle type is its own update function (81 table
slots, `RegisterPartTypes` 0x27d4e0) with its own spawner, and the only code shared between types is the pool, the
timers (`FastDecTimer`), `FastTweenColor`, the vector helpers and the renderer's kinds. So a "type" is per-item code
(one module each, no invented abstraction); the shared *mechanisms* found are (1) **copies** (identical bodies:
clusters.tsv / the decomp), made table rows, and (2) **a record holding a moby pointer**, which 9 types need
(14, 31, 39, 61, 67, 68, 74, 78, 79): ported once as `Particles::moby_frames` / `joint_frames`, filled by the moby loop
after its pass (`World::refresh_particle_mobys`, called from `refresh_particle_anchors`) for the mobys `Particles::
moby_refs` lists; each update applies its own game test (0xfe / 0xfd, state ≥ 0x80, class change, or none).
A third shared piece is **the weather state** (types 0, 73 and `SpawnImpactSparks`): `Particles::weather` (0x160250
the camera step, 0x160260 the floor, written by class 1400) and `Particles::grid` (the level height grid).

**Level identity.** `cargo run -p rc-trace -- overlay-diff --fn L01:<update/spawner>…` (work/overlay_diff/prt001_*):
every update is `=` on all levels except the small ones the matcher could not pair: types 5 / 7 (two identical
functions: `?`), 49 / 70 (`U` cells that pair a neighbouring *spawner*, the reference extent running past the tiny
update), 67 on level 13 (`s`: level 00's superset build), 73 (level 09: a callee). Spawners outside level 01 were found by
scanning every overlay's export for `CreatePart(n)` (the per-level `CreatePart` from the 0x27c4a0 cluster).

**Def tables.** Spawners index the part-def table (0x1b2500 on level 01, 0x1b2380 on 13, 0x1b2880 on 08, 0x1b2980
on 17), usually at their own type; exceptions pinned from the addresses: type 10 takes `def[3]`, type 40 `def[12]`,
types 79 / 80 `def[53]`, type 71 `def[60]`.

### Ported (module, update / spawner, consumers)

| Type | Module | Update / spawner | Consumer | What it is |
|---|---|---|---|---|
| 68 | `type68` | L01 0x289cc0 / L13 0x280400 | **63 flying biter (ported)**: `glow_on` spawns it | the thruster ribbon (kind 3) on joint list 0 |
| 10 | `type10` | 0x27f788 / 0x27f660 | **flyer driver `classes::flyer` (ported)**, flag bit 0 (none on Novalis; the family's other classes, G-CLS-015) | engine spark puff |
| 5, 7 | `type05` | 0x27e850 (= 0x27f200) / 0x27e750 | **decoy pop, Glove of Doom canister pop, the morph's flash (ported)**; U412 (170) | growing flash |
| 18 | `type18` | 0x2815b8 / 0x281430 | **the Suck Cannon's thrown flight `react::flight` (ported)** | smoke trail |
| 74 | `type74` | 0x28a908 / 0x28a7a8 | the Sonic Summoner's jets (U371 1242; not ported) | puff in a moby's frame |
| 78 | `type78` | 0x28ae98 / 0x28ad08 | the Morph-o-Ray beam (`hero::morph_ray`, **not wired**: hero seam) | homing spark |
| 28 | `type28` | 0x2831b0 / 0x282ef0 | U397 (1281) | pulsing mote |
| 41 | `type41` | 0x2858a8 / 0x285768 | U134 (816) | held sprite |
| 69 | `type69` | 0x289df0 / L00 0x274948 | U436, U33, U439, U458, U34 | twinkle (modes 1–4: no spawner, counted) |
| 67 | `type67` | 0x2899c0 / 0x289850 | U203 (1139) | orbiting spark and trail |
| 1 | `type01` | 0x27dbc8 / 0x27daf0, `SpawnImpactSparks` 0x2780b0 | 1400 (U399 / U293), type 73 | rain splash |
| 0, 73 | `type00`, `type73` | 0x27d928 / L08 0x272cb8; 0x28a5b8 / L08 0x27f5f0 | 1400 | rain streak (kind 3), flake |
| 61 | `type61` | 0x2888f0 / L07 0x29b8d8 | U254 (1106) | joint glow pulled to the camera |
| 70 | `type70` | 0x28a078 / L00 0x274cf8 (9 overlays) | **Hoven's hero footprints** (`hero::fx::footprint`, level12 `0x228e58`); unported classes | flat decal (kind 1) |
| 14 | `type14` | 0x280ac8 / 0x280970 | class 156 (`0x2c7530`, code-made projectile) | muzzle flash |
| 31 | `type31` | 0x283be8 / L15 0x25e2c0 | L15 / L18 classes | line closing on a moby (kind 2) |
| 39 | `type39` | 0x284c98 / L08 0x279f00 | L08 class | glow held until the caller ends it |
| 43 | `type43` | 0x286238 / L08 0x27b450 | L08 class | tinted smoke fading in and out |
| 48, 50 | `type48` | 0x286e20, 0x286f90 / L05 0x29bbe0, 0x29be70 | L05 classes | glint; falling spark |
| 49 | `type49` | 0x286eb8 / L03 0x260130 | L03 / L07 / L15 classes | tracer line (kind 2) |
| 51 | `type51` | 0x287060 / L13 0x27d8d0 | L07 / L13 / L14 classes | rising dust |
| 65 | row on `type25` | 0x289388 / L07 0x29c4e8 | L07 / L15 / L17 classes | = type 25's spark (below) |
| 77 | `type77` | 0x28ac58 / L08 0x27fe00 | L08 class | drifting puff |
| 79, 80 | `type79` | 0x28b160, 0x28b2a8 / L17 0x26fb60, 0x26fdd0 | L17 classes | sparks; flashes |

**Copies (table rows, no new code).** Type 7's update 0x27f200 is type 5's 0x27e850 (one cluster hash, 176 bytes, in
every overlay). Type 65's update 0x289388 is type 25's 0x282760 without the variant's swapped colour rates; its only
spawner gives positive channels, so the variant branch cannot run and `type25::update` is the same function for it.

### Coverage (per update / spawner; every call, branch, draw and side effect)

| address | what | ported / not |
|---|---|---|
| 0x289cc0 (68) | kill: pointer 0, class ≠ +0x36, state 0xfe / 0xfd | `type68::update` (test `follows_the_joint…`) |
| | draws: `randf(0.05, 1)` then `randf(±jitter)` | ported, order tested |
| | `0x2645a8` joint point → end 1; `0x264508` joint matrix row 0 → end 2 = end 1 + unit(row 0)·(j/100 − 1)·len | ported (`joint_frames`; no joint list: the moby's origin/rows) |
| | +0x2c restored, +0x1c = w1·w | ported |
| L13 0x280400 (68 spawn) | kind 3, 0x48, byte9 from moby+0x32, def[68][0], both ends at the joint, fields +0x30..+0x3c | `type68::spawn` (test `spawn_record`); `flying_biter::glow_on` passes the game's arguments |
| 0x27f660 / 0x27f788 (10) | spawn: one `randf(60900, 90300)`, `def[3]`, ticks(87); update: alpha −0.04, kill (alpha then timer), size +12000, blue < 0.225 branch (unreachable from the spawn, ported), pos += vel, vel ·0.9, rot −1 | `type10` (test `spawn_fades…`); flyer: the speed draw then the spawn's draw (made without a system too) |
| 0x27e750 / 0x27e850 (5, 7) | life 0 → no record; +0x24 = grow/life; update `FastDecTimer__FRi`, size +step, last `ticks(6)` alpha `t·127/ticks(6)` | `type05` (test `flash_grows…`); decoy / canister: jitter (3), colour (3 raw), grow, life draws in order; chicken: `(1e6, 0, chicken pos, 0x7f ×3, ticks(15))` |
| 0x281430 / 0x2815b8 (18) | 3 offset draws, raw rand (rot), size draw; offset through the rows (row 3·w: stack garbage, taken as 0 [L]); +0x28 **stale** (not written: kept as the game) | `type18` (test `trail_puff…`); `react::flight` (5 draws, also without a system) |
| 0x28a7a8 / 0x28a908 (74) | raw rand (rot); offset = rows·(pos − moby); update: size from the descriptor (integer), vel ·(1 − 0.02), kill on 0xfe / 0xfd / null, pos = moby + rows·offset, `FastTweenColor(t/life, c1, c0)`, timer | `type74` (descriptor = `Particles::descs74`); test `rides_the_moby…` |
| 0x28ad08 / 0x28ae98 (78) | rot by mode (only `randi(255)` draws), target's damage-record word (+0x3c, never read); update ramps (≤ 5 ticks in, then out), homing when d.x ≤ 2 and d.y ≤ 2 (signed), sideways push 0.2 (gp−0x69b4 image value [L]), gravity 0x13f5e0 | `type78` (tests `ramps_and_lives`, `homes_on_its_target`); `Particles::gravity` stays (0, 0, −1): the hero hook does not write it (seam) |
| 0x282ef0 / 0x2831b0 (28) | 2 offset draws, 3 raw (colour: `rand() % 0x60`… from the disassembly), alpha draw, size draw, 2 velocity draws without rows (with rows: `MatrixMulVec3(spread, 0, −0.0025)`); update pulse / tail / kill 0.0244 | `type28` (tests: both spawn paths, `pulses_and_dies`) |
| 0x285768 / 0x2858a8 (41) | blend −1 rule, frame `def[41][k]`; timer −1 = held | `type41` |
| L00 0x274948 / 0x289df0 (69) | draws size, `randi(256)`, `rand_range(10, 30)`; update box, **modes 1–3** (moby pvars +0xd0 / +0x1f0 / +0xe0) and **mode 4** (moby + 0.2 z): no spawner sets them → counted in `PartStats::unported_branch`, position held; `randf_sym(0, 64)` a tick | `type69` |
| 0x289850 / 0x2899c0 (67) | `randi(255)` in each spawn (a head's puff: one draw a tick); mode 0 fade `0x26e3b0`, moby-riding puffs; heads: timer, last-10 fade, puff spawn, mode-1 orbit (`EulerToMatrix(0, a, 0)`, the moby rows), angle steps | `type67` (tests: orbit + trail, beam puffs) |
| 0x2780b0 + 0x27daf0 / 0x27dbc8 (1) | grid height + dz, camera + 5 test, `CollLine_Fix(0x12)` (world mesh only here [L]), xy distance alpha; splash z + 0.1 (the game also moves the caller's hit point), 10-frame animation | `type01::impact_sparks`, `impact_sparks_here` |
| L08 0x272cb8 / 0x27d928 (0) | kinds 0 / 1 / 2 (one draw), ribbon, weather step, the cell-change floor test (kind 2 skips it), gravity 9.8·dt², floor drift | `type00` |
| L08 0x27f5f0 / 0x28a5b8 (73) | two draws (the kind's dropped `randf`, the rotation); splash on landing (`SpawnImpactSparks`) | `type73` |
| L07 0x29b8d8 / 0x2888f0 (61) | randf, raw rand ×2; update frame step, size easing, alpha pulse, pull to the camera | `type61` |
| others (70, 14, 31, 39, 43, 48, 50, 49, 51, 77, 79, 80) | every field and branch as in the module docs; no sounds, lights or hits in any | their modules, one test each |
| **sounds / lights / hits / other mobys** | none of the ported types plays a sound, adds a light or touches another moby; the only hits come from types 40 and 58 (not ported: G-PRT-008) | — |

### Not ported (with the reason)

| Type | Why |
|---|---|
| 40 (0x284f70 / 0x284d88, U323 702 L10), 58 (0x288068 / 0x287e70, U254 1106 L07) | they **deliver hits** from inside `UpdateParts`: 40's flame line `CollLine_Fix(…, 0, moby, hit template 0x10001)` and its splits; 58's fire emitter `FUN_0026eaa8(…, hero moby, …)` when Ratchet is within √0.3. A particle → hit-record seam is a new shared system (G-PRT-008); with it both port as per-type code |
| 71 (0x28a0e0 / L15 0x2647f0, L17) | the packed-char vectors (`fun_00214a98` / `0x2764a8`) and the hero point 0x13f420: portable, left for its level-15/17 consumer |
| 42 | its only spawner (0x2858f0) is called from its own update: nothing creates the first one |
| 3, 9, 17, 20, 24, 29, 30, 33, 36, 37, 38, 54, 63, 75, 76 | no `CreatePart(n)` in any overlay (no spawner, no consumer): not ported (no speculative ports) |

**Native, not emulated.** All new updates are plain `f32` (the game's operation order, no PS2 float model); colours
through the existing `tween_color`; draw kinds 0–3 through the existing `particle_render` (kind 3 ribbons for 0 / 68,
kind 2 lines for 31 / 49, kind 1 decals for 70) with `display_blend` and the level's part textures; no GS/VU modelling.
[L] items: type 18's row-3 term, type 78's 0.2 push (image value), the world-only line tests (types 1 / 49), a moby
missing from `moby_frames` leaving 67 / 79 / 39 in place, type 70's level 15 / 17 pairing.

**Checked on screen** (Gemlik, `RC_LEVEL=13 RC_HERO_AT=508,440,300,1.5708`, frame 150): the flying biters of group 8
fly with a small yellow-white flame ribbon under their bodies (4 type-68 records alive and drawn, `unported types
killed []`); two captures byte-identical.

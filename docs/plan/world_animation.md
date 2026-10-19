# World animation: water, texture scroll, fog zones, underwater

Addresses are **level01.elf** unless marked "boot". gp = 0x166c00. Data tables are in the overlay's
loaded sections (`extracted/levels/01/overlay.bin`, `rc_formats::font::read_overlay`: 0x15ef00+0x3368,
0x166480+0xa5650). NTSC: one game tick = one frame = 1/60 s; `0x15f5cc` = logic tick counter (`++` in `0x2ab960`, frozen in pause modes), `0x15ed60` = 1.0,
dt `0x15ed6c` = 1/60 (1/50 on PAL). `SetTimeBase` 0x276258 (boot `set_time_base` 0x214970) also writes a second dt copy
`0x15ed7c` (same values); in level01 only the UV scroller 0x30f208 reads that copy. FX texture *n* = `extracted/levels/01/textures/fx/0nn_*.png` =
`GetEffectTex(n)` (0x21ae98; PSMT8, TCC 1, MODULATE).

## 0. Answers in brief

- **No tfrag, tie or shrub texture animation exists.** `TfragProc`, `LightTfrags`, `PatchTfragGifs`
  (0x2a7c90), `PatchTieGifs`, `DmaTfragTextures`, the shrub path and `PatchMobyGifs` never read the frame
  counter; the Patch*Gifs only rewrite TBP/CBP for GS paging. There is no level-wide animated texture table.
- **Novalis water is not tfrag.** Tfrags hold the blue stone beds (tfrag tex 13/14/28). Every animated
  surface is level code, in a moby update that registers a per-frame **draw callback** (`0x21afe0` /
  `0x21b198` lists: the `0x21afe0` list is drained by 0x21b030 after mobys and before particles, the `0x21b198` list
  by 0x21b1e8 after particles). Four mechanisms: (1) **static strip meshes**
  with 2-layer UV scroll and optional vertex bob (676, 678, 761, 1225); (2) **ripple heightfield patches**
  (751, shared engine module): a wave sim per 16×16-unit patch, lit per vertex, water layer + sphere-map
  reflection layer, drawn only while the camera is in one of 7 zone cuboids; (3) the **fire / smoke field
  quads** (760, scrolled per element and by the 809 global; earlier read as waterfall foam, §3); (4) a **reflective env-map overlay** (1848).
- **Fog zones** = gameplay env transitions (section 0x80). Inside a zone, fog colour, near/far and
  intensities are lerped along the cuboid's local X. Bit 0 cross-fades the *hero's* light banks.
- **Underwater** = flag `0x167494`. It switches to the alternate fog, adds a full-screen tint and lowers
  the particle far distance to 64 units.

## 1. Strip-mesh water (classes 676, 678, 761, 1225), confidence high

Updates: 676 `0x2f6128` (undefined in Ghidra, raw asm), 678 `0x2f6180`, 761 `0x2feb58`, 1225 `0x309c98`.
Each tick they register a callback: `0x2f60f8`, `0x2f6150`, `0x2feb28`, `0x309bf8`. Each callback runs
`fun_001f76a0` (VU1 program 57843 = resident id 7, view·proj 0x167140 and guard-band 0x1671c0 with −cam·1024
baked in, the fog lanes) and then `FUN_002b96e0(n, table)`. All four use update distance 0xff (always run).

**Descriptor** (0x60 bytes each; tables 761 `0x1fbc80`×4, 676 `0x1e2fc0`×8, 678 `0x1e3380`×2, 1225 `0x202d80`×3):

| off | type | field |
|---|---|---|
| 0x00 | ptr | vertices, 16 B each: `f32 x, y, z0, z1` |
| 0x04 | ptr | base UV, 8 B each: `f32 u, v` (u 0/1 across the channel, v steps 0.25 per rung) |
| 0x08 | ptr | bounding sphere `f32 x,y,z,r` (678 computes it at init: centroid and max distance) |
| 0x0c | s32 | vertex count = one GS tri-strip |
| 0x10, 0x14 | f32 | layer-1 / layer-2 scroll speed |
| 0x18 | f32 | UV wobble amplitude A |
| 0x1c | s32 | wobble phase step (**1** on every Novalis descriptor: the wobble turns, see §3.1) |
| 0x20..0x2c | f32×4 | layer-1 direction (dx1, dy1), layer-2 direction (dx2, dy2) |
| 0x30 | u32 | vertex RGBA (A = 0) |
| 0x34, 0x38 | s32 | FX texture, layer 1 / layer 2 |
| 0x3c, 0x3d | u8 | ALPHA FIX, layer 1 / layer 2 |
| 0x40..0x4c | f32×4 | runtime scroll s1 = (u, v), s2 = (u, v), start 0 |
| 0x50 | s32 | runtime phase counter (stays 0) |
| 0x5c | f32 | z blend w (written by 761 and 1225) |

**Per call** (`0x2b96e0`: every frame, for every strip, before culling):
`s1 += (dx1, dy1)·[0x10]`, `s2 += (dx2, dy2)·[0x14]`. Each lane wraps: `if (x > 1) x −= 1; else if (x < −1) x += 1`.
Then `phase += [0x1c] & 0xffffff`. Cull with `FastBSphereCheck(400.0, sphere)`: −1 skips the strip,
1 = inside draws with VU1 entry 0xc, 0 = crossing draws with entry 0xe (clip).
**Per vertex** (`0x262618`, VU0 macro):
```
k  = ((((bits(x) + bits(y)) >> 16) & 0xff)·64 + phase)·8 & 0xfff      // raw f32 bit patterns
θ  = (k / 4096)·2π − π ;  w = (A·sin θ, A·cos θ)                     // vcallms 0x192 / 0x190
z  = z0·[0x5c] + z1·(1 − [0x5c])
uv1 = (u, v) + s1 + w ;  uv2 = (u, v) + s2 − w
```
**GS** (`0x1cac80` DIRECT, then an A+D block): pass 1 context 1 (PRIM 0x7c: strip, Gouraud, TME, FGE, ABE)
uses TEX0_1 = FX[0x34] and `ALPHA_1 = FIX<<32 | 0x64` ⇒ `C = Cd + (Cs − Cd)·FIX/128`. Pass 2 re-sends only
the ST (`0x21fa98`, second copy with CTXT = 1) and uses TEX0_2 = FX[0x38], ALPHA_2. Both passes:
`TEST = 0x5360b` (alpha ≥ 0x60 else FB_ONLY; Z GEQUAL), CLAMP 0 (repeat), TEX1 `0xff9000000260`
(bilinear, no mips), MODULATE. Vertex A = 0 always fails the alpha test, so **colour is blended and Z is
never written**.

**Novalis data:**

| class | strips | where (x, y, z) | speeds 0x10/0x14 | dirs | A | RGBA | tex | FIX | z animation |
|---|---|---|---|---|---|---|---|---|---|
| 761 rapids | 4 × 40–48 v | (171–323, 111–204), z 60→94 | 0.007 / 0.004 | (−0.1, 0.9) / (0.1, 0.9) | 0.025 | 80 80 80 | 44/44 | 0x28 | ±0.05 bob, 64-tick sine |
| 676 channels + falls | 8 × 30–38 v | (22–178, 137–297), z 38.5–60.2 | 0.0025 / 0.0012 | (−0.1, −0.9) / (0.1, −0.9) | 0.007 | a0 a0 b8 | 44/44 | 0x30 | none |
| 678 pools | 2 × 4 v | (260–309, 138–184), z 82.25 | 0.0012 / 0.0007 | as 676 | 0.007 | a0 a0 b8 | 44/44 | 0x30 | none |
| 1225 dark pools | 3 × 70–94 v | (93–120, 60–86), z 59.6–65.8 | 0.0014 / 0.0008 | as 676 | 0.007 | a0 a0 b8 | 43/43 | 0x30 | z0/z1 from data, 64-tick sine |

(RGBA = R G B. Strips 4 and 6 of 676 fall from z 60.2 to 38.9: the waterfalls. The texture drifts downstream.)
- **761 init** (guarded by `0x161ce0` = gp−0x4f20): every vertex `z0 = z1 = z − 0.45`. Then per rung pair,
  alternating per strip and per pair, `z0 ±= 0.05` and `z1 ∓= 0.05` (the two vertices of a rung opposite).
  Every tick after that, `[0x5c] = 0.5 + 0.5·sin(((t & 63) − 32)·0.09817477)` for all 4 strips (π/32 per tick).
- **1225** writes the same w, but **after** drawing, so it lags one frame (0x202e9c, stride −0x60).
- Corrects moby_update_catalogue.md: 1225 is dark water (not a "glow"), 761 is the rapids.

## 2. Ripple patches (class 751 + engine module), confidence high (sim/draw), medium (interp timing)

The module is present in levels 01, 05, 07, 11, 12 and 13 (hash clusters of `0x2b7a48`, `0x261df8`, `0x262038`).
On Novalis the 751 instance is at (209.1, 314.6, 55), pvar 789. Patches: 21 × 0x1190 at **0x1e34c0**
(18 used). Globals: `0x1612d0` = patch base, `0x1612d4` = bounds table 0x1fa6c0, `0x1612d8` = count 21.

**Patch record:** +0x00 `f32 x, y, z` centre (z = water level); +0x0c `u8[8]` neighbour patches (0xff =
none; edges +0c..+0f, corners +10..+13) for halo exchange; +0x14 / +0x18 `s32` FX env map (TEX0_2) / FX
water (TEX0_1); +0x1c / +0x1d `u8` ALPHA_2 / ALPHA_1 FIX; +0x1e `u16` active mask of the 4×4 sub-blocks
(bit = (col>>2) | (row & 0xc)); +0x20/+0x24 UV scroll, +0x28/+0x2c its speed per draw; +0x30/+0x34 wobble
angles θ0/θ1, +0x38/+0x3c their steps, +0x40 wobble radius; +0x44 edge-pin flags (1/2/4/8 → first row,
last column, last row, first column pinned to z); +0x50 3 height buffers × 0x5c0 (16×16 `f32`, row stride
0x40, relative to z, then neighbour halos).

`0x2b7a48` init sets every patch to θ = (π/2, −π/2), steps (0.02, −0.031) rad, r = 0.03, speed (−0.0016,
−0.0016), zero heights. 751 init (`0x2fd0e8`) then sets constants `0x1cad00` = (16, 16, −8, −8, cell 1, 1,
refl 0.9, damping threshold 0.16, step 0.1) and light `0x1cad30` = (cos a, sin a, −1)·0.57735 with
`a = FastArcTan(0x180350, 0x180354)` (directional light 0, direction A x/y). Patches 13–16 get speed
(−0.0016, 0), patch 15 pin flag 8, and every patch two random drops (radius 2, amplitude 0.2).

**Zones** (pvar = cuboid indices 2..8; per-zone first patch 0x1fa5e8, count 0x1fa608, drop odds 0x1fa628;
sub-block masks per patch 0x1fa590):

| zone | cuboid centre, half size | patches (z) | FX water/env | FIX 1/2 | drops |
|---|---|---|---|---|---|
| 0 | (164.9, 96.3, 61.0), (27, 17, 15) | 0–3 (61.0) | 41 / 40 | 0x44/0x1c | 1/200 per tick per patch |
| 1 | (114.6, 98.1, 57.0), (42, 20, 20) | 4, 5 (56.5, 56.0) | 43 / 42 | 0x40/0x48, 0x48/0x48 | 1/2000 |
| 2 | (143.9, 60.9, 66.0), (22, 46, 10) | 6–11 (65.5) | 43 / 42 | 0x40/0x48, 0x48/0x48 | 1/2000 |
| 3 | (142.9, 128.4, 57.0), (36, 8, 12) | 12 (56.0) | 43 / 42 | 0x48/0x48 | 1/2000 |
| 4 | (280.3, 159.9, 92.0), (30, 24, 15) | none | — | — | — |
| 5 | (131.8, 199.0, 90.0), (150, 60, 70) | 13–16 (39.0) | 41 / 40 | 0x54/0x24 | 1/200 |
| 6 | (157.5, 121.2, 60.5), (10, 28, 10) | 17 (60.5) | 41 / 40 | 0x48/0x1c | 1/2000 |

**Per tick** (751 state 1):
1. Zone activation. For each zone, if the camera (0x167240) is inside its cuboid (`0x274820`), each of its
   patches gets `+0x1e = mask` and a random drop with odds 1/N (`FUN_0026c930(N) == 0`): `0x2b82a8(x ± U(4),
   y ± U(4), r 1, amp −0.05, additive)`. Otherwise `+0x1e = 0`, which means the patch is neither drawn nor
   simulated. The highest active zone index Z drives two things:
   - the underwater colours (§6): zones 1–3 → fog (0, 0x28, 0x30) and tint (0, 0x28, 0x30, 0x40); other
     zones → fog (0x18, 0x30, 0x90) and tint (0x18, 0x30, 0x90, 0x30).
   - zone 0 or 6: a drip (class 787, `0x2ffcd0`) every 300–1200 ticks from one of 5 points at 0x1fa650. It
     ripples (0.5, −0.35) where it hits surface 0.
   - zone 5: mist and flat ripple particles at (177, 165–176, 39) (particles.md §9).
2. **Sim clock** `0x2b7fe0`. The accumulator `f = 0x161224` starts at 1.1. When `f ≥ 1 − 0.1`: set f = 0.1,
   run the step (`0x2b7f30`), and render buffer `0x161220` = the previous buffer. Otherwise render buffer
   = 2 = `lerp(prev, cur, f)` (`0x261d78`). Then f += 0.1. On the PS2 FPU that is one step every **9**
   ticks, drawn with t = 0, 0.2 … 0.9 (pinned in §3.1). (Lerp `0x261d78` pinned: every lane `out = buf[0x16121c]·(1 − f) + buf[0x161218]·f`,
   `vmulaw ACC, a, (1−f)` then `vmaddx out, b, f`; the step writes into 0x16121c's buffer and swaps, so
   the draw lerps from the older to the newest state.)
3. **Step** (`0x262038` per active patch, buffers 0x161218 ↔ 0x16121c swapped after). The current buffer
   plus halos goes to SPR (`0x261c10`, 24-float rows). For each cell:
   `n = 0.25·Σ(8 neighbours) − prev`, then `h' = |n| < 0.16 ? n : n·0.9375`, written over prev. Edge
   rows and columns are copied into the neighbours' halo slots (+0x0c..+0x13 links).
4. Register the draw callback `0x2fd0c0` → `0x2b91c8(0x1e34c0, 21)`.

**Disturbance** `0x2b82a8(x, y, r, amp, patches, n, additive)`. It covers cells within ±r of (x, y), with
`v = amp` if d² < 1 else `amp / fun_001f9988()` (≈ amp/d, medium). Additive mode: `h = v + h_render`.
Otherwise `h = v` unless `|h_cur| > |v|`. It writes the current buffer and the neighbour halos. Callers:
hero splashes `0x23cf98` (0.4, 0.3), (0.5, −0.4) and `0x2370b8` (0.4, 0.35), (0.4, −0.3); drips; random drops.

**Height query** `0x2b8910` (used by `0x26ed38`, the hero, 695 floats and the camera test): finds an
*active* patch containing the point (z within [z−1, z+1), sub-block bit set) and bilinearly samples the
render buffer, + z. Optionally it returns the normal. Otherwise it falls back to the flat plane
`0x1612dc/e0/e8/ec` (enable, centre, height, radius), which is 0 on Novalis (set by level-05 code 0x318a68).

**Draw** `0x2b91c8`, per patch with a mask (four-corner clip-code test against 0x1671c0/0x167200):
- `0x261c10` heights → SPR. The vertex grid `0x2b8c08` is 17×17 at 1-unit spacing from (x−8, y−8), with
  z = h + z_patch; the pin flags overwrite edge z with z_patch.
- Per vertex (`0x261df8`):
  - `n = −(hl − hr, hu − hd, 2)` normalised (cell 1). l/r = x∓1 and u/d = y∓1 heights, so n points down.
  - grey `g = 64 + 128·clamp(3.25·(L·n − 0.573) + 0.573, 0, 1)`: the byte of `x + 65536.5` (0x47800040)
    bits, RGB = g, A = 0. Flat water gives g = 139.
  - eye `e = normalize(cam − p)`. If `e·n ≤ 0` (camera above), `R = e − 2(e·n)n`, else `R = e`.
  - env UV = `0.5 + 0.45·R.xy`.
- `0x2b80d0` water UV, shared by all sub-blocks: `s += speed`, wrapped to [0, 1). `θ0 += 0.02`,
  `θ1 += −0.031` (wrapped to ±π). Base table 0x1cafe0 maps each 4×4-cell sub-block to [0,1]², so the
  texture repeats every 4 units. Vertex k uses selector 0x1cb1b0[k]: `uv = base_k + s + 0.03·(cos θ_sel,
  sin θ_sel)`.
- GS: DIRECT setup 0x1cac80; A+D (only when changed) TEX0_1 = FX[+0x18], ALPHA_1 FIX +0x1d, TEX0_2 = FX[+0x14], ALPHA_2 FIX +0x1c.
- Each active 4×4 sub-block (16 per patch) is frustum-tested and emitted by `0x262388` as 46-vertex strips
  (order tables 0x1cad40/0x1cada0/0x1cae00). Pass 1 (ctx 1) sends XYZ, colour and water UV. Pass 2 (ctx 2)
  sends only the env UV. VU1 entry 0xc, or 0xe when it crosses the guard band. No Z write (A = 0).

## 3. Fire / smoke fields (760) and the 809 scroll (formerly "waterfall foam")

**Correction (2026-09-27): 760 is fire and smoke, not waterfall foam** (creatures.md §7, the port:
`rc_game::moby_update::classes::fire_field`, `rc-engine` water_render.rs "Fire fields"). The four Novalis fields sit on
the bombed city roofs; their flame quads are orange (+0x38 = 0x204080) and additive with FX 46 (a flame silhouette)
scrolling upwards, over a grey FX-47 smoke curtain. The waterfall foam is 751's zone-5 particle spray (types 57 / 56,
particles.md "In the port (2026-09-27): the waterfall foam"). The quad geometry and the element life cycle, "not
traced" below, are traced there: element tables at 0x1fa840..0x1fab40 (levels 00 / 14: the same bytes at 0x1e11a0 /
0x1e04d0), per-corner alpha bytes gp−0x4f68 / −0x4f60; timers 0x16137c..0x16138c from 809's init.


760 (4 instances: (114.7, 199.8, 51.3), (96.8, 234.8, 58.3), (49.0, 225.0, 57.1), (143.5, 312.4, 44.3); pvar
802–805: element range s16 [0, 20|40], FX 6+40 = **46 (foam crest)** and 7+40 = **47 (mist)**) registers
`0x2fe080` on list 2 (`0x21b198`). That callback draws camera-oriented quads with `FastDrawQuadReal`:
TEX1 bilinear, ALPHA 0x8000000048 (additive `Cs·As + Cd`; a second set 0x8000000044); positions per element
from 0x1cbc60 scaled by pvar +0x48/+0x4c; V scroll `v_i −= [0x1d4900 + 4i]·1.0` per frame, wrapped at −8 by
+8; the second quad set adds the **809 global** `0x161374` to U.

809 (`0x2ba658`, no pvar): every tick `0x161374 −= 0.0025·[0x15ed60]`, `+= 8` when ≤ −8; at init it turns
the 60-tick periods at 0x161368.. into frame counts and reciprocals for 760's element timers. The exact
quad geometry and the element life cycle are not traced.

### 3.1 In the port (water, 2026-09-26)

Code: `rc_formats::water` (overlay reader and tables), `rc_game::water` (state and arithmetic on the PS2
float model, unit tests), `rc-engine` `water_render.rs` + `assets/shaders/water.wgsl` (`RC_WATER=0` off,
`RC_WATER_STATS=1` prints the ripple state every 60 ticks). Tables are read at run time from the level
overlay (`overlay.bin`, the "ratchet executable", wad_layouts_rac1.md §4), not from an ELF.

Pinned from the disassembly (were open or wrong above):
- **Wobble.** `0x262618`: `k = ((((bits(x) + bits(y)) >> 16) & 0xff)·64 + [+0x50])·8 & 0xfff` (sll 6, addu,
  sll 3, andi), `vitof12`, `·2π (0x40c90fdb) − π (0x40490fdb)`, `vcallms 0x192` = sin → w.x, `0x190` = cos →
  w.y, `·A` (`vmulz.xy`). Only `hash & 7` survives the mask (eight phase groups). +0x1c = **1** on all 17
  Novalis strips and `0x2b96e0` adds it to +0x50 every call, so the wobble turns once per 512 draws; the
  port stores `hash & 7` per vertex at load and computes the eight offsets per frame on the CPU.
- **z blend** `vsubx.w vf3 = 1 − w; z = z0·w + z1·(1 − w)` (`vmulx`, `vmulw`, `vaddy`). 761 init constants
  0.45 = 0x3ee66666, 0.05 = 0x3d4ccccd; bob `(cvt(t & 63) − 32.0)·0x3dc90fdb`, VU0 sine, `·0.5 + 0.5`.
- **1225** update (0x309c98, undefined in Ghidra) is 13 instructions: `sb 0xff → +0x30` (update distance),
  `sh 0xff → +0x32`, register `0x309bf8`. So the far-away 1225 instance (z 4588) always updates.
- **Callback order** on Novalis = moby instance order: 676 (instance 689), 678 (690), 751 (875), 761 (892),
  1225 (963); 760 (888–891) is on list 2. One instance per class, so every strip is drawn once per frame
  (two GS passes). The lists are cleared by `0x2ab920` at the start of every tick, so on a 30 fps frame
  with a catch-up tick only the last tick's registrations are drawn: the strip scroll advances once per
  **rendered** frame, the 761 bob and the ripple sim once per tick.
- **Ripple buffer layout** (`0x261c10`): 16×16 cells, then +0x400 row −1, +0x440 column −1, +0x480 row 16,
  +0x4c0 column 16, +0x500 row 17, +0x540 column 17, +0x580.. corners (−1,−1), (−1,16), (16,−1), (16,16),
  (16,17), (17,16). Links: +0x0c first-row side (gets rows 0/1 as its rows 16/17), +0x0d first-column side
  (columns 0/1 → its 16/17), +0x0e (row 15 → its row −1), +0x0f (column 15 → its column −1), +0x10..+0x13
  the corners. The step and the disturbance write the same mirrors.
- **Step kernel** `0x262038` (COP1): `ACC = ul·0.25; ACC += u, ur, l, r, dl, d, dr (·0.25 each); n = ACC −
  prev·1.0`; kept when `(bits(n) & 0x7fffffff) − bits(0.16) < 0`, else `n − n·0.0625`. Threshold 0.16 is
  the disc value of 0x1cad1c (751 init does not write it; the disc constants are (32, 32, −16, −16, 2, 2,
  0.95, 0.16, 0.1), overridden to (16, 16, −8, −8, 1, 1, 0.9, ·, 0.1)).
- **Clock period.** Start f = 1.1 (0x3f8ccccd), buffers cur 0 / prev 1 / drawn 2. With the FPU's truncating
  adds `1 − 0.1` = 0x3f666667 and the accumulator reaches only 0x3f666664 after 0.1 + 7·0.1, so f = 0.9 is
  still a lerp tick: **step every 9 ticks**, drawn t = 0 (the pre-step state), 0.2, 0.3 … 0.9 (round to
  nearest would give 8). The step writes the new state over prev and swaps, so the draw is one step behind.
- **Disturbance** `0x2b82a8`: `fun_001f9988` = `0x2210f0` = VU0 `vsqrt` + `vaddq.x` of d² (so `amp/|d|`);
  rows `trunc(fy ∓ r)` clamped to 0..15, cell centres accumulated from `origin + cell·r0`, `d² < 1 → amp`.
  Writes the newest buffer [0x161218]; additive mode adds the **drawn** buffer [0x161220].
- **Zones.** Cuboid test `0x274820`: `l = inv3x4·(p − pos)` (cuboid +0x40 rows, +0x30 position), inside
  when every `−1 ≤ l ≤ 1`. The 751 pvar holds cuboid indices 2..8. Random drops draw `randi(N)` then
  `randf(−4, 4)` for x, then y; init draws `randf(−6, 6)` x then y, twice per patch, over all 21 patches.
- **Vertex shading** `0x261df8` (VU0 macro, software-pipelined): edge vectors (0, −1, h − h(r+1)),
  (−1, 1, h(r+1) − h(c+1)), (0, 1, h − h(r−1)), (1, −1, h(r−1) − h(c−1)); `n = −normalize(e11×e10 +
  e13×e12)`; grey = low byte of `clamp(3.25·(L·n − 0.573) + 0.573, 0, 1) + 65536.5`; `d = (−ê)·n`, R = ê when
  d < 0 (sign bit), else `−ê + 2(n·d + ê)`; env UV = `(R.xy·0.9)·0.5 + 0.5`. Light L = (fast_cos a,
  fast_sin a)·0x3f13cd36, z = 0xbf13cd36, a = FastArcTan(light set 0 A.x, A.y).
- **Water UV** `0x2b80d0`: scroll wrapped `> 1 → −1`, `< 0 → +1`; θ via `fast_add_rotations`; selector 0
  `u = base.u + (s.u + cos θ0·r)`, `v = base.v + (s.v + sin θ0·r)`; selector 1 `v = (base.v + s.v) + sin θ1·r`.
  Advanced only for patches that pass the draw-time corner test.
- **Sub-block strip**: order 0x1cae00/12 = 0x1cad40/4 = 0x1cada0/8 = 0, 17, 1, 18, … (5×5 vertices, rungs
  joined by the degenerate pairs 21, 21, 17, 17 …). Sub-block i starts at vertex `(i & 3)·4 + (i >> 2)·68`.

Render (see `water_render.rs`): Transparent3d at depth bias 5e5 + callback slot = after all moby draws,
before the particles (1e6), in callback order. Blend: FIX/128 baked into the fragment alpha with
SrcAlpha/OneMinusSrcAlpha (`GsPass::BlendNoZ`: no Z write, GEQUAL), not `BlendFactor::Constant`. Fog: the
frame's `GameFog` uniform (zones/underwater included). `fog_state.rs` uses `RippleSim::patch_height` for the
underwater test's water height and the highest active 751 zone for the underwater colours.

Differences / not done: the game's single `rand` stream is shared by all moby updates; the water has its
own `srand(1234)` stream until the moby loop is unified. FastBSphereCheck(400) strip culling and the
guard-band clip codes are not reproduced (GPU clipping; the patch corner test uses the port's frustum).
Drips (787) and zone-5 mist particles consume their random draws but are not spawned; the hero's splashes
are not hooked (no hero in water). (Since 2026-09-27: 760 is the fire fields, drawn; the zone-5 spray spawns its
particles.) Formerly: 760 foam/mist is not drawn (quad geometry and element timers still
untraced; `rc_game::water::foam_*_tick` hold the two scroll rules), 1848 not ported. Other levels' ripple
modules: see §3.2.

### 3.2 Other levels' water (2026-09-28)

The ripple module is on 01, 05, 07, 11, 12 and 13; the strip module only on 01. On 05 / 07 / 11 / 12 / 13 the manager
is a patch template: every placed moby owns one patch (`pvar` index), writes its z as the patch's water level (+0x08)
every tick, sets the patch's mask (+0x1e) from `FUN_00275690(48 or 64, moby)` (its bounding sphere in view: the
patch is simulated and drawn only then; 12 also requires the camera in a cuboid), and drops `randi(200) == 0` ripples
(r 1, additive, amp −0.05; 11: −0.03); the moby of patch 0 runs `RipplePatchesInit` + the constants + light + bounds
(+ on 07 / 13 two drops per patch), links the patches by position (16-unit neighbours; 05 has an older linker that
only fills the edge and two corner slots), and every tick the clock and the draw callback `FUN_002b91c8(table, n)`.
11 / 12 build their patches from the mobys (centre = moby, FX 0x38/0x39 on 11, `pvar+0 / +4 + 0x28` on 12, FIX
0x1c / 0x44, mask 0xffff) and double the moby scale (the collision covers the 16×16 patch). The water the hero swims
in is these mobys' collision (surface-0 faces at the moby z). The movements: 05 group commands spring the level
between pvar +4 and +0 (4.5 up), the flood (button class 0x33e) rises in six timed stages to 44 (slower after each
drowning); 07 group commands move ±4 u/s, a cuboid trigger then rises 0.32 and 0.42 u/s to 54; 11 the tide springs
between 132.36 and 137; 12 +5 per command, and Ratchet's water level is the moby's z within 8 in x or y; 13 swings
between pvar +0 and +4 by cos. 05's class 982 sets the flat plane `0x1612dc..ec` (centre = Ratchet, z 59.5 ±
0.25·sin, radius 48) and the dive lock `0x13f52e = 5` while Ratchet is in its cuboid.

Port: `rc_game::water::managers` (the ports, native `f32`; the module stays on its PS2-float port),
`rc_game::water::world` (`LevelWaterData` found by code identity, `WaterWorld` in `Services::water`),
`rc-engine` `water_render.rs` draws the module of any level. Level generalisation: docs/plan/level_generalisation.md
"Water on every level".

### 3.3 The sea and the other liquid surfaces (2026-09-28)

**What holds the sea.** No tfrag, tie, shrub, sky or moby geometry lies at the sea height on 05 / 11 / 12 (surveyed:
flat tfrags, tie / moby instances at the collision water height, sky shells by elevation). The seas are drawn by
level classes from a **draw callback**, with geometry built by code; the placed moby has no class geometry (Wrench's
class names: 854 "Ocean (Aridia)", 879 "Ocean (Rilgar)", 1018 "Ocean (Umbris)", 1111 "Ocean (Pokitaru)"). Three
mechanisms (confidence high: disassembly and the level data):

1. **The liquid grid module** (reference level05: init `0x2ce098(f12 scale, a0 state, a1 FIX)` / wrapper `0x2ce110`
   (FIX 0x80), draw `0x2ce830(state)` = `0x2ce130` (fog override from the record, the 64×64 image: `0x277b20` blends two
   FX frames texel by texel, `trunc(A·(1−t) + B·t)` on the **stored** CLUT entries, uploaded PSMCT24; GS state
   `ALPHA = FIX<<32 | 0x64`, TEST 0x5360a or 0x5370b when FIX < 0x61), DMA of the strip order `0x1cb280`, `0x2775c8`
   (block cull: flag ≠ 0, |block centre − camera|² < record w = 160000, frustum), `0x277900` (one 124-vertex strip
   per 7×7-cell block through the water-strip VU1 program 0x10e810: positions from the grid at the record's z, ST =
   `0x1cb390 · scale`, per-block RGBA), `0x2ce720` (fog restore). The state record (0x40 bytes) is in the class's data:
   `rc_formats::sea::LiquidGrid` has the layout. Users (each level's own class wrapper, found by code identity):

   | level | class (update) | record | z | cell / grid | FIX | anim FX / ticks | fog RGB | list |
   |---|---|---|---|---|---|---|---|---|
   | 03 | 994 (`0x2dc9b0`) | `0x1dc1a0` | −24 | 40 / 42×35 | 0x20 | 40+4 / 30 | 60 60 60 | after the mobys |
   | 05 | 879 (`0x316110`) | `0x211b20` | 59.5 ± 0.25 (61.5 on the Hoverboard) | 30 / 42×35 | 0x80 | 43+16 / 15 | 00 00 14 | after the ties |
   | 07 | 460 (`0x2f8058`) | `0x1d3400` | 34.55 | 10 / 14×14 | 0x80 | 63+16 / 30 | 78 3c 14 | after the ties |
   | 07 | 1018 (`0x30d440`) | `0x208560` | 38.37 | 30 / 42×35 | 0x80 | 42+16 / 30 | 0d 0d 0d | after the ties |
   | 08 | 327 (`0x2da0f0`) | `0x1d7060` | 15 | 30 / 42×35 | 0x80 | 40+16 / 30 | 0d 0d 13 | after the mobys |
   | 09 | 317 (`0x2ef7f8`, the same code as 460) | `0x1fc740` | 25 | 15 / 49×49 | 0x80 | 44+16 / 30 | c8 64 14 (the lava) | after the ties |
   | 14 | 1260 (`0x303370`) | `0x1e9c60` | 16 | 40 / 42×35 | 0x40 | 40+4 / 30 | 20 20 20 | after the mobys |

   The record's fog is near 10000 / far 260080 integer units (≈ 9.8 / 254 game units) with intensities 255 → 0, so the
   liquid fades to its own fog colour with distance. "After the ties" is the draw list `0x16e100` (count `0x15f42c`) that
   `DrawWorld` drains after `DrawTies` and before `DrawShrubs` (`RunDrawCallbacks_2` 0x21b0a8); only these levels'
   copies of the register function fill it (level05 `0x228110`, 07 `0x221290`, 09 `0x218810`, 12 `0x21bf18`, 02
   `0x20a390`; level 14 has one for the before-mobys list `0x16e300` too).
2. **The camera-following ocean 1111** (level11 update `0x30b358`, callback `0x30a908`; the identical code on 16 at
   `0x2dee10` / `0x2de3c0`): a band from 75 behind the camera to pvar 10 ahead (width ±87 → ±750), a wall when the camera
   is high, and two additive layers of 4×4 tiles of pvar 6 around the camera with ST scrolled by the camera's motion ·
   2·scale/(5·S) and by constant speeds (`0x161df8` scales 2, 3; `0x1d9af0` speeds). Details: `rc-engine`
   `sea_render.rs` module doc; `rc_game::water::sea` for the update.
3. **The Hoven liquid 1901** (level12 update `0x30bff0`, callback `0x30be68`, after-ties list): static strips
   (group 1: 10 strips, tables `0x205a08` / `0x2059e0` / `0x205a58` / counts `0x1fbd10`, drawn while the camera is in the
   moby's cuboid; group 2: 5 strips of the open sea at z ≈ 25, tables `0x208ab0` / `0x208a98` / `0x208ae0` / `0x205a80`)
   through the generic strip emitter (level01 `0x21fda8`), ST + a scroll per layer (`0x1620e0..` speeds), FX 0x2c / 0x2f
   (FIX 0x7f) and 0x30 (additive FIX `0x1620d4` = 0x78). The first init sets every colour (`0x30bba0`).

**Not ported (other liquid layers found on the way):** 02's 854 runs the module's image and fog set-up with its own
arguments (`0x2a4818`) and draws seven small liquid meshes through `0x2a48a0` (records `0x1f3c00..0x1f3e10`, cuboid
gated), not the grid; 09's callback also draws 108 scrolling lava-flow meshes (`0x21e8c0(0x1f3080, 0x6c, …)`, its own
second animation FX 0x2c.., period 20) and 10 more meshes (`0x2c1978(state, 0x1f63c0, 10)`); 12's class 293 draws 47
strips through the two-texture strip module `0x2bc210` (FX 44 / 45; the same code on 14 at `0x2ab3e8`); 08's 327 adds a
splash when Ratchet falls in. The water-strip VU1 program (resident id 7) has more users per level (survey:
`tools`-free clustering of the callers of the level's `setup_water_strip_vu1` copy), not all of them liquids.

**In the port.** `rc_formats::sea` (tables), `rc_game::water::sea` (the class ports `ClassUpdate::Sea(i)`, their data in
`LevelWaterData::sea` and state in `WaterWorld::sea`; the after-ties list is `DrawCallbacks::ties`), `rc-engine`
`sea_render.rs` (the draws, through the shared draw-callback material `fx_draw::FxPrimMaterial`: display-blend effects,
or `FxPrimParams::opaque` for the FIX ≥ 0x61 / 0x7f Z-writing strips drawn as opaque world surfaces). `RC_SEA=0` turns
it off. Guards: `rc-game/tests/world/sea_levels.rs` (the inventory on 19 levels, each port's update on its level).

## 4. Reflective overlay (1848), confidence high

`0x30f208` runs every tick: `0x162110 += dt·0.025` and `0x162114 += dt·0.025` (dt = the copy at `0x15ed7c`, 1/60), each wrapped to
±1. It registers `0x30f0e0`, but only within update distance 32 of (252.8, 193.2, 95.6).

The callback sets TEX0_1 = FX **40** (sphere env map), `ALPHA_1 = 0x2000000064` (25 % FIX blend), CLAMP 0
and bilinear filtering. It draws 5 meshes (148, 49, 148, 148, 89 vertices; positions 0x208108, normals
0x208138, colours 0x208150; around (237–277, 159–209, 95–107)). Per vertex (`0x30ef18`):
```
e = normalize(p − cam),  r = normalize(e − 2(e·n̂)n̂) + (0, 0, 1)      // n̂ = normalized stored normal
uv = 2·(r.xy/(2|r|) + 0.5) + (0x162110, 0x162114)
```
Classic sphere-map formula, drawn by `0x21fda8`; probably glass or a shiny structure, not water.

## 5. Environment (fog) zones, confidence high

Gameplay section 0x80 (wad_layouts_rac1.md §3.11): count, then `count × vec4` = XY bounding circle
(xyz, **w = r²**), then `count × 0x80` records. The loader `0x255958` copies them to 0x17f140 / 0x17f340
and turns hero light −1 into 0xb.

**Lookup** `0x26bbc0(p)`: walk the circles (2-D: `dx² + dy² < w`; the loop also tests index `count`,
harmless). At the **first** circle hit compute `l = p·inv` (record +0x00 rows); if any |l.xyz| > 1 return
"none" without trying later records, else return i and **t = (l.x + 1)/2**.

**Fog** `fun_001ee4b0(cam)` (boot 0x1ee4b0; level copy 0x2102b8), called each camera update right after
the underwater test (`0x20eca8`), when flags & 2: `i = trunc(t·255)`, colour channel `(c2·i + c1·(255 − i))
>> 8`; near/far distance `(d1·(1−t) + d2·t)·1024`; intensity `255 − (k1·(1−t) + k2·t)·255` for near
(+0x60/+0x70) and far (+0x68/+0x78) separately.

It writes the **level fog globals** 0x15f444..0x15f454 directly. `UpdateFog` (end of `DrawDebugProfiler`,
every frame) copies them into the view context, and `UpdateViewContext` recomputes cf10/cf14/cf20, the
projection row2.w and the **tfrag LOD morph constants** (`SetTfragDists`). Leaving a zone restores
nothing: the last value sticks until another zone is entered. The records are built so their ends match
the outdoor fog.

**Hero lighting** `0x26be04` (flags & 1; called for the hero moby only, from `0x228870`/`0x228000`/`0x231348`):
hero +0x80 = byte lerp of hero colours 1/2, and hero +0x38 = `light1 | light2 << 8 | i << 16`, which is the
moby light-bank cross-fade (moby_skinning_lighting.md +0x38). The nearest point light is added to +0x3c.

**Novalis** (11 records; colours RGB; each side is near dist, near d, far dist, far d; F = 255 − 255·d):

| # | circle centre, r | flags | side 1 (t = 0) | side 2 (t = 1) |
|---|---|---|---|---|
| 0 | (147.8, 130.7), 9.2 | 3 | (5, 5, 15) 0, 0, 50, 0.8 | (105, 126, 179) 0, 0, 240, 0.6 |
| 1 | (177.6, 83.8), 7.9 | 3 | same as 0 | same as 0 |
| 2 | (186.0, 127.9), 14.4 | 1 | (0, 39, 75) 20, 0.25, 50, 0.6 | outdoor |
| 3, 4 | (260.9, 178.5) 6.4; (278.9, 192.3) 6.6 | 3 | (0, 44, 120) 15, 0, 100, 0.4 | outdoor |
| 5, 6 | (227.8, 115.5) 4.6; (229.1, 125.2) 5.9 | 2 | (0, 44, 120) 15, 0, 100, 0.4 | (105, 125, 179) 0, 0, 240, 0.6 |
| 7, 8 | (45.1, 147.6) 5.5; (51.5, 135.9) 5.1 | 2 | (72, 83, 12) 0, 0, 30, 0.75/0.74 | (105, 125, 179) 0, 0, 240, 0.6 |
| 9 | (53.3, 126.9), 5.7 | 2 | (72, 83, 12) 0, 0, 30, 1.0 | (72, 83, 12) 0, 0, 30, 0.5 |
| 10 | (36.7, 116.6), 6.6 | 2 | (72, 83, 12) 0, 0, 30, 1.0 | (72, 83, 12) 0, 0, 25, 0.5 |

"Outdoor" = the level settings (105, 127, 180) Dn 0, Df 240, d_far 0.6. Hero lights: 0–1 bank 1→0,
2–4 bank 2→0, 7–8 bank 3→0 (hero colours 0x191926 → 0x282828, 0x2a2e33 → 0x282828).
Records 0–1 and 3–4 are cave and tunnel mouths (dark blue); 7–10 are a yellow-green gas area. The
cuboids are small (half sizes 3–7 units), so each record is a transition portal.

### 5.1 In the port (2026-09-26)

**Section 0x80 layout** (verified, `rc_formats::gameplay::parse_fog_zones`; loader `FUN_00255958`):
`s32 count, pad[3]`; `count × f32 (x, y, z, r²)` circles (z unused); `count × 0x80` records:
+0x00 4 rows inverse matrix (row-vector, `l = x·r0 + y·r1 + z·r2 + r3`); +0x40/+0x44 hero colour t=0/t=1;
+0x48/+0x4c hero light bank t=0/t=1 (−1 → 0xb at load); +0x50 flags; +0x54/+0x58 fog colour t=0/t=1
(bytes r, g, b); +0x5c..+0x6b side 1 and +0x6c..+0x7b side 2 = (near dist, near d, far dist, far d); +0x7c unused.
The Novalis disc test checks all 11 records against the table above.

**Pinned from the asm** (verified):
- Lookup `0x26bbc0` (VU0): p.w = 1 (`vaddx.w vf10, vf0, vf0`); circle test `(dx² + dy²) − w` sign bit; matrix
  `vmulax/vmadday/vmaddaz/vmaddw` (products rounded, accumulated left to right); `vclipw.xyz` against 1.0;
  t = `0.5·(l.x + 1)`. The loop runs `count + 1` times (the stale slot past the end, not modelled).
- Lerp `0x2102b8` (EE FPU, round toward zero): `i = cvt.w.s(t·255)`, `u = 1 − t`; far F = `255 − (k2·t + k1·u)·255`,
  near F likewise, near/far dist = `(d2·t + d1·u)·1024` (side-2 product first, then the add); colour byte
  `(c2·i + c1·(255 − i)) >> 8` with c1 = +0x54 (t = 0), c2 = +0x58.
- Hero cross-fade `0x26be04`: `pmulth` / `paddub` / `psrlh 8`, i.e. per byte `min(255, (c1·(255−i) >> 8) +
  (c2·i >> 8))` (saturating, no carry between the two products); +0x38 = `light1 | light2 << 8 | i << 16`.
- Frame order: the zone write happens in the camera update, `UpdateFog` copies at the end of the frame
  render, so what is drawn lags the camera by one update (inferred from the call order; the port keeps it).

**Code**: `rc_game::fog_zones` (`lookup`, `apply`, `update`, `hero_light`, `update_fog`) on the `ps2v`
float model; `rc-engine/src/fog_state.rs` runs each frame in `PostUpdate` before the occlusion/LOD systems:
`UpdateFog` → `game_camera::GameFog` (view-context fog + particle far), then the underwater test and the zone
lookup for the fly camera's eye. Consumers of `GameFog`: every material's fog uniform (tfrag, tie, moby, shrub,
billboard), the tie per-instance F (`TieLodState::fog`), the tfrag LOD constants (`TfragLodUniform::set_fog`
= `SetTfragDists`, recomputed from the live Dn/Df/In/If; the formulas match `SetTfragDists` 0x2a79f0 and
`UpdateViewContext` 0x219580: cf20 = (If − In)/((Df − Dn)/1024)) and the particle cull. `RC_FOG_ZONES=0`
disables the zones. Verified on screen: inside zone 7 the rock walls take the gas colour (74, 86, 28) and
a far of ~51 units; a walk-through from the outdoor face to the gas face (`RC_CAM_PATH`) blends t 0.99 → 0.09
with no snap and keeps the last values after leaving. Not done: the hero cross-fade (`hero_light` is the
hook for when the hero moby renders); the zone lookup reads the main camera entity, which `play_camera.rs` drives
in game mode (2026-09-29: the earlier "uses the fly camera (no camera modes)" was stale).

## 6. Underwater, confidence high

**Test** `0x20e9f0` (each camera update):
- If camera mode (`[0x167280]+0x86`) = 6 or `0x15f5c4` ≠ 0, clear the flag.
- Otherwise cast `CollLine_Fix` from cam.z + 0.75 to cam.z − 0.75 (flags 0x12), up to 6 times, re-casting
  below each non-water hit. On surface id 0 (`fun_001f0b58` = `id & 0x1f`, water): `0x167494 = cam.z <
  h + 0.04`, h from `0x26ed38` (ripples included). No hit within ±0.75 leaves the flag **unchanged**.
- Other writers: teleport `0x2368e0` sets it to (hero sub-state == 0x11); `0x2406b0` and `0x2cb788` clear it.

**Effects** while `0x167494` ≠ 0:
- `UpdateFog` (0x218d70) uses the alternate set: colour 0x161204..06 (per zone, §2), near 0x161208 = 0,
  far 0x16120c = 32768 (**32 units**), near F 0x161210 = 255, far F 0x161214 = **48**. The latter four
  are written by 751 init.
- Global far `0x16017c` (gp−0x6a84) = 0x40000 (**64 units**, ftoi12) instead of 0x1f4000 (500). This is
  read by `PartProc` and 3 level functions (tfrag distance cull 0x160ec0 is separate).
- `DrawDebugProfiler` (0x21a1b8, in the `0x15f3f4 & 0x40` pass) sets `ALPHA_1 = 0x8000000044` and draws
  full-screen sprites (DIRECT at 0x13cc90) with RGBAQ = 0x161200..03. That is `Cd + (Cs − Cd)·A/128` =
  50 % (0, 40, 48) in zones 1–3, or 37.5 % (24, 48, 144).
- Voices are released (`0x31a078`). The sky is not touched (not traced further).

### 6.1 In the port (2026-09-26)

- Test `0x20e9f0` pinned: segment `cam.z ± 0.75` (FPU add/sub), `CollLine_Fix` flags 0x12, up to 6 casts, a
  non-water hit restarts at its hit point with `z − 0.01` (end unchanged); on surface id 0 (`0x2151d8`) `flag =
  cam.z < h + 0.04` (`c.lt.s`); `h = 0x26ed38(hit point)`: active ripple patch height, else the flat plane
  (off on Novalis), else **the hit point's own z**. Ported as `UnderwaterState::update` (callbacks) /
  `update_with_mesh`; the engine passes `|_| None` for the height until `rc_game::water`'s patch query is
  wired in (then only the ±ripple offset differs). Verified: camera at (177, 170, 38.6) under the zone-5
  pool surface turns the flag on from the collision mesh alone.
- Tint pinned: `emit_rgba_draw_packet(0x161200, 0x161201, 0x161202, 0x161203)` after `ALPHA_1 = 0x8000000044`,
  i.e. `Cv = ((Cs − Cd)·A >> 7) + Cd`. Static bytes 0x161200 = (0x18, 0x30, 0x90, **0x40**), fog colour
  0x161204 = (0x18, 0x30, 0x90); 751 then sets zones 1–3 → tint (0, 0x28, 0x30, 0x40) = 50 %, fog (0, 0x28, 0x30);
  other active zones → (0x18, 0x30, 0x90, 0x30) = 37.5 %; no active zone leaves both. **Order: after the HUD**:
  the tint is in the `0x15f3f4 & 0x40` pass, which follows the `& 0x80` pass that draws `HudDraw` (0x24fb50),
  the help/UI frames (0x2266c0) and the letterbox (0x1f4d98), all appended to the same DMA chain; the black
  and white fades and the rect overlays (0x1f4fb8) come after the tint. So the HUD is tinted.
- Sky: **unchanged underwater**. The only readers of 0x167494 are 0x20e9f0, 0x20ef58, `UpdateFog`,
  `DrawDebugProfiler` (tint), 0x2368e0, 0x2406b0, `sound_update`, 0x2cb788 and 0x31a078; none is in the sky
  path, and the sky shells are drawn with FGE 0, so the alternate fog does not reach them either. Only the
  full-screen tint covers the sky.
- Port: `fog_state.rs` switches `GameFog` to the alternate set (32 units, F 48) and the particle far to 64 units
  (`particle_render` reads `GameFog::particle_far12`), and puts `UnderwaterTint` (a Bevy `FullscreenMaterial`,
  `assets/shaders/underwater_tint.wgsl`) on the main camera, scheduled after `bevy::ui_render::ui_pass` and
  before upscaling. The shader re-encodes the frame buffer to display bytes and blends in integers like the
  GS (checked on sky pixels: (244, 249, 251) → (134, 148, 197) = the GS formula exactly). `RC_UNDERWATER=1`
  / `=0` force the flag. Not done: the 751 zone colour choice (hook `UnderwaterLook::set_from_ripple_zone`,
  fed `None` until rc_game::water exposes the highest active zone), the camera-mode-6 / 0x15f5c4 clears and
  the other flag writers, voice release.

## 7. Other per-frame world animation on Novalis

- **Sky shell rotation**, **shrub sway** and **tie sway**: done in the port. **No tie/shrub texture or UV animation and
  no tfrag colour cycling.**
- **Moby glow** +0x90 (init = class header +0x40, `InitMobyInstance`). It is pulsed by updates: vendor 11
  `(sin(t)·48 + 96)·0x010101 | 0x80000000`, ship 531 `0x2a1c40`. It is consumed by the MobyProc mode-0x10
  glow list (pinned and ported 2026-09-28: docs/plan/moby_skinning_lighting.md §10).
- **Point lights** (class 1504 moves a light slot near the camera) relight ties, shrubs and tfrags per
  frame through the existing Light* passes. Without point lights those passes are static.

## 8. Port plan

- **Formats.** Read the tables by address from the overlay lump (`rc_formats::font::read_overlay`).
  Parse the strip descriptor tables (addresses above; per level, since the tables are Novalis-only), the
  751 patch/zone/constant tables, the 760/1848 tables and the env transitions (the gameplay section is
  already split). Golden: assert the counts and values listed here.
- **rc-game.** Per tick: the 761 z pulse; the 809 and 1848 scrolls; the 751 zone activation (reuse the
  cuboid test), sim clock, 8-neighbour step, disturbances (hero hooks later) and drips; the fog-zone lookup
  (§5) writing the level fog; the underwater test (reuse `collision_query`). Keep the PS2 float model for
  the sim (it feeds the heights the hero stands on); the UV scrolls are cosmetic.
- **rc-engine renderers:**
  - *Strip water*: one mesh per strip. Two draws per strip, in callback order after mobys and before
    particles: layer 1 then layer 2, each `mix(dst, tex·vcol/128, FIX/128)`, depth test GEQUAL, no depth
    write, fog on, repeat, bilinear. Scroll and z blend are uniforms; w per vertex is precomputed on the
    CPU from the bit-pattern hash. Expected: flowing translucent blue channels, rapids and falls on Novalis.
  - *Ripple patches*: upload 17×17 heights per active patch per frame. Per vertex on the CPU (exact):
    normal, grey, reflection UV. Draw the 16 sub-blocks with the mask, two passes (water FX 41/43 with the
    shared animated UV, then env FX 40/42 with the reflection UV). No Z write. Expected: pools that
    ripple under drips and the hero, visible only while the camera is in the zone cuboid.
  - *760 fire fields*: ported (§3); *1848 env overlay*: later.
  - *Fog*: feed the zone-lerped level fog, or the underwater alternate fog, into the existing fog uniforms
    and `SetTfragDists` every frame, so tfrag and tie LOD shift with them. Draw the full-screen tint quad
    after the world when underwater.
  - *Hero lighting*: apply the flags & 1 cross-fade to the hero's light banks once Ratchet renders.

## 9. Unknowns

- VU1 57843 entries 0xc/0xe (perspective ST, clip) assumed standard. (`fun_001f9988` = VU0 sqrt and
  strip +0x1c = 1 are pinned, §3.1.)
- 1848's mesh identity; (the tint pass position is pinned: after the HUD, §6.1)
  relative to the HUD.
- ~~Other levels: which classes use the ripple module (05/07/11/12/13), and lava/goo (level-specific code,
  e.g. `0x21fa98` strip-emitter copies in 09/12/14), not surveyed.~~ Surveyed 2026-09-28 (§3.2, §3.3,
  level_generalisation.md's water survey); the lava / goo meshes left are G-REN-026.

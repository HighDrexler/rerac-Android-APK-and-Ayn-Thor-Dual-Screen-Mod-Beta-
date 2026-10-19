# Shrub lighting (`LightShrubs`) and the shrub draw rules

Not ported yet. The loader (`crates/rc-formats/src/shrub.rs`) exposes every input named here:
`ShrubVertex::normal`, `ShrubClass::normals`, `ShrubInstance::{colour, dir_lights, matrix, draw_distance}`,
`ShrubClass::billboard`, `LevelShrubClass::billboard_texture`. Format: `docs/formats/shrub_sky_rac1.md` §1.3b, §1.9b.
Conventions (colour floats `65536 + c/128`, light-set layout, `vf21`, PS2 float model) are those of
`docs/plan/tfrag_lighting.md` §2, §4–§6. Tags: **[verified]** read in the disassembly (and data where
stated); **[inferred]** reasoned, not observed.

## 1. Code and data

| What | Where | Notes |
| --- | --- | --- |
| `LightShrubs(u16 *list)` | level01 `FUN_0029e7e8` (0x5b4 bytes); boot copy not located | EE + VU0 macro code; the per-normal work is VU0 micro program **436083** (resident, loaded at frame end): `vcallms 0` (normals 0–3 → vf09–vf12) and `vcallms 0x2c` (4–7 → vf13–vf16). **[verified]** |
| Runtime record | `*(0x160494) + i*0x20` | built by the loader `FUN_00255958` (pointer 0x3c loop): +0x00 bsphere centre/radius (world), +0x10 draw distance, +0x17 `trunc(fade_distance)`, +0x18 index, +0x1a class index, +0x1b dirty byte, +0x1c light selector = `dir_lights` (u16), +0x1e point-light nibbles (0xffff = none). **[verified]** |
| Matrix block | `gp-0x6764` (`0x16049c`) `+ i*0x40` | the instance matrix; col0.w = ambient `r | g<<8 | b<<16 | 0x80<<24` from `colour`; col1.w = average palette colour (written after the load-time pass); col3.w = class `scale`. **[verified]** |
| Palette table | `gp-0x6760` (`0x1604a0`) `+ i*0x60` | output: 24 RGBA8 words per instance. **[verified]** |
| Normals | class blob `normals_offset` (relocated pointer at class +0x2c) | 24 × s16[4]; read with `ld` + `pextlh/psraw`, `itof15` (÷32768, not 32767). **[verified]** |
| Light banks | 0x180340 (16 directional sets × 0x40), 0x180740 (point lights × 0x20) | same banks and layouts as `LightTfrags` / `LightTies`. **[verified]** |

## 2. The math (per listed instance `i`)

```
sel = rec.u16[0x1c]                                              [verified]
if sel & 0xff00 == 0: A = bank[sel & 0xf]; colA, dirA, colB, dirB = A.qw0, A.qw1, A.qw2, A.qw3   (no renormalise)
else: t = (sel >> 8)/256 (itof12 of (sel>>4)&0xff0); w = 1 - t; S = bank[sel & 0xf], T = bank[(sel>>4) & 0xf]
      each qw = S.qw*w + T.qw*t; dirA, dirB *= rsqrt((x*x+y*y) + 1.0*z*z)
backA = colA.w; backB = colB.w; colA.w = colB.w = 0
-- point lights: nibbles p of rec.u16[0x1e] | 0xf0000 from the low end until 0xf (max 4)   [verified]
dirP = 0; colP = 0; k = -2
for p: L = pbank[p] (qw0 colour, qw1 pos xyz + radius r); v = rec.centre - L.pos; d2 = (v.x²+v.y²)+1.0*v.z²
       skip if r*r - d2 < 0; dist = sqrt(d2)
       colP += L.col * (1 - dist/r) (xyzw); dirP += v / dist; k += 1
if k >= 0 (two or more lights): dirP *= rsqrt(|dirP|²)
backP = colP.w; colP.w = 0; dirA.w = dirB.w = dirP.w = 0
-- instance rotation: c0..c2 = matrix columns 0..2, each * rsqrt(|c|²)   (scale removed)
R = rows(-(c0,c1,c2))            // -Mᵀ: turns a world direction into object space, negated
LA = R·dirA; LB = R·dirB; LP = R·dirP                       // three object-space light vectors
amb = ambient bytes (col0.w) as 65536 + c/128, alpha 0x80
-- VU0 436083, for each of the 24 normals n (s16 / 32768)                                     [verified]
d = (LA·n, LB·n, LP·n)   (vmulax/madday/maddz: ((x*a)+y*b)+z*c per light)
d = max(d, d * (backA, backB, backP))           // leaky clamp, per light
c = amb*1.0 + colA*d.x + colB*d.y + colP*d.z (xyzw)
c.rgb = min(c.rgb, I = 0x478000f3)              // clamps at 243, not 255 (tfrag uses 0x478000ff)
palette[i][n] = low byte of each lane (r, g, b, a = 0x80)
```

Differences from `LightTies`/`LightTfrags` worth porting exactly: one colour per (instance, normal)
instead of per vertex; the object-space trick (lights rotated into the instance frame once, normals
used raw); the point lights folded into one aggregated third light at the instance centre; the 243
clamp. Everything else (float order, leaky clamp, 1/128 grid) is the tfrag model. **[verified]**

## 3. Output and consumer

* `ShrubProc` (level01 0x29cdf0) uploads `palette[i]` per drawn instance as 6 qw with `UNPACK V4-8`
  unsigned to VU slot + 4 (24 qw of r, g, b, a); VU1 program 56467 colours each vertex with
  `palette[vertex.normal].rgb`. **[verified]**
* Vertex alpha is the w of palette entry 0's quadword: for a fading instance `ShrubProc` then
  uploads a 5th instance qw over entry 0 = (palette[0].rgb, fade alpha) (§4); for opaque ones the
  lit alpha 0x80 stays. **[verified for the fading list; opaque path inferred]**
* VU1 quirk: in 6-vertex packets (stop flag on vertex 2) vertex 3 fetches its colour from
  `n + 2*base`, i.e. another instance's data (7 retail packets). **[verified by interpreter run]**

## 4. Timing

* Level load: after `LightTies`, the loader lists every shrub instance (`0..n-1`, 0xffff end) and
  calls the pass once; then it stores the average of the 24 palette colours in col1.w. **[verified]**
* Every frame: `ShrubProc` appends a visible instance to the list at scratchpad 0x70003200 when its
  dirty byte (+0x1b) is set or it has point lights (+0x1e != 0xffff); `DrawShrubs` copies the list
  to 0x1bd430, and the EE second half of the frame calls `LightShrubs(0x1bd430)` (render_pipeline.md).
  The pass restarts from the ambient, so without point lights a per-frame relight equals the load-time
  result. **[verified list build/copy; call site from render_pipeline.md]**
* A port can therefore light once at load and relight only instances touched by point lights. The port does
  that in the vertex shader (docs/plan/tfrag_lighting.md §9, "Point lights on world geometry").

## 5. Draw rules (`ShrubProc`, level01 0x29cdf0; VU1 56467 / 912339)

* Class skipped when `mode_bits & 1`; `(mode_bits & 6) >> 1` selects wind sway (EE matrix
  perturbation with the 256-byte table copied to scratchpad 0x70003b00; retail: 64 classes mode 2,
  3 classes mode 1). **[verified; the sway math is §7]**
* Per instance, with `z` = camera-space depth of the bsphere centre (clamped >= 0), `D` = runtime draw
  distance (`max(draw_distance, 16)`, and `>= trunc(fade_distance) + 24` for billboard classes, capped
  by a global max at scratch 0x3fa4) and `F = trunc(fade_distance)`: **[verified code, units inferred]**
  * no billboard: mesh alpha `min(16 * (D - z), 128)` (fades over the last 8 units);
  * billboard class, `z < F`: mesh, opaque;
  * `F <= z < F + 8`: cross-fade: mesh alpha `16 * (F + 8 - z)`, billboard alpha `16 * (z - F)`;
  * `z >= F + 8`: billboard only, alpha `min(8 * (D - z), 128)` (fades over the last 16 units).
  * Alpha 128 → opaque list (scratch 0x70002c00); below → translucent list (0x70003700) with the 5th qw.
* Billboards are not VU1 shrub packets: `ShrubProc` projects 4 corners itself (corner/ST table
  copied from 0x1c3130) into a 4-vertex GS triangle strip coloured col1.w (average palette) with the
  billboard alpha; full rule in §6. **[verified]**
* Fog is one value per instance: `clamp(fog_scale * w_origin + fog_offset, fog_min, fog_max)` from
  `vf30.zw`/`vf29.xy` (written by `UpdateViewContext`), stored in every vertex's XYZF2. **[verified]**
* VU1 constant block (VU 0..2): `vf31` A+D tag `(1, 0x40000000, 0xeeee, 5.0f)`, `vf30` =
  `(0x4bc00408, 2²³, fog scale, fog offset)`, `vf29` = `(fog min, fog max, -, -)`; up to 5 instances
  per `MSCALF` batch, 0x1c qw each (4 matrix + 24 palette). **[verified]**
* Program 912339 (second list, uploaded only when that list is non-empty) clips; which instances go
  there (frustum edge?) is not decoded. **[inferred]**

## In the port

`crates/rc-formats/src/shrub_light.rs` (pass) and `crates/rc-engine/src/shrub_render.rs` + `shrub_light.rs`
+ `assets/shaders/shrub.wgsl` (renderer). Findings made while porting (level01 `FUN_0029e7e8`,
`FUN_00255958`, `ShrubProc` disassembly):

* **VU0 entries.** `LightShrubs` calls `vcallms 0` and `vcallms 0x160` (byte address; entry 0x2c): the same
  code as the tie entries 0x58 / 0x84. `work/vu/436083.txt` prints `maddz.xyzw vf08, vf26, vf08` twice
  at line 60/61; the binary has it once (instruction 0x3b, then `mul.xyz vf17, vf05, vf30` at 0x3c): a
  listing artefact, not a quirk. **[verified in 436083.bin]**
* **Differences from `LightTies`** (all in the EE code): blended light sets scale xyz only
  (`vmulw.xyz` / `vmulx.xyz`), so the blended back factor is `A.w + B.w`; the instance columns are
  normalised with `vrsqrt` (`c · rsqrt(|c|²)`), not `1 / FastVecLength`; the ambient is col0.w of the
  matrix block = `b << 16 | g << 8 | 0x80000000 | r` (OR'ed raw s32 channels; retail channels are 0..255),
  converted with `pextlb/pextlh/padduw 0x47800000`. **[verified]**
* **Run-time centre.** Record +0x00 = `t + scale · (M3 · bsphere.xyz)`, radius `scale · bsphere.w · max|col|`
  (same helpers as ties): the class bounding sphere is in units of `scale` world units, confirming the
  §1.5 caveat of shrub_sky_rac1.md (e.g. Novalis class 92: radius 25.9 × 0.1257 = 3.26 for a 3.02-unit
  vertex extent). **[verified code + data]**
* **Draw distance.** Instance +0x04 is a real f32 (`lwc1`), unlike ties' s32. Loader: `max(dd, 16)`;
  billboard classes store `trunc(fade_distance)` as a byte (+0x17) and raise D to `F + 24`; the global cap
  at `gp-0x675c` is 500.0 (loader). Novalis: 984 × 32, 104 × 200, 81 × 5 (→ 16), 22 × 50, 8 × 242, 6 × 16,
  3 × 40. **[verified]**
* **VU1 transform.** `ShrubProc` uploads `V · [c0·s, c1·s, c2·s, (t − eye)·1024]` (s = class scale from
  col3.w); VU1 multiplies raw `itof0` positions: world = `t + M3 · raw · s / 1024`. The 2²³ bias in VU1's
  vf30/vf17 only builds GS-packet slot addresses. **[verified]**
* **Opaque-path alpha.** The opaque list sends the 5-qw matrix block (5th qw = 0) *before* the palette
  REF, so the palette unpack overwrites entry 0 again: opaque vertices keep the lit alpha 0x80; the
  translucent list sends the palette first, then the 5-qw block with (palette[0].rgb, fade alpha).
  **[verified DMA order]**
* **Fade arithmetic.** Non-billboard alpha = `min(ftoi12(D − z), 0x8000) >> 8`; billboard classes: z < F
  opaque mesh; F ≤ z < F + 8 mesh alpha `(0x8000 − ftoi12(z − F)) >> 8` (plus the billboard); beyond, and
  always when F = 0, billboard only. **[verified]**
* **6-vertex quirk.** The 7 retail packets whose drawn vertex 3 hits it (level 01 classes 126 / 156, 04:
  55, 05: 193, 06: 268, 14: 126, 18: 581) read `n + 2·vi13`, vi13 = `buffer + 5 + 0x1c·slot`. Only slot 0
  of buffer 0xee lands in instance data (the other buffer's slot 3 palette entry 18 + n, slot 4's matrix for
  n = 6..9, slot 4's palette entry n − 10 for n ≥ 10); every other slot reads a GS-packet output buffer.
  The contents are run-time state; the port models slot 0 / buffer 0xee with the instance's own palette
  standing in for the neighbour's (Novalis class 156, n = 20 → entry 10; class 126, n = 7 → a matrix
  column: kept as entry 7). **[address verified; colour model inferred]**
* **Fog.** VU1: `F = clamp(qw4.z · q_num + 254.99998, far_int, near_int)` (vf30.zw = 0x1de9d8/dc,
  vf29.xy = 0x1de9e0/e4 from `UpdateViewContext`); qw4.z comes from the 0x167180 view matrix
  (`0x16cfc0 · 0x167100`), not decoded, so the port uses the tfrag slope / offset on the origin depth.
  **[inferred]**
* **GS state.** Every retail shrub GIF tag has PRIM 0x7c: strip, Gouraud, textured, fog, ABE = 1.

## 6. Billboards (`ShrubProc` second half; level01 0x29dae4..0x29deec = boot 0x2298dc..0x229ce4)

Addresses: level01 first, boot (`fun_00228be8`, the exact-match copy) in parentheses; level01 = boot + 0x74208
for code. All **[verified]** in the disassembly and the ELF data unless tagged.

* **List** (0x29d274.. (0x22906c..)): for an instance of a class with a billboard pointer (class +0x1c) that
  passed the distance / frustum tests and is wholly inside the guard band, with `iz = ftoi12(max(z, 0)) − F·4096`:
  `iz < 0` → mesh only, opaque; else `a = min(iz, 0x8000) >> 8`; `a < 0x80` → mesh at `(0x8000 − iz) >> 8` **and**
  billboard entry `(index, a)`; `a = 0x80` (and always when F = 0) → billboard only, entry alpha
  `min(ftoi12(D − z) >> 1, 0x8000) >> 8`. The list is `(u16 index, u16 alpha)` at scratch 0x70002000 (0x300 entries,
  counter s0 → class +0x38 and 0x70003600 per class). An instance that is not wholly inside the guard band goes to
  the clip list (program 912339) before this test and is never a billboard. Port: `rc_formats::shrub::shrub_fade`.
* **GS setup** (0x29db78 (0x229970)): DMA/VIF DIRECT of 0x1c3090 (boot 0x1deb10): A+D TEST_1 = 0x5360b (ATE,
  ATST GEQUAL, AREF 0x60, AFAIL RGB_ONLY, ZTE, ZTST GEQUAL), CLAMP_1 = 5 (clamp S, T). Matrix vf28..31 = 0x186f80
  (view × projection), vf21 = 0x18cf20 (fog clamps), vf22 = 0x18cf10 (cf10, the Q numerator), vf23 = 0x18cea0
  (screen offset, fog offset); the corner table 0x1c3130 (boot 0x1debb0, 8 qw) goes to scratch 0x70003800.
* **Pass 1** (per class in class order, entries with `alpha & 0x80`), **pass 2** after 0x1c3060 (boot 0x1deae0:
  TEST_1 = 0x53001, alpha test NEVER + AFAIL RGB_ONLY = blend without Z write) with `alpha & 0x7f`. Alpha 0 is
  never drawn. Before a class's first sprite of a pass: the A+D tag 0x160b80 (NLOOP 1, 3 × A+D) + the class
  record's qw 1..3 (TEX1, TEX0, MIPTBP1, already converted at load, §8).
* **Sprite** (0x29dcd4..0x29de2c (0x229acc..0x229c24)): per entry, block = matrix block of `index`:
  `t = col3.xyz`, `scale = col3.w`, colour = `col1.w | alpha << 24` (`pextlb/pextlh` → RGBAQ), lengths
  `(lo, hi) = itof12` of the halves of col2.w (loader: `lo = min(trunc((|c0| + |c1|)·½·4096), 0x10000)`,
  `hi = trunc(|c2|·4096)` or 0 above 0x10000). `vf15 = (·, width·lo, height·hi, z_ofs·hi)·scale` (`vmulx.y`,
  `vmuly.zw`, `vmulw.yzw` by vf1.w). `d = (t − eye)·rsqrt((x² + y²) + z²)`; basis columns `(d.x, d.y, 0)`,
  `(d.y, −d.x, 0)`, `(0, 0, 1)`, `(t − eye)·1024`; corner k = table row `(0, y, z, 1)` with y·vf15.y and
  `z·vf15.z + vf15.w`, i.e. **world corner = t + y·W·(d.y, −d.x, 0) + (z·H + Z)·ẑ** with (W, H, Z) = vf15/1024.
  Rows (y, z; s, t): (−½, 1; 0, 0), (½, 1; 1, 0), (−½, 0; 0, 1), (½, 0; 1, 1). So: a Z-up cylindrical billboard
  about the instance origin, bottom edge at `z_ofs`, width shrinking by `cos(elevation)` (d is normalised in 3D).
  Projection `Q = cf10 / w`, `ST·Q`, `XY·Q + (2048, 2048)`, `F = clamp(w + cf14, If, In)` → `ftoi4`. GIF tag 0x160ba0/b0:
  NLOOP 1, EOP, PRE, **PRIM 0x7c** (triangle strip, Gouraud, textured, fog, ABE), 12 regs `ST RGBAQ XYZF2` × 4.
* **Colour**: col1.w = the loader's `sum/24` per channel of the 24 lit palette colours (top byte 0), so RGBA =
  (average lit colour, billboard alpha), MODULATE.
* Retail billboard records: `width·scale = height·scale = 1024` on Novalis (1-unit quads before the column
  lengths), `z_ofs` 0 or −½ height (centred), K −207..−167 (/16), MMIN 4; F from 5 to 256 (256 → byte 0).

## 7. Wind sway (the block at level01 0x29d4f8..0x29d670 inside `ShrubProc` 0x29cdf0 = the block at boot 0x2292f0..0x229468 inside `shrub_proc` 0x228be8)

The sway code is not a function of its own: it is a block inside the shrub renderer, so the addresses above are
block ranges, not function starts.

Entered from the per-instance matrix build when `mode = (mode_bits & 6) >> 1` (kept at scratch 0x70003f9c) is
non-zero, with vf1..vf3 = instance columns × scale and vf4 = t − eye (world units). Returns through 0x70003fa0
(= 0x29d3ec) to the normal `V · [c0, c1, c2, (t − eye)·1024]`. **[verified]**

```
scratch 0x70003b00 = 256 signed bytes (0x1c4370; boot 0x1dfdf0) = trunc(−127·sin(2πi/256))   (all 256 checked)
vf8 = (0.1, 0.02, 1.0, 0.0), vf9 = (0.2, 6000.0, 1/6000 (0x392ec33e), 0.5)   (0x1c4470; boot 0x1dfef0)
T   = frame counter gp−0x7808 = 0x15f3f8 (boot gp−0x77c8 = 0x15f438; `+= 1` per main-loop frame after sceGsSyncV)
A   = EE address of the instance's 0x40-byte matrix block (s3; = base 0x16049c-pointer + index·0x40)
d2 = (x² + y²) + 1·z² of vf4;  if 6000 − d2 < 0: no sway (MAC sign flag y via cfc2 vc1 & 0x40)
k  = 1 − d2·(1/6000)
u  = A·67 + T;   w = A·123 + T (mode 1)  or  A·123 + 2T (mode ≥ 2)          (u32)
s(i) = table[i & 0xff]/128   (lb, sll 5, vitof12)
g  = ((s(u)·s(u >> 1))·0.2 + (1 − 0.2))·0.1                              ∈ [0.06, 0.1]
sx = (s(w + 64)·0.02 + g·1.0)·k,   sy = (s(w >> 1)·0.02 + g·0.0)·k,   ·0.5 more when mode = 1
columns c → (c.x + sx·c.z, c.y + sy·c.z, c.z)   (M = [(1,0,0), (0,1,0), (sx, sy, 1)] · C)
```

So the whole instance is sheared in world space, proportionally to height above its origin: a constant lean of
6–10 % of the height towards +x (the wind) with a gust term of period 512 frames, plus ±2 % oscillation (x period
128 frames in mode 2 / 256 in mode 1, y period 256 / 512), fading out linearly in d² up to √6000 ≈ 77.5 units from the camera. The VU1
program is untouched (no per-vertex height term: 56467 just multiplies by the matrix). Phase: only `A mod 512`
matters (`A·67`, `A·123`, the `>> 1`), i.e. instances fall in 8 phase groups by `index mod 8` plus the block
base's low bits (heap state; **the port takes base ≡ 0 mod 512 [inferred]**), and T's value at level start is
run-time state too. Retail: 64 classes mode 2 (`mode_bits` 4), 3 classes mode 1.

**`TieProc` runs the same block** (level01 0x2aa53c..0x2aa6b0, between its LOD paths and its VU1 upload): the
same table and `vf8` / `vf9`, the same `u = phase·67 + T` / `w = phase·123 + T·(2 if mode 2 else 1)` math and
the same column shear. Only the phase differs: it takes `(A >> 9) & 0x1ff`, the matrix block's 512-byte page
rather than the address, so eight consecutive instances (0x40 bytes each) share a phase. `flags_24` supplies
the mode exactly as a shrub's `mode_bits` (`& 9` skips, `(& 6) >> 1` is the mode). The block base is heap
state too; **the port takes it as 0 [inferred]**. The port: `rc_formats::tie::wind_sway`,
`tie_lod` writes the shear into the per-frame LOD buffer and `tie.wgsl` applies it.

## 8. Shrub ad-gif conversion (boot `fun_00203b08`, called per shrub class by `transition_load_wad` 0x1ea830)

The shrub class init (also: relocates the packet / normal / billboard pointers, registers the class in
0x1d7f30 and the paging distance `trunc(mip_distance·1024)` in 0x1d8cb0, copies the entry's 16 texture bytes to
0x1d92b0 + class·0x10) rewrites every ad-gif block of every packet in place, with `e = shrub_textures[idx]`,
`idx = entry.textures[tex0.data_lo]`: **[verified]**

* TEX1 = `(e.ty − 1) << 2 | tex1.hi << 6 | 0x20 | tex1.lo << 32` (MXL = ty − 1, MMAG linear, MMIN from disc, K from disc);
* CLAMP = `clamp.lo | clamp.hi << 2 | idx << 24` (the byte `fun_00228a30` reads back per frame);
* MIPTBP1 = `max(1, w >> 7) << 14 | (e.mipmap + base) << 20 | 1 << 34 | (e.pad + base) << 40 | 1 << 54`;
* TEX0 = `max(1, w >> 6) << 14 | 0x1300000 | log2 w << 26 | log2 h << 30 | 1 << 34 | (e.palette + base) << 37 | 1 << 63`.

This is **bit for bit the tfrag rule** (`TfragAdGifs::gs_registers`, `fun_002040e0`) with the texture index taken
through the class slot table; unit test `ad_gif_conversion_is_the_tfrag_rule`. TBP0 / TBP1 stay 0 and are ORed in
each frame by `fun_00228a30` (level01 0x29cc38) from the shrub paging table 0x1d88b0, filled by `fun_0022a330`
(uploads base level `2^n` square and mip 1 from `+ 4·(2^(n−1))²`: the tfrag layout, so
`texture::decode_tfrag_mip_levels` applies; all 963 retail shrub textures decode). Retail: MMIN 4 everywhere,
K −142..−110 (/16), CLAMP (0,0), (0,1), (1,1); within a class every block naming one slot has the same TEX1 and
CLAMP. The header `mip_distance` is therefore **not** the TEX1 K source at run time (Wrench's fit); it is the
per-class paging distance. Mesh Q = 1/w with w from 0x186fc0 (w = z/n), so the mip rule is the tfrag one,
`round(log2(z/32) + K)`.

Billboard branch (class entry `ShrubBillboardInfo` present; else a default texture from 0x19e540): TEX1 =
`(max_mip − 1) << 2 | bb.tex1.hi << 6 | 0x20 | bb.tex1.lo << 32`; TEX0 = `(texture_offset + base) | max(1, w>>6) << 14
| log2 w << 26 | 0x1300000 | log2 h << 30 | (palette_offset + base) << 37 | 1 << 34 | 1 << 63`; MIPTBP1 =
`(mip1 + base) | max(1, w>>7) << 14 | (mip2 + base) << 20 | max(1, w>>8) << 34 | (mip3 + base) << 40 | max(1, w>>9) << 54`.
`max_mip` is the level count; the record's on-disc TEX0 data (1, 2, 8) is overwritten; the register address bytes
(0x14, 0x06, 0x34) are on the disc. **[verified]**

## In the port (billboards, sway, mips)

* `rc_formats::shrub`: `ShrubAdGifs::gs_registers`, `ShrubBillboard::gs_registers` / `ShrubBillboardRegs`,
  `packed_column_lengths`, `billboard_extent`, `billboard_corners`, `BILLBOARD_CORNERS`, `shrub_fade`,
  `sway_table`, `wind_sway`, `SWAY_VF8/9`; `rc_formats::texture::decode_billboard_mip_levels` (all 83 retail
  billboards decode).
* `crates/rc-engine/src/shrub_billboard.rs` + `assets/shaders/shrub_billboard.wgsl`: one mesh per billboard class,
  4 vertices per instance; the vertex shader evaluates §6 every frame (list rule, alpha, corners from the view
  position) from a static record (origin, F, centre, D, extent, average colour). Three draws per class
  (Transparent3d, `depth_bias` order): pass 1 texels As ≥ 0x60 with Z write, pass 1 texels below without, pass 2
  without; SrcAlpha blend, clamp sampler, mip chain with `round(log2(z/32) + K)` per pixel, per-vertex fog.
  Novalis: 108 instances in 3 classes (126 lanterns, 141 railing posts, 231). `RC_NO_BILLBOARDS=1` disables.
* Sway: `shrub_render::update_sway` (PostUpdate) calls `wind_sway` per sway instance each frame with
  T = `Time<Fixed>` elapsed / timestep (the 60 Hz fixed clock that also ticks moby animation; no second counter)
  and `A = index·0x40`, into a `vec2` storage buffer; `shrub.wgsl` applies `x += sx·y, z −= sy·y` (Bevy axes)
  to the model-space offset. Sway instances get `NoFrustumCulling`. `RC_SWAY=0` disables. PS2 float rounding
  (truncating FMAC) is not reproduced (IEEE f32, same operation order).
* Mips: `load_shrubs` decodes each used shrub texture's chain with `decode_tfrag_mip_levels` and billboard
  chains from gs_ram (re-read through `disc_source`, since `level_load` does not pass gs_ram to `load_shrubs`);
  per-vertex K (info bits 20..31), MXL per material, `textureSampleLevel` as tfrag / tie; per-slot CLAMP wrap.
* Not modelled: the guard-band clip list (912339; such instances are never billboards in the game), the list
  limits, T's start value and the block base's low bits (sway phase), billboard ordering against other
  translucent draws.
* GS test state: shrub meshes use TEST_1 0x5320b (AREF 0x20) for the opaque list and 0x530cb (AREF 0x0c) for the fading list, blocks 0x1dea80 / 0x1deab0, sent at boot 0x228e24 / 0x2296dc; ported in `gs_state.rs`.

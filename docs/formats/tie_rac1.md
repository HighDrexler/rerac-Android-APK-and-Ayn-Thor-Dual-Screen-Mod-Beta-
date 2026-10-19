# RAC1 tie class and instance format

*Reverse-engineered specification for the "tie" (instanced static prop) format of **Ratchet & Clank** (2002, PS2; Wrench game id `rac` / RAC1), written from a reading of the Wrench source tree (GPL; used as reference only, no code reproduced). Every claim is attributed to the Wrench file it came from. Facts that Wrench's code demonstrably relies on to parse real game data are marked **known**; things I deduced from field names, arithmetic or struct sizes are marked **inferred**; things Wrench does not model at all are listed under Unknowns.*

*Conventions assumed from the already-established context and not re-derived here: level `core_data` is the decompressed level core; `TieClassEntry {s32 offset_in_asset_wad; s32 o_class; s32 unknown_8; s32 unknown_c; u8 textures[16]}` lives in the core index (`src/wrenchbuild/level/level_core.h`); the tie `TextureEntry` table is a sibling array; world coordinates are Z-up with 1024 fixed-point units per world unit; VIF code decoding is covered by the tfrag spec.*

---

## 1. Where the pieces live

| Piece | Container | Wrench reference |
| --- | --- | --- |
| Tie class geometry blob | level core `data` section, at `TieClassEntry.offset_in_asset_wad`, aligned to 0x40 | `src/wrenchbuild/level/level_classes.cpp` (`unpack_tie_classes`, `pack_tie_classes`) |
| Tie class index | core index array `LevelCoreHeader.tie_classes` (`ArrayRange` at 0x20) | `src/wrenchbuild/level/level_core.h` |
| Tie texture table | core index array `LevelCoreHeader.tie_textures` (`ArrayRange` at 0x40) | `src/wrenchbuild/level/level_core.h`, `src/wrenchbuild/level/level_textures.h` |
| Tie instances | gameplay file, section pointer at header offset **0x34** | `src/instancemgr/gameplay.cpp` (`RAC_GAMEPLAY_BLOCKS`), `docs/gameplay.md` |
| Tie class-number list | gameplay file, section pointer at header offset **0x30** ("tie classes") | same |
| Tie collision | *not* in the class blob — baked into the single world-space level collision mesh | `docs/collision.md`, `docs/collision_recovery.md` |

RAC1 has **no** "tie groups" section and **no** separate "tie ambient rgbas" section; both appear only from R&C2 onwards (`src/instancemgr/gameplay.cpp`: `TieAmbientRgbaBlock` and `GroupBlock<TieGroupInstance>` are registered in `GC_UYA_GAMEPLAY_BLOCKS` and `DL_ART_INSTANCE_BLOCKS`, not in `RAC_GAMEPLAY_BLOCKS`; confirmed by the header maps in `docs/gameplay.md`). In RAC1 the ambient RGBA buffer is stored inline in the instance record instead (§4).

All offsets inside a class blob are **relative to the start of the blob** (Wrench parses each blob as a self-contained `Buffer`; `src/engine/tie.cpp`, `read_tie_class`). The blob contains no absolute pointers, which is why a tie class can be relocated freely in the core data section.

---

## 2. Tie class blob layout

### 2.1 Header

RAC1 uses a 0x70-byte header; GC/UYA/DL use a different, larger (0x80-byte) header. Wrench models them as two separate structs and normalises RAC1 into the later layout before parsing geometry (`src/engine/tie.h`: `RacTieClassHeader`, `GcUyaDlTieClassHeader`; `src/engine/tie.cpp`: `read_tie_header`).

**RAC1 tie class header (offset 0x00; Wrench models 0x70 bytes, the retail disc shows 0x80 — corrections in §9b)**

| Offset | Size | Type | Name (Wrench) | Meaning |
| --- | --- | --- | --- | --- |
| 0x00 | 4 | s32 | `packets[0]` | Offset to LOD 0 (highest detail) packet-header table |
| 0x04 | 4 | s32 | `packets[1]` | Offset to LOD 1 packet-header table |
| 0x08 | 4 | s32 | `packets[2]` | Offset to LOD 2 (lowest detail) packet-header table |
| 0x0c | 4 | u32 | `vert_normals` | Offset to the **64 light-slot normals** (`s16 x, y, z, 0`, unit length in 1/32767, 0x200 bytes, always immediately before the ad-gifs). Read by `LightTies` (§9b). Wrench never reads it. |
| 0x10 | 4 | f32 | `near_dist` | LOD distance threshold (highest detail) |
| 0x14 | 4 | f32 | `mid_dist` | LOD distance threshold (middle) |
| 0x18 | 4 | f32 | `far_dist` | LOD distance threshold (lowest / cull) |
| 0x1c | 4 | f32 | `unknown_1c` | On disc equal to `unknown_48`; `TieProc` zeroes it and uses bytes 0x1c..0x1f as per-frame LOD counters (verified) |
| 0x20 | 1 | u8 | `packet_count[0]` | Number of packets in LOD 0 |
| 0x21 | 1 | u8 | `packet_count[1]` | Number of packets in LOD 1 |
| 0x22 | 1 | u8 | `packet_count[2]` | Number of packets in LOD 2 |
| 0x23 | 1 | u8 | `texture_count` | Number of AD-GIF (material) records at `ad_gif_ofs` |
| 0x24 | 2 | u16 | `flags_24` | Render mode bits: `TieProc` skips the class when `& 9`, and `(& 6) >> 1` selects wind sway (the same shear block as `ShrubProc`, phased by `(matrix block address >> 9) & 0x1ff`; verified from the EE code; values 0/2/4 on disc) |
| 0x26 | 2 | u16 | — | 0 on disc; run-time instance count read by `TieProc` |
| 0x28 | 4 | u32 | `unknown_28` | 0 on disc; run-time instance list pointer |
| 0x2c | 4 | u32 | `ad_gif_ofs` | Offset to the AD-GIF table (`texture_count` × 0x50 bytes, §3) |
| 0x30 | 16 | Vec4f | `bsphere` | Bounding sphere. `Vec4f` is four `f32` in x, y, z, w order (`src/engine/basic_types.h`); w is **inferred** to be the radius, xyz the centre in class space. |
| 0x40 | 4 | f32 | `scale` | Position scale factor; see §5 |
| 0x44 | 4 | s32 | `o_class` | The class's own number (= core-index `o_class` on all 1,804 retail classes) |
| 0x48 | 4 | f32 | `unknown_48` | Distance-like float (5, 10, 12, 15, 16, 18, 20) |
| 0x4c | 4 | u32 | `unknown_4c` | 0 |
| 0x50 | 0x30 | `TieLodInfo[3]` | `lod_info` | Per LOD: `u32 strip_vertex_count` (Σ strip vertex counts), `u32 triangle_count` (Σ (count−2)), `u32 strip_count`, `u32 pad`; all three match the packets of every retail class |
| 0x80 | … | — | header extension | Up to the first packet table: bbox min / max (`Vec4f`, w = 1), then 8 corner points that `TieProc` reads at 0xa0..0x11f for frustum tests. The first packet table is at 0x140 on 1,803 classes and at 0xc0 (no corners) on one. |

Fields Wrench actually consumes (**known-good**, since they are what make real level data parse): `packets[3]`, `packet_count[3]`, `texture_count`, `near/mid/far_dist`, `ad_gif_ofs`, `scale`, `bsphere`. Everything in the 0x44–0x6c tail is dead weight as far as Wrench is concerned.

### 2.2 RAC1 versus GC/UYA/DL headers

`GcUyaDlTieClassHeader` (`src/engine/tie.h`) keeps the same *concepts* but moves nearly everything and adds a lot:

| Field | RAC1 offset | GC/UYA/DL offset | Note |
| --- | --- | --- | --- |
| `packets[3]` | 0x00 | 0x00 | same |
| `packet_count[3]` | 0x20 | 0x0c | moved up next to the packet pointers |
| `texture_count` | 0x23 | 0x0f | |
| `near/mid/far_dist` | 0x10/0x14/0x18 | 0x10/0x14/0x18 | identical |
| `ad_gif_ofs` | 0x2c | 0x1c | |
| `vert_normals` | 0x0c | 0x34 | GC+ also has `vert_normal_count` (s16 @ 0x38) |
| `scale` | 0x40 | 0x40 | identical |
| `bsphere` | 0x30 | 0x50 | |
| Per-LOD `TieLodHeader[3]` (`vert_count`, `tri_count`, `strip_count`, pad; 8 bytes each) | **absent** | 0x60 | RAC1 gives no per-LOD vertex/triangle/strip totals at all |
| `instance_index` (s32), `instance_count` (s16) | **absent** | 0x20, 0x3e | GC+ class blobs know about their instances |
| `cache_sizes[3]` (s16) | **absent** | 0x24 | |
| `rgba_remap_ofs[3]` (s16), `glow_remap_ofs[3]` (s16) | **absent** | 0x2a, 0x78 | |
| `ambient_rgbas` (s32), `ambient_size` (s16) | **absent** | 0x30, 0x3a | GC+ stores ambient RGBA bookkeeping in the class |
| `mode_bits` (s16) | **absent** | 0x3c | |
| `o_class` (s16), `t_class` (s16) | **absent** | 0x44 | GC+ blobs are self-identifying |
| `mip_dist` (f32) | **absent** | 0x48 | |
| `glow_rgba` (s32) | **absent** | 0x4c | |
| Header size | 0x70 | 0x80 | |

The practical RAC1 differences: no per-LOD geometry counts, no self-described class id, no ambient/glow remap tables, no mip distance, no instance back-references — and ~0x2c bytes of unidentified tail that plausibly hold RAC1 analogues of some of these (**inferred, unverified**).

### 2.3 LODs and packet tables

There are always exactly **three** LODs (`TieClass::lods[3]`, loop `for (s32 i = 0; i < 3; i++)` in `read_tie_class`, `src/engine/tie.cpp`). A LOD may be empty (`packet_count[i] == 0`).

For LOD *i*: at blob offset `packets[i]` there is a densely packed array of `packet_count[i]` × 0x10-byte packet headers. Packet *j*'s data begins at blob offset `packets[i] + packet_table[j].data` — i.e. the `data` field is **relative to the start of that LOD's packet table**, not to the blob (`read_tie_class`).

`near_dist`/`mid_dist`/`far_dist` select which LOD the renderer uses at runtime (**inferred** from names plus `docs/renderers.md`: "The LOD system is similar to that of the tfrag renderer"). Wrench's exporter only ever uses LOD 0.

### 2.4 Packet header (`TiePacketHeader`, 0x10 bytes)

`src/engine/tie.h`. Byte-granular unless noted; `_ofs`/`_size` fields are in **quadwords (0x10 bytes)** relative to the start of the packet data.

| Offset | Type | Name | Meaning |
| --- | --- | --- | --- |
| 0x0 | s32 | `data` | Offset of this packet's data, relative to the LOD's packet table (**known** — used by the reader) |
| 0x4 | u8 | `shader_count` | Number of AD-GIF/material blocks referenced by this packet (**inferred**; Wrench ignores it and instead walks the 4-entry AD-GIF offset arrays) |
| 0x5 | u8 | `bfc_distance` | **Corrected:** = `5 × shader_count`, the quadwords of ad-gif data (verified on every packet) |
| 0x6 | u8 | `control_count` | **Corrected:** = `3 + strip_count`, the V4_8 elements (VU quadwords) of unpack header + strips that matter |
| 0x7 | u8 | `control_size` | Quadwords of unpack header + strips; `TieProc` uploads them V4_8 USN (`num = 4 × control_size`) right after the ad-gifs |
| 0x8 | u8 | `vert_ofs` | Quadword offset of the vertex region within the packet data (**known** — reader multiplies by 0x10) |
| 0x9 | u8 | `vert_size` | Quadword size of the vertex region (**known**) |
| 0xa | u8 | `rgba_count` | **Corrected (`color_ofs`):** quadword offset of the per-vertex colour-index region (= `vert_ofs + vert_size`) |
| 0xb | u8 | `multipass_ofs` | **Corrected (`color_count`):** 4-byte colour-index elements = `ceil(dinky/4) + fat`. The region holds two copies, each padded to a quadword: copy A uploaded to VU 0xcc, copy B (every used index + 0x40) to 0xf8, one per per-instance double buffer (`TieProc` template qwords at boot 0x160a00) |
| 0xc | u8 | `scissor_ofs` | Quadword offset of a per-strip-vertex GS slot step table (`strip_vertex_count + 1` bytes: first slot, then +3 steps, strip starts flagged with the sign bit); not used by program 13507, not decoded further |
| 0xd | u8 | `scissor_size` | Its size in quadwords; the packet ends at `(scissor_ofs + scissor_size) × 16` (verified) |
| 0xe | u8 | `nultipass_type` | **Corrected:** strip count (= unpack header `strip_count`) |
| 0xf | u8 | `multipass_uv_size` | **Corrected:** Σ strip `vertex_count` |

`TiePacket` in Wrench carries `multipass` and `scissor` byte vectors, but `read_tie_packet` never populates them: the multipass and scissor regions are **not parsed at all** by Wrench.

---

## 3. Per-packet data stream

### 3.1 Important caveat about "VIF stream"

Wrench does **not** decode VIF codes for ties. `src/engine/tie.cpp` reads the packet data at fixed byte offsets and uses the quadword offsets/sizes from `TiePacketHeader`; `src/engine/tie.h` includes `core/vif.h` only for the fixed-point helper macro. So, unlike the tfrag path (which walks real VIF code lists), the tie path treats a packet as **raw quadword regions**.

The `/* PACK UNPK */` twin-offset columns that Wrench writes above every field of `TieUnpackHeader`, `TieStrip`, `TieDinkyVertex` and `TieFatVertex` (`src/engine/tie.h`) document, for each record, its byte offset in the file (**PACK**) and its byte offset once resident in VU1 data memory (**UNPK**). The expansion pattern is unambiguous:

| Record | Packed stride | Element type | Unpacked stride | Implied unpack |
| --- | --- | --- | --- | --- |
| `TieUnpackHeader` | 0x0c (12 × u8) | u8 → u32 | 0x30 (3 qw) | V4_8, 3 quadwords |
| `TieStrip` | 0x04 (4 × u8) | u8 → u32 | 0x10 (1 qw) | V4_8, 1 quadword per strip |
| `TieDinkyVertex` | 0x10 (8 × u16) | u16 → u32 | 0x20 (2 qw) | V4_16, 2 quadwords per vertex |
| `TieFatVertex` | 0x18 (12 × u16) | u16 → u32 | 0x30 (3 qw) | V4_16, 3 quadwords per vertex |

So ties use the same V4_8 / V4_16 "expand each element into a 32-bit VU field" convention as tfrags (V4_16 for vertex info). Whether the STROW/STMOD/STCYCL/UNPACK tags that perform these expansions are stored inside the class blob outside the regions Wrench parses, or are generated by the engine at load/draw time from the quadword offsets and sizes in `TiePacketHeader`, is **not answerable from Wrench** — Wrench never touches them. (`TieDinkyVertex` also has one suspicious UNPK annotation: the last field is listed at 0x20 where the regular stride predicts 0x1c; that looks like a typo in Wrench rather than a real gap.)

### 3.2 Packet data layout

All offsets below are relative to the first byte of the packet data (`src/engine/tie.cpp`, `read_tie_packet`):

| Offset | Size | Contents |
| --- | --- | --- |
| 0x00 | 0x10 | `s32 ad_gif_dest_offsets[4]` — GS-packet quadword offsets at which AD-GIF blocks 1..3 are emitted (see indexing note below) |
| 0x10 | 0x10 | `s32 ad_gif_src_offsets[4]` — byte offsets into the class's AD-GIF table; `offset / 0x50` = material index |
| 0x20 | 0x0c | `TieUnpackHeader` |
| 0x2c | 4 × `strip_count` | `TieStrip[strip_count]` |
| `vert_ofs`×0x10 | `vert_size`×0x10 | Vertex region: `TieDinkyVertex[dinky_count]` (0x10 each) immediately followed by `TieFatVertex[...]` (0x18 each) filling the rest of the region |
| `multipass_ofs`×0x10 | — | multipass data (not parsed) |
| `scissor_ofs`×0x10 | `scissor_size`×0x10 | scissor data (not parsed) |

Only 4 AD-GIF slots exist per packet, giving a hard limit of **4 materials per tie packet** (**known** from the fixed `read_multiple<s32>(…, 4, …)` reads).

**`TieUnpackHeader` (0x0c bytes @ 0x20)**

| Offset | Type | Name | Meaning |
| --- | --- | --- | --- |
| 0x00 | u8 | `unknown_0` | **`dinky_single_only`**: non-zero = no double-write phase for dinky vertices (§3.4) |
| 0x01 | u8 | `unknown_2` | **`no_fat`**: non-zero iff there are no fat vertices |
| 0x02 | u8 | `unknown_4` | unknown (often the first strip's GIF tag offset) |
| 0x03 | u8 | `strip_count` | Number of `TieStrip` records that follow at 0x2c (**known**) |
| 0x04 | u8 | `unknown_8` | **`dinky_single_end`**: GS slot of the dinky vertex that ends VU1's single-write loop |
| 0x05 | u8 | `unknown_a` | **`dinky_double_end`**: GS slot ending the dinky double-write loop |
| 0x06 | u8 | `unknown_c` | **`fat_single_end`**: GS slot of the last single-write fat vertex |
| 0x07 | u8 | `unknown_e` | **`fat_double_end`**: GS slot of the last fat vertex |
| 0x08 | u8 | `dinky_vertices_size_plus_four` | Size of the unpacked "dinky" vertex block **in VU quadwords, plus 4**. Wrench derives `dinky_count = (value - 4) / 2` (**known**), which is consistent with 2 quadwords per dinky vertex and a 4-quadword bias (**inferred**: 3 quadwords of unpacked header plus one more). |
| 0x09 | u8 | `fat_vertices_size` | `3 × fat_count + 6` (VU1's fat conversion-loop bound; verified). Wrench reads fat vertices to the end of the vertex region instead, which gives the same count (8,402 packets end the region with 8 bytes of padding, less than one fat vertex) |
| 0x0a | u8 | `unknown_14` | **`dinky_count`** |
| 0x0b | u8 | `unknown_16` | **`fat_count`** |

Note that Wrench's field names for this record are written in *unpacked* numbering (`unknown_2` sits at packed byte 0x01, etc.), a consequence of the dual-offset annotation style.

**`TieStrip` (4 bytes, 1 per strip)**

| Offset | Type | Name | Meaning |
| --- | --- | --- | --- |
| 0x0 | u8 | `vertex_count` | Vertices in this strip (Wrench ignores it and derives the count from the GS-offset walk) |
| 0x1 | u8 | `pad_1` | padding |
| 0x2 | u8 | `gif_tag_offset` | Quadword offset in the GS packet where this strip's GIF tag is written (**known** — drives the walk) |
| 0x3 | u8 | `rc34_winding_order` | Non-zero flips the strip's winding for backface culling. Named `rc34_*` in Wrench and added by a commit titled "Export corrected winding order for rc3/4 ties" (`src/engine/tie.h`, commit 97d74d73), i.e. it is believed meaningful only in R&C3/Deadlocked; for RAC1 it is expected to be 0 (**inferred**). |

**`TieDinkyVertex` (0x10 bytes) — the common vertex form**

| Offset | Type | Name | Meaning / scale |
| --- | --- | --- | --- |
| 0x00 | s16 | `x` | Position X, class-space fixed point; multiply by `scale / 1024` (§5) |
| 0x02 | s16 | `y` | Position Y |
| 0x04 | s16 | `z` | Position Z |
| 0x06 | u16 | `gs_packet_write_ofs` | Quadword offset in the GS packet this vertex writes to (**known** — this is the sequencing key) |
| 0x08 | u16 | `s` | Texture coordinate S, signed 1/4096 fixed point (`vu_fixed12_to_float` casts to s16 then × 1/4096 — `src/core/vif.h`) |
| 0x0a | u16 | `t` | Texture coordinate T, same scale |
| 0x0c | u16 | `q` | Perspective/Q component of the ST/RGBAQ pair; carried through by Wrench but not used in export |
| 0x0e | u16 | `gs_packet_write_ofs_2` | Second GS-packet quadword offset, or 0. Non-zero and different from the first means the same vertex is emitted twice, letting one stored vertex serve two strips (**known** — Wrench duplicates the vertex in that case). |

**`TieFatVertex` (0x18 bytes) — used for the tail of the vertex region**

| Offset | Type | Name | Meaning |
| --- | --- | --- | --- |
| 0x00 | s16 | `unknown_0` | **Corrected: LOD-morph delta x.** VU1 draws a fat vertex at `position + k × delta` (`mulx.xyz vf05, vf05, vf27` / `add.xyzw vf05, vf05, vf28`, program 13507 L25), `k` = per-instance qword 4 x. Not lighting data. |
| 0x02 | s16 | `unknown_2` | morph delta y |
| 0x04 | s16 | `unknown_4` | morph delta z |
| 0x06 | u16 | `gs_packet_write_ofs` | as dinky |
| 0x08 | s16 | `x` | Position X (same scale as dinky) |
| 0x0a | s16 | `y` | Position Y |
| 0x0c | s16 | `z` | Position Z |
| 0x0e | u16 | `pad_e` | padding |
| 0x10 | u16 | `s` | S, 1/4096 |
| 0x12 | u16 | `t` | T, 1/4096 |
| 0x14 | u16 | `q` | Q |
| 0x16 | u16 | `gs_packet_write_ofs_2` | as dinky |

Dinky and fat vertices are geometrically interchangeable: Wrench widens every fat vertex into the dinky form, keeping only position, UV, Q and the two write offsets (`read_tie_packet`).

### 3.3 The GS-packet address space: how strips become triangles

Everything in a packet is sequenced by **quadword offsets into the GS packet that VU1 builds**, not by file order. Wrench reconstructs the draw order by simulating that address space with a single cursor (`read_tie_packet`). The step sizes are the load-bearing facts:

| Element | Quadwords consumed in the GS packet | Why (inferred from the step size) |
| --- | --- | --- |
| AD-GIF / material block | **6** | 1 GIF tag in A+D mode with nloop = 5, followed by five 16-byte A+D register writes (`TieAdGifs` is exactly 5 × `GifAdData16`) |
| Strip GIF tag | **1** | one GIF tag opening a `GS_PRIMITIVE_TRIANGLE_STRIP` PACKED-mode primitive |
| Vertex | **3** | three PACKED-mode registers per vertex — consistent with ST, RGBAQ, XYZ(F)2 (`src/engine/gif.h` register enums) |

The reconstruction algorithm:

1. Start the cursor at 0. The first AD-GIF block is always at GS-packet offset 0, so material = `ad_gif_src_offsets[0] / 0x50`, and the cursor advances by 6.
2. Loop while strips or vertices remain, testing in this order:
   - If the next unconsumed strip's `gif_tag_offset` equals the cursor → begin a new primitive (triangle strip) with the current material and the strip's winding flag; cursor += 1.
   - Else if the next unconsumed vertex's `gs_packet_write_ofs` equals the cursor → append that vertex to the current primitive; cursor += 3. (A vertex arriving with no open primitive is a hard error: "Tie has bad GS packet data.")
   - Else if `ad_gif_dest_offsets[k-1]` equals the cursor, where *k* is the next AD-GIF index → switch material to `ad_gif_src_offsets[k] / 0x50`; cursor += 6.
   - Otherwise the packet is malformed.
3. Note the off-by-one indexing: `ad_gif_dest_offsets[k-1]` holds the GS-packet position of AD-GIF block *k*. Slot 0 of the dest array therefore describes the *second* material switch, because the first block's position is implicitly 0.

Before that walk, the vertices must be normalised, because **the file does not store vertices in GS-packet order** (Wrench's comment attributes this to buffering/pipelining in the VU program):

1. Expand each stored vertex into one or two logical vertices (the `gs_packet_write_ofs_2` duplication above), for dinky and fat vertices alike.
2. Sort all logical vertices ascending by `gs_packet_write_ofs`.
3. Drop consecutive duplicates with identical write offsets. Wrench's stated reason: a packet needs a minimum of 4 regular vertices, so tiny packets are padded with repeats that can be removed safely.

Triangulation is ordinary triangle-strip alternation with the winding flag as a parity offset: for vertex index *i* ≥ 2 inside a primitive, if `i % 2 == winding_order` emit (i-2, i-1, i), otherwise emit (i, i-1, i-2) (`recover_tie_class`, `src/engine/tie.cpp`). With `winding_order == 0` (the RAC1 case) this is the standard strip flip.

### 3.4 What the game actually does (EE `TieProc` + VU1 program 13507), and the loader's walker

**Upload per packet** (`TieProc`, level01 0x2a9a90, DMA templates at boot 0x1609d0): REF 1 qw `ad_gif_dest` → VU 0 (V4_32); `shader_count` × REF 5 qw from `ad_gif_ofs + ad_gif_src[k]` → VU 1 + 5k; `MSCAL 4`; REF `control_size` qw (unpack header + strips) V4_8 USN → VU 1 + 5·`shader_count`; REF `vert_size` qw V4_16 (signed) → VU 0x32; `MSCAL 6` (program L1: ad-gif and strip-tag placement, vertex format conversion); the two colour-index copies V4_8 USN → 0xcc / 0xf8. Then per instance: REF 6 qw of per-instance transform/LOD data → 0xc6 (FLG, double-buffered), REF 0x10 qw of lit instance colours (the instance's 64 × RGBA from `LightTies`) STMOD 1 → 0x346 / 0x386, `MSCAL 0` (program L9: draw).

**Placement** (program 13507; line numbers are `work/vu/13507.txt`):
* Ad-gifs, L1/L2 (lines 12–45): block 0 at GS slot 0, block k at `ad_gif_dest[k-1]`; `mtir vi04, vf07.x` / `ibgtz vi04, L2` / `mr32.xyz vf07, vf07` loop while the next entry is > 0. Each block = GIF tag template (VU 966) + the 5 A+D qwords, written into both output buffers (`vf21.x`, `vf21.y`).
* Strip GIF tags, L3/L4 (lines 46–84): `ilwr.x vi07` = `vertex_count`, `ilwr.z vi04` = `gif_tag_offset`; template VU 967 with `iswr.x vi07` → **NLOOP = the stored `vertex_count`**; the last strip gets `vi07 += 0x8000` (two `iaddiu vi07, vi07, 0x4000`, lines 67–68) = **EOP**.
* Vertices, L9..L39: processed in stored order, dinky then fat. Each writes ST / RGBA / XYZ to `gs_slot` (`mtir vi12, vfNN.w`; `sq vf15, 0(vi12)`, `sq vf11, 1(vi12)`, `sq vf19, 2(vi12)`); in a double-write phase also to `gs_slot_2` (`mtir vi13, vfNN.w`, same three stores at `vi13`). The phases (L8 loads the markers: `vi05` = unpack[0], `vi14` = [1], `vi04` = [4], `vi06` = [5], `vi07` = [6], `vi08` = [7], lines 140–145):
  * dinky single-write loop L10 exits after storing the vertex at slot `dinky_single_end` (`ibeq vi04, vi12, L13`, line 194); the 3 vertices already in the pipeline are stored single-write (L13 / L21–L24). If `dinky_single_only` (`ibne vi00, vi05, L21`, line 313) the dinky vertices end there;
  * otherwise the double-write loop L14..L17 runs until the vertex at `dinky_double_end` (`ibeq vi06, vi12, L18`, line 351) plus 2 more (L18–L20);
  * then, unless `no_fat` (`ibeq vi00, vi14, L25..L28`), fat vertices: single-write loop L29..L32 up to and including the vertex at `fat_single_end` (`ibeq vi07, vi12, L33..L35`), then double-write L33..L39 up to and including the vertex at `fat_double_end` (`ibeq vi08, vi12, L40`);
  * L40–L42: `xgkick` of the buffer.
* Colour: each dinky vertex takes the next colour-index byte (`mtir vi09, vf24.{x,y,z,w}`, `lq vf11, 838(vi09)`); each fat vertex takes one 4-byte element `(c0, c1, c2, 0xff)` starting at the next 4-byte boundary, and VU1 blends `c0 × z + ((c1 + c2) / 2) × w` with per-instance weights (`mulay/maddy vf29, vf30, vf27`, `maddz vf11, vf11, vf27`, lines 569–583). Indices are 0..63 = light slots (VU 838 = 0x346 is the instance palette).

Program 224979 (`TieProc`'s second VU1 program) places ad-gifs and strip tags with the identical L1–L4 code (its lines 31–87); its per-vertex path was not analysed.

**Loader walker** (`crates/rc-formats/src/tie.rs`, ported from the retired C++ `tie.cpp`), exactly the rules above:
1. Resolve vertices in processing order with the phase markers; a vertex in a double-write phase also occupies `gs_slot_2`. Missing markers, markers that do not end at the last dinky / fat vertex, or a double-write vertex with `gs_slot_2 = 0` (VU1 would overwrite the first ad-gif tag) are errors.
2. Slot map: the last writer in processing order wins (93 retail slots are written twice, always with identical data).
3. GS walk from slot 0: an ad-gif at the cursor (positions from step "Ad-gifs"; `ad_gif_dest[3] > 0` or a placed count ≠ `shader_count` is an error) sets the material to `ad_gif_src[k] / 0x50` and advances 6; a strip tag at the cursor reads `vertex_count` vertices from slots `cursor + 1 + 3i` and advances `1 + 3 × vertex_count`; stop after the last (EOP) strip. An unwritten slot, no tag at the cursor, or a written slot no strip reads is an error.
4. Triangles per strip as in §3.3 (parity with `winding`, 0 on every retail strip).

On the retail disc this agrees with Wrench's reconstruction (sort, dedupe, cursor) on every packet; the golden test checks that independently. The difference is only in what is trusted: the strip's stored `vertex_count` (VU1's NLOOP) rather than a count derived from the cursor, and the phase markers rather than "any non-zero `gs_slot_2`" (on disc, `gs_slot_2 ≠ 0` exactly in the double-write phases).

### 3.5 LOD selection and morphing (`TieProc`, VU1 13507 / 224979)

Verified from the boot disassembly of `TieProc` (boot 0x235be8..0x2370b8; level01 0x2a9a90 is the same code) and the VU1 listings; ported in `crates/rc-engine/src/tie_lod.rs` + `assets/shaders/tie.wgsl`. This section supersedes the "not ported: morph factor" remarks of §9b and the unknowns of §10 about `near/mid/far_dist` and the colour weights.

**Pass 1, cull** (0x235f1c..0x236040; per instance, run-time record: +0x00 world sphere centre / radius, +0x14 draw distance). `vf24` = camera position (w = 1024), `vf25..27` = view rows 0x186f40, so camera-space `(x, y, z)` is in **world units**. Culled when: distance = 0; `(min(dist, 720.0) − r) − (z − r) < 0` (`pminw` with gp−0x5c90, `vsubw.y vf19`, sign of `vf3.y`); `n/1024 − (z + r) ≥ 0` (vf19 = 0x18cda0 × 1/1024); the occlusion bit (+0x18/+0x19 against 0x70003000) is clear; or the sphere is wholly outside a side plane (`tan·z − (|x| − r·k) < 0`, vf20 = 0x18cdb0, vf22 = 0x18cee0, as for tfrags). An instance not wholly inside the guard band (box corners through 0x187040) is flagged (+0x08 = 0) for the clipping program 224979 but gets the same LOD. The record keeps `(z − r, max(0, z), flag, dma address)` (`vmax.z vf1, vf0, vf2`, `vmr32.xy vf1`).

**Pass 2, LOD** (0x23626c..0x2364c8). `vf16` = class header 0x10 = `(near_dist, mid_dist, far_dist, ·)` (`lqc2 vf16, 0x10(t8)`), `depth = max(0, z)` of the **centre** (world units; no radius, no instance scale). `vsuby.xyz vf2, vf16, vf1` = dists − depth, then by sign bit (`dsll32`/`bgez`, `prot3w`/`bltz`, `bgez`):

| Condition (in this order) | Packet list | k (qw4.x) | chain / counter |
| --- | --- | --- | --- |
| near − depth ≥ 0 | LOD 0 | 0 | s0 / t4 (class +0x1c) |
| far − depth < 0 | LOD 2 | 0 | s2 / t6 (+0x1e) |
| mid − depth ≥ 0 | LOD 0 | (depth − near) / (mid − near) | s0 / t4 |
| otherwise | LOD 1 | (depth − mid) / (far − mid) | s1 / t5 (+0x1d) |

k is `vdiv Q` of `0 − (near − depth)` by `mid − near` (0x2363c0..0x236454; LOD 1: 0x236328..0x2363bc), so it lies in (0, 1] with no explicit clamp. Per-instance VU1 **qw4** = `(Q, ·, 256 − 256·Q, 256·Q)` (`lui at, 0x4380` / `pextlw` / `pcpyld` = 256.0 ×4, `vmulq.w`, `vaddq.x`, `vsubw.z`), stored at `sp + 0x40`; the static paths store `(0, 0, 256.0, 0)` (`por at, zero, zero; lui at, 0x4380; prot3w at, at`). **qw5** = `(0, 0, 0, F)` with the fog value from the centre depth: `F = max(min(cf24 + depth·cf20, In), If)` (`vadda.y`/`vmaddx.y` with vf18 = 0x18cf20 = `(cf20, cf24, In, If)` from `UpdateViewContext`, `vminibcz.y`, `vmaxy.w`) — the same line as the tfrag F, taken at the sphere centre (not at depth + r). qw0..3 = the instance matrix (columns × class scale, translation − camera, × 1024) × the guard-band view-projection 0x186fc0. The 6 qw go to VU 0xc6 (FLG) followed by the colour-index copy at 0xcc (template boot 0x160a10).

Novalis classes use the distance sets 10/20/30 (54 classes), 10/20/1024 (35) and a few others (20/30/40, 30/40/50, 50/75/100, 70/90/110, …): LOD 0 is drawn statically only within 10 units, morphs over 10..20, and LOD 1 morphs over 20..30 (or 20..1024).

**VU1 morph** (13507 L25..L39, lines 562–1006; 224979 lines 588–670 identical). A fat vertex's three unpacked qw are converted with `itof0` (delta and position alike, L7) and processed as
* position: `mulx.xyz vf05, vf05, vf27` (delta × k, qw4 = vf27) then `add.xyzw vf05, vf05, vf28` (+ base position, w = GS slot + buffer offset): **`position + k·delta` in s16 units**, then × scale/1024 by the instance matrix. k = 0 is the full-detail shape.
* colour: `addi.y vf27, vf00, I` with I = 0.5 (qw4.y is overwritten), `mulay/maddy vf29, vf30, vf27` → `avg = c1·0.5 + c2·0.5`, then `mulaw.xyzw ACC, vf29, vf27` and `maddz.xyzw vf11, vf11, vf27` → `c0·(256 − 256k) + avg·256k`, every step a truncating VU FMAC. The palette lanes (VU 838) are **`0x4b000000 + byte`**: the lit colours are unpacked V4_8 USN with STMOD 1 and STROW = 0x4b000000 ×4 (boot 0x1de8e0). The sum lands in [2^31, 2^32) where one float step is 256, so the GS (PACKED RGBAQ takes bits 0..7) sees `floor(c0·(1 − k) + ⌊(c1 + c2)/2⌋·k)` or one less (truncated products); at k = 0 exactly `c0`, at k = 1 exactly the truncated average. With the VU adder model, `256 − 256k` rounds so that `w + z ∈ [256, 256 + 2^-16)`, so a lane never drops below 2^31 (no byte wrap). Dinky vertices store `pal[c0]` as is.

LOD 1 packets have fat vertices too (Novalis 3,175 of 32,259) and morph towards LOD 2 over mid..far; LOD 2 has none and never morphs.

**The geometry the morph targets** (disc check, `tie.rs` test `novalis_fat_vertices_morph_onto_the_next_lod`): for every Novalis class, every LOD-0 fat vertex at k = 1 lies on the LOD-1 triangle surface (44,635 of 44,635 within 2.45 s16 units, i.e. the position quantisation; at k = 0 only 14,255), and every LOD-1 fat vertex at k = 1 on the LOD-2 surface (3,175 / 3,175, worst 2.26). The targets are not LOD-1 *vertices* (fewer than 0.2 % coincide): a fat vertex is one the coarser LOD removes, slid onto the coarse surface. Dinky vertices shared by both LODs have the same positions. So LOD n at k = 1 is the LOD n+1 surface, and the switch at `depth = mid` (or `far`) is seamless.

---

## 4. Texture binding and AD-GIF register blocks

The class's AD-GIF table sits at `ad_gif_ofs` and holds `texture_count` records of 0x50 bytes each (`read_tie_class`). One record = one material slot.

**`TieAdGifs` (0x50 bytes)** — `src/engine/tie.h`

| Offset | Register (GS) | Wrench name |
| --- | --- | --- |
| 0x00 | `TEX0_1` (0x06) | `d1_tex0_1` |
| 0x10 | `TEX1_1` (0x14) | `d2_tex1_1` |
| 0x20 | `MIPTBP1_1` (0x34) | `d3_miptbp1_1` |
| 0x30 | `CLAMP_1` (0x08) | `d4_clamp_1` |
| 0x40 | `MIPTBP2_1` (0x36) | `d5_miptbp2_1` |

Each entry is a `GifAdData16` (`src/engine/gif.h`): `s32 data_lo` @0x0, `s32 data_hi` @0x4, `u8 address` @0x8 (the GS register number, per the `GifAdAddress` enum), then 7 bytes of padding to 0x10. The 16-byte stride (rather than the 12-byte `GifAdData12`) is what makes a 5-register block occupy 5 quadwords, and with its GIF tag, 6 — matching the cursor arithmetic in §3.3.

Material slot resolution, end to end:

1. A primitive's `material_index` = `ad_gif_src_offsets[k] / 0x50` = index into the class's AD-GIF table.
2. That same index selects `TieClassEntry.textures[index]` in the core index — a byte index into the tie `TextureEntry` array, terminated by `0xff` (`unpack_level_materials`, `src/wrenchbuild/level/level_textures.cpp`: it walks slots 0..15 and stops at the first `0xff`). So the *n*-th AD-GIF record corresponds to the *n*-th material slot of the class, which corresponds to the *n*-th entry of `textures[16]`.
3. The `TextureEntry` (`src/wrenchbuild/level/level_textures.h`) gives `data_offset` (relative to `textures_base_offset`), `width`, `height`, `type`, `palette` (× 0x100 = byte offset into GS RAM for 256 × u32 CLUT), `mipmap`, pad. Textures are 8-bit paletted.
4. Wrench's packer caps a tie class at **15** textures ("Too many textures on tie class …", `src/wrenchbuild/level/level_textures.cpp`), presumably leaving slot 16 for the `0xff` terminator.

The `TEX0`/`MIPTBP` bit fields in the AD-GIF block encode the GS texture base pointer, buffer width, pixel format, dimensions and CLUT location. Wrench never parses those bits for ties (it only copies the blocks verbatim), so how the engine patches the base pointers when textures are uploaded to GS RAM is **not documented by Wrench** for ties.

---

## 5. Class space to world space

Two steps, both taken from Wrench:

1. **Class-space position.** `pos = (f32) vertex.{x,y,z} * (header.scale / 1024.0)` — `recover_tie_class`, `src/engine/tie.cpp`. The s16 vertex components are therefore in units of `1/1024` world unit *scaled by* the per-class `scale` factor, which lets a class trade precision against extent. Wrench applies the identical factor to X, Y and Z (uniform).
2. **World transform.** `world = instance.matrix × vec4(class_pos, 1)`. The instance's 4×4 matrix is used directly, with nothing else applied: `src/wrenchvis/wrenchvis.cpp` feeds `instance.transform().matrix()` straight into the occlusion rasteriser alongside the already-scaled class mesh, and `src/editor/renderer.cpp` (`upload_instance_buffer`, `draw_tie_instances`) uploads that same matrix as the per-instance model matrix for the class mesh.

Consequences: the instance matrix is a full affine transform in **world units** (it may contain rotation, non-uniform scale and shear — Wrench decomposes it into pos/rot/scale only for display purposes, `TransformComponent::set_from_matrix` in `src/instancemgr/instance.cpp`), stored column-major as four `Vec4f` columns with the translation in the fourth (`Mat4::unpack`, `src/engine/basic_types.h`; `glm::mat4` indexing `m[3]` = translation column). Coordinates are the usual Z-up world frame; no axis swap happens anywhere in the tie path.

One quirk worth recording: the game stores **0.01f** in matrix element `[3][3]` (blob offset 0x4c of the instance record), not 1.0. Wrench overwrites it with 1.0 on read and re-writes 0.01 on save (`swap_matrix`, `src/instancemgr/gameplay_impl_common.inl`). The same convention applies to shrub and moby matrix instances. Its purpose is unknown; a renderer must not feed the raw w row into a homogeneous transform without fixing it.

---

## 6. Per-vertex colour and lighting: class versus instance

**In the class:** nothing Wrench can identify. The parsed vertex records carry no RGBA — only position, UV and Q. The evidence that per-vertex colour exists at all is indirect:

- `TiePacketHeader.rgba_count` (offset 0xa) — a per-packet count of RGBA entries (**inferred** from the name).
- The GS-packet arithmetic of 3 quadwords per vertex, which matches ST + **RGBAQ** + XYZ2 (**inferred**).
- `TieFatVertex.unknown_0/2/4` and the header's `vert_normals` pointer — candidate lighting inputs (**inferred**).
- In GC/UYA/DL the class header openly carries `ambient_rgbas`, `ambient_size`, `rgba_remap_ofs[3]`, `glow_rgba` and `glow_remap_ofs[3]`; RAC1 has none of these named, so any RAC1 equivalent is hiding in `unknown_1c/24/28` or the 0x44–0x6c tail (**unknown**).

**In the instance:** RAC1 stores an inline **0x80-byte ambient RGBA buffer** at offset 0x50 of every tie instance record (`RacTieInstance`, `src/instancemgr/gameplay_impl_classes.inl`). Wrench treats it as opaque bytes, copying it in and out and zero-filling when absent. From R&C2 onwards this buffer moves out of the fixed-size record into the separate variable-length "tie ambient rgbas" section (§7), where its length is expressed as a count of **2-byte units**, strongly suggesting one 16-bit colour per vertex (**inferred**; 0x80 bytes would then cover 64 vertices in RAC1, a fixed budget rather than a per-class count).

**Directional lighting:** the instance's `directional_lights` field is an index into the gameplay "lights" / directional-lights section (`src/editor/gui/inspector.cpp` exposes it as a foreign id referencing an `INST_DIRLIGHT`; RAC1 pointer 0x04, `RAC_GAMEPLAY_BLOCKS`). Each directional-light record is 0x40 bytes: two colour/direction `Vec4f` pairs (`DirectionalLightPacked`, `src/instancemgr/gameplay_impl_env.inl`).

Wrench's tie renderer in the editor does not use any of this: it draws the recovered mesh with plain textures.

---

## 7. Instance record and the tie-related gameplay sections

### 7.1 Section framing

The gameplay file begins with a table of s32 section pointers at fixed header offsets; each section's payload is aligned to 0x10 (`src/instancemgr/gameplay.cpp`). For RAC1: **0x30 = "tie classes"**, **0x34 = "tie instances"**.

The instance section itself is a `TableHeader` (`s32 count`, 12 bytes pad, total 0x10 — `src/instancemgr/gameplay_impl_common.inl`) followed by `count` records packed back to back. `InstanceBlock` assigns each instance an id equal to its **position in this array**, which is what tie groups and occlusion mappings index by.

### 7.2 RAC1 tie instance record (0xe0 bytes)

From `RacTieInstance` (`src/instancemgr/gameplay_impl_classes.inl`), with `static_assert(sizeof(RacTieInstance) == 0xe0)`:

| Offset | Size | Type | Field | Meaning |
| --- | --- | --- | --- | --- |
| 0x00 | 4 | s32 | `o_class` | Tie **class number** (not an index) — matches `TieClassEntry.o_class` in the level core |
| 0x04 | 4 | f32 | `draw_distance` | Draw/cull distance. Declared s32 in the packed struct but surfaced as an `f32` instance property (`Instance::draw_distance()` returns `f32`, `src/instancemgr/instance.h`); the shrub equivalent is declared `f32` in its packed struct. Treat the declared type in Wrench as unreliable here and the field as a distance threshold. |
| 0x08 | 4 | s32 | `pad_8` | always written as 0 |
| 0x0c | 4 | s32 | `occlusion_index` | This instance's occlusion id, matched against the tie block of the occlusion mappings table (§7.4) |
| 0x10 | 0x40 | Mat4 | `matrix` | Local-to-world transform: four columns of four `f32` (x,y,z,w), translation in the last column (0x40–0x4c). Element [3][3] at 0x4c holds 0.01f on 39,954 retail instances and 0.0 on 4,758 (§5); the w row of the other columns is 0. |
| 0x50 | 0x80 | u16[64] | `ambient_rgbas` | **Corrected:** one RGBA5551 ambient colour per light slot (`LightTies` expands them with PEXT5: 5-bit channels to bits 7..3, alpha bit to 0x80) (§6, §9b) |
| 0xd0 | 4 | s32 | `directional_lights` | Directional-light selector (retail values 0–8, 15, and one 0xff00) |
| 0xd4 | 4 | s32 | `uid` | Unique id for this instance, independent of its array position |
| 0xd8 | 4 | s32 | `pad_58` | always written as 0 |
| 0xdc | 4 | s32 | `pad_5c` | always written as 0 |

**Caveat on the offsets in the last four rows:** Wrench's inline comments for `directional_lights`, `uid` and the two pads read 0x50/0x54/0x58/0x5c, which is a copy-paste slip left over from the GC/UYA/DL layout — the struct is packed and the preceding `ambient_rgbas[0x80]` starts at 0x50, so the real offsets are 0xd0/0xd4/0xd8/0xdc. The `static_assert` on 0xe0 confirms this arithmetic.

### 7.3 Later-game comparison

`GcUyaDlTieInstance` is **0x60 bytes** and is byte-for-byte the RAC1 record with the 0x80-byte ambient buffer removed: `o_class` 0x00, `draw_distance` 0x04, pad 0x08, `occlusion_index` 0x0c, `matrix` 0x10–0x4f, `directional_lights` 0x50, `uid` 0x54, pads 0x58/0x5c. So the only structural change from RAC1 to R&C2+ is where the ambient RGBA data lives.

### 7.4 Tie-related gameplay sections

**"tie classes" (RAC1 pointer 0x30).** A plain list: `s32 count`, then `count` × `s32` class numbers. Wrench's reader for this block is a **no-op** — it regenerates the block on write as the set of distinct `o_class` values across the tie instances, in first-use order (`TieClassBlock`, `src/instancemgr/gameplay_impl_classes.inl`). So the section is redundant with the instances; it presumably tells the loader which tie classes to pull into memory for the level. (Note the `ClassBlock` used for mobies has the same shape, count followed by class numbers.)

**Instance-to-class mapping.** An instance names its class by **number**, and the number is resolved against `TieClassEntry.o_class` in the level-core index — not by position. Wrench builds an explicit `class number → mesh` map and looks each instance up in it (`src/wrenchvis/wrenchvis.cpp`, `src/editor/renderer.cpp`); a missing entry is fatal ("Cannot find tie model!"). The class blob itself is located via that entry's `offset_in_asset_wad`.

**"tie ambient rgbas" (R&C2+ only; GC/UYA pointer 0x94, DL art-instances pointer 0x20).** A sequence of variable-length records until a terminator, read as (`TieAmbientRgbaBlock`, `src/instancemgr/gameplay_impl_classes.inl`):

| Field | Type | Meaning |
| --- | --- | --- |
| index | s16 | Index of the tie instance in the tie instance array; **-1 terminates the section** |
| size | s16 | Payload length in **2-byte units** (byte length = size × 2) |
| data | u8[size × 2] | The instance's ambient RGBA buffer |

Instances with no ambient data are simply omitted. Records are emitted in ascending instance order. **RAC1 has no such section** — the equivalent data is the inline `ambient_rgbas[0x80]`.

**"tie groups" (R&C2+ only; GC/UYA pointer 0x38, DL art-instances 0x0c).** Not present in RAC1. For completeness, the format (`GroupBlock`/`GroupHeader`, same file) is: header `s32 group_count`, `s32 data_size`, 8 bytes pad; then `group_count` × `s32` pointers (byte offsets into the member data, or -1 for an empty group); pad to 0x10; then the member data as u16 instance indices, where bit 15 set marks the **last** member of a group and the low 15 bits are the instance index. The editor draws tie groups as boxes and lets a group reference tie instances by id (`src/editor/gui/view_3d.cpp`, `docs/instance_reference.md`: `TieGroup { tielinks members }`).

**"occlusion" (RAC1 pointer 0x8c).** Header of three counts (tfrag, tie, moby) plus pad = 0x10 bytes, then `total_count` × 8-byte mappings (`OcclusionMappingsBlock`, `src/instancemgr/gameplay_impl_misc.inl`). Each tie mapping pairs a `bit_index` into the 128-byte per-octant visibility mask with an `occlusion_id` copied from the instance's `occlusion_index` (`src/wrenchvis/wrenchvis.cpp`). Background in `docs/occlusion_culling.md`: the level is diced into 4×4×4 octants, each with a 128-byte PVS mask; ties participate, shrubs do not.

**Editor-only tie instance fields.** Wrench's own instance schema for a tie (`src/instancemgr/instance_schema.wtf`, `docs/instance_reference.md`) is: components `COM_TRANSFORM | COM_CLASS | COM_DRAW_DISTANCE`, transform mode `MATRIX`, plus `occlusion_index`, `directional_lights`, `uid`, `ambient_rgbas`, and a Wrench-invented `has_static_collision` flag that has no on-disc counterpart (it controls whether the class's collision gets merged into the baked level collision).

---

## 8. What Wrench knows about the tie VU1 program and GIF output

Wrench contains **no** tie VU1 microprogram, no disassembly, and no runtime emulation; there is no `tie_renderer.md` in `docs/` (unlike the tfrag, shrub and moby renderers), and `docs/renderers.md` says only that ties draw "large instanced geometry", that their LOD system resembles the tfrag one, and that the renderer was inherited from Naughty Dog. Everything Wrench "knows" is inferred from the data layout:

**Effectively known** (the parser would fail on real data otherwise):
- VU1 builds a GS packet whose addressing unit is the quadword, and every record that must land in that packet carries its destination quadword offset (`gs_packet_write_ofs`, `gs_packet_write_ofs_2`, `TieStrip.gif_tag_offset`, `ad_gif_dest_offsets`).
- Stride arithmetic in that packet: 6 quadwords per AD-GIF material block, 1 per strip GIF tag, 3 per vertex.
- A vertex can be emitted to two GS-packet locations, and small packets are padded to at least 4 vertices with duplicates.
- Source data is stored out of GS order (Wrench attributes this to double-buffering inside the VU program).
- Positions are s16 with a per-class scale; UVs are signed 12-bit-fraction fixed point.

**Inferred from field names, sizes and GS semantics:**
- Primitives are triangle strips (`GS_PRIMITIVE_TRIANGLE_STRIP`, `src/engine/gif.h`), each opened by one GIF tag.
- Each vertex writes three PACKED-mode registers, most plausibly ST, RGBAQ and XYZ(F)2 — hence the `q` field and the existence of `rgba_count`.
- Material blocks are GIF tags in A+D mode with nloop 5, setting TEX0_1, TEX1_1, MIPTBP1_1, CLAMP_1 and MIPTBP2_1.
- The PACK/UNPK annotations imply V4_8 unpacks for the header and strips and V4_16 unpacks for both vertex forms.

**Not modelled at all:** the VIF tag stream (if any) inside the blob, backface culling (`bfc_distance`), multipass rendering (`multipass_ofs`, `nultipass_type`, `multipass_uv_size`), scissoring (`scissor_ofs`, `scissor_size`), vertex lighting and normals, the `mode_bits`-style state flags (GC+ only), and how the LOD distances are compared against camera distance.

---

## 9. Wrench's export algorithm, for cross-checking an independent implementation

Read path (`read_tie_class` → `read_tie_packet`, `src/engine/tie.cpp`):

1. Read the header; if the game is RAC1, read `RacTieClassHeader` and copy `packets`, `packet_count`, `texture_count`, `near/mid/far_dist`, `ad_gif_ofs`, `scale`, `bsphere` into the common (GC/UYA/DL-shaped) header struct. Everything else in the RAC1 header is dropped.
2. Record `scale`.
3. For each of the 3 LODs: read `packet_count[i]` packet headers at `packets[i]`; for each, parse the packet data starting at `packets[i] + header.data`.
4. Per packet: read the two 4-entry AD-GIF offset arrays, the unpack header, the strip array; slice the vertex region with `vert_ofs`/`vert_size`; split it into `(dinky_vertices_size_plus_four - 4) / 2` dinky vertices followed by fat vertices to the end of the region; widen fat to dinky; duplicate vertices with a distinct second write offset; sort by write offset; drop consecutive duplicate offsets; then walk the GS-packet cursor (§3.3) to produce a list of primitives, each with a material index, a vertex list and a winding flag.
5. Read `texture_count` AD-GIF records from `ad_gif_ofs`.

Mesh build (`recover_tie_class`):

6. Create one COLLADA material and one `"<i>.png"` texture path per AD-GIF record, named by index.
7. Create a single mesh with texture coordinates, then iterate **LOD 0 only**; for each primitive emit a submesh bound to `primitive.material_index`, append its vertices with `pos = xyz * (scale / 1024)` and `uv = vu_fixed12_to_float(s, t)`, and triangulate with the strip-parity rule from §3.3. Vertices are duplicated per primitive (no sharing across submeshes).
8. The result is written as `mesh.dae` next to the class and registered as the class's `editor_mesh`; the raw blob is preserved as a `BinaryAsset` core (`src/wrenchbuild/classes/tie_class.cpp`).

Notable limits of the reference implementation, useful as a sanity list: no normals, no vertex colours, LODs 1–2 discarded, multipass/scissor discarded, and `write_tie_class` / `pack_tie_class` are **empty stubs** — Wrench cannot rebuild a tie blob from a mesh and repacks the original bytes instead (`src/engine/tie.cpp`, `src/wrenchbuild/classes/tie_class.cpp`, and `docs/asset_reference.md` on `TieClass.editor_mesh`: "a hack since we haven't got the tie exporter working"). Consequently the round-trip test `test_tie_class` cannot pass as written.

---

## 9b. Verified on the retail NTSC-U disc (2026-09-26)

The retired C++ reference (`src/core/tie.cpp`) implemented §2–§3.4 and §7.2 and its `rc_extract tie --level N` wrote `tie_dump.bin`; the Rust port matched it byte for byte on all 19 levels (below), and now matches the committed snapshot hashes of the same sections. Retail totals: **1,804 classes; 42,725 packets (LOD 0/1/2 = 25,385 / 12,086 / 5,254); 253,427 strips; 1,938,607 resolved vertices (598,972 fat); 1,662,131 triangles (LOD 0/1/2 = 1,062,775 / 424,509 / 174,847); 44,712 instances**, every instance class resolving against the core index. A top-down raster of LOD-0 instances transformed by the column-major instance matrix lands them on the terrain footprint, confirming the matrix convention and the `scale/1024` rule.

Corrections to the Wrench-derived sections above (all verified on every retail class/packet/instance, sources: `TieProc`, `LightTies`, VU1 program 13507):
* **Header is 0x80 bytes**, not 0x70: 0x44 = the class's own `o_class`, 0x50 = `TieLodInfo[3]` (strip-vertex, triangle and strip totals per LOD, all matching), then bbox min/max and corner points up to the first packet table (§2.1).
* **`vert_normals`** = 64 light-slot normals, `s16 (x, y, z, 0)` in 1/32767, 0x200 bytes ending at `ad_gif_ofs`. `LightTies` (level01 0x2ab218) reads them via the class pointer (+0x0c) for the 64 slots of each instance.
* **Packet header bytes 0x5–0xf** have real meanings (§2.4): 0x5 = 5·`shader_count`, 0x6 = 3 + strips, 0xa/0xb = colour-index region offset/count, 0xc/0xd = slot step table, 0xe = strips, 0xf = strip vertices. There is no "rgba_count" or "multipass" data.
* **Unpack header**: bytes 0, 1, 4–7, 10, 11 are the VU1 phase flags/markers and the dinky/fat counts (§3.2, §3.4).
* **Fat vertex `unknown_0/2/4`** are an s16 LOD-morph delta, not normals.
* **Per-vertex colour** is not stored as RGBA: each vertex has a light slot (0..63) through the colour-index region; fat vertices carry two more slots for the morph blend. The instance's `ambient_rgbas` are **64 RGBA5551 colours, one per slot**, and the class normals are per slot, so `LightTies` lights 64 slots per instance, not vertices.
* Instance `draw_distance` is an f32; `matrix[3][3]` is 0.01 or 0.
* Strip vertex counts are real (VU1 NLOOP), and every retail slot is written by exactly the vertices the phase rules say (Wrench's heuristics coincide on disc).

C++ fixes made while porting: the header grew to 0x80 with named fields; `TieFatVertex` fields renamed (delta, not normals); dinky/fat counts come from the unpack header (bytes 10/11) instead of being derived from byte 8 and the region size (same result on disc); the walker now follows §3.4 (strip `vertex_count`, phase markers) and keeps colour slots, morph deltas, both colour copies, the slot table, the normals and the header extension; instances keep the raw `matrix[3][3]` (the old loader overwrote it with 1.0).

**Rust loader.** `crates/rc-formats/src/tie.rs` (`parse_tie_class`, `parse_level_ties`, `tie_triangles`, `parse_tie_instances` from the decompressed gameplay file's pointer 0x34) mirrors the retired C++ `tie.{h,cpp}`. `tests/formats/golden.rs::ties_for_every_level` compares every section of every packet of every class, and the instance records, for levels 0–18 against the committed snapshot hashes (first against the retired C++ `tie_dump.bin`), and additionally checks the per-LOD totals, that Wrench's reconstruction gives the same strips, that colour copy B = A + 0x40, that every light slot is < 64 and that every triangle's ad-gif resolves to a tie texture. Not ported: `LightTies` itself (the inputs are exposed), the per-instance morph factor `k` and colour weights (computed per frame in `TieProc`; `k = 0` is the full-detail shape), the clipping program 224979.

## 10. Unknowns

Class header (RAC1): `unknown_48` (a distance), `unknown_2` of the unpack header, the exact `TieProc` formula for the morph factor and colour weights, and the slot step table's consumer (probably program 224979). The units and exact meaning of `bsphere.w` (assumed radius) and of `near/mid/far_dist` (assumed camera distances in world units) are unverified. (Per-LOD vertex/triangle/strip totals: resolved, `lod_info` at 0x50.)

Packet header: resolved (§2.4, §9b), except the exact encoding of the slot step table at `scissor_ofs`.

Unpack header: only byte 2 remains unknown; the `+4` / `+6` biases in bytes 8/9 are VU1 conversion-loop pipeline slack (program 13507 L5–L7).

Vertices: whether `q` is ever non-trivial; the fat form is the LOD-morphing vertex (resolved).

VIF/VU1: whether the class blob contains VIF tags at all, where the UNPACK/STCYCL/STROW state comes from, the VU1 program's transform and lighting maths, and the actual PRIM/GIF register values it emits (only the AD-GIF register *set* is known).

Instances: the meaning of the 0.01f (or 0) in matrix[3][3]; the encoding of `directional_lights` (LightTies' light selection lives in a run-time 0x20-byte record). Resolved: `draw_distance` is f32; `ambient_rgbas` are 64 RGBA5551 colours indexed by light slot; the pads are 0 on every retail instance.

Elsewhere: `TieClassEntry.unknown_8` and `unknown_c` in the core index; the tie `TextureEntry.type` field; and — since Wrench's reader for the "tie classes" gameplay block is a no-op — whether that list carries ordering significance beyond first-use order.

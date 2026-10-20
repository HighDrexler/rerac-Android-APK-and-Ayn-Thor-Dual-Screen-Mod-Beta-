# Cutscenes, level transitions and FMVs (RAC1 NTSC, level01.elf addresses unless marked)

Confidence: **[H]** read in the decompilation and checked against disc data, **[M]** read in code but not checked
on data or in a trace, **[L]** inference. gp = 0x166c00, so `uGpffff8184` = 0x15ed84 (current level), `89c0` =
0x15f5c0 (destination), `89c4` = 0x15f5c4 (game mode), `89d8` = 0x15f5d8 (skip one render), `87fc` = 0x15f3fc (fade
0..1), `8168` = 0x15ed68 (tick scale). `ticks(n)` = `fun_001f96f8` = round(n·0x15ed68), which is n on NTSC.
The mode table (0 gameplay, 1 PSS movie, 2 in-engine scene, 3 page menu, 6 ship/space) is in menus.md §1. Ship
take-off, planet page and fly-away are in menus.md §5. Save/visited flags are in game_state.md.

## 1. Where the data lives [H]

The TOC is copied to **0x137b80** (`load_disc_sectors_into_global_buffer` boot 0x12f2b8, sector 1500, 0x2960 bytes).
The current level's amalgamated header follows it at **0x13a4e0**: data 0x13a4e8, gameplay NTSC 0x13a4f0 and PAL
0x13a4f8, music[15] 0x13a628, scenes 0x13a664. Every TOC table below is `0x137b80 + field offset`.

| Data | Where | Used by |
|---|---|---|
| Level scenes (in-engine cutscenes) | level header +0x184: **15 × 0x250**, see §3.1 | `0x2ac330(id)`, mode 2 |
| Space scenes (take-off/landing) | TOC 0x12e8 `anim_looking_thing_2`[20] = 10 NTSC + 10 PAL | `0x2a24b8(sub)`, mode 6 |
| "Item" scenes (3 chunks, Ratchet + one item class) | TOC 0x1530 `things`[28] = 14 NTSC + 14 PAL | `FUN_002594e0` [M, caller not traced] |
| Space-flight mini-level (loading flight) | TOC 0x13b8 `transition` (1 WAD) | `FUN_002a5868`, mode 6 sub 4 |
| Story title cards | TOC 0x1388 `space_plates`[6] (per language) | `fun_00231878`/`fun_00231bd8` |
| Ship class assets | TOC 0x12c8 `spaceships`[1 + ship] | level loader state 1 |
| Scene/space speech | level `scenes[id].sounds[lang]` (id < 10000), `space_audio` (40000+k) | dialogue player 0x151720 |
| FMVs | TOC 0x17f8 `mpegs`[88] (SectorByteRange) | §5 |

**Correction to the extractor's view** (fixed in the port 2026-09-27: `extracted/levels/NN/scene/KK_ntsc|pal.bin`,
`speech/KK_<lang>.bin`, docs/formats/disc_layout.md §2.4). Wrench (and formerly `disc.rs` → `sceneMM/`)
reads the scene block as 30 × 0x128 {sounds[6], wads[68]}. The game indexes it as **15 records of 0x250 bytes**
(`fun_00215970`: `0x13a664 + id·0x250 + lang·4`; `FUN_002591d0`: `0x13a67c` NTSC or `0x13a798` PAL
`+ id·0x250 + chunk·4`):

| Off | Type | Field |
|---|---|---|
| 0x000 | u32 lsn[6] | speech VAG by language 0x15ed88 (0 En, 1 empty, 2 Fr, 3 De, 4 Es, 5 It) |
| 0x018 | u32 lsn[71] | NTSC chunk WADs |
| 0x134 | u32 lsn[71] | PAL chunk WADs |

A chunk's size is `lsn[n+1] − lsn[n]` sectors. The last entry is a 1-sector sentinel. So the even directory
`scene2k` is scene k NTSC. Odd `scene2k+1` is scene k PAL, where `sound/003..005` are really PAL chunks 0..2 and
`wad/000..` are chunks 3... Novalis has scenes 0–5 with both halves, plus scene 6 (NTSC only, no speech, a copy of
scene 0).

## 2. The chunk format (scene chunks and space-scene chunks) [H]

Each chunk is a WAD-compressed blob. It is decompressed into 0x16cd38 by `FUN_00259288` (the parser) via
`fun_0020b618`. NTSC chunks hold 96 ticks (0x60), PAL 80 (0x50). Offsets are from the chunk start.

| Off | Type | Meaning |
|---|---|---|
| 0x00 | s16 | scene **end tick** (the same in every chunk; e.g. 1508) → 0x16cd20 |
| 0x02 | s16 | 0 |
| 0x04 | s32 | subtitle table offset; values < 0x400 mean none → 0x16cd2c |
| 0x08 | s16 | audio start tick (−6 on Novalis) → 0x16cd28; `continue_audio_stream_if_ready` runs once tick ≥ it |
| 0x0a | s16 | −1 (unknown) |
| 0x0c | u16 | actor count → 0x16cd24 |
| 0x10 | s32 | camera table offset → 0x16cd34 |
| 0x14 | s32[count] | actor record offsets |

**Camera table**: one 0x20-byte record per tick. There are ticks-per-chunk + 1 records (97 NTSC, 81 PAL). The last
record equals the first record of the next chunk. The final chunk has end − 96·(n−1) + 1 records.

| Off | Type | Meaning | Written to |
|---|---|---|---|
| 0x00 | f32 ×3 | eye position | 0x167240..48 |
| 0x0c | u8 (+pad) | **cut flag** (the whole word is copied into 0x16724c) | returned |
| 0x10 / 0x14 / 0x18 | f32 | angles (rad): identity → `sceVu0RotMatrixX` 0x125360 (+0x10) → `…Y` 0x125408 (+0x14) → `…Z` 0x1252b8 (+0x18) [H, disassembled] | basis |
| 0x1c | f32 | t = tan(hfov/2) (Novalis: 0.554 or 0.414; gameplay 0.63) | 0x16cf70 |

**Angle order [H].** Each SDK routine computes `q_i ← R·q_i` for the four stored rows q_i (one `vmulax/vmadday/vmaddaz/vmaddw`
chain per row, R's columns in vf6..vf9: X = (1,0,0),(0,c,s),(0,−s,c); Y = (c,0,−s),(0,1,0),(s,0,c); Z = (c,s,0),(−s,c,0),
(0,0,1)), so from the identity the stored rows end up as the **columns of Rz(+0x18)·Ry(+0x14)·Rx(+0x10)** (X applied
first, standard right-handed rotations). Sine/cosine come from `sceVu0ECosSin` 0x125240: cos θ = a 9th-order sine
polynomial (coefficients boot 0x132e00: 2.6019e−6, −1.98074e−4, 8.33303e−3, −0.1666666) of π/2 − |θ|, sin θ =
±√(1 − cos²) with the sign of θ. Checked on data: scene 5's first record gives forward = (−0.94, 0.003, 0.33), i.e. from
(162.6, 129.8, 82.7) towards the incoming ship.
The camera basis is 0x167450 = −row2, 0x167460 = −row0, 0x167470 = row1 of that matrix (forward, left, up). `UpdateViewContext` runs
first. The mirror cheat 0x15edb4 recomputes 0x167460 as a cross product.

**Actor record** (chunk-relative; actor k persists across chunks in slot `0x16ce58[k]`):

| Off | Type | Meaning |
|---|---|---|
| 0x00 | s32 | moby class (in mode 6, actor 0 with class 533 is replaced by `0x160548[ship]`) |
| 0x04 | s32 | 0x19 or 0x10 [unknown] |
| 0x08 | s32 | 0 |
| 0x0c | s32 | **position track** offset: one vec4 f32 (w = 0) per animation frame |
| 0x10 | 0x1c | a `MobySequenceHeader` (moby_rac1.md §4.2): bsphere, u8 frames (49 NTSC / 41 PAL; the last chunk of a scene has ticks/2 + 2, e.g. 36 for 68 ticks), loop sound 0, **trigger count 0xff** (no trigger words follow: parse as 0), 0xff, 0, 0 |
| 0x2c | u32[frames] | frame offsets **relative to +0x10** (the parser relocates them in place), then the frames |

There is no per-actor rotation track. Orientation comes from the skeletal frames (the root joint). The streamed
sequence is a *new* slot on the class: on spawn, slot = class+0xc (the sequence count), then class+0xc++;
`class+0x48+slot·4` points at +0x10. Each new chunk re-points the slot.

**Subtitle table**: 16-byte entries `{s16 start, s16 end (scene ticks, inclusive), s16 text[5] (En, Fr, De, Es, It;
offsets from the table start), s16 0}`, ended by start = −1. The strings (Latin-1, NUL-terminated) follow the table.
Each chunk repeats the entries that overlap it. The tables hold every line **twice** with separate strings (scene 5:
"Ooomph!" then "Ooopmh!"); the draw takes the first match, so the second set is never shown [H for the data, L for why].

## 3. Playback engine (mode 2) [H unless marked]

**Start `0x2ac330(id)`** (id ≥ 0; if 0x1415f8 == 0 it takes the `FUN_002319b0` path instead [L: debug/host]):
1. `music_Pause(0)`, wait for the stream to go idle, 0x15f5d8 = 1. Clear 0x16cce0[0x1c0] and 0x17c7c0..0x17c900.
2. Buffers: base = 0x1611cc − 0x3c000; read buffer 0x16cd3c, decompress buffer 0x16cd38. Set 0x16cd10 = id,
   tick 0x16cd14 = 0, chunk 0x16cd1c = 0, 0x15f59c = 1.0.
3. `FUN_002258b0`. Fade 0x15f3fc = **1.0**. Mode 0x15f5c4 = **2**. Hero `SetState(100, 2)` (0x23cf98), 0x1413f5 = 1,
   `FUN_002486c0`. The actor loop's `FUN_0024a0e8` re-shows the always-on extras (item slots 4 / 5) on the scene
   actor each tick. The dialog NPC 0x179588 and 0x1ba25c get mode |= 1 (hidden).
4. Read chunk 0 (`FUN_002591d0(0)`: sync CD read). `FadeToBlack(ticks(6))` (blocking, draws only black quads).
   Parse chunk 0: spawn actors with `CreateMoby(class)`: +0x32 = 0x1ff, +0x72 = 0xff, mode |= 6, +0x94 = 0,
   +0x38 = Ratchet's +0x38 (or 0x38383800000000), +0x73 = 0x18 if class+6 ≠ 0.
5. Mobys of runtime classes 74 and 203 get mode |= 0x80 (cleared at the end) [M: classes unidentified].
6. Speech: 0x1516ec = id, so the dialogue player streams `sounds[language]` (group 2). Wait for state 0x15172a = 3
   (buffered). Then 0x16cd28 = −3, start the stream, prefetch chunk 1, and wait 3 vsyncs. So the audio leads the
   picture by about 3 ticks [M].

**Tick `0x2aca80`** (world updates gated by debug mask 0x16c4e0; `sound_update`, lights):
1. Fade: 0x15f3fc −= **0.34** per tick (black → clear in 3 ticks). Scene tick 0x16cd14++ and chunk tick 0x16cd18++.
2. **End** when tick ≥ header end. **Skip** only when tick ≥ ticks(18) and the fade is 0:
   * Start (pressed 0x13cae4 & 0x800) skips if the game is beaten (0x15eea0), or completed (0x15ee20), or replaying
     from the menu (0x15eed8), or on level 0.
   * Otherwise it needs L1+L2+R1+R2 held (0x13cae0 & 0xf) **plus** Start.
3. Chunk roll: when 0x16cd18 ≥ 96/80, wait for the read, chunk++, parse (0x16cd18 = 0), prefetch chunk+1.
4. Camera `FUN_002ac8d8` from record 0x16cd18 (60 Hz, no interpolation).
5. Actors: frame f = tick>>1, blend t = (tick&1)·0.5. On an odd tick with the cut flag set, t = 1.0 (snap across
   the cut).
   * Moby +0x50 = f, +0x51 = f+1, +0x54 = t, then `update_moby_animation_state`. So the animation runs at 30 Hz,
     interpolated to 60.
   * pos(+0x10) = lerp(track[f], track[f+1], t). Then +0x71 = 0xff and `fun_0020def8`.
   * Class 0 → `FUN_0024a1d0`. Classes 10/419/1365 → `FUN_00228bc0` [L: Clank/attachment]. +0x7f ≠ 0 →
     `FUN_0026f0e0`.
6. The effects are not in the chunk. Class 1546 (`0x30c190`, "cutscene FX driver") reads 0x16cd10/0x16cd14 and spawns
   particles on actor joints (e.g. scene 5, ticks < 120: a trail from joint 5 of actor 2).

**Render**: the mode-2 render (0x21aa90) sets the draw mask 0x15f3f4 = 0x7f, which clears HUD bit 0x80, so there is
**no HUD**. `DrawWorld` then draws subtitles (`fun_001f4be0` 0x21b620, only when option 0x15ee40 is on):
* The first entry with start ≤ tick ≤ end is drawn.
* `DrawUIFrame` box alpha 0x60 at y = screen_h − 0x3c, width from the text, then `font_print_window_regular`.
* Language index = 0x15ed88 − 1 for 2..5, else 0.

Then the fade quad (black, alpha 0x15f3fc·128) and the white quad (0x15f400·128). **No letterbox bars** exist in this
path [M].

**End `0x2ac608`**:
* Speech stop (0x15172a → 5). `FadeToBlack(12)`. Mode **0**. FOV 0.63 (0x3f2147ae). Fade 0.
* Actors are deleted and their class sequence slots freed.
* Hero `SetState(0,1)` and a ground snap (ray 0.5 up, accepted if |Δz| < 4.5). 0x1413f5 = 0, `FUN_002487a8`.
* Optional hero teleport to 0x16ccf0/0x16cd00 if 0x16cd26 ≠ 0. The only writer is `FUN_002783a8(dist, npc)`, called by
  `TalkingNpcUpdate` 0x2ff118 (state 1, dist 2.2) after the dialog record started its scene: position = NPC + dist·(cos,
  sin)(NPC yaw), yaw = NPC yaw + π (facing the NPC). The start clears 0x16cce0..0x16cea0, so scenes started by anything
  else (scene 5 included) do not teleport [H].
* The NPC dialog continues (`FUN_0027b550(npc, rec, 1)`). Music resumes via `FUN_0027a460(ticks(30))`.

**Who starts scenes**: NPC dialog records (`FUN_0027b028`/`0x27b550`, record +4 = scene id, bit 0x4000 = FMV instead).
Infobots (menus.md §6). The ship (scenes 4/5/6, movie 0xb, level 10 scene 9). An unidentified class (`0x30a6d0`, pvar[0] scene on touch).
Mission NPC 730/790 (`0x2fad68`). The pause-menu replay (post-actions 3/4/5/6).

## 4. Transition sequence

### 4.1 Mode 6 space scenes (`FUN_002a4080`) [H]
They use the same chunk format. The chunk list comes from a pre-read lump: a 0x800-byte table of `{s32 off, s32
size}` (data at +0x800+off, ends at size 0, up to 70), loaded by `FUN_00259628(idx, fade)` into 0x16cd40[].
* Camera and actors are **ship-local**: world = ship.rot(+0xc0)·p + ship.pos, and the camera Z angle += ship yaw.
* Fade −0.125 per tick. ✕/△ (0x13cae4 & 0x50) after 30 ticks skips to the per-ship skip tick (take-off) or to
  end − 30 (landing).
* Take-off = sub 0, lump `ship+1` (598 ticks; actors 0, 10, ship, 535–537). Landing = sub 8, lump `ship+5` (738
  ticks; actors 0, 10, ship).
* Specials: lumps 0/4 on level 10 without 0x13d4c6, lumps 8/9 on level 14 with ship 2.
* Stream id 40000 + (ship+6) for take-off; ship+3 (ship when entered from level start) for landing.
* At the landing end: mode 0, music unpaused, 0x15f5d8 = 1, hero teleported to 0x13e090/0x13e0a0 (`0x2368e0`).

### 4.2 Normal flight A → B (revisit) [H for order, M for ticks]
1. △ at the ship → take-off scene (sub 0, about 10 s) → planet page (mode 3, 0x1ba170 = 0xe) → `FUN_002a2848(dest)`.
2. Fly-away (sub 3): memcard save, ship on spline 0x1b0930 for 120–150 ticks, then accelerates. Fade to black +0.0625/tick at the end.
   Then 0x15f570 = 1 → `entry` leaves its loop → **`DoSpaceTransition` 0x2a68f8**:
   * Bit 0x80000000 set in 0x1742d0; mode 6.
   * Ship index 0x13e056 = 0, then 1 if planet 8 is unlocked or dest > 7, then 2 if planet 14 or dest > 13.
   * Stop sounds, music and streams. Unload the level sound bank (0x15f5f4+0x1c).
   * Clear the fog/colour block 0x16d0d8..f8. Background black.
3. First-visit story cards/FMVs where they apply (§4.3). Then visited[old] = 2 (not for 7/14 while their exit flag is
   unset), 0x15ee4a = 1, **0x15ed84 = dest**, 0x15ee48 = 0.
4. `FUN_002a5868`: read and decompress the `transition` lump. It holds tfrag/tie/moby classes, particle textures,
   the planet image `[level]` and the area caption `[lang·19 + level]`.
   * Scene variant v = rand & 3; v = 4 when dest == 0, or dest == 1 while planet 3 is locked.
   * v selects the chunk table header[0x14+v]. The sound bank is header[0x19].
   * Sub 4 is set.
5. `read_file_entry_with_retry(level)` reads the level header. Then the frame loop:
   `VU1 send/swap → FUN_002a4080 (sub 4 = FUN_002a33b0 flight) → dispatch_game_state_update (space sky 0x2a3b90 →
   0x29f260, shell translation 0x160520 = (tick − 120)·20·k·dir)`. The async loader `FUN_00257dc8` is polled every frame.
6. Loader states (0x15ee48): 0 data WAD → 0x15ee4c (layout from 0x1ff8000 down). 1 `spaceships[ship+1]`. 2 gameplay
   NTSC/PAL. 3 wait for the flight to clear 0x15ee4a, then stop sounds. 4/5 unload/resolve banks. 6 load the level
   `sound_bank` from the data container. 7 done. There are no separate core/texture reads: everything is in the data
   WAD. The occlusion, music and scenes lumps are streamed later.
7. Loop until the flight sets 0x15f5d8. Then wait for the loader, then for the memory card. Return to boot, which
   runs the new overlay `entry`.
8. `entry` 0x259c40:
   * Mode 6, `FUN_00258128(1,0)` level init (unpack, `FUN_00255958`…), then `music_start_track(0)` (paused unless
     first Novalis).
   * visited[dest] = 1, playtime records. Then **`FUN_002a29c0`**: landing scene (sub 8), except when dest == 0, or
     dest == 1 with planet 3 locked, or dest == 14 with 0x13d3f0 == 0 → mode 0 at once.

**There is no "Now loading" screen.** Loading hides behind the flight (sub 4) or behind a story card (§4.3), which
stays up until the load finishes [H].

### 4.3 First arrival at Novalis (new game, Veldin → Novalis) [H for order, M for durations]
`DoSpaceTransition` branch `0x15ed84 == 0 && dest == 1 && visited[1] == 0`. Cards are
`fun_00231bd8(lang−1, a, b, ticks, load)`. Card index c = `space_plates[lang]` entry c+1: a 512×64 PSMT8 image drawn
over a band with a 64×64 scrolling texture (u += 1/600 per tick).
* Alpha rises 4/tick over 32 ticks. It falls 8/tick over the last 16.
* A second line is drawn at y = cy after 64 ticks; the first then sits at cy − 0x2e.
* After each card: `FadeToBlack(2)`.

| # | Step | Duration |
|---|---|---|
| 1 | `FadeToBlack` | 6 ticks |
| 2 | Cards 5+6 "Chairman Drek's flagship" / "Veldin Orbit" | 240 |
| 3 | FMV transition #3 = `mpegs[43]` (NTSC), then `FadeToBlack(4)` | movie |
| 4 | FMV #4 = `mpegs[44]` | movie |
| 5 | Card 7 "Back on Chairman Drek's flagship" | 180 |
| 6 | FMV #5 = `mpegs[45]` | movie |
| 7 | 0x15ed84 = 1, visited[1] = 2; header read; card 8 "Entering atmosphere of Planet Novalis" (load = 1: the loader is polled each frame; the end moves to ≥ now + 20 until the load is done) | ≥ 240 |
| 8 | Overlay `entry`: level init; music track 0 **unpaused**; `FUN_002a29c0` → **mode 0** (planet 3 locked, so no ship landing) | 1 tick |
| 9 | Tick 1: 730/790 (`0x2fad68`, update distance 128, hidden) state 0 → 1 | 1 |
| 10 | Tick 2: state 1, mission byte ≠ done, mode 0, global flag 0x13d397 == 0 → **`0x2ac330(5)`**, flag := 1 | – |
| 11 | Scene 5 (§4.4): fade 6, 1508 ticks, then fade 12 → mode 0, hero idle at the level spawn: **control** | 25.1 s |

Steps 9–10, pinned: state 0 of 730/790 runs in the load pass (`FUN_002792d0` runs every moby's update once with no
distance gate; moby_update_catalogue.md), and it sets update distance 0xff, so the **first gameplay tick** runs state 1 and
calls `DialogStreamStart(5)` from inside the moby loop (the mission byte `levels[1].missions[moby+0xb0]` is not 0xff on
a new game) [H from the code; a PCSX2 trace would confirm].

### 4.4 Novalis scene 5 (the arrival) [H]
* Duration: `speech5` VAG 24.55 s at 44,056 Hz, 5 languages. 1508 ticks = 15 × 96 + 68, 16 NTSC chunks.
* Actors [0 Ratchet, 10 Clank, 530 (scene ship), 1365]; all four classes are in Novalis's core (530 too). The ship
  flies in from (40.59, 129.49, 126.10) (frame 0) to (145.24, 129.65, 87.00) (frame 48) during chunk 0, crashes by tick
  106 at (159.3, 129.7, 81.8) and ends below the pad at (176.9, 129.7, 50.6); Ratchet falls to (171, 130, 68) by tick 180. t = 0.414 throughout. The first camera is at (162.6, 129.8, 82.7).
* Cuts at ticks 107, 181, 261, 365, 761, 921, 1095, 1265.
* 8 subtitle lines: "Ooomph!" at 259, "Clank? Where are you?" at 395, ..., "If there are any left." at 1341–1454.
* FX: 1546 instance 976 at the pad draws the ship's trail for ticks < 120.

Other Novalis scenes: 0 plumber (774, 2318 ticks) and 1 his exit (838); 2 Skid (918); 3 infobot hand-over (811/731,
1078); 4 Qwark ad reaction plus ship 531 (1536); 6 = scene 0 without audio.

## 5. PSS movies (mode 1 / blocking) [H for tables, M for decode]

`mpegs[i]` = {lsn, bytes}. Base 0x139378 = TOC + 0x17f8.

| NTSC | PAL | Caller | Use |
|---|---|---|---|
| 2..20 (2 empty) | 21..39 | `0x2acf50(n)` (mode 1, `0x2ad0c0`) | in-level story: infobot pvar byte 0xd, NPC record id\|0x4000, ship 0xb, menu post-action 3 |
| 40..50 | 52..62 | `fun_00231608(n)` blocking (`fun_0023a3b8`); menu replay `0x2ad050` | transitions: 0–2 new game, 3–5 Veldin→Novalis, 6 Eudora, 7 leaving 7, 8 to 13, 9 leaving 14, 10 to 16 |
| 70..74 | 75..79 | `0x2acfe8(n)` (menu post-action 6) | extras (73/78 are 353/299 MB) [L] |
| 80..83 | 84..87 | boot `0x1e9488(i)` | title attract loop after 1500 idle ticks, i cycles 0–3 from a random start |
| 0, 1, 64–69 | – | not found | [L] boot logos? |

Format:
* MPEG-2 program stream (PSS).
* Video: MPEG-2, 512×416 on NTSC, frame-rate code 5 (30 fps), decoded by the IPU through sceMpeg (`readMpeg`
  0x31a308, `sceMpegDemuxPssRing`).
* Audio: private stream 1, sub-id 0xFF, with an `SShd` header: type 0x10 (SPU ADPCM), 48000 Hz (44100 in `mpegs[0]`),
  2 channels, interleave 0x20. It is fed to the IOP through the 989snd movie-sound calls (`snd_init/start/
  update_movie_adpcm`).
Mode 1 saves VRAM first and restores it after (menus.md §1). The decoder is out of scope.

## 6. Level 00 opening (new game) [H for order]

New Game resets to level 0 (game_state.md §4). `DoSpaceTransition` then takes branch `dest == 0 && visited[0] == 0`:
* Fade 6.
* Cards 0+1 "Kyzil Plateau, Planet Veldin" / "(11:13am local time)", 240 ticks → **FMV `mpegs[40]`**.
* Card 2 "Meanwhile, in a factory on a nearby planet...", 180 → **`mpegs[41]`**.
* Level := 0. Card 3+4 "Approaching Planet Veldin..." / "(11:47am local time)" with loading → **`mpegs[42]`** plays
  after the load.
* Veldin `entry` → `FUN_002a29c0` (dest 0 → mode 0, no landing).
So the opening is **three FMVs** with no in-engine scene first.

**No ship on level 00 is correct.** Level 0's settings ship x ≤ 0, so `FUN_00241940` (L00) never creates it.
Leaving Veldin still goes through the planet page → `FUN_0028ed58` (L00 copy of `0x2a2848`), reached only from the
page-menu post-action (`FUN_00276bd0`). What opens that page on Veldin is **not traced** [L].

## 7. Port plan

**Formats (`rc-formats::scene`)**
1. `SceneTable::parse(level_header)` → 15 × {sounds[6], ntsc[71], pal[71]} with sector-difference sizes. Fix
   `disc.rs` `level_stream_lumps` naming to `sceneK/{sound/L, ntsc/N, pal/N}`. This renames extractor output, so
   update the extractor and its verify step together (then `tools/extract`, now `rerac-extract` and its SHA-1 table).
2. `SceneChunk::parse(decompressed)` → header, camera records, actors (class, sequence as a `MobySequence` with
   sequence-relative frame offsets, position track), subtitles.
3. `SpaceSceneLump` (0x800 table) for `anim_looking_thing_2`, `things` and `transition`[0x14+v].
4. `SpacePlates` (PIF8 list: bg + 17 cards).
5. Goldens, all 19 levels, NTSC and PAL:
   * frames = (entries − 1)/2 + 1; 97/81 camera records; last record = the next chunk's first.
   * Σ chunk ticks = end; a subtitle table only when the offset is ≥ 0x400; every class id exists in the level (or
     the ship ids).
   * Pinned values for Novalis: scene 5 = 1508 ticks, cuts {107, 181, 261, 365, 761, 921, 1095, 1265}, actors
     [0, 10, 530, 1365], first subtitle (259, 336, "Ooomph!").

**Game (`rc-game::cutscene`)**
* `ScenePlayer` state machine: Start (§3 steps) → Playing(tick, chunk) → End, plus the skip rule.
* It drives the existing APIs:
  * camera: position + basis + t → `GameProjection`;
  * actors: spawn, register a streamed sequence slot, set frame/next/blend → moby_animation's evaluator, position
    lerp;
  * audio: dialogue VAG stream with a 3-tick lead;
  * hero `SetState(100)` then idle, with the ground snap.
* Triggers first: 730/790 state 1 → scene 5 on the Novalis first arrival, then NPC dialog records and the infobot.
* `SpaceScene` (mode 6) = the same player plus the ship-local transform. Then `Transition` = the ordered list of §4.2/4.3
  as data-driven steps (card, fmv, fade, load, flight).

**Engine**
* Full-screen fade quad (black ×0x15f3fc·128, white ×0x15f400·128) at the end of the 2D pass.
* A blocking fade step (N frames of black ramps with the world frozen).
* Subtitle box (`DrawUIFrame` alpha 0x60 + text) and title cards (2 × 512×64 quads over a scrolling band).
* No letterbox needed. HUD hidden in mode 2.

**Movies**: defer the decoder. The Novalis path needs `mpegs[43..45]`, so play them as external files or skip them with
a card for now.

## In the port (2026-09-27)

**Files.** `crates/rc-formats/src/scene.rs` (`SceneTable`, `parse_scene_chunk`, `Scene::load`), `toc.rs` (`SceneRecord`
15 × 0x250, lump names), `disc.rs` (`scene_region`, `scene_speech`); at the time also the C++ `toc` reader and `tools/extract` (`unpack` names,
a `scene` dump command; retired 2026-09-27); `crates/rc-game/src/scene_player.rs`,
`crates/rc-game/src/audio/scene.rs`; `crates/rc-engine/src/scene_render.rs`, `assets/shaders/fade.wgsl`, the
`SceneLayer` hook in `hud_render.rs`.

**Golden** (`tests/formats/golden.rs` `scenes_for_every_level`): the Rust path re-serialised in the C++ dump layout was
byte-identical to the C++ dump for all 19 levels, NTSC and PAL, and is now checked against the committed snapshot hashes: 138 scenes, 275 regions, 4,081 chunks, 348,572 ticks, 1,004 actors,
4,284 subtitle entries, 1,858 cut ticks. Invariants checked on every chunk: camera table ends at the first actor record,
last record = the next chunk's first, enough frames for the last shown tick, stable actor classes, track w = 0. Scene
actor classes outside the level core and gadget table exist on 12 levels (e.g. level 0 {1375, 1472, 1600}); Novalis
has none. Mutations (camera angle, class, subtitle byte) are caught.

**Player** (`ScenePlayer::tick`, one call per 60 Hz frame): 6 `FadeToBlack` frames (alphas 22, 43, 64, 86, 107, 128,
accumulated over the held image), 5 black frames (the fade's closing vsync, the stream start + 3-vsync lead, the
triggering frame's skipped render), scene ticks 1..end−1, the end-tick frame (last image held, speech stop),
12 `FadeToBlack(12)` frames, 1 closing black frame with the `SceneEnd` (hero `SetState(0,1)` + ground snap, FOV 0.63,
music resume after 30 ticks); the frame accounting of the blocking loops is [M]. Camera rows on the PS2 float model.
Scene 5: 1532 frames from the trigger; in the engine the trigger is app frame 3 (gameplay tick 1 runs on frame 2), scene
tick T is drawn on app frame T + 13, the cut at 181 lands between frames 193 and 194, control returns on frame 1535.

**Engine.** The gameplay tick is suspended while a scene runs (`GameTick.run_if`), the hero and his items hidden (the
always-on extras excepted: they ride the scene actor, `FUN_0024a0e8`), the HUD hidden (`SceneLayer`), `MenuMode` =
`Mode::Cutscene`; actors are extra mobys with the streamed sequence in an extra class slot (slot = class sequence
count), Ratchet's light word/ambient; camera applied after `play_camera::apply`;
fade = a GS-blend full-screen pass after the UI pass and the underwater tint; subtitle box per `fun_001f4be0`
(window 200/0x208/0x28/0x1d8, anchor 0x100, y h−0x38, line 0x12, flags 7; box y = h−0x3c, alpha 0x60, text
0x80b0b0b0). Speech (`speech/05_en.bin`, 44,056 Hz) plays on a group-2 stream voice; the music voices are frozen at the
start and restored 30 ticks after the end; `sound_update` uses the cutscene master volumes meanwhile. The new-game
template has option 0x15ee40 (subtitles) = 0; the port shows them unless `RC_SUBTITLES=0`. `RC_SCENE=0` disables
scenes, `RC_SCENE=k` forces scene k after gameplay tick 1. Two runs are frame-identical.

**Not yet.** FX driver 1546 (the ship trail of scene 5), the world freeze during the blocking fades (mobys keep their
generic advance), particles during scenes (stepped from the suspended tick), the other triggers (NPC dialog records,
infobots, the ship, 0x30a6d0, pause-menu replay), mode-6 space scenes, the "item" scenes, title cards, FMVs, the
mirror/FOV cheats, classes 74/203 mode 0x80.

## 8. Unknowns
* Actor record +0x04 (0x19/0x10) and header +0x0a. Why every subtitle table holds two sets. (The rotation axes and the
  teleport writer are pinned above.)
* Classes 74/203. The meaning of class 1365 in scene 5
  (positions are near (20..29, 16..24, 7..11)).
* The load time of card 8 and a trace confirming the first-tick trigger (§4.3) and the blocking-loop frame counts.
* What opens the planet page on Veldin; mpegs 0, 1, 64–69; the caller of `things` (`FUN_002594e0`); `FUN_002319b0`
  (the 0x1415f8 == 0 path).

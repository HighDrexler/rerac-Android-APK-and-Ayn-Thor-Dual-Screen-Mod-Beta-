# R&C2 (Going Commando) bring-up

Status: **started 2026-10-16; the front end runs (milestones 1–3).** Goal for 0.3.5: Going Commando can be selected, starts, and shows its own main menu.
New Game says "not yet". Nothing of R&C1 is rewritten for it.

## 1. Approach

- **One engine, two games.** The engine (`rc-engine`), the renderer, the format readers (`rc-formats`) and the data
  layer (`rc-data`) are shared. Where a format differs between the games, the reader takes the game as a parameter;
  it is not copied.
- **R&C2's game logic lives in its own crate, `rc-game2`,** the way R&C1's lives in `rc-game`. Systems that turn out
  to be the same code in both games (by `tools/ghidra/scripts/crossgame_diff.py`) move to shared modules when a
  second consumer exists, not before (no speculative ports).
- **Evidence first.** Every R&C2 behaviour comes from the R&C2 binaries in the `ratchet2` Ghidra project
  (`docs/workflows/ghidra.md`, R&C2 section), never from R&C1 by analogy.

## 2. Milestones

1. **Extractor (done).** Build `SCUS_972.68` v2.00 NTSC-U is supported. R&C2 keeps its assets in the ISO filesystem,
   so its Tier 0 archive is the filesystem's files under their disc paths (`SYSTEM.CNF`, `SCUS_972.68`,
   `IOPRP255.IMG`, `RC2.HDR`, `G/*.WAD`; `G/DUMMY.` is padding and skipped): 96 files, 3,807,638,923 bytes, table
   `crates/rc-extract/data/scus_972_68.tsv`. `extract-info.json` gains `"game"`. There is no Tier 1 cache for R&C2
   yet. The development data folder is `<repo>/extracted-rac2`.
2. **Front-end map (Ghidra).** The boot flow from the boot ELF's entry (0x131ae8) to the main menu: logos and
   movies, the title, the main menu's pages, the game-mode values, and the assets each needs (fonts, menu textures,
   text, background, music).
3. **Engine boot path (done).** A data folder holding `RC2.HDR` starts Going Commando (`rc-engine` `disc_source::game`,
   `rac2.rs`); the front end's logic is `rc-game2` (`front_end.rs`, `gui.rs`), its data readers `rc-formats` `rac2/`.
   The splashes, the Insomniac movie, the title (couch movies, "Press START") and the main menu run and draw through
   one `hud_render::Layer`. Not yet: any sound (movie audio, menu sounds, music), what the rows open (New Game, Load,
   Options log "not in this version yet"), the subtitles over movies, PAL / other languages.
   Run: `RC_DATA_DIR=<repo>/extracted-rac2 cargo dev` (front end, then New Game → Aranos);
   `RC_LEVEL=n` starts in level n without the front end (`RC_FRONTEND=1` keeps it).
4. **Ship.** The launcher lists Going Commando, installs it to `games/rac2`, and starts it on the main menu.

## 3. Disc facts (SCUS_972.68 v2.00)

- Boot ELF 2,621,256 bytes, SHA-1 `4124a79c1e5a1ceb56f514df0dbbc11f072af56f`; image 1,867,488 sectors.
- `RC2.HDR` (sector 1001) is the table of contents: the global WAD headers, then the level table with three
  sector ranges per level (level, audio, scene), sector numbers relative to each file
  (`docs/formats/disc_layout.md` §2.10, §4).
- Global WADs: `MPEG`, `MPEG1`, `MPEG2`, `MISC`, `HUD`, `BONUS`, `AUDIO`, `SPACE`, `SCENE`, `GADGET`, `ARMOR`.
  Per level `n` = 0..26: `LEVELn`, `AUDIOn`, `SCENEn`.

## 4. Code facts so far

- The level overlays' entry is the main loop (level 0: 0x2a3b88). The game mode is the word at 0x1a8f00, cases
  −1..9. The memory-card check is the same as R&C1's (icon 0x755d).
- The boot ELF holds the front end's strings (`\DEMOLOGO.NTS`, `\INFOBOT2.PSS`); R&C1's front end also lived in its
  boot ELF.
- `crossgame_diff` over level 1: 16% of functions identical or constants-only, 8% the same shape, 75% changed. The
  front end (`entry`, the page menus, the movie player) changed.

## 5. The front end (boot ELF `SCUS_972.68`)

Addresses are the boot ELF's. Names are ours.

**Program flow.** `entry` (0x131ae8) → `main` (0x131d98): runs a code pointer in a loop, starting with the front
end (0x26ede8); after each run `0x131cb8` copies the code overlay of what was loaded next (segments
`{dest, size, ?, id}` from `*(0x1a7288)`) and returns its entry.

**Disc reads.** `RC2.HDR` is read into memory at 0x14b540: the `MPEG` header there, `MISC` at 0x14b868
(`+4` the absolute sector, then `{sector offset, sector count}` pairs from `+8`). MISC pairs (v2.00):
(1, 9), (10, 367), (377, 62), (439, 844), **(1283, 1109) the front end**, (2392, 2081), (4473, 2772).

**The front-end blob** (MISC pair 4, WAD-compressed, 2,985,592 bytes unpacked) starts with 13 `{offset, size}` pairs:

| Pair | Offset | Size | Use |
| --- | --- | --- | --- |
| 0 | 0x80 | 0x11400 | texture set (`0x292650`): `{data offset, count, table offset, …}`, rows `{palette, pixels, w, h}` (16-byte units from the data, 8-bit, 256-colour CSM1 palettes); slot 1 is the 16×16-cell font |
| 1 | 0x11480 | 0xe1088 | `0x2b7700` (textures) |
| 2 | 0xf2540 | 0xd0010 | the menu backdrop: 16-byte header `{512, 416}`, then 512×416 RGBA32 (the station interior) |
| 3–8 | 0x1c2580.. | | not mapped yet |
| 9 | 0x1f8980 | 0xd17b0 | text: six language tables (index 1 empty); each `{count, size}` then `count` rows `{string offset from the table, id, -1, 0}` |
| 10 | 0x2ca140 | 0x4ab7 | splash "Sony Computer Entertainment America presents" (WAD: `{w, h}` 16-byte header, RGBA32) |
| 11 | 0x2cec00 | 0x4533 | splash "A game created and developed by" |
| 12 | 0x2d3140 | 0x5d38 | the copyright page (shown by game mode 1) |

**Boot sequence** (0x26ede8): wait for the memory card and pad checks; unpack the blob; splash 10 fades in
(`0x26ea00`: 0x80 → 0 in steps of 2), 45 frames; fade 30; splash 11; sound and text set up; then movie 0xb2 (the
Insomniac logo) plays (`0x2894c8(0x1bacc0, 0xb2)`) and the game-mode loop starts with the idle counter at 0x708. So
the first mode-0 frame starts the **title**: the attract playlist (`0x26ec58`: 24 entries from three shuffles of 0xb3..0xbb, each
the couch scene with a different clip on the TV) with "Press START" over every frame (`0x27db38`: text 0x2db6 at
(60, 328), colour pulsing 0x5ff0c070 ↔ 0x7fe0e0e0 over 100 frames, drawn by the PSS player's frame callback
`0x2b73a0` while `0x1bacc0` is set; otherwise that callback draws subtitles, `0x27dc40`). Start ends the playlist,
mode 0 comes back with the counter reset, draws the station image and opens the main menu (kind 6) over it. A minute
on the menu (0x708 frames) goes back to the title.

**Movies.** Row `i` of the MPEG header (`RC2.HDR` + 0x10 + 16·i: `{sector from MPEG.WAD's start, bytes, 0, 0}`)
spans `MPEG.WAD`, `MPEG1.WAD` and `MPEG2.WAD`, which lie back to back on the disc (sectors 2611, 527644, 1160568).
Ids map to rows by `0x2efe68` (ids above 0xb0: id − 0x91; 0xb2 = row 33).

**Game mode** (current 0x1a8bb0, pending 0x1a8bb8 = −2 when none; push `0x2b58f8(mode, 1, arg, …)`, pop
`0x2b5a88`; applied by `0x2b5b38`, which runs the old mode's exit and the new one's enter):

| Mode | Front-end loop | Meaning |
| --- | --- | --- |
| 0, −1 | draw the backdrop, push mode 3 with kind 6; after 0x708 idle frames: shuffled attract movies (0xb3..0xbb) | title |
| 1 | `0x2f6e10`: the PSS player (`0x34fca0`), movies from MPEG.WAD (header 0x14b544, 16-byte rows) | movie |
| 3 | `0x2cb860` update, `0x2ccc18` draw; 0x708 idle frames → pop | menu |
| 4, 6 | other screens | not mapped yet |

**The menu** (mode 3, enter `0x2cac28(kind)`): sub-state 0x1f27c0 runs 1 → 2 → 3 → 4 (page shown). `0x2cb038`
picks the page from the kind (0x1f27c8): kind 6 = the main menu, page 0x259438. A page is 14 panel rectangles
`{x centre, y centre, w, h}` (0xe0 bytes), then `+0xe8` the focused panel and `+0xec` 14 panel pointers. A panel is
0x30 bytes: `{update, draw, open, close, flags, ?, x, y, w, h, …}`; flag 1 = draws itself. The main menu page has one
panel (0x1aa618) that drives a GUI object at `*(0x1a8d04) + 0x3e9c8`, opened with the menu definition 0x261520
(`0x342dc0`) and drawn by `0x29d958`.

**Menu definitions** are rows of 0x14 bytes `{scale (f32), text id, kind, target, 0}`, ending in a zero row. The main
menu (0x261520): "New Game" (0x2c35, kind 4, 0x260ec8), "Load" (0x2c2b, kind 5, 0x260d98), "Options" (0x2bec,
kind 3, page 0x259568).

**The main menu's look.** Origin (250, 120) (`0x342bf0`). Per item `i`: two GUI sprites at (150, 120 + 60·i) —
0x73 (colour 0x60442d00) and 0x74 (0x55f0c070; the selected item's pulses 0x20fffeed ↔ 0x70fffeed over 20 frames,
`0x2aa3f0`), both at keyframe 0.25 — then the label centred at (250, 131 + 60·i) (`0x348e70`; list step 60, offset
11), colour 0x70fffeed selected, 0x80f0f0f0 normal, 0x80808080 disabled. Behind: the station image, darkened by a
full-screen black quad at alpha 0x38 (`0x27e440`). Colours are GS RGBA bytes (alpha 0x80 = opaque). The screen is
512×416 (`0x284c18(0x200, 0x1a0, …)`).

**GUI sprites** (the `BE_H` block, HUD.WAD; sector and count in the HUD header at `RC2.HDR` + 0x368 + 0x1350; v2.00:
sector 16415, 64 sectors): `+0x18` count, then `{id, offset}` rows. A sprite: `u16 vertices, prims, bones, textures,
frames, frame stride; f32 scale (× 0.0032116016); offsets of vertices, prims, textures, ?, frames; +0x24 packed;
+0x28 colour`. Packed vertices are one word: bone = bits 28..31, x = sext(bits 0..13)·4, z = bits 12..27 (signed);
else 16 bytes `{s16 x, y, z, bone; s16 u, v (/4096); RGBA}`. Prims are four vertex indices (a quad drawn a, b, d, c);
a word whose top 24 bits are all ones selects a texture. A bone key is 0x18 bytes: quaternion (s16 / 32768), scale
(u16 / 4096), translation (s16) and parent (s16, −1 = none); two keys blend linearly (`0x3015d8`). On screen
(`0x3018a0`): x = pos.x + (256 / 0.62)·s·v.x / 16, y = pos.y − (208 / (0.62·0.775))·s·v.z / 16; vertex colour × draw
colour / 128.

**Font** (`0x27fcb0`): glyph table at 0x263b10, 4 bytes per character `{u, v, y offset, advance}` in the 16×16 cells
of texture slot 1; a glyph is drawn 16·scale wide at (x, y + y offset·scale); characters 8..15 change the colour,
those below 0x20 are pad-button icons (24×16, drawn grey), 0x80..0xa7 carry an accent from character + 0x40.

## 6. Aranos (level 0): what is shared, what is new

Goal (2026-10-16): the first level, Aranos (level 0; the planet table in the help text starts Aranos, Oozla, Maktar
Nebula, …), playable with its cinematics, mobys, effects, sound and saves. R&C1 does not change behaviour: every R&C2
branch is taken on `rc_formats::Game::Rac2` only, and R&C1's default paths stay byte for byte what they were.

### 6.1 Evidence

- **Data** (`crates/rc-formats/examples/rac2_probe.rs` over `G/LEVEL0.WAD`). The level file is a 0x60-byte header
  `{0x60, 0, id, reverb}` then `{sector, sectors}` parts: data, core sound bank, gameplay (WAD), occlusion, chunks.
  The data part opens with a 0x80-byte `{offset, size}` table: overlay (0x80), core index, gs_ram, a 0xfdc-byte
  lump (not mapped), HUD header, four HUD banks, core data (WAD), transition (`rc_formats::rac2::level`).
  - The **core index** has R&C1's header and tables, unchanged: 833 tfrags, 351 textures, shrubs, sky (6 shells),
    collision and occlusion parse with R&C1's readers as they are.
  - **Ties**: the class header holds R&C1's fields in another order (`tie::rac2_tie_header`); the packets, strips and
    GS packets are R&C1's (73 of 74 classes walk). There are no per-vertex colour indices (count 0) and the region
    at +0x34 is not a fixed 64-normal table: R&C2 lights ties another way (to map: R&C2's tie draw).
  - **Moby classes**: R&C1's class header and packets; only the vertex table header differs, eight u16s (0x10
    bytes) with no RGBA multiplier blob. R&C2's relocation (L00 `0x2ffe70`) converts an "old format" class — R&C1's
    eight u32s — to that form. `moby::parse_moby_class_for` reads every class of every level checked (127–208 per
    level). Class 0 (Ratchet) has no mesh in the level (`{0, 0x5502163f, …}`). The core's words 0x88 / 0x8c are not a
    gadget table on R&C2 (Aranos: 0xa7, 8); gadgets are in `GADGET.WAD`.
  - **Gameplay**: another section table (pointer order differs; 0x1c89c0 bytes on Aranos).
- **Code** (`tools/ghidra/scripts/crossgame_diff.py`, `work/crossgame/level01.tsv`, R&C1 level 1 against R&C2):
  identical or nearly so: `TfragProc`, `PatchTfragGifs`, the sky (`LoadSky`, `SkyDrawShell*`, `SkyTransformVertices`),
  `MobyAnimEval`, `MobyAnimProc`, `ProcessMobyAnimData`, `PatchMobyGifs`, `PatchShrubGifs`, collision
  (`CollCellLookup`, `coll_sphere_hero_groups`, `CollType`), `UpdateFog`, `ParseOcclGrid`, GS register resets,
  `SoundInstanceUpdate`, `memcard_GetDataSize`, many particle types (08, 21, 26, 27, 45, 47, 52, 60, 66, 72).
  Rewritten (0.1–0.4): the hero (`HeroStatePhysics`, `HeroStateTransitions`, `HeroInit`), `GameStateUpdate`,
  `LoadLevelCoreData`, `MobyProc`, `TieProc`, `ShrubProc`, `LightTies`, the path enemies.

### 6.2 Decisions

1. **One engine app, the game picks its logic.** `rc-engine` keeps one Bevy app: the renderers (tfrag, tie, shrub,
   sky, mobys, particles, water, display, GS state, AA, graphics settings), the frame pace, the capture, audio out.
   The game-specific layer is a plugin: R&C1's `gameplay`, menus, travel and front end; R&C2's `rac2` (logic in
   `rc-game2`). No second copy of a renderer.
2. **Formats branch, not fork.** A reader whose layout differs takes the game from `LevelCore::game` (or a `Game`
   argument) and maps R&C2's bytes into the same structure (`tie::parse_tie_class_for`). A reader gets an R&C2
   twin only when the structure itself differs (moby classes, the gameplay table) — in `rc_formats::rac2`.
3. **Truly shared code moves once.** What the diff shows identical (the list above) is one implementation; when
   R&C2 is its second consumer it moves out of `rc-game` into a shared place with a re-export left in `rc-game`, so
   R&C1's paths and behaviour do not change. Nothing is moved before R&C2 calls it.
4. **Rewritten code is reimplemented, with R&C1 as the map.** The hero, the game state, the moby classes, the
   menus and the cameras are written from R&C2's code in `rc-game2`, in the same organisation as R&C1's: the hero as
   a state registry (state → group module, the game's dispatchers for SetState, physics and transitions), one module
   per state group, a `physics` pipeline and an `anim` interface, field names with the R&C2 addresses. R&C1's
   modules are the hint for where to look, never the source of a behaviour.
5. **Port options are the same options.** The settings (graphics preset, anti-aliasing, detail, textures, HUD
   resolution, sound volumes where the port adds them) are engine settings in the one settings file and apply to
   both games. Each game shows them on a "Port Options" page in its own menu style: R&C2's page is built from its
   own GUI sprites, font and sounds, as R&C1's is from its page machinery.
6. **Saves: same model, R&C2's tables.** R&C1's save code reads its chunk tables from the boot ELF, checks a
   CRC-16 and keeps one folder per card (`rc_formats::save_game`, `rc_game::memcard`). `memcard_GetDataSize` is
   identical in R&C2 and `memcard_TestChecksum` nearly so: R&C2's saves are read with the same reader given R&C2's
   table addresses and file names (475 KB per save, R&C2's icon), on the same native card folder under a per-game
   directory; R&C2's progression (what a save holds) is `rc-game2`'s.

### 6.3 Milestones (each one runs and is committed on its own)

| # | Milestone | Shared | New |
| --- | --- | --- | --- |
| A | Aranos world: the level file's lumps, tfrags, shrubs, sky, collision, textures, ties (unlit), fly camera (done, `RC_LEVEL=0`); New Game loads Aranos (done: the front end runs over the loaded level in the one app, crate::rac2; the save-slot page between is §I) | the renderers, `level_load` | `rac2::level`, the R&C2 load path |
| B | The gameplay file: section table, tie / shrub / moby instances, level settings (fog, background, death height), cameras, volumes, splines, lights | the instance readers where equal | the R&C2 section map |
| C | Moby classes: mesh, skeleton, animation, textures; drawn and animated at their instances (done: 384 of 472 Aranos instances placed, 156 animated; not yet the loader's ground snap, the spawn rules, Ratchet's armour mesh) | moby renderer, anim eval | the R&C2 vertex table header |
| D | Tie and moby lighting as R&C2 does it | light passes where equal | R&C2's tie colours |
| E | Sound: the level's bank, sound instances, music, movie audio, the front end's sounds | `audio_out`, 989snd, VAG | R&C2's bank tables |
| F | Ratchet: hero states for Aranos (walk, jump, swing, wrench, the tutorial's gadgets), follow camera, collision | collision queries | `rc-game2::hero` |
| G | Aranos's mobys: the tutorial's objects, enemies, crates, bolts, doors, the ship; HUD and pause menu | moby scheduler services | `rc-game2` classes |
| H | Cinematics: Aranos's scenes (SCENE0) and movies, subtitles | `scene_render`, movies | R&C2's scene format |
| I | Saves and the level end (to Oozla, or the "not yet" screen) | `save_game`, card folder | R&C2's tables |

### 6.4 Ratchet (milestone F, started 2026-10-16)

**What the code shows.** R&C2's hero is R&C1's hero evolved, not rewritten: the same driver (`0x2b37e8` = R&C1
`0x231d18`: stick with the d-pad below 0.25, per-state physics, yaw / position rings, rows, capsule, move, post-move),
the same two dispatchers (physics `0x2ba8e0`, transitions `0x2cb650`), SetState `0x2c4560` with R&C1's bookkeeping,
the same helpers (stick target `0x2b3af8`, turn `0x2b4000`, speed `0x2b42c0`, drag `0x2b5600`, edge brake `0x2ba008`,
wall check `0x2b4560`, move `0x2b5c68`, probe `0x2b4a08`, snap `0x2b5358`) and the same core state ids (idle 0, walk
2, stop 3, crouch 4, fall 6, jump 7, run jump 9), with R&C2's constants (run speed 5.95, jump 1.28–2.3 / 1.0–2.7,
gravity 0.00825) and additions (gravity frames, strafe, 0x93 states). From the velocity on, the hero block's fields
sit at R&C1's address + 0x4aae0. The crossgame diff's 0.1–0.4 similarity came from those additions inside long
functions. So the port follows R&C1's organisation (`rc_game2::hero`: registry, states, physics, ground, walk, air,
jump), each function read from R&C2's code, in native f32. Details that differ were found that way (the stick
magnitude `(max(|x|, |y|) + |stick|) / 2`, the spring's pre-clamp, the snap constants).

**Shared, used as they are:** the pad record (`0x3306b0` = R&C1's `ProcessPadInput`), the collision kernels, and
Ratchet's animation player (SetAnim `0x2d2508` = R&C1's `0x247a90`). New in R&C2 and now read by `rc_formats`:
Ratchet's mesh is the armour's (`rac2::armor`), and his sequences are compressed (frame pointers with 0xf in the top
nibble, decoded by `0x313fa8`: `moby_anim::decode_compressed_frame`).

**Done:** idle, walk / run (the walk ↔ run selector `0x2ca358`), stop, crouch, fall with its landings, jump 7 and run
jump 9 (height ramp, windup, air control `0x2b6cd0` / `0x2b62a0`, landing `0x2cab40`), the double jump 0xe and the
flip 0xb. R&C2 reuses R&C1's id 0xb (R&C1's long jump) for the crouch flip: R1, a direction and ✕ together, its
direction picked by `0x2c3cf8`, also chained on landing. R&C2's long jumps (10, 0xd, 0xf, 0x10) need the Heli-Pack or
Thruster-Pack (`0x2cadb0`), which Ratchet does not have on Aranos's opening. Engine: crate::rac2_play
(pad, hero, camera, Ratchet's instance and pose); the follow camera (§6.5). **Not yet:** every other state (the pack jumps,
the wrench, swingshot, gadgets, water, ledges, damage / death), the moby collision of the queries, the
lighting of Ratchet, his sounds, Clank on his back.

### 6.5 The follow camera (milestone F, 2026-10-16)

**What the code shows.** The camera update `0x2d9f38` calls the current camera's type update from the table at
`0x2a3a0c` (0x14 bytes a type). Type 0, the follow camera (update `0x4278c8`, init `0x424908`), is R&C1's type 0
evolved: the same pre-steps (collision flags `0x2d9ba8`, motion `0x2d9760`) and the same steps in the same order
(platform carry, state tweaks, targets with the target modes and the vertical target, leash, stick, avoidance with
the sphere chain and end sphere, placement, rows, spring back), each a function of its own. R&C2's differences:

* the vertical spring is 0.0125 (R&C1 0.015), and in the air the state tweaks ramp it from the smoothed target's
  vertical gap;
* the leash blends at 0.85;
* the stick's yaw rates are 1° / 1.3° / 2.2° a tick (the option at `0x1a7bb4`);
* target mode 1 also starts in group 0x1e;
* the knock-down state is 0x74 (R&C1 0x77);
* the placement has no 15° clamp: a 0.25 sphere (flags 0x36) pushes the camera out up to three times instead;
* the rows' pitch bias applies only on the ground;
* the snap behind Ratchet at the init only pulls in on the line from the look point, 0.5 short of the wall.

The hero tells the camera about jumps through `0x18c2c0` (0x50 in the jump states; the smoothed hero height holds
while it is set). The port is `rc_game2::camera` (one function per step, the L00 address on each), in native f32.
**Not ported** (none is in use on Aranos's opening): the camera-tweak requests `0x2d7568`, the camera-collision grid,
the moby collision of the camera's queries, the avoidance's moby nudges, the focus mobys, the stick auto-centre, the
Clank and Giant Clank cameras, water, the knock-down view, the other camera types, the blends and the shakes.

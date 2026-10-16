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
   Run: `RC_DATA_DIR=<repo>/extracted-rac2 cargo dev`.
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

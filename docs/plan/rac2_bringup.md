# R&C2 (Going Commando) bring-up

Status: **started 2026-10-16.** Goal for 0.3.5: Going Commando can be selected, starts, and shows its own main menu.
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
3. **Engine boot path.** A game switch in the engine (from the data folder's `extract-info.json`) and an R&C2 front
   end in `rc-game2`: the logo movie and the main menu.
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

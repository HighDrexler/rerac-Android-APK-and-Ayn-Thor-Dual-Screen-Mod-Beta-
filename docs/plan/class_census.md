# Class census: the unported moby classes and the shared systems they need

A static census (2026-09-28) of every moby class placed on levels 00–18 whose level update the port does not run
(gaps.md G-CLS-001). For each class it finds the update function on every level, groups the level copies that are the
same code into one **class-port unit** (the way `LevelPorts` matches ports), walks the unit's static call graph down to
the **shared functions** it calls (engine code in the overlay, level code other classes also call, the boot ELF), and
tags each shared function with the game system it belongs to and whether the port has it.

Nothing in `crates/` changed. The tool is a new `rc-trace` subcommand; the tags are a committed table.

**Numbers at this run.** 668 unported class-levels: **6,029 placed, 5,533 created** instances (G-CLS-001 said 5,543:
the sea port landed during the run and took 10), in **448 class-port units**. They call **473 shared functions**;
430 are tagged, 43 (all of them rare) are not. Every unit's call graph was walked (100 % of classes and instances); a
system verdict exists for **429 units / 5,485 created instances (99.1 %)**; 19 units / 48 instances call an untagged
function and nothing else the port lacks.

**Re-run 2026-09-28 (after the cheap-wins batches and the other ports of that day):** 548 unported class-levels,
4,526 placed / **4,030 created** instances in 417 units; verdicts cheap 197 units / 1,308, missing 161 / 2,470, partly
40 / 204, unknown 19 / 48. The unit ids change between runs (they are numbered by instances).

**Re-run 2026-09-29 (W0, priority_order.md: the re-tag and the data census).** The unported family is unchanged
(548 class-levels, 4,526 placed / **4,030 created** instances, 417 units: no class port landed between the runs), but
the census now also sees **direct reads of engine state** (§Method 7: a `G:` tag per global, `X:` overrides), and the
tags were verified against the code: the Suck Cannon interface (G-TRI-015), the help system, the splash `0x2ff768`, the
burn sparks `0x271258`, the move-collide `0x26d270`, the Doom bot's hop `0x270340`, the feather spawner `0x2e2cc0`, the
ammo read `0x249530`, the walk-to-point states `0x249580` / `0x2495d0` and `0x279070` (Lombyte's "OpenShipMenu": it is
`PromptRelease`, interact.rs) are **has**. New: the fighters 1843 / 1319 (ship-combat mode, G-LVL-009) and the emitters
838 / 855 (voice handoff, G-AUD-010) are blocked by overrides; branches on hero states 0x32 / 0x51 and on the bodies
0x1413f4 = 1 / 2 / 3 (G-HERO-002 / G-HERO-005), the cheat flag `0x15edb7`, `SwitchCharacter` and the leave-body copies
are tagged; 30 of the 43 untagged functions are named. **480 shared keys, 467 tagged, 13 untagged**; unresolved units
19 / 48 → **6 / 8**. Verdicts:

| verdict (per unit) | units | created instances | 2026-09-28 re-run |
|---|---|---|---|
| **cheap**: every shared call is one the port has (or trivial arithmetic, or the class family's own level code) | 208 | 1,177 | 197 / 1,308 |
| **missing**: calls at least one function of a system the port lacks | 163 | 2,594 | 161 / 2,470 |
| **partly**: calls a function the port has only in part, or **branches on a hero state / body the port lacks** | 40 | 251 | 40 / 204 |
| **unknown**: nothing missing, but one untagged function | 6 | 8 | 19 / 48 |

**Re-run 2026-09-29 (W1 lane 1, the path helpers: G-CLS-025).** The path helpers are ported (`rc-game/src/path.rs`;
seven census keys re-tagged has: `0x262e40` / `0x277d40`, `0x261d78`, `0x264558` / `0x2a0260`, `0x263710` / `0x261b48`)
and three of their units landed (below, "In the port: path units": 1564, 257, 1667–1671: 203 instances). **538**
unported class-levels, 4,323 placed / **3,827 created** instances in **414 units**; verdicts cheap 219 units / 1,498,
missing 147 / 2,030, partly 42 / 291, unknown 6 / 8. The `path` system blocks **1 unit / 2 instances** (947 on level
10: `0x277260`, unported). A caution on the re-tag: the census counts a function as the unit's own when only that unit
calls it, so a unit can turn "cheap" while its private code holds a whole system. Level 03's air-traffic family
(75, 115–120, 132, 795: `0x29dba8`, 246 instances) is the case: its own code includes a level-03 explosion `0x24ce98`
(3,136 words: the beam-explosion family compiled into the overlay; G-CLS-028) and the effect-moby spawner `0x2bad40`
(class 235; G-CLS-026). It is not cheap.

"Cheap" means the class needs **only its own code** ported: the shared systems are there. It is not free: the
appendix gives each unit's private code size (functions / words).

Two systems are **conditional**: they run only when the player does something specific, so a class that needs only
them still works in normal play. The big-head **cheat** manipulator (the enemies' `0x278720` and its level copies) and
the cheat flag `0x15edb7` run only with the cheat on; **memcard_Save** only at a save point. (The Suck Cannon interface
was the third; the port has it since G-WPN-003 closed.) A **branch on a hero state or body** the port lacks (state
0x32, Clank's 0x51, the bodies 1 / 2 / 3) is a dead branch, not a blocker of the class: the port can take the class
with the branch filed under G-HERO-002 / G-HERO-005.

**Ready now** (blockers only conditional, or only such branches): **243 units / 1,653 created instances** — 208 / 1,177
cheap, 17 / 246 conditional-only, 18 / 230 with a hero-state branch (the same measure on the 2026-09-28 tags, counting
the Suck Cannon interface as conditional: 215 / 1,722; the difference is the fighters 112 and the emitters 33 leaving,
and the help / splash / burn / react units joining).

## Method (to regenerate)

```
cargo run -p rc-trace -- class-census          # writes work/census/*.tsv (about 15 s)
```

`tools/trace/src/class_census.rs` (the `class-census` command; `--out DIR`, `--tags FILE`, `--extracted DIR`). Inputs:
`extracted/levels/NN/{overlay.bin, gameplay_ntsc.bin}`, the port's registry `rc_game::moby_update::classes::LevelPorts`,
`tools/ghidra/names/clusters.tsv`, the decompiler export's `work/decomp/<program>/index.tsv` (names only) and the tag
table `tools/ghidra/names/census_systems.tsv`.

1. **Class list and instances.** Per level the class table (`LevelOverlay::vtbl`), and per class the placed instances
   (the gameplay instance list) and the created ones (`moby_spawn::loader_spawns` with an empty save, as
   `all_levels_smoke` counts them). A class is unported when `LevelPorts::get` gives no port and its update is not 0;
   the created total (5,533) equals `all_levels_smoke`'s report at the same moment.
2. **Call graph.** Function starts exactly as `LevelOverlay` finds them (`jal` targets, class-table updates, a frame
   set-up after `jr ra`, code addresses formed with `lui`/`%lo`). Edges: `jal`; a `j` out of the function (tail call);
   a code address formed with `lui`/`%lo` when the target also has start evidence of its own (a `jal` target, a
   class-table entry, or right after `jr ra` + delay slot) — callbacks such as draw callbacks and state functions
   stored into moby +0x74; a `.data` table of such starts formed the same way; and the fall-through into a start that
   has no evidence of its own. `jalr` sites are counted as unresolved indirect calls: **none** of the 448 units' own
   functions contains one (indirect calls live in engine code), so no call is left unresolved.
3. **Units.** From the update, a callee joins the unit when every caller of it is already in the unit; the callees
   left outside are the unit's shared functions. Level copies of an update are one unit when their masked code (the
   `Relocation` masking: `rc_formats::level_overlay::mask`) hashes the same.
4. **Identity of a shared function across levels**: its boot address (`boot:`); its level-01 copy by the port's
   `Relocation` (a function start in level 01, same extent for functions under 8 words); else its level-01 member in
   `clusters.tsv`; else the cluster (`C:` + 12 hex of the cluster hash: engine code that the level-01 overlay does not
   link); else the masked-code hash (`H:`).
5. **Has / partly / missing** was decided per function by searching `crates/` and `docs/` for the level-01 address
   (`shared.tsv` columns `crates_refs`, `crates_files`) and reading the cited code: a function the port cites as its
   model is "has", one it cites as counted / not ported is "missing", one without a citation was read in the decomp
   (`work/decomp/`, and Ghidra for spot checks of the graph, e.g. level03 `0x29dba8`'s callees) and tagged by what it
   does, marked `L` (inferred). Two extra statuses: `trivial` (arithmetic a port writes inline: zero a vector, fabs,
   memset) and `family` (level code shared only by the classes of one family, so part of their own port).
6. **Outputs** in `work/census/`: `classes.tsv` (level, class, placed, created, update, unit, port), `units.tsv`
   (instances, levels, classes, update copies, level-01 copy, private functions / words, `jalr`, shared keys),
   `shared.tsv` (per shared key: units, instances, the port's citations, callees), `globals.tsv` (every boot global an
   unported unit's own code forms, with the units), and with the tags `unit_systems.tsv` (verdict, systems, blocking
   keys, untagged) and `systems.tsv` (the ranking below).
7. **Direct reads of engine state** (2026-09-29). A call census misses a class that reads or writes engine state
   directly (the fighters test Ratchet's state and the ship moby; 838 rewrites a voice slot's owner). The tool now
   walks each unit's own code with a small register file: every `lui` (+ `addiu` / `ori`) constant, every load or
   store whose base register holds one (`base + offset`: the hero state `0x1413d4` read as `0x13f350 + 0x2084`), and
   every loaded global compared with a constant (`xori`, or `beq` / `bne` against a register holding one) becomes a
   data key: `G:<address>` or `G:<address>=<constant>` (`G:001413d4=00000032`: the hero state against 0x32). Only the
   keys the tag table names count as dependencies (a `G:` row has a system and status like a function); the rest are
   listed in `globals.tsv` for the next tagging pass. Stores through a computed index (838's `0x13e5d8 + slot·0x70`)
   are still invisible: those get an `X:<class>` **override** row with the reason (the fighters 1843 / 1319, the
   emitters 838 / 855).

**Limits.** The census sees **calls, and the tagged direct reads of §7**. A read through a computed index or a
pointer chain (a voice slot's owner by slot index, another moby's fields) shows no dependency; the saved-game flags
0x13d388 of G-SAV-010 are formed by 34 units but are not a blocker (`interact::set_global_flag` is ported). Tags marked
`L` were read from the decompiled code without a port to compare with. A function's status is about the function; a
system's status is "partly" when the port has any of its functions.

## Main table: missing or partial systems, ranked by the instances they block

A unit counts toward every system it needs; "classes" counts distinct class numbers. Addresses are level 01 unless
marked; `C:` rows cite one level copy (the others: `clusters.tsv`).

| system | status | gap row | classes | units | instances | example classes (instances) | key shared functions (× units calling) |
|---|---|---|---|---|---|---|---|
| Big-head cheat manipulator and flag [conditional] (`cheat`) | missing | G-SAV-006 | 57 | 48 | 1,497 | 827 (155); 1246 (126); 749 (106); 193 (93) | `L01:00278720` FUN_00278720×35; `C:ec9f80c602e5` FUN_00251d70×8; `G:0015edb7=00000000` cheat_bighead_flag×4; `C:606d87e75d7d` FUN_0025ea60×2 |
| Path helpers not linked on Novalis (`path`) | partly | G-CLS-025 | 1 (was 53) | 1 (was 31) | 2 (was 1,108) | 947 (2) (was 75/115/116 (246); 827 (155); 749 (106); 1564 (92)) | `L01:00277260` FUN_00277260×1 (2026-09-29 W1: `C:c4c12687d93c` FUN_00262e40, `C:49a176c9f6de` FUN_00270d00, `C:17a7139ba6a7` FUN_00263710, `C:aeb1c69a6e0e` FUN_00264558, `L01:00277d40`, `C:a3bbdace838c` FUN_002a0260 are has: `rc_game::path`) |
| Hero scripted states, bodies and the branches on them (`herostate`) | partly | G-HERO-005, G-HERO-008 (~~G-HERO-002~~ closed 2026-09-29), G-LVL-002 (H2) | 96 | 68 | 880 (2026-09-29 after G-HERO-002: 34 units / 52 class-levels / 487 created; the state-0x32 branches and the level `SetState` copies are has) | 827 (155); 193 (93); 638 (64); 1843 (60) | `G:001413d4=00000032` hero_state_0x32×14; `G:001413f4=00000002` body_giant_clank×13; `G:001413f4=00000001` body_clank×12; `C:8a18636e69bd` FUN_002223f8×11; `C:ea215dba76e7` FUN_002356a0×5; `C:2f2e9be4c73e` FUN_00230b38×4 |
| NPC look-at manipulators and animation leftovers (`anim`) | partly (0x2777d8 has, 2026-09-30) | G-HERO-009 | 4 (was 47) | 4 (was 42) | 65 (was 422) | 294 (40); 1059 (14); 563 (10); 282 (1) (was 638 (64); 238 (54); 623 (51); 294 (40)) | `C:94cf214dfe7f` FUN_0027b9c0×2; `L01:0026c7a8` MobyAnimBlendEx×2; `L01:002b52a8` FUN_002b52a8×1 (`L01:002777d8` ported: `moby_update::manip::look`) |
| Creature-layer copies not linked on Novalis (`creature`) | has (2026-09-29) | ~~G-ENM-009~~ | 0 (was 23) | 0 (was 22) | 0 (was 363) | (was 238 (54); 294 (40); 1023 (40); 1112 (31)) | the three clusters re-tagged has / ported ("In the port: creature units") |
| Effect-moby spawners not linked on Novalis (`fxmoby`) | has (2026-09-30) | ~~G-CLS-026~~ | 0 (was 31) | 0 (was 21) | 0 (was 276) | (was 623 (51); 79 (30); 541 (29); 1511 (26)) | `C:494499ccf189` = L01 `BreakFxB` 0x278ad8 (padding in the hash); the L06 / L08 / L09 / L10 spawners re-tagged `family` (creatures.md §12) |
| Point-light flicker (`light`) | not a system (2026-09-30) | ~~G-REN-024~~ | 0 (was 12) | 0 (was 6) | 0 (was 256) | (was 1246 (126); 580 (82); 615 (19); 612 (15)) | `C:5d1f33c5603a` = L01 0x2723f8, the hit flash (`creature`, has; creatures.md §12) |
| Moby platforms and riders (`platform`) | has (2026-09-30) | ~~G-CLS-024~~ | 0 (was 7) | 0 (was 7) | 0 (was 181) | (was 1246 (126); 812 (25); 574 (17); 1381 (8)) | `L01:002752c0` / `002753b0` / `00275528` / `00275290` / `002755f8` and `C:584a926e540b` / `C:6e3b7a963b84`: `triggers.rs` (triggers.md §5b) |
| Ship-combat mode [override] (`shipmode`) | missing | G-LVL-009 | 2 | 2 | 112 | 1843 (60); 1319 (52) | `X:1319` fighter_1319×1; `X:1843` fighter_1843×1 |
| Particle types not ported (`ptype`) | missing | G-PRT-001 | 18 | 14 | 90 | 28 (43); 1139 (12); 1440 (9); 170 (5) | `C:1fa9adb9d4ec` FUN_00274948×5; `L01:002780b0` SpawnImpactSparks×2; `C:dd572bc43542` FUN_0029b8d8×1; `L01:0027e750` PartType05Spawn×1; `L01:00282ef0` PartType28Spawn×1; `L01:00284d88` PartType40Spawn×1 — **2026-09-29**: every tagged spawner but types 40 / 58 (`PartType40Spawn`, `PartType58Spawn`: G-PRT-008) is ported and re-tagged `particles has` (particles.md "Update types, 2026-09-29"); a census re-run moves those units off `ptype` |
| Break-effect variant (`breakfx`) | partly | G-CLS-014 | 5 | 5 | 87 | 1382 (34); 573 (27); 1048 (20); 556 (5) | `C:6e363a2f7f51` FUN_00251f08×5 |
| Follow / script camera settings from mobys (`camera`) | partly (2026-09-30) | G-HERO-026 | 3 (was 38) | 3 (was 29) | 3 (was 74) | 1422 (1); 1470 (1); 1051 (1) (was 615 (19); 351/1301 (10); 877 (6); 1380 (4)) | `L01:00313b48` FUN_00313b48×3 — **W3 lane 3 2026-09-30** (player_controller.md §15): `C:3b375f0ab07b` FUN_002e9758 (×24) and `C:33058002480b` FUN_002f89b0 (×4) are the script camera's Euler-target setter `0x316e28` (the same code, the camera pointer through `$gp`: `Camera::camera_script_targets`, has); the follow camera's setters 0x313560..0x3137f8 ported (`Camera::set_*`, the class-17 regions their first consumer) and re-tagged has. The units still need their own code; `0x313b48` (3 units) is a one-line setter left for its first ported caller |
| Blob shadows (`shadow`) | partly | G-REN-025 | 7 | 7 | 70 | 1112 (31); 1269 (20); 1110 (12); 871 (4) | `C:bd2754fc3a9b` FUN_00259fe8×6; `L01:0026eec8` FUN_0026eec8×1 |
| Save bytes variant [L] (`death`) | partly | G-SAV-003 | 4 | 3 | 45 | 44 (21); 44 (17); 78 (7) | `C:641f527e8cd4` FUN_002a35d8×3 |
| Level sound on a slot (`lsound`) | partly | G-AUD-007 | 14 | 4 | 41 | 466/480/485 (38); 1118 (1); 1109 (1); 1108 (1) | `C:8d520530637e` FUN_0029aec0×3; `C:bcf4ccdaa994` FUN_0027eca0×1 — **W2 2026-09-29**: L04 `0x27eca0` is `PlayLevelSoundAtMoby`'s copy (has); the 466 family (38) is cheap |
| Save [conditional] (`save`) | missing | G-SAV-002 | 39 | 38 | 39 | 1005/1016 (2); 1750 (1); 1428 (1); 1455 (1) | `L01:00261448` memcard_Save__Fii×37; `L01:00260e80` memcard_MakeWholeSave×1 |
| Voice handoff between mobys [override] (`voicehand`) | missing | G-AUD-010 | 2 | 2 | 33 | 838 (26); 855 (7) | `X:838` voice_handoff_838×1; `X:855` voice_handoff_855×1 — **closed W2 2026-09-29**: `SoundSink::hand_over`; 838 ported (`laser_fence`), 855 cheap |
| Draw-callback sprite helper (`fxdraw`) | partly | G-REN-020 | 20 | 8 | 20 | 1907/1908/1910 (7); 1405/1406/1407 (7); 293 (1); 1919 (1) | `L01:0021e340` DrawSpriteHelper_A×8 — **closed W2 2026-09-29**: VU1 set-up only, re-tagged has; its units are cheap |
| Particle spawner variant (types from a table) (`particles`) | partly | G-PRT-001 | 5 | 3 | 5 | 1400 (2); 1629 (2); 1400 (1) | `C:b376dbd21da4` FUN_00272cb8×3 |
| Level height grid (`heightmap`) | missing | G-LVL-008 | 3 | 2 | 3 | 1400 (2); 1400 (1) | `L01:00278020` FUN_00278020×2 — **closed W2 2026-09-29**: `rc_formats::level::HeightGrid`; 1400 still needs `ptype` |
| Cinematic leftovers (slideshow, the ship registration) (`cine`) | partly | G-CUT-003, G-LVL-001 | 2 | 2 | 2 | 1422 (1); 1353 (1) | `L01:002a2360` FUN_002a2360×1; `L01:002ad558` EnterSlideshowMode×1 |
| Group command variants (`group`) | partly | G-CLS-023 | 2 | 2 | 2 | 1108 (1); 1051 (1) | `C:40920504317a` FUN_002f9948×2; `C:bcbd2dbe1730` FUN_002f99b0×2 |
| Leaving the level (`levelexit`) | missing | G-LVL-001 | 2 | 2 | 2 | 436 (1); 834 (1) | `L01:002a29a0` FUN_002a29a0×2 |
| An angle spring variant (`math`) | partly | — | 1 | 1 | 1 | 1106 (1) | `L01:0020cf28` FUN_0020cf28×1 |

Sole blockers (the one system a unit still needs, cheats and saves aside; the W0 run — after W1 the path entry is 2 (1 unit)): path 648 (17 units), herostate 297 (31 units), fxmoby 114 (12 units), creature 84 (5 units), lsound 38 (1 units), ptype 38 (8 units), platform 36 (3 units), voicehand 33 (2 units), anim 23 (18 units), fxdraw 20 (8 units), camera 17 (8 units), light 14 (2 units), death 7 (1 units), shadow 4 (1 units), particles 2 (1 units), levelexit 2 (2 units), cine 1 (1 units).

**What the port already has** (and the unported classes lean on most): math, timers and RNG (VecAdd, `ticks`, the
`rand` stream: called by most units), moby create / delete / matrix, animation (`MobyAnimBlend`, joints, manipulators), class
sounds and voice slots, the Suck Cannon reaction interface, the help messages and prompts, hit messages and the hit resolver, the creature layer (knockback, hit flash, burn sparks, the water splash, turn, walker,
move-collide, target, arena paths, death explosion, `SpawnBeamExplosion`, `BreakFxB`), the ported particle types' spawners, collision queries,
trigger volumes, death bits and bolt drops, the shadow probe, draw callbacks with `FastDrawQuadReal` / FX textures /
the glow quad, the script camera and cinematic calls, the HUD banner, groups, splines and the flyer path driver, point
lights, the target list, mission flags.

## Cheap wins: classes whose shared calls the port already has

**208 units / 1,177 created instances** (2026-09-29 run) call only functions the port has (or trivial arithmetic, or
their family's own level code). Each needs its own update ported, nothing shared first. Unit ids are this run's (they
renumber per run; the "In the port" tables below keep the ids of their own runs). The largest 45 (the rest are in
the appendix, verdict "cheap"); "private" is the unit's own code in 32-bit words:

| unit | classes | levels | created | update (first copy) | private words |
|---|---|---|---|---|---|
| U479 | 408 | 15, 17 | 48 | L15:2cb4c8 | 604 |
| U473 | 123 | 15, 17 | 46 | L15:2a6e68 | 496 |
| U498 | 471 | 16 | 37 | L16:2c9878 | 524 |
| U469 | 77 | 15, 17 | 36 | L15:2a2488 | 248 |
| U560 | 1355 | 18 | 36 | L18:2efb88 | 606 |
| U534 | 99 | 17 | 35 | L17:2a8da0 | 616 |
| U436 | 30 | 14 | 27 | L14:2b3bf0 | 786 |
| U509 | 933 | 16 | 25 | L16:2ddde0 | 302 |
| U410 | 127,128,159,169 | 13 | 24 | L13:2c7f38 | 278 |
| U455 | 1224 | 14 | 24 | L14:3015d0 | 894 |
| U215 | 1038 | 06, 10, 17 | 21 | L06:2f7288 | 426 |
| U171 | 133 | 05 | 20 | L05:2dac80 | 558 |
| U502 | 552 | 16 | 20 | L16:2cf4a8 | 330 |
| U552 | 568 | 18 | 20 | L18:2d5918 | 404 |
| U392 | 1259 | 12 | 18 | L12:302f30 | 648 |
| U185 | 843 | 05 | 16 | L05:30e508 | 188 |
| U252 | 1069 | 07 | 16 | L07:3112c8 | 600 |
| U413 | 224,228 | 13 | 16 | L13:2e1cc8 | 416 |
| U483 | 1209 | 15, 17 | 15 | L15:2e5958 | 354 |
| U536 | 669 | 17 | 15 | L17:2d77f0 | 518 |
| U211 | 911 | 06 | 14 | L06:2f3ad8 | 690 |
| U345 | 1544 | 10 | 14 | L10:2ea1f0 | 698 |
| U514 | 1401 | 16 | 14 | L16:2e37a0 | 856 |
| U125 | 868,905,928 | 03 | 12 | L03:294c08 | 356 |
| U472 | 93 | 15 | 12 | L15:2a3ba8 | 200 |
| U212 | 1021 | 06 | 11 | L06:2f4f00 | 280 |
| U325 | 939 | 10 | 11 | L10:2d8c00 | 146 |
| U433 | 8 | 14 | 11 | L14:2ac618 | 2198 |
| U40 | 27 | 01 | 10 | L01:2bd100 | 352 |
| U281 | 468,469 | 08 | 10 | L08:2ea4c0 | 200 |
| U306 | 1172 | 09 | 10 | L09:303d10 | 294 |
| U476 | 196,197,1958 | 15, 17 | 10 | L15:2bddb0 | 192 |
| U510 | 1143 | 16 | 10 | L16:2e1088 | 54 |
| U59 | 695 | 01 | 9 | L01:2f8268 | 218 |
| U219 | 1054 | 06 | 9 | L06:2fbfb0 | 444 |
| U309 | 1206 | 09 | 9 | L09:305a28 | 122 |
| U327 | 1015,1282 | 10 | 9 | L10:2d90a8 | 174 |
| U388 | 339 | 12 | 9 | L12:2ec1d0 | 170 |
| U162 | 1101,1102,1531,1532 | 04 | 8 | L04:2e17d8 | 170 |
| U180 | 823 | 05, 07 | 8 | L05:30c0a8 | 94 |
| U253 | 1080 | 07 | 8 | L07:311bc8 | 208 |
| U381 | 240 | 12 | 8 | L12:2e3ed8 | 1108 |
| U409 | 111 | 13 | 8 | L13:2c4428 | 1632 |
| U463 | 1417 | 14 | 8 | L14:306ee0 | 742 |
| U468 | 67 | 15 | 8 | L15:29aff0 | 296 |

**Ready with a conditional blocker only** (the cheat manipulator / flag or `memcard_Save`; 17 units / 246): U268 (252: 77); U406 (63: 58); U299 (52: 46); U520 (1445: 24); ~~U334 (1199: 15); U405 (29: 9); U333 (1196: 7)~~ (ported 2026-09-29, W3); U163 (1120: 1); U164 (1190: 1); U311 (1290: 1); U318 (18: 1); U339 (1326: 1); U349 (23: 1); U459 (1354: 1); U485 (1388: 1); U488 (1419: 1); U570 (1750: 1).

**Ready with a hero-state branch only** (a dead branch on state 0x32 / 0x51 or on the body 0x1413f4, to file under
G-HERO-002 / G-HERO-005 in the class port; 18 units / 230): U300 (193: 93); ~~U335 (1202: 57)~~ (ported 2026-09-29, W3); U478 (233: 28); U387 (336: 11); U439 (250: 11); U330 (1067: 6); U336 (1229: 5); U338 (1302: 5); U471 (92: 4); U548 (1772: 2); U231 (1302: 1); U301 (263, 264, 265, 266, 267: 1); U331 (1073: 1); U356 (361: 1); U415 (388: 1); U425 (1271: 1); U493 (1469: 1); U544 (1428: 1).

Newly ready since the 2026-09-28 tags: the help directors and NPCs that needed only the help system (1413, 1324, 1343,
1347, 1348, 1349, 1000, 1344, 422, 558, 253, 1179, 586, 1035, 318 and more: every "partly: help" unit), Kerwan's
director 1342 (`0x249530` is the ammo read, ported), the enemies that needed only the Suck Cannon interface (568;
193, 252, 63, 1202, 52, 1445, 1199, 29, 1196 stay conditional on the cheat), the splash / burn callers 35 and 8, the
pushables 695 (`0x26d270`). No longer cheap: the fighters 1843 / 1319 (G-LVL-009) and the emitters 838 / 855
(G-AUD-010), by override; 4 units read the cheat flag directly (1440, 427, 44, 638: conditional).

Notes: U21 (class 1060, 334 lamps on 00 / 02 / 05 / 18, first run) and U92 (12 classes on 10 levels, 67 instances,
first run) are ported (below). Under G-CLS-027 these are one batch of class ports.

### In the port: cheap wins, levels 00–08 (2026-09-28, G-CLS-027 part A)

Unit ids below are the census's after the re-run of 2026-09-28 (the run renumbers them: U21 above is U27 here). Each
unit is one row of `rc_game::moby_update::classes::units::PORTS` (`ClassUpdate::Unit(i)`, shared with the 09–18 half),
found on every level by code identity; the module doc of each port carries the unit's coverage table (every call,
branch and side effect of the update and its private helpers, with the file / function or the gap).

| unit | classes (levels) | created | reference | port | notes |
|---|---|---|---|---|---|
| U27 | 1060 lamps (00, 02, 05, 18) | 334 | level00 0x2df4f8 | `units::lamp` | colour pulse, ambient (red, blue, 0: the game's argument slip, kept), one glow callback per group and tick through the pvar **shared data** (gameplay 0x4c, now loaded: `Services::pvar_shared`); `Callback::UnitGlow` drawn by `rc-engine` fx_draw with the shared glow quad `0x2781d0` |
| U99 | 12 classes on 10 levels | 67 | level02 0x2dd4d0 | `units::empty` | `jr ra; nop`: `LevelPorts` maps every table entry that starts with `jr ra; nop` (and has no other port) to it; also the unplaced 433 / 531 / 618 / 1289 … of every level (Novalis included: they were parked with mode 2, the game updates them) |
| U268 | 21 loose-piece classes (08) | 85 | level08 0x2dba40 | `units::loose_piece` | rest (state 2) until set loose by another moby: tumble about an anchor, bounce (`coll_sphere` + reflect ×0.5), fade, delete below z 5; the mover is outside the unit |
| U95 | 296, 652, 653 conveyors (02, 12) | 42 | level02 0x2dc6b0 | `units::conveyor` | carry riders by a heading velocity (`triggers::carry_riders`); one-way or reversing every 180 ticks |
| U241 | 886 timed switches (07, 12) | 30 | level07 0x30bf90 | `units::timed_switch` | group state machine (`scheduler::group_state` / `group_cmd`), countdown ticks, mission done |
| U247 | 9 mover classes (07, 13) | 28 | level07 0x310df0 | `units::linked_mover` | slides out / back on a linked moby's states; the in-game map zone flags `0x184528[i]` it writes (`MapState::set_zone_flag`) |
| U229 | 1512 vents (06) | 26 | level06 0x308c68 | `units::vent` | steam puffs / spark bursts (type 2), the delayed death explosion |
| U280 | 621 rail mines (08) | 25 | level08 0x2f44c0 | `units::grind_mine` | a hit or a grinding Ratchet → `SpawnBeamExplosion`; the path mode's path pointer is taken as a spline index [L] |
| U185 | 852, 853 rising blocks (05) | 21 | level05 0x314eb0 | `units::rising_block` | `0x270830` spring (moved from the water managers to `creature::turn::spring`: two consumers) |
| U221 | 1091–1098, 1103 bobbing blocks (06) | 21 | level06 0x300df0 | `units::bob_block` | |
| U281 | 648 smoke emitters (08, 10) | 21 | level08 0x2f5830 | `units::smoke_emitter` | the default plume's third jitter lands on x again (kept) |
| U139 | 915–917 markers (03, 10) | 19 | level03 0x2dc310 | `units::marker` | deleted by their first update |
| U170 | 341 Hydrodisplacer pads (05, 07, 11, 12, 18) | 17 | level05 0x2f8080 | `units::hydro_pad` | |

**Not cheap after all** (found while porting): U180 (838, 05: 26) and U186 (855, 05: 7) hand one looping voice between
the group's emitters (the nearest to the camera rewrites the slot record's owner and position, `0x13e5d8 + slot·0x70`):
G-AUD-010. **Census artefacts**: U36 (class 27, 01: 10) is the particle emitter the engine runs as an external
(`LevelPorts::external`); the census counts only `LevelPorts::get`. Not reached (budget): U211, U168, U182, U248, U207,
U123, U208, U278, U215 and the smaller units of the appendix (G-CLS-027).

Tests: `rc-game/tests/classes/cheap_classes_a.rs` (every unit resolves on exactly its census levels with the census's counts;
unit addresses distinct; one headless behaviour test per unit on one of its levels); unit tests in each module.

### In the port: cheap wins, levels 09–18 (2026-09-28, G-CLS-027 part B)

Same registry and conventions as part A (unit ids of the re-run before it). The coverage tables are in
docs/plan/class_units_levels_09_18.md and the module docs; tests `rc-game/tests/classes/cheap_classes_b.rs`.

| unit | classes (levels) | created | reference | port | notes |
|---|---|---|---|---|---|
| U408 | 212, 1412 asteroids (13) | 130 | level13 0x2e1638 | `units::asteroid` | idle spin; drift / split / respawn ported, the wake needs the ship-combat mode (G-LVL-009) |
| U303 | 1181 chain links (09) | 124 | level09 0x304360 | `units::chain_link` | the break runs down the chain and tells the platform core |
| U553 | 885…936 grouped pieces (18) | 119 | level18 0x2e9768 | `units::barricade` | a hit sets the group flying (`scheduler::group_state`) |
| U294 | 1182–1189 tethered platforms (09) | 80 | level09 0x2c26c8 | `units::tethered_platform` | bob / wobble, pieces on the core's joint, the fall into the lava, global-flag writes (`interact::set_global_flag`) |
| U417 | 1261 explosive tanks (13) | 55 | level13 0x307b10 | `units::explosive_tank` | + its fireball class 1634 (0x30c3b8) |
| U533 | 1359…1373 sliding doors (17) | 52 | level17 0x2e87d8 | `units::fleet_door` | |
| U477 | 937 cogs (15) | 48 | level15 0x2e46b0 | `units::linked_cog` | |
| U563 | 1584 carriers (18) | 38 | level18 0x2fa728 | `units::veldin_carrier` | |
| U500 | 650 floats (16) | 29 | level16 0x2d5cb8 | `units::rising_float` | |
| U479 | 1250 belts (15) | 18 | level15 0x2e73c0 | `units::quartu_belt` | |
| U499 | 647 pieces (16) | 16 | level16 0x2d59e0 | `units::extending_piece` | |
| U559 | 1432 (18) | 13 | level18 0x2f7ab0 | `units::hidden_prop` | |
| U493 | 482 (16) | 8 | level16 0x2cb600 | `units::marker::update` | `DeleteMoby(self)` |
| U484 | 1425 bubble vents (15) | 6 | level15 0x2eb928 | `units::bubble_vent` | |
| U456 | 1397 (14) | 6 | level14 0x3061d8 | `units::oltanis_switchboard` | |

**Not cheap after all**: U543 (1843, 17: 60) and U370 (1319, 11: 52), one fighter source per level, need Ratchet's
ship-combat mode (hero state 0x32 with the ship moby 0x140940): G-LVL-009. Not attempted: U474 (408: its own draw
callback) and the rest of the part B appendix (117 units / 959 instances after the re-run).

**Re-run after both halves (2026-09-28):** 548 unported class-levels, 4,526 placed / **4,030 created** instances, 417
units; verdicts: cheap 197 units / 1,308, missing 161 / 2,470, partly 40 / 204, unknown 19 / 48.

### In the port: path units (2026-09-29, W1 lane 1, G-CLS-025)

The path helpers first (`rc-game/src/path.rs`: the system-or-not table of every copy is its module doc; `pose`
`0x262e40` / `0x277d40`, `push_from_walls` `0x261d78`, `nearest_at_distance` `0x264558` / `0x2a0260` = the flyer
driver's `0x28b510`, which now calls it; `0x263710` / `0x261b48` are `ClampToPath`, `World::clamp_to_path_hit`), then
the units the helpers alone blocked, in `units::PORTS`; coverage tables in the module docs, tests
`rc-game/tests/classes/path_classes.rs` (an owned-slot sound sink, so `SoundIsAlive` is the game's check).

| unit | classes (levels) | created | reference | port | notes |
|---|---|---|---|---|---|
| U36 | 1564 path gliders (00, 07, 10, 18) | 92 | level00 0x2e3a88 | `units::path_glider` | 6 units/s on the first segment, pitch halved, flap / glide blends on the animation wrap; the data words are the same on the four levels |
| U523 | 1667–1671 air traffic (16) | 90 | level16 0x2e76b8 | `units::kalebo_traffic` | one member's init places the group (shuffled, even t); spacing by target-speed swaps; unseen vehicles jump ahead; type-22 exhaust (odd classes), loop sound (even) |
| U495 | 257 rail cars (16) | 21 | level16 0x2c3d38 | `units::rail_car` | runs when Ratchet grinds (state group 0xf) in its cuboid; loop sound 0, sound 1 within 7; hidden at the end, reset out of view |

**Not path-blocked after all**: U127 (level 03's 75, 115–120, 132, 795: 246) is a flyer-driver variant (the level-01
driver's tangent / Hermite / arc-length helpers, `classes::flyer`) whose own code is a level-03 explosion `0x24ce98`
(3,136 words; G-CLS-028), the class-235 exhaust moby `0x2bad40` (G-CLS-026), `BreakFxB` wrecks per class, `BoltBurst`
`0x24f1b8`, a skill point / banner for 795, class sounds and type-22 sparks. U572 (1906, 18: 18) calls the path
helper from its own walk code but is a creature (hit handler `0x25e9d8`, death explosion `0x260790`, react
`0x25bd28`: G-ENM-009). U326 (947, 10: 2) needs `0x277260` (left in G-CLS-025). The other former path units (827,
749, 580, 1262, 238, 294, 857, 1213 / 1212 …) wait on the cheat, the light flicker, the creature copies, the hero
states or their own draw callbacks (unit_systems.tsv).

### In the port: creature units (2026-09-29, W1 lane 2, G-ENM-009)

System or not first (creatures.md §9.1): the census's three `creature` clusters were two already-ported functions
(`C:d001715051e4` = `0x274b78`, `C:ad33de15cdab` / `C:a8d6490c966c` = `0x26de80`; re-tagged has) and the random
wander `C:9a9b8f2fffcb` `0x261630` (six levels; the census had folded it into each consumer's private code), ported as
`creature::walker::wander`; the joint hit `C:76f90a069f65` / `0x26e830` got a real port (`attack::joint_hit`). Units,
in `units::PORTS`; coverage tables in creatures.md §9.3, tests `rc-game/tests/classes/creature_classes.rs`.

| unit | classes (levels) | created | reference | port | notes |
|---|---|---|---|---|---|
| U25 | 749 horny toads (00, 18) | 106 | level00 0x2d4610 | `units::horny_toad` | wander / chase / bite (joint hit, flags 1); dies from any hit; lure, Suck Cannon (`react::VELDIN_749`), presses floor switches |
| U287 | 1023 hopping gunners (08, 09), shot 1292 | 40 | level08 0x301158, 0x307298 | `units::hop_gunner` | carried gun 1025, hop arc, 20-shot bursts, club, knockback / death, morph keep byte |

Census after (2026-09-29, `cargo run -p rc-trace -- class-census`): no unit lists `creature`; 412 units / 3,681 created
instances unported (was 417 / 4,030 before this wave's two lanes); verdicts cheap 219 / 1,498, missing 145 / 1,884,
partly 42 / 291, unknown 6 / 8. Freed by this row (only the cheat left, conditional): 340 (04: 28), 333 (08: 8), 217
(04: 5), 578 (03: 3).

### In the port: W2 lane 2, help directors and quick wins (2026-09-29, level_scripting.md §8)

| unit (this run) | classes (levels) | port |
|---|---|---|
| U183 | 838 laser fences (05; 26) | `units/laser_fence.rs` + `Callback::UnitQuads` (G-AUD-010) |
| U32, U119, U145, U165, U204, U232, U292, U305, U341, U391, U419 | the help directors 1413 (00), 1324 (02), 1342 (03), 1343 (04), 1347 (05), 1348 (06), 1349 (08), 1000 (09), 1344 (10), 422 (12), 558 (13) | `units/help_*.rs`, `units/hints.rs` |

Re-tags (census_systems.tsv): `C:bcf4ccdaa994` (L04 `0x27eca0` = `PlayLevelSoundAtMoby`) has; `L01:0021e340`
`DrawSpriteHelper_A` has (VU1 set-up only); `X:855` has (the hand-off store is ported); `L01:00278020` has
(`HeightGrid::height`). Census before → after this lane (both runs include the other lanes' ports of the same
moment): 412 units / 3,681 created (cheap 219 / 1,498, partly 42 / 291, missing 145 / 1,884) → **397 / 3,454**
(cheap 217 / 1,532, partly 33 / 233, missing 141 / 1,681). Newly cheap: the 466 family (U154, 38), 855 (U189, 7),
the four sprite-callback units (U379, U384, U488, U532: 16). Found while porting: the save-only story classes also
write items, bolts and health directly (G-CLS-029), 558 writes the hero's airless flag 0x14161b (G-HERO-032).

### In the port: W2 lane 1, the ready enemy units (2026-09-29, creatures.md §10)

| unit (this run's id) | classes (levels: created) | reference update | port |
|---|---|---|---|
| U553 | 568 rolling mines (18: 20; a pool of the boss 1422) | level18 0x2d5918 | `units::rolling_mine` |
| U301 | 193 pack biters (09: 49, 15: 44) | level09 0x2e27d8 | `units::pack_biter` |
| U268 | 252 hover zappers (08, 14: 77), and their two draw callbacks | level08 0x2d2af0, 0x2d4108, 0x2d3878 | `units::hover_zapper` |
| U407 | 63 flying biters (13: 58) | level13 0x2b50d8 | `units::flying_biter` |
| U300 | 52 buzz bombs (09, 16: 46) | level09 0x2c5990 | `units::buzz_bomb` |
| U521 | 1445 area stalkers (16: 24) | level16 0x2e5e08 | `units::area_stalker` |
| U426 | 1271 the wave gate (13: 1) | level13 0x30af50 | `units::wave_gate` |

Census re-run after them (2026-09-29, with W2 lane 2's ports of the same day): **512** unported class-levels, 3,602
placed / **3,325 created** instances in **393 units** (verdicts cheap 217 / 1,532, missing 138 / 1,553, partly 32 /
232, unknown 6 / 8); these seven units took 319 created instances. Left of the lane's list: 1202 (57), 1199 (15),
1196 (7) on level 10 and 29 (9) on level 13 — 29 is three classes (the turret, its rider 36, its shot 1238: the last
two created by code, so outside the census). Findings for the census itself: `0x294cb0` / `0x265b38` is ported now
(`path::toward`); level13 `0x280400` is a type-68 spawner, not `PartType44Spawn` (G-PRT-007; U407 was "has
particles"); 568 is "cheap" but inert without 1422.

### In the port: G-HERO-002's consumers, and what it unblocked (2026-09-29, hero_states.md "Scripted control")

The hero states 0x1f / 0x32 / 0x78 are ported (`hero/scripted.rs`); the tags `G:001413d4=00000032` and the level
`SetState` copies (`C:8a18636e69bd`, `C:7b3e1e804b94`, `C:ea215dba76e7`, `C:2f2e9be4c73e`, `C:8cdb2fd899d9`,
`C:ec679874da9e`, `C:b73827d476e2`, `C:84d6abe16fc6`) are **has** (every state a class passes them is ported: 0, 6,
0x1e, 0x32, 0x3d, 0x72, 0x77, 0x78, 100, found by grepping every level's class code for its `SetState` copy); level 05 /
16's copies stay partly (the Hoverboard's 0x6b, G-HERO-008), the per-body idles partly (G-HERO-005).

| unit (this run's id) | classes (levels: created) | reference update | port |
|---|---|---|---|
| U216 | 1039 kill cuboids (06, 08, 13, 18: 4) | level06 0x2f7930 | `units::kill_volume` |
| U274 | 438 Batalia's circling fighters (08: 25) | level08 0x2de848 (+ trail 0x2dec90) | `units::batalia_fighter` |

**Now cheap for a class job (herostate cleared, census 2026-09-29):** 440 the Batalia turret (08: 1; the one portable
setter of 0x32, 1,606 words, entered by walking in once its mission byte is done, else from the Water Worker 1283,
which needs saves), 587 (18: 28), 336 (12: 11), 1150 / 1151 (09: 5), 1178 (11: 4), 1772 (17: 2; its 0x32 branch also
reads the ship moby: G-LVL-009), 263–267 (09: 1), 361 (11: 1), 388 (13: 1); partly: 347 (17: 8, anim). Cleared of
herostate but blocked elsewhere: the fighters 1843 / 1319 (the `shipmode` override), 1059 / 294 / 1231 (cheat),
the 0x32 setters 1201 / 1267 / 1242 / 69 / 1379 and 1380, 713, 1083, 21 / 244, 1270 (the type-5 script camera
`C:3b375f0ab07b`), 1106 (the Umbris boss, 0x78's setter: cheat, ptype, shadow).

Census re-run after them: `herostate` **66 → 34 units, 93 → 52 class-levels, 786 → 487 created** (the rest: the
bodies 1 / 2 / 3, the per-body idles, SwitchCharacter and leaving a body, G-HERO-005; the Hoverboard's 0x6b).

### In the port: W3, the remaining ready enemy units (2026-09-29, creatures.md §11)

| unit (this run's id) | classes (levels: created) | reference update | port |
|---|---|---|---|
| U335 | 1196 Orxon's path scouts (10: 7) | level10 0x2df270 | `units::orxon_flyers` (`update_1196`) |
| U336 | 1199 Orxon's swoop flyers (10: 15 of 51 placed) | level10 0x2e01a8 | `units::orxon_flyers` (`update_1199`) |
| U337 | 1202 Orxon's brawlers (10: 57 of 65 placed) | level10 0x2e1d38 | `units::orxon_brawler` |
| U407 / U408 / U424 | 29 Gemlik's gun turrets (13: 9), their rider 36 and shot 1238 (created by code: 0 placed) | level13 0x2b41b8, 0x2b4c80, 0x306300 | `units::gemlik_turret` |

Tag fixes: `C:dc2339668382` (the next waypoint toward a target, L09 / L13 / L15) is **has** (`path::toward`); level13
`0x280400` has its own row `C:29928a61391b`: a **type-68** spawner (`particles::type68`, G-PRT-007 closed the same day), not `PartType44Spawn` (its code shares
nothing with L01 0x286450; the old "has particles" of U407 came from the CreatePart copy it calls). New rows: level13
`0x26a498` = `SpawnBeamExplosion` (`C:d7574994a069`, has), `0x265af0` the hit-record writer (`C:c294ae52ce36`, has),
the type-51 spawner `C:70fa5037a649` (L07 / L13 / L14, has: `particles::type51`), the scorch `C:98f51a1cea64` (L10 / L12 /
L13, has: `explosive_tank::scorch`).

Census re-run after them (2026-09-29, with the day's other ports): **480** unported class-levels, 3,188 placed /
**2,955 created** instances in **375 units** (verdicts cheap 214 / 1,333, missing 134 / 1,465, partly 21 / 149,
unknown 6 / 8); these four units took 88 created instances (4 class-levels, plus the two created-by-code classes the
census now lists as ported rows). The lane's list is done: nothing of G-ENM-010's "ready" list is left.

### In the port: cheap wins round 2 (2026-09-29, G-CLS-027)

Taken in the census's order by created instances (unit ids of the 2026-09-29 13:49 run; the re-run renumbers them),
all levels. Each unit is a row (or rows) of `units::PORTS`; the coverage table of every ported function (each call,
branch and side effect, with its port or its gap) is the module doc of the port; tests
`rc-game/tests/classes/cheap_classes_c.rs` (every unit resolves on exactly its census levels with the census's created
counts, and runs headless on one of them covering its side-effect rows) and the unit tests in each module.

| unit | classes (levels: created) | reference | port | notes |
|---|---|---|---|---|
| U480 | 408 alarms (15, 17: 48) | level15 0x2cb4c8 | `units::quartu_alarm` | state machine, pulsing light, the level's alarm word, fade, loop sound 5, drone release; the family call `set_off` (0x2cbac0) is made by the guards 44 (unported); the full-screen tint's colour is ported (`tint`), its draw is **G-REN-030** |
| U470 | 77 alarm drones (15, 17: 36) | level15 0x2a2488 | `units::quartu_drone` | released by 408 (`release` 0x2a2868), rise, home, explode (death explosion + sphere hit 0x10001), type-60 sparks, the group glow (`Callback::UnitQuads`, 0x2a29b8) |
| U474 | 123 swinging lasers (15, 17: 46) | level15 0x2a6e68 | `units::swing_laser` | circle / slide / pendulum paths; the beam: a template line hit (0x10001), a world line, 40 type-60 sparks a tick, a camera-facing quad (0x2a7628) |
| U411 | 127, 128, 159, 169 rotators (13: 24) | level13 0x2c7f38 | `units::linked_rotator` | the turning twin of `linked_mover` (U247): same pvar layout, `linked_mover::link_state` shared |
| U215 | 1038 orb holders (06, 10, 17: 21) + the orb 1040 they create | level06 0x2f7288 / 0x2f7ab8 | `units::orb_holder` | the glow is the gold bolts' item glow with other data (`gold_bolt::glow_init_with` / `item_glow_with`, third consumer); death bits both ways |
| U503 / U502 / U514 | 552 barrier posts (16: 20), 546 switches (6), 1387 walls (4) | level16 0x2cf4a8 / 0x2cf198 / 0x2e36e8 | `units::kalebo_barrier` | one family: posts pair by height, beams (`Callback::UnitQuads`), loop 0; switches turn the wall's collision, re-arm |
| U185 | 843 sliding blocks (05: 16) | level05 0x30e508 | `units::cuboid_slider` | placed by a cuboid; `turn::spring` |
| U473 | 93 swing doors (15: 12) | level15 0x2a3ba8 | `units::swing_door` | cuboids for Ratchet, the camera and the run list's targetable mobys (the run list rebuilt per door [L]) |
| U307 | 1172 chain anchors (09: 10) | level09 0x303d10 | `units::chain_anchor` | the chain end of `chain_link` (U303); resolver, flash, `SetDeathBits`, the break passed on, beam explosion |
| U477 | 196, 197, 1958 sliding doors (15, 17: 10) | level15 0x2bddb0 | `units::slide_door` | |

**Not cheap after all** (a system the census does not see): U171 (133, 05: 20) adds `ticks(60)` to the hoverboard
boost timer 0x13fc14 when Ratchet hits it (G-HERO-008 / G-LVL-007); U510 (933, 16: 25) reads the hero capsule's contact
moby 0x13f58c (G-HERO-033) and resolves damage on a stack record; U281 (468 / 469, 08: 10) ramps its loop's volume
(`SoundSetVolume` 0x2a1968, tagged has L, but no `SoundSink` method: G-AUD-011); U326 (939, 10: 11) launches the
class 938 it creates (0x2d85c8: its bounce 0x2d8170 and the engine 0x24d508, outside the census: G-CLS-001);
1143 (16: 10) and 823 (05, 07: 8) attach a manipulator to a class joint list (the list target is not loaded for class
mobys: G-HERO-009); 1378 (10, 17: 6) writes the airless flag 0x14161b (G-HERO-032). **Census false positives**: U127
(03: 246, the flyer variant: G-CLS-028 / G-CLS-026, see the path units above) and U40 (class 27, 01: 10, run as an
external). **Deferred for size** (reached, not attempted): U154 (the 466 family, 04: 38, 3,032 words), U325 (857, 10:
31, 4,300 words), U499 (471, 16: 37: its own scrolling draw and a voice handoff), U561 (1355, 18: 36: two draw
callbacks), U535 (99, 17: 35), 1210 (03: `CarryRiders` with a rotation change, not ported), U488 / U532 (the
sprite-callback units: their draws are environment meshes, level15 0x2eac90 …). Reached down the list to the units of 10 instances; the rest of the appendix is open.

Tools: `rc-trace overlay-diff` now prints the counterpart address of identical cells too (`= addr`), which gives a
level-address → level-01 map for any list of functions (used here to name every call of a unit's decomp).

Census before → after (both `cargo run -p rc-trace -- class-census`; the "after" run includes the other lanes' ports
of the same afternoon): 393 units / 3,325 created (cheap 217 / 1,532) → **375 units / 2,955 created** (cheap 223 /
1,365, missing 125 / 1,432, partly 20 / 147, unknown 7 / 11). This round: 13 unit rows, 253 created instances.

### In the port: cheap wins round 3 (2026-10-01, G-CLS-027)

Taken in the census's order by created instances (unit ids of the 2026-09-30 run), skipping the other lanes' units
(platforms and carry, flicker, crates, effect spawners, manipulators, the camera) and the deferred big ones. The
coverage table of every ported function is the module doc of its port; tests `rc-game/tests/classes/cheap_classes_d.rs`
(the resolution on exactly the levels whose table runs the code, with the created counts, and one behaviour test per
unit on its side effects; the turret headless on level 14, the rest on synthetic tables).

| unit | classes (levels: created) | reference | port | notes |
|---|---|---|---|---|
| U440 | 30 pop-up turrets (14: 27) + their shot 681 | level14 0x2b3bf0 / 0x2ece00 | `units::popup_turret` | the hit handler 0x2b4340 (resolver, flash, `SetDeathBits`), rise / wake / sweep / fire, beam explosion; the shot's line test passes class 0xfc, impact sparks, sound 0, type-51 fizzle |
| U212 (+ U544) | 1021 petal doors (06: 11; 17: 2, 18: 1 run the same code and constants) | level06 0x2f4f00 | `units::petal_door` | creates its seven petals (the same class); `turn::turn_toward`; sound 0 |
| U203 | 1139 hoverboard sparkles (05, 16: 12) | level05 0x31abe0 | `units::board_sparkle` | three type-67 sparks a tick in view on the board (movement group 0x16) |
| U390 | 339 animated idlers (12: 9) | level12 0x2ec1d0 | `units::anim_idler` | blends only |
| U329 | 1015 / 1282 trip blocks (10: 9) | level10 0x2d90a8 | `units::trip_block` | the box test 0x2d9000; its global flag `0x13d3d4[i]` |
| U162 | 1101 / 1102 / 1531 / 1532 switched movers (04: 8) | level04 0x2e17d8 | `units::switched_mover` | death bits by hand; 1101's sounds (`PlayClassSoundByClass`) |
| U248 | 1013 / 1014 / 1064 / 1065 panels (07: 6) | level07 0x30cf90 | `units::linked_slider` | the map zone flags `0x184528[i]` (`MapState::set_zone_flag`), as `linked_mover` |
| U487 (part 2) | 1209 pressure pads (15: 9, 17: 6) | level15 0x2e5958 | `units::pressure_pad` | the group's leader clock (sounds 0 tick / 1 solved / 2 reset / 3 press), ambient green / white blink, group states 3 / 4, the collected byte and death bits |
| U211 (part 2) | 911 flame jets (06: 14) | level06 0x2f3ad8 | `units::flame_jet` | sweeping or timed; loop sound 0 (flags 4), two type-2 puffs a tick near Ratchet, hit lines (damage 1, 0x10001; 0x10000 with head item 6) |
| U542 (part 2) | 669 underwater laser spinners (17: 15) | level17 0x2d77f0 / draw 0x2d7cf0 | `units::water_laser` | only while Ratchet is in the water (`help::hero_in_water`); 3 × 5 hit lines (flags 9, 0x10001), type-60 particles on the fan edges, six FX 0x28 additive quads (`Callback::UnitQuads`) |
| U349 (part 2) | 1544 particle vents (10: 14) | level10 0x2ea1f0 | `units::orxon_vent` | kinds puffs / drips / column at the vent or each spline point; drips write the ground z into the spline's point w (as the game); type-2 blobs only |

**Not cheap after all** (reached, not ported): U560 (587, 18: 28: the platform carry `0x2755f8` and the stood-on moby
0x13f64c, then `0x227dd0(0x77, 1)` a hero state: G-CLS-024 lane / hero), U459 (1224, 14: 24) and U394 (1259, 12: 18)
(electric arcs: each builds its bolt in a large pvar block, 0x624 / 0x2300 bytes, and draws it in its own GS callback,
level14 0x301fa8, level12 0x302c58: census `fxdraw`, not tagged on these), U416 (224 / 228, 13: 16: the ship pickups add
to the ship's ammo and shield words 0x140946 / 0x14094c and test hero state 0x32 against the ship moby 0x140940:
G-LVL-009), U253 (1069, 07: 16) and U125 (868 / 905 / 928, 03: 12) (`CarryRiders` 0x2755f8: the platform lane).
Already known: U127 (G-CLS-028), U513 (933, G-HERO-033), U171 (133, G-HERO-008), U344 (1378, G-HERO-032), U491 / U535
(sprite callbacks). **Open cheap units next** (not reached this round): U487 (1209, 15 / 17: 15, pressure pads: group
state, the collected byte and death bits), U540 (669, 17: 15, has a draw callback), U211 (911, 06: 14), U347 (1544, 10:
14), U518 (1401, 16: 14), U223 (1066, 06: 6), U311 (1206, 09: 9), U374 (1248, 11: 6).

Census before → after (both `cargo run -p rc-trace -- class-census`; the after run includes the other lanes' ports of
the same session): 480 unported class-levels / 3,188 placed / 2,955 created, 375 units, 224 cheap → **455 / 3,040 /
2,807, 361 units** (cheap 227 / 1,388: the other lanes' re-tags moved units into the cheap list). This round: 9 unit
rows (8 census units), 85 created instances. **Part 2** (2026-10-01, tests `cheap_classes_e.rs`, the level runs headless on 06 / 10 / 15 / 17): the four rows
above, 58 created instances (census 450 / 2,963 / 2,730, 357 units → 447 / 2,933 / 2,700, 354 units; 235 cheap / 1,344
left). Not reached: U129 (Kerwan's air traffic 75 / 115–120 / 132 / 795, 246: now cheap, the largest; its driver is a
variant of the flyer driver), U172 (79, 05: 30; Ghidra has no function at 0x2d7140, disassembly only), U562 (587, 18:
28: the carry is ported now and the death fall is `HeroCall::SetState(0x77)`; `fun_0022da68(1, 0, m)` still to
identify), U127 (868 / 905 / 928, 03: 12), U255 (1069, 07: 16), U520 (1401, 16: 14).

**Round 4** (2026-10-01, tests `cheap_classes_f.rs`, headless on 03): U128 (census 2026-10-01 id; was U129) Kerwan's
air traffic 75 / 115–120 / 132 / 795 (246) with its exhaust trail 235 (`units::air_traffic`, level 03's own copy of the
flyer driver; closes G-CLS-028; 795's skill point stays G-SAV-007) and U126 868 / 905 / 928 (12, `units::kerwan_mover`).
Census 447 / 2,933 / 2,700, 354 units → 434 / 2,549 / 2,316, 351 units (with the other lanes' ports of the day; cheap
233 / 1,086 left). Skipped: 1069 (07: the rocking float writes Ratchet's position 0x13f3d0 and platform delta 0x13f440
while he rides it (`0x288968`): a hero-side seam, `hero/**` was off-limits); 587 (18: `fun_0022da68` is
`PlayClassSound` (level01 0x2a1618, `World::play_sound`); its difficulty word 0x1623a8 is written only by the boss 1422's
`0x2f5e18` (unported: 0 until then); not reached for time), 1401 (16: the class-933 spawner `0x2de298` and knock-back;
not reached).

### In the port: cheap wins round 5 (2026-10-01, G-CLS-027)

Taken from the census of 2026-10-01 08:45 (unit ids of that run; the verdict **cheap**, largest first), skipping the
other lanes' units (the camera classes, spawners and group commands, Pokitaru's boats, the creatures 574 / 452) and
1069 / 587 (the brief). The coverage table of every ported function is the module doc of its port; nothing was tested
in this round (the test request: `docs/test-requests/2026-10-01-cheap-wins-5.md`). 25 census units / 248 created
instances, 27 `units::PORTS` rows (plus two draw-only rows).

| unit | classes (levels: created) | reference | port | notes |
|---|---|---|---|---|
| U506 | 471 reversing belts (16: 37) | level16 0x2c9878 | `units::kalebo_belt` | the group's belt push (`CarryRiders` on each member), Ratchet in cuboid +0x64 / +0x68 reverses it; the group voice (handoff); draw 0x2c9cd0: two quad layers (FX 0x13 / 0x10), the second scrolling with the speed (two `UnitQuads` rows) |
| U541 | 99 sliding laser emitters (17: 35) | level17 0x2a8da0 | `units::fleet_laser` | shuttle between home and a cuboid / linked moby; beam hit (0x10001, sideways push), 15 type-79 sparks a tick (`World::part79`, the spawner's first consumer), beam quad 0x2a95b8 |
| U485 | 221 shaking critters (15: 24) | level15 0x2c2938 | `units::quartu_critter` | NPC look-at record, group-busy test, type-21 dust, death: sound, explosion, bolts, three pieces |
| U567 | 1355 divers (18: 36) | level18 0x2efb88 | `units::veldin_diver` | released only by the boss 1422 (`veldin_diver::activate` = 0x2f0030; 1422 unported: inert until then); trails + glows (two callbacks) |
| U162 | 617 path riders (04: 4) | level04 0x2d6f68 | `units::eudora_path_rider` | |
| U180 | 810 rocking floats (05: 4) | level05 0x30bdd8 | `units::rilgar_rocker` | carries its riders through the rock |
| U195 | 895 flaps (05: 4) | level05 0x3166a0 | `units::rilgar_flap` | |
| U283 | 467 / 472 linked lifts (08: 4) | level08 0x2ea398 | `units::batalia_lift` | global flag 0x35 |
| U436 | 1577 watchers (13: 4) | level13 0x30be60 | `units::gemlik_watch` | |
| U158 | 484 (04: 3) | level04 0x2ce060 | `units::marker::update` | `DeleteMoby(self)` |
| U313 | 1206 hazard columns (09: 9) | level09 0x305a28 | `units::gaspar_hazard` | sphere hits along z |
| U60 | 695 floating pushables (01: 9) | level01 0x2f8268 | `units::floating_pushable` | `walker::move_ground`, the water height |
| U256 | 1080 sinking floats (07: 8) | level07 0x311bc8 | `units::umbris_sinker` | `HeroOnMoby` |
| U505 | 470 glowing beacons (16: 8) | level16 0x2c9480 | `units::kalebo_glow` | the gold bolts' item glow, fourth consumer |
| U341 | 1240 spark fountains (10: 4) | level10 0x2e5be0 | `units::orxon_sparks` | type 64 |
| U263 | 104 / 106 / 1129 linked platforms (07: 1, 13: 4) | level07 0x31aee0 | `units::linked_mover::platform_update` | **the same state machine as U247 at another pvar base**: `linked_mover::run(w, id, base)` now takes the base |
| U222 | 1054 split doors (06: 9) + the half 1055 | level06 0x2fbfb0 | `units::blarg_doors` | group state 3 (`scheduler::group_state`) |
| U88 | 1504 wandering lights (01: 1, 06: 4) | level01 0x30b618 | `units::wandering_light` | a dynamic point light (`point_lights`) |
| U376 | 1248 rising gates (11: 6) + halves 1247 | level11 0x316320 | `units::pokitaru_gate` | |
| U194 | 893 hinged hatches (05: 4) | level05 0x316258 | `units::rilgar_hatch` | manipulator angle (`manip::set_axis`) |
| U191 | 855 water spouts (05: 7) | level05 0x3150f0 | `units::rilgar_spout` | type-50 droplets, type-46 rings at the patch height, the group voice (G-AUD-010's last consumer) |
| U438 | 1805 breakable props (13: 8) | level13 0x30cdb8 | `units::gemlik_breakable` | the skill point of the last of the group: G-SAV-007 |
| U252 | 1041 lobbing turrets (07: 8) + their shot 882 | level07 0x30d4d8, 0x30bbc8 | `units::umbris_lobber` | the shot's update is in the class table but the census lists it only as created (0 placed) |
| U118 | 1212 path ships (02: 6, 09: 3) | level02 0x2ec4b8 | `units::path_ship` | **shared**: the engine-trail blob (`units::engine_trail`, now also `batalia_fighter`'s); the skill points: G-SAV-007 |
| U155 | 434 crank lifts (04: 3) | level04 0x2c6bb8 | `units::eudora_crank_lift` | posed by a bolt crank 280; sinks back unwinding it (writes the crank's progress and angle) |

**Plug-and-play found this round**: `units::engine_trail::Blob` (one row per copy of the path flyers' exhaust blob:
level08 0x2dec90, level02 0x2eca30); `linked_mover::run` at any pvar base (the U247 movers and the U263 platforms);
`World::part79` (level 17's spark spawner 0x26fb60); the gold-bolt item glow and the group voice handoff gained their
fourth / third consumers with no new code.

**Not cheap after all** (reached, not ported): 1066 (U225, 06: 6: wakes dormant creatures of its group, 0x2e9d10: a
spawner, the spawn lane's), 899 (U140, 03: 4: spawns path movers 898 along a path, `0x2db198`; spawner),
1885 (U319, 09 / 13: 8: lobs creatures, `0x30b218`; spawner), 1401 (U521, 16: 14: spawner), 1906 (U579, 18: 18: a
creature woken by the boss 1422), 432 / 1052 (U154, 04: 4: they swap their own class at run time, `+0xa6` / `+0x22` /
the class header: no class-swap service in the port: G-CLS-031), 434 (U155, 04: 3: drives the bolt crank 280's pvars
back: ported after all, see the table), 664 (U307, 09: 6) and 1293 /
1320 (U316, 09: 6) (write Ratchet's platform delta 0x13f440 and z 0x13f448 while he rides them, like 1069: G-HERO-034),
643 (U451, 14: 5: reads the hero's contact moby 0x13f58c: G-HERO-033), 111 (U415, 13: 8: the ship moby 0x140940 and
hero state 0x32: G-LVL-009), 79 (U172, 05: 30: no Ghidra function at 0x2d7140, a disassembly-only port deferred for
size), 466… (U156, 04: 38: its path driver 0x2cb8b8 has no Ghidra function, ~1,000 instructions), 857 (U328 / U212, 10 /
06: 35: 4,300 words, Ratchet's state and the difficulty), class 8 (U440, 14: 11: 2,198 words, deferred for size).

Census before → after (both `cargo run -p rc-trace -- class-census`; the after run includes the other lanes' ports
of the same morning): 430 unported class-levels / 2,405 placed / 2,205 created, 347 units → **399 / 2,131 / 1,933, 320
units**.

### In the port: class swap and spawners (2026-10-01, G-CLS-031, G-CLS-027)

The round-5 units left for a class-swap service and the spawn lane. The coverage table of every ported function is the
module doc of its port; nothing was tested in this job (the test request: `docs/test-requests/2026-10-01-classswap-spawners.md`).
6 census units / 36 placed instances, 8 `units::PORTS` rows (the created classes 898, 1886 have their own rows).

| unit | classes (levels: placed) | reference | port | notes |
|---|---|---|---|---|
| U154 | 432 / 1052 crank followers (04: 4) | level04 0x2c6858 | `units::eudora_crank_follower` | posed by a bolt crank 280; **the class-swap service** `moby_update::class_swap` (432 → 1052 at the init, back to 432 when the crank is wound); the renderer draws a placed moby by its live class |
| U225 | 1066 creature wakers (06: 6) | level06 0x2fda30 (the wake 0x2e9d10) | `units::blarg_waker` | wakes the dormant (state 0xf) 827s of its group one every 20 ticks; 827 is not ported (U211), so none is ever dormant in the port |
| U140 | 899 path-mover lines (03: 4) + the movers 898 | level03 0x2db280, 0x2db198, 0x2db020 | `units::kerwan_path_spawner` | platforms 2.9 apart along the path at 3 u/s, `CarryRiders`; the spawner loops 898's sound 0 |
| U319 | 1885 pod launchers (09: 4, 13: 4) + the pods 1886 | level09 0x30ab80, 0x30b3b0, 0x30a778 (level13 0x30d550 / 0x30dd80: the same code) | `units::pod_launcher` | health, flash, death blast and pieces 1733–1735; the pods bounce and hatch a revived dead member of the group (193 on 09, 63 on 13: both ported); the target search's cuboid list (`target::acquire_with`) |
| U521 | 1401 mine drones (16: 14) | level16 0x2e37a0 | `units::kalebo_mine_drone` | waits in its cuboid, flies path A carrying a new race mine 933 (`0x2de298`), drops it (its state 1), flies back along path B; exhaust glows (type 23); the mine's own update (933, U517) is not ported |

**System or not**: the class swap is a service now (`class_swap::swap` + its `PORTS` registry); the four "spawners" share
no helper (each creates or wakes its own family's mobys with its own code), so there is no spawner system to port: a
spawner is a class port that calls `World::create_moby` / the group walk. Left from the round-5 list: 1906 (U579, a
creature woken by the boss 1422).

### In the port: W3 lane 2, moby services (2026-09-30, triggers.md §5b, creatures.md §12)

The lane's four systems: G-REN-024 and G-CLS-026 were ported engine functions under other hashes (the hit flash,
`BreakFxB`: two padding words after `jr ra` change the cluster hash; the masked overlay diff finds them identical),
G-CLS-009 is dead code (no class table has 503–510), G-CLS-024 is ported (`triggers.rs`' moby side). Tests: the
module tests of `triggers`, `bolt`, `crate_`, `infobot`, `units::carriers`, `units::falling_platform`,
`units::light_fixture`.

| unit (2026-09-29 ids) | classes (levels: created) | reference | port | notes |
|---|---|---|---|---|
| U102 | 707 / 734 turntables (02: 4) | level02 0x2ddc00 / 0x2df8c0 | `units::carriers::turntable` | 5°/s, `CarryRiders` with the turn |
| U126 | 1210 joint-carried platform (03: 5) | level03 0x2953f8 | `units::carriers::joint_platform` | the class never leaves state 0 |
| U179 | 812 pinned platforms (05: 25) | level05 0x30bf98 | `units::carriers::pinned_platform` | pinned to the carrier +0x80 (`to_local` / `from_local`) |
| U565 | 1381 falling platforms (18: 8) | level18 0x2f16f0 | `units::falling_platform` | its carriers 1584 as children (`record_children` / `place_children`); the boss 1422 starts it |
| U207, U472 | 1511 light fixtures (05: 26, 14: 22) | level05 0x31c8e0, level14 0x307ad8 | `units::light_fixture` | `BreakFxB` pieces with a velocity; glow quad (FX 11) |

Freed for the class ports (G-CLS-027 round 3; census 2026-09-30 ids): cheap 668 (U101, 02: 7, the nests of 580),
79 (U172, 05: 30), 587 (U562, 18: 28), 1069 (U255, 07: 16), 868 / 905 / 928 (U127, 03: 12), 1885, 1041, 1805,
455, 625, 1454; conditional on the cheat only: 1246 (U375, 11: 126), 580 (U95, 02: 82), 612 (U96, 02: 15), 1231
(U373, 11: 7), 574, 452, 623, 541. Census before → after (the after run includes the other lanes' ports of the same
session): 2,955 → 2,730 created unported; the systems `light` (256), `platform` (181) and `fxmoby` (276) block
nothing now; this lane ported 90 created instances (6 unit rows).

### In the port: W3 lane 1, the manipulators (2026-09-30, moby_animation.md §9, hero_gameplay.md §19)

The shared manipulator records (`moby_update::manip`: `AttachManipulator` / `DetachManipulator` on class mobys with the
loaded list targets, `FUN_00221e38`, the NPC look-at `0x2777d8`) and their consumers: U510 / U514 1143 (every level's
vendor hologram and Kalebo's 10 logos, `units::hologram_logo`), U180 823 (8, `units::sweep_light`, with its beam
callback), U155 481 (4, `units::spinner_float`), the vendor 11 and the talking NPC 774 (already ported classes: their
manipulators), the Visibomb 172's fins. Census after (with the other lanes' ports of the day): 2,714 unported created
(was 2,955); `anim` 4 units / 65 created (was 42 / 422). Freed: 347 (8) and 447 / 920 (3) are cheap now (not ported
here for size: 347 is 1,138 words, 447 / 920 two functions of 388 words with a talk and scene family); the other `0x2777d8` callers
wait on the cheat (G-SAV-006), the effect mobys or the save (G-SAV-002).


### In the port: W3 creature units, the classes freed by lane 2 (2026-10-01, creatures.md §13)

The creature units whose only blocker left was the big-head cheat (G-SAV-006, conditional: each ports its normal
path and lists the cheat branch as NOT ported). Tests: `crates/rc-game/tests/classes/creatures_w3.rs`.

| unit (2026-10-01 ids) | classes (levels: created) | reference | port | notes |
|---|---|---|---|---|
| U375 | 1246 biters (11: 126) | level11 0x314318 | `units::pokitaru_biter` | beach / swimmer / boat boarders; the boats 1075 (U363) are not ported: the 30 boarders stay hidden |
| U95, U101 | 580 sand sharks (02: 82), their nests 668 (02: 7) | level02 0x2d3e50, 0x2dcb38 | `units::aridia_sandshark` | the nests relaunch killed sharks; its own reaction table (`react::ARIDIA_580`) |
| U96 | 612 flame-throwing sentries (02: 15) | level02 0x2d7748 | `units::aridia_flamer` | its own type-12 flame emitter (level02 0x264e70) and the kept flames' hits |
| U373, U378 | 1231 ball throwers (11: 7), their ball 1297 (created) | level11 0x310180, 0x318b30 | `units::pokitaru_thrower` | the ball's glow (type 59), trail (type 2), blast (damage 2) |

Census before → after (both `cargo run -p rc-trace -- class-census`; the after run includes the other lanes' ports
of the day): 447 unported class-levels / 2,700 created → 430 / 2,205; the `cheat` system 37 units / 965 created →
33 / 735 (these four units: 237 created instances). Not taken (time): 574 (U130, 03: 17) and 452 (U280, 08: 3).

## Unique classes: the ones that need something no other class needs

A blocking function (missing or partly) that exactly one unit calls:

| unit | classes | levels | created | its own blocker [system] |
|---|---|---|---|---|
| U209 | 827 | 06,10 | 155 | `G:001413d4=00000051` hero_state_0x51 [herostate] |
| U549 | 1843 | 17 | 60 | `X:1843` fighter_1843 [shipmode] |
| U374 | 1319 | 11 | 52 | `X:1319` fighter_1319 [shipmode] |
| U154 | 466,480,485,486,488,490,493,494,495,498,555 | 04 | 38 | `C:bcf4ccdaa994` FUN_0027eca0 [lsound] |
| U183 | 838 | 05 | 26 | `X:838` voice_handoff_838 [voicehand] |
| U394 | 1269 | 12 | 20 | `L01:0026eec8` FUN_0026eec8 [shadow] |
| U96 | 615 | 02,04,06,08,11,13,18 | 19 | `C:0e46409356dd` FUN_0022c418 [herostate] |
| U422 | 1262 | 13 | 16 | `C:7c53e29c001b` FUN_00261b48 [path] |
| U95 | 612 | 02 | 15 | `C:a8d6490c966c` FUN_0025b710 [creature] |
| U203 | 1139 | 05,16 | 12 | `L01:00289850` PartType67Spawn [ptype] |
| U157 | 563 | 04 | 10 | `L01:002b52a8` FUN_002b52a8 [anim] |
| U189 | 855 | 05 | 7 | `X:855` voice_handoff_855 [voicehand] |
| U411 | 170 | 13 | 5 | `L01:0027e750` PartType05Spawn [ptype] |
| U396 | 1281 | 12 | 4 | `L01:00282ef0` PartType28Spawn [ptype] |
| U322 | 702 | 10 | 3 | `L01:00284d88` PartType40Spawn [ptype] |
| U134 | 816 | 03 | 3 | `L01:00285768` PartType41Spawn [ptype] |
| U174 | 439 | 05,16 | 2 | `C:4d615d92f5c8` FUN_0021e398 [herostate], `C:ef4e6bb0b47f` FUN_0024cee8 [herostate] |
| U326 | 947 | 10 | 2 | `L01:00277260` FUN_00277260 [path] |
| U319 | 22 | 10 | 1 | `C:12bb06e65f13` FUN_00204110 [herostate] |
| U220 | 1061 | 06 | 1 | `C:754a80d04806` FUN_00227cf8 [herostate], `L01:00231348` FUN_00231348 [herostate] |
| U393 | 1267 | 12 | 1 | `C:84d6abe16fc6` FUN_002400d0 [herostate] |
| U254 | 1106 | 07 | 1 | `C:dd572bc43542` FUN_0029b8d8 [ptype], `L01:0020cf28` FUN_0020cf28 [math], `L01:00287e70` PartType58Spawn [ptype] |
| U570 | 1750 | 18 | 1 | `L01:00260e80` memcard_MakeWholeSave [save] |
| U370 | 1242 | 11 | 1 | `L01:0028a7a8` PartType74Spawn [ptype] |
| U426 | 1353 | 13 | 1 | `L01:002a2360` FUN_002a2360 [cine] |
| U564 | 1422 | 18 | 1 | `L01:002ad558` EnterSlideshowMode [cine] |
| U218 | 1051 | 06 | 1 | `L01:003137b0` FUN_003137b0 [camera] |

Besides these, the unit-private code of the big units is by definition unique: U118 (level 03's nine 75..132 / 795
classes, 246 instances, 1,820 words behind one update `0x29dba8`), U198 (827, 2,150 words), U359 (1246, 1,924 words),
U15 (749, 1,736 words) and U289 (193, 1,636 words) are the largest.

## Classes the census could not resolve

6 units / 8 instances have no missing function but call an untagged one (the function was not read; all rare). 14
more units already have a known blocker and also call an untagged function (listed in the appendix as "untagged ×n").

| unit | classes | levels | created | verdict | untagged shared functions |
|---|---|---|---|---|---|
| U144 | 1012 | 03 | 2 | unknown | `C:f638735f808a` (FUN_002d3918) |
| U344 | 1424 | 10 | 2 | unknown | `C:ecc426044254` (FUN_002f5a50) |
| U343 | 1421 | 10 | 1 | unknown | `C:ecc426044254` (FUN_002f5a50) |
| U451 | 921 | 14 | 1 | unknown | `C:ecc426044254` (FUN_002f5a50) |
| U458 | 1352 | 14 | 1 | unknown | `C:ecc426044254` (FUN_002f5a50) |
| U568 | 1563 | 18 | 1 | unknown | `L01:002d2d08` (FUN_002d2d08) |

Eight units call **no shared function at all** (U93 331, U107 743, U114 854, U143 997, U202 1099, U321 353, U417 533,
U464 1418): pure class code, counted as cheap.

### Untagged shared functions (for a later pass)

Key, a level copy, units calling it, created instances of those units, size in words. `C:` keys are cluster hashes
(`tools/ghidra/names/clusters.tsv` lists every copy). 13 left (43 before 2026-09-29): `C:7a28b0d324fa` is a one-call
wrapper whose callee differs per copy (the cluster hash masks it), `boot:001251e8` has no decompiler entry (between
`sce_vib_get_profile` and `sce_vu0_unit_matrix`), the rest were not read.

| function | site | units | instances | words |
|---|---|---|---|---|
| `C:7a28b0d324fa` | L00:25a400 | 2 | 18 | 8 |
| `boot:001251e8` | L07:1251e8 | 1 | 14 | 0 |
| `C:1d746ca2af76` FUN_00312b40 | L06:312b40 | 4 | 5 | 12 |
| `C:ecc426044254` FUN_002f5a50 | L10:2f5a50 | 4 | 5 | 10 |
| `C:f638735f808a` FUN_002d3918 | L03:2d3918 | 2 | 5 | 202 |
| `L01:00219580` UpdateViewContext__Fv | L03:1f12e8 | 3 | 3 | 392 |
| `L01:00265210` FUN_00265210 | L11:276260 | 3 | 3 | 20 |
| `L01:002bcb90` FUN_002bcb90 | L00:2a7210 | 1 | 3 | 138 |
| `C:883715506a19` FUN_002b84e8 | L13:2b84e8 | 2 | 2 | 42 |
| `C:fe1252f3da20` | L06:2e5e10 | 2 | 2 | 56 |
| `C:a80dbefc5193` FUN_00301058 | L18:301058 | 1 | 1 | 18 |
| `L01:0021c4f0` fun_001f5ab0 | L11:22c158 | 1 | 1 | 280 |
| `L01:002d2d08` FUN_002d2d08 | L18:2bf908 | 1 | 1 | 748 |

## Appendix: every unit

One row per class-port unit, largest first. *created/placed*: instances over all its levels. *updates*: the update in
each level (`Lnn:address`). *private*: the unit's own functions / words. *needs*: the systems of the functions it calls
that the port lacks (missing) or has in part (partly); *has*: systems it calls that the port has. System keys: `anim`
animation, `breakfx` break effects, `camera` follow / script camera settings, `cheat` big-head cheat, `cine` script
camera and cinematic calls, `coll` collision queries, `creature` creature layer, `death` death bits and bolt drops,
`family` the family's own level code, `fxdraw` draw callbacks, `fxmoby` effect-moby spawners, `gadget` gadget
paths, `group` moby groups, `heightmap` level height grid, `help` help messages, `herostate` hero states and bodies,
`hit` hit messages, `hud` HUD, `levelexit` leaving the level, `light` point lights, `lsound` level sounds, `math` math /
timers / RNG, `menu` menu mobys, `mission` mission flags and items, `moby` moby lifecycle, `particles` / `ptype` ported /
unported particle types, `path` paths and splines, `platform` platforms, `react` Suck Cannon interface, `save` saves,
`shadow` shadows, `shipmode` the ship-combat mode (override), `sound` class sounds, `talk` the talk slots, `target` target list,
`trig` trigger volumes, `voicehand` the voice handoff (override), `water` water, `map` the in-game map. The per-function
tags are `tools/ghidra/names/census_systems.tsv`.

| unit | classes | levels | created/placed | updates | private | verdict | needs | has |
|---|---|---|---|---|---|---|---|---|
| U127 | 75, 115, 116, 117, 118, 119, 120, 132, 795 | 03 | 246/246 | L03:29dba8 | 9/1820 | missing | missing: path | anim creature death group hit hud lsound math moby particles path sound |
| U209 | 827 | 06 10 | 155/193 | L06:2e8678, L10:2cc8b8 | 11/2150 | missing | missing: cheat,path; partly: herostate | anim coll creature death fxmoby hit math moby particles react shadow sound |
| U371 | 1246 | 11 | 126/126 | L11:314318 | 4/1924 | missing | missing: cheat; partly: light,platform | anim creature death hit math moby particles react shadow |
| U25 | 749 | 00 18 | 106/106 | L00:2d4610, L18:2e02e8 | 9/1736 | missing | missing: cheat,path | anim creature death hit math moby react shadow sound |
| U300 | 193 | 09 15 | 93/170 | L09:2e27d8, L15:2bc608 | 2/1636 | missing | missing: cheat; partly: herostate | anim coll creature death group hit math moby react shadow |
| U36 | 1564 | 00 07 10 18 | 92/92 | L00:2e3a88, L07:31ede0, L10:2eada8 +1 | 1/136 | missing | missing: path | anim math moby |
| U523 | 1667, 1668, 1669, 1670, 1671 | 16 | 90/90 | L16:2e76b8 | 3/466 | missing | missing: path | math moby particles sound |
| U94 | 580 | 02 | 82/115 | L02:2d3e50 | 8/2582 | missing | missing: cheat,path; partly: light | anim coll creature death hit math moby particles react shadow sound trig |
| U268 | 252 | 08 14 | 77/148 | L08:2d2af0, L14:2d98f0 | 4/1414 | missing | missing: cheat | anim creature death fxdraw hit math moby react shadow sound |
| U504 | 638 | 16 18 | 64/65 | L16:2d2cf0, L18:2dc918 | 9/1990 | missing | missing: cheat; partly: anim,herostate | anim creature death fxmoby hit hud lsound math moby particles sound |
| U549 | 1843 | 17 | 60/60 | L17:2f40d8 | 6/1180 | missing | missing: shipmode; partly: herostate | anim coll creature fxdraw hit hud lsound math moby sound |
| U406 | 63 | 13 | 58/85 | L13:2b50d8 | 9/3080 | missing | missing: cheat | anim coll creature death hit math moby particles path react shadow sound trig |
| U335 | 1202 | 10 | 57/65 | L10:2e1d38 | 7/2976 | missing | missing: cheat; partly: herostate | anim coll creature death hit hud lsound math moby particles shadow |
| U380 | 238 | 12 | 54/71 | L12:2e1be0 | 11/2310 | missing | missing: cheat,creature,path; partly: anim | anim creature death group hit math moby particles react shadow |
| U374 | 1319 | 11 | 52/52 | L11:319838 | 7/1446 | missing | missing: shipmode; partly: herostate | anim coll creature fxdraw hit hud lsound math moby sound |
| U175 | 623 | 05 | 51/56 | L05:301f48 | 8/2570 | missing | missing: cheat,fxmoby; partly: anim | anim coll creature death hit math moby particles shadow trig |
| U479 | 408 | 15 17 | 48/48 | L15:2cb4c8, L17:2cbe80 | 4/604 | cheap | — | anim coll fxdraw herostate math sound |
| U299 | 52 | 09 16 | 46/61 | L09:2c5990, L16:2a1088 | 2/902 | missing | missing: cheat | anim creature death group hit math moby shadow sound |
| U473 | 123 | 15 17 | 46/46 | L15:2a6e68, L17:2ac740 | 1/496 | cheap | — | anim coll fxdraw math particles |
| U435 | 28 | 14 15 | 43/58 | L14:2b17d8, L15:2955c0 | 19/2144 | missing | missing: cheat,ptype; partly: herostate | anim coll creature death fxdraw hit light math moby path |
| U287 | 1023 | 08 09 | 40/41 | L08:301158, L09:3009d0 | 5/1788 | missing | missing: cheat,creature | anim coll creature death hit math moby shadow |
| U384 | 294 | 12 | 40/40 | L12:2e72c0 | 12/1836 | missing | missing: cheat,creature,path; partly: anim | anim coll creature death group hit math moby sound trig |
| U154 | 466, 480, 485, 486, 488, 490, 493, 494, 495, 498, 555 | 04 | 38/38 | L04:2ca420 | 14/3032 | partly | partly: lsound | anim coll creature group hit hud math moby particles sound trig |
| U498 | 471 | 16 | 37/37 | L16:2c9878 | 4/524 | cheap | — | fxdraw math platform sound trig |
| U469 | 77 | 15 17 | 36/36 | L15:2a2488, L17:2a7b58 | 1/248 | cheap | — | anim creature fxdraw hit math particles |
| U560 | 1355 | 18 | 36/36 | L18:2efb88 | 5/606 | cheap | — | creature fxdraw math moby sound |
| U534 | 99 | 17 | 35/35 | L17:2a8da0 | 3/616 | cheap | — | anim coll fxdraw hit math moby particles trig |
| U542 | 1382 | 17 | 34/40 | L17:2eeb68 | 15/1568 | missing | missing: cheat; partly: anim,breakfx | anim coll creature death fxdraw hit math moby react shadow trig |
| U256 | 1112 | 07 | 31/31 | L07:3196e0 | 2/556 | missing | missing: creature,shadow | anim coll creature death hit math moby shadow |
| U324 | 857 | 10 | 31/31 | L10:2d4078 | 17/4300 | missing | missing: path | anim coll creature fxdraw group hit math moby particles shadow sound target trig |
| U170 | 79 | 05 | 30/30 | L05:2d7140 | 3/576 | missing | missing: fxmoby,path | anim fxdraw hit math moby sound |
| U500 | 541 | 16 | 29/29 | L16:2cddb8 | 10/1538 | missing | missing: cheat,fxmoby | anim coll creature death fxdraw hit math moby particles sound |
| U150 | 340 | 04 | 28/63 | L04:2c2270 | 8/2512 | missing | missing: cheat,creature | anim coll creature death hit math moby particles react shadow sound trig |
| U478 | 233 | 15 | 28/28 | L15:2c5630 | 8/2072 | partly | partly: herostate | anim coll creature death group hit math moby path shadow sound trig |
| U556 | 587 | 18 | 28/28 | L18:2d82c0 | 3/546 | partly | partly: herostate | math moby particles platform sound |
| U129 | 573 | 03 | 27/87 | L03:2c5bb8 | 4/1138 | missing | missing: cheat,creature; partly: breakfx | anim creature death hit math moby path react shadow trig |
| U436 | 30 | 14 | 27/27 | L14:2b3bf0 | 4/786 | cheap | — | anim creature death hit math moby trig |
| U183 | 838 | 05 | 26/26 | L05:30dc68 | 2/356 | missing | missing: voicehand | fxdraw math sound |
| U205 | 1511 | 05 | 26/26 | L05:31c8e0 | 4/606 | missing | missing: fxmoby | death fxdraw hit math moby particles sound |
| U179 | 812 | 05 | 25/25 | L05:30bf98 | 1/68 | partly | partly: platform | math platform |
| U274 | 438 | 08 | 25/25 | L08:2de848 | 2/422 | missing | missing: path; partly: herostate | coll creature hit hud lsound math moby particles |
| U509 | 933 | 16 | 25/25 | L16:2ddde0 | 1/302 | cheap | — | creature hit math moby sound |
| U520 | 1445 | 16 | 24/52 | L16:2e5e08 | 5/796 | missing | missing: cheat | anim creature death hit math moby react shadow |
| U410 | 127, 128, 159, 169 | 13 | 24/24 | L13:2c7f38 | 1/278 | cheap | — | math sound |
| U455 | 1224 | 14 | 24/24 | L14:3015d0 | 4/894 | cheap | — | coll fxdraw math moby particles |
| U477 | 221 | 15 | 24/24 | L15:2c2938 | 2/450 | missing | missing: fxmoby; partly: anim | anim creature death hit math moby particles sound |
| U465 | 1511 | 14 | 22/22 | L14:307ad8 | 5/606 | missing | missing: fxmoby | death fxdraw hit math moby particles sound |
| U215 | 1038 | 06 10 17 | 21/21 | L06:2f7288, L10:2d9550, L17:2e3690 | 3/426 | cheap | — | hit math moby particles |
| U467 | 44 | 15 | 21/21 | L15:2979d8 | 16/3360 | missing | missing: cheat; partly: anim,death,herostate | anim coll creature death fxdraw herostate hit math moby particles shadow sound trig |
| U495 | 257 | 16 | 21/21 | L16:2c3d38 | 1/174 | missing | missing: path | math moby sound trig |
| U217 | 1048 | 06 | 20/27 | L06:2f7d78 | 4/1780 | missing | missing: cheat,path; partly: breakfx | anim coll creature death hit math moby shadow trig |
| U171 | 133 | 05 | 20/20 | L05:2dac80 | 4/558 | cheap | — | anim creature hit math moby particles sound |
| U394 | 1269 | 12 | 20/20 | L12:304e00 | 1/428 | missing | missing: creature,shadow | anim coll creature death hit math moby |
| U502 | 552 | 16 | 20/20 | L16:2cf4a8 | 6/330 | cheap | — | fxdraw math sound |
| U552 | 568 | 18 | 20/20 | L18:2d5918 | 2/404 | cheap | — | anim creature hit math react |
| U96 | 615 | 02 04 06 08 11 13 18 | 19/19 | L02:2d8ad0, L04:2d4ae0, L06:2e1940 +4 | 3/694 | missing | missing: camera; partly: herostate,light | anim cine creature fxdraw help herostate math sound | **ported: `units::trespasser_lock` (2026-10-01 gadgets)**
| U392 | 1259 | 12 | 18/18 | L12:302f30 | 3/648 | cheap | — | coll fxdraw hit math moby trig |
| U572 | 1906 | 18 | 18/18 | L18:2fbb98 | 4/834 | missing | missing: path | anim creature fxdraw hit math react |
| U130 | 574 | 03 | 17/19 | L03:2c6fd0 | 7/3452 | missing | missing: cheat,creature; partly: anim,platform | anim camera coll creature death hit math moby particles path shadow sound trig |
| U533 | 44 | 17 | 17/17 | L17:29f8d8 | 25/3300 | missing | missing: cheat; partly: anim,death,herostate | anim coll creature death fxdraw herostate hit math moby particles shadow sound trig |
| U185 | 843 | 05 | 16/16 | L05:30e508 | 1/188 | cheap | — | math sound |
| U252 | 1069 | 07 | 16/16 | L07:3112c8 | 2/600 | cheap | — | creature math moby platform sound |
| U413 | 224, 228 | 13 | 16/16 | L13:2e1cc8 | 1/416 | cheap | — | anim coll math moby sound |
| U422 | 1262 | 13 | 16/16 | L13:307e98 | 7/2028 | missing | missing: cheat,path | anim coll creature death fxdraw herostate hit math moby particles shadow trig |
| U334 | 1199 | 10 | 15/51 | L10:2e01a8 | 3/1764 | missing | missing: cheat | anim coll creature death hit math moby shadow sound |
| U95 | 612 | 02 | 15/15 | L02:2d7748 | 6/1580 | missing | missing: cheat,creature; partly: light | anim coll creature death hit math moby particles shadow trig |
| U118 | 1213, 1973 | 02 09 10 | 15/15 | L02:2ece18, L09:306720, L10:2ebb08 | 4/726 | missing | missing: path; partly: herostate | anim creature fxdraw group hit hud lsound math moby particles sound |
| U480 | 491 | 15 | 15/15 | L15:2cf3a8 | 7/2642 | missing | missing: cheat,fxmoby; partly: herostate | anim breakfx coll creature death group herostate hit math moby particles path sound trig |
| U483 | 1209 | 15 17 | 15/15 | L15:2e5958, L17:2e5f50 | 2/354 | cheap | — | anim group math moby sound |
| U511 | 1356 | 16 18 | 15/15 | L16:2e22d8, L18:2f0920 | 8/1032 | missing | missing: fxmoby,path; partly: anim,herostate | anim creature death hit math moby sound |
| U536 | 669 | 17 | 15/15 | L17:2d77f0 | 2/518 | cheap | — | anim coll fxdraw herostate math moby particles |
| U211 | 911 | 06 | 14/14 | L06:2f3ad8 | 4/690 | cheap | — | coll math moby particles sound trig |
| U250 | 1059 | 07 | 14/14 | L07:30f5f0 | 20/3740 | missing | missing: cheat,creature; partly: anim,herostate; untagged ×1 | anim coll creature death group hit math moby particles target trig water |
| U345 | 1544 | 10 | 14/14 | L10:2ea1f0 | 4/698 | cheap | — | creature math moby particles |
| U514 | 1401 | 16 | 14/14 | L16:2e37a0 | 7/856 | cheap | — | anim creature death hit math moby particles path trig |
| U223 | 1068 | 06 10 | 13/15 | L06:2fdbd0, L10:2dabe8 | 6/2074 | missing | missing: cheat,fxmoby,path; partly: herostate | anim coll creature death fxdraw herostate hit hud lsound math moby particles shadow trig |
| U151 | 427 | 04 | 13/13 | L04:2c4850 | 8/2400 | missing | missing: cheat,creature; partly: anim | anim coll creature death hit math moby particles shadow sound |
| U125 | 868, 905, 928 | 03 | 12/12 | L03:294c08 | 2/356 | cheap | — | math platform sound |
| U203 | 1139 | 05 16 | 12/12 | L05:31abe0, L16:2e0f78 | 1/66 | missing | missing: ptype | math moby |
| U255 | 1110 | 07 | 12/12 | L07:319040 | 1/406 | missing | missing: creature,shadow | anim coll creature death group hit math moby shadow |
| U472 | 93 | 15 | 12/12 | L15:2a3ba8 | 1/200 | cheap | — | math sound target trig |
| U212 | 1021 | 06 | 11/11 | L06:2f4f00 | 1/280 | cheap | — | creature math moby sound |
| U325 | 939 | 10 | 11/11 | L10:2d8c00 | 2/146 | cheap | — | math moby sound |
| U387 | 336 | 12 | 11/11 | L12:2ebea8 | 3/394 | partly | partly: herostate | creature death hit hud lsound math moby |
| U433 | 8 | 14 | 11/11 | L14:2ac618 | 15/2198 | cheap | — | anim coll creature death fxdraw hit math moby particles path shadow sound target |
| U439 | 250 | 14 15 | 11/11 | L14:2d96e0, L15:2c6f08 | 1/100 | partly | partly: herostate | anim math |
| U474 | 148, 255 | 15 | 11/11 | L15:2a7880 | 1/496 | partly | partly: herostate; untagged ×1 | creature death fxmoby math moby particles |
| U157 | 563 | 04 | 10/17 | L04:2d16b8 | 7/2424 | missing | missing: cheat,creature; partly: anim | anim coll creature death hit math moby shadow |
| U40 | 27 | 01 | 10/10 | L01:2bd100 | 2/352 | cheap | — | math moby particles |
| U281 | 468, 469 | 08 | 10/10 | L08:2ea4c0 | 1/200 | cheap | — | math sound |
| U306 | 1172 | 09 | 10/10 | L09:303d10 | 2/294 | cheap | — | creature death hit math moby sound |
| U320 | 351, 1301 | 10 | 10/10 | L10:2be858 | 1/484 | missing | missing: camera; partly: herostate | cine math particles sound |
| U476 | 196, 197, 1958 | 15 17 | 10/10 | L15:2bddb0, L17:2c0f00 | 1/192 | cheap | — | math moby sound trig |
| U510 | 1143 | 16 | 10/10 | L16:2e1088 | 1/54 | ported (2026-09-30, `units::hologram_logo`) | — | anim math |
| U33 | 1440 | 00 | 9/9 | L00:2e0b88 | 12/1972 | missing | missing: cheat,ptype | anim breakfx coll creature death fxdraw hit light math moby shadow |
| U59 | 695 | 01 | 9/9 | L01:2f8268 | 2/218 | cheap | — | creature math water |
| U117 | 1212 | 02 09 | 9/9 | L02:2ec4b8, L09:305dc0 | 3/542 | missing | missing: path | anim creature fxdraw group hit hud lsound math moby particles sound |
| U219 | 1054 | 06 | 9/9 | L06:2fbfb0 | 2/444 | cheap | — | math moby sound trig |
| U309 | 1206 | 09 | 9/9 | L09:305a28 | 1/122 | cheap | — | coll math sound |
| U327 | 1015, 1282 | 10 | 9/9 | L10:2d90a8 | 2/174 | cheap | — | group math moby |
| U388 | 339 | 12 | 9/9 | L12:2ec1d0 | 1/170 | cheap | — | anim math |
| U405 | 29 | 13 | 9/9 | L13:2b41b8 | 5/630 | missing | missing: cheat | anim creature death math moby trig |
| U315 | 1885 | 09 13 | 8/9 | L09:30ab80, L13:30d550 | 7/686 | missing | missing: fxmoby | anim creature death group hit math moby |
| U162 | 1101, 1102, 1531, 1532 | 04 | 8/8 | L04:2e17d8 | 2/170 | cheap | — | math sound |
| U180 | 823 | 05 07 | 8/8 | L05:30c0a8, L07:304738 | 1/94 | ported (2026-09-30, `units::sweep_light`) | — | anim fxdraw math moby |
| U249 | 1041 | 07 | 8/8 | L07:30d4d8 | 2/516 | missing | missing: creature,fxmoby | anim creature death hit math moby shadow sound |
| U253 | 1080 | 07 | 8/8 | L07:311bc8 | 2/208 | cheap | — | math moby platform sound |
| U271 | 333 | 08 | 8/8 | L08:2dabf0 | 7/1612 | missing | missing: cheat,creature | anim creature death hit math moby particles shadow |
| U381 | 240 | 12 | 8/8 | L12:2e3ed8 | 6/1108 | cheap | — | creature group math moby particles platform |
| U385 | 326 | 12 | 8/8 | L12:2e9f68 | 8/1640 | missing | missing: cheat,creature; partly: herostate | anim creature death help hit math moby particles shadow sound trig |
| U409 | 111 | 13 | 8/8 | L13:2c4428 | 9/1632 | cheap | — | creature hit math moby path sound |
| U431 | 1805 | 13 | 8/8 | L13:30cdb8 | 2/256 | missing | missing: fxmoby | creature death group hit hud lsound math moby sound |
| U463 | 1417 | 14 | 8/8 | L14:306ee0 | 4/742 | cheap | — | anim creature hit math moby path sound trig |
| U468 | 67 | 15 | 8/8 | L15:29aff0 | 1/296 | cheap | — | math sound trig |
| U497 | 470 | 16 | 8/8 | L16:2c9480 | 3/254 | cheap | — | creature math particles |
| U535 | 347 | 17 | 8/8 | L17:2cb310 | 6/1138 | cheap (2026-09-30 census: `0x2777d8` ported) | — | anim coll creature hit math moby particles sound |
| U561 | 1381 | 18 | 8/8 | L18:2f16f0 | 4/546 | missing | missing: platform | camera math moby |
| U100 | 668 | 02 | 7/7 | L02:2dcb38 | 4/568 | partly | partly: light | anim creature death hit math moby shadow |
| U189 | 855 | 05 | 7/7 | L05:3150f0 | 2/458 | missing | missing: voicehand | group math particles sound |
| U333 | 1196 | 10 | 7/7 | L10:2df270 | 9/974 | missing | missing: cheat | anim coll creature death hit hud lsound math moby shadow |
| U369 | 1231 | 11 | 7/7 | L11:310180 | 7/802 | missing | missing: cheat; partly: light | anim creature death hit math moby particles shadow |
| U470 | 78 | 15 17 | 7/7 | L15:2a2bf0, L17:2a82c0 | 3/724 | partly | partly: death | fxdraw math particles sound |
| U475 | 154 | 15 | 7/7 | L15:2aa280 | 1/434 | partly | partly: herostate; untagged ×1 | creature death fxmoby math moby particles |
| U487 | 1405, 1406, 1407, 1408, 1409, 1565, 1567 | 15 17 | 7/7 | L15:2eb0a0, L15:2eb458, L15:2edf58 +4 | 1/26 | partly | partly: fxdraw | fxdraw math moby |
| U531 | 1907, 1908, 1910, 1911, 1913, 1918, 1952 | 16 17 | 7/7 | L16:2e9df0, L17:2f57a0, L17:2f5ab8 +4 | 1/20 | partly | partly: fxdraw | fxdraw math |
| U128 | 455 | 03 08 14 | 6/7 | L03:2bef68, L08:2e6530, L14:2e1b20 | 3/872 | missing | missing: fxmoby | anim creature death hit math moby shadow trig |
| U190 | 877 | 05 | 6/6 | L05:3156d0 | 3/616 | missing | missing: camera | cine math platform sound |
| U222 | 1066 | 06 | 6/6 | L06:2fda30 | 1/104 | cheap | — | anim math trig |
| U247 | 1013, 1014, 1064, 1065 | 07 | 6/6 | L07:30cf90 | 1/292 | cheap | — | math sound |
| U258 | 1126 | 07 | 6/6 | L07:31a250 | 2/578 | cheap | — | anim creature death hit math moby sound |
| U303 | 664 | 09 | 6/6 | L09:2f86a0 | 1/310 | cheap | — | math platform sound |
| U312 | 1293, 1320 | 09 | 6/6 | L09:3091b0 | 1/414 | cheap | — | math platform sound |
| U323 | 794 | 10 | 6/6 | L10:2c9c70 | 3/468 | cheap | — | coll hit math moby |
| U330 | 1067 | 10 | 6/6 | L10:2da2c8 | 1/242 | partly | partly: herostate | fxdraw math moby sound |
| U342 | 1378 | 10 17 | 6/6 | L10:2e9028, L17:2e9608 | 1/100 | cheap | — | fxdraw math |
| U372 | 1248 | 11 | 6/6 | L11:316320 | 1/362 | cheap | — | math moby sound trig |
| U408 | 101 | 13 | 6/6 | L13:2c25b8 | 4/946 | cheap | — | anim coll creature death hit math moby particles sound |
| U441 | 386 | 14 | 6/6 | L14:2dee28 | 4/442 | cheap | — | fxdraw hit math moby sound trig |
| U501 | 546 | 16 | 6/6 | L16:2cf198 | 2/208 | cheap | — | hit math sound |
| U537 | 835 | 17 | 6/6 | L17:2dc3f8 | 1/242 | cheap | — | coll creature hit math moby particles water |
| U87 | 1504 | 01 06 | 5/5 | L01:30b618, L06:308868 | 1/256 | cheap | — | light math moby trig |
| U120 | 1479 | 02 | 5/5 | L02:2ef020 | 1/630 | cheap | — | math moby particles |
| U126 | 1210 | 03 | 5/5 | L03:2953f8 | 1/66 | cheap | — | anim math platform |
| U149 | 217 | 04 | 5/5 | L04:2ba520 | 6/1888 | missing | missing: cheat,creature | anim camera creature death hit math moby shadow sound trig |
| U169 | 35 | 05 | 5/5 | L05:2d1688 | 3/724 | cheap | — | anim cine creature herostate math moby particles sound water |
| U201 | 998 | 05 | 5/5 | L05:318c78 | 9/852 | missing | missing: path | cine help math platform sound |
| U214 | 1035 | 06 | 5/5 | L06:2f6470 | 2/762 | cheap | — | coll fxdraw help hit math moby particles sound |
| U260 | 104, 106, 1129 | 07 13 | 5/5 | L07:31aee0, L13:2c32d0 | 1/324 | cheap | — | math platform sound |
| U302 | 276, 1298, 1299 | 09 | 5/5 | L09:2edc30 | 2/364 | missing | missing: fxmoby | creature death hit math moby sound |
| U305 | 1150, 1151 | 09 | 5/5 | L09:3033a0 | 2/506 | partly | partly: herostate | math platform sound trig |
| U332 | 1100 | 10 | 5/5 | L10:2dd650 | 1/126 | cheap | — | math moby |
| U336 | 1229 | 10 | 5/5 | L10:2e4a88 | 4/1366 | missing | missing: cheat; partly: herostate | anim coll creature death help hit math moby particles shadow trig |
| U338 | 1302 | 10 | 5/5 | L10:2e7cd8 | 4/990 | partly | partly: herostate | fxdraw math mission particles sound |
| U355 | 318 | 11 | 5/5 | L11:2f2518 | 5/742 | cheap | — | cine fxdraw help herostate hit math moby music sound |
| U411 | 170 | 13 | 5/5 | L13:2cefc0 | 5/982 | missing | missing: ptype | anim creature death fxmoby hit math moby particles sound |
| U416 | 404, 405 | 13 | 5/5 | L13:2ed4a8 | 3/370 | cheap | — | anim coll hit light math moby |
| U444 | 643 | 14 | 5/5 | L14:2ec810 | 1/312 | cheap | — | creature hit math moby path sound |
| U450 | 908 | 14 | 5/5 | L14:2fc0f0 | 4/686 | cheap | — | creature hit hud lsound math moby particles path sound |
| U503 | 556 | 16 | 5/5 | L16:2d04a0 | 8/1540 | missing | missing: cheat,path; partly: anim,breakfx | anim coll creature group hit math moby particles path |
| U49 | 613 | 01 | 4/4 | L01:2f3120 | 1/274 | cheap | — | math path |
| U102 | 707, 734 | 02 | 4/4 | L02:2ddc00, L02:2df8c0 | 1/34 | cheap | — | math platform |
| U138 | 899 | 03 | 4/4 | L03:2db280 | 2/186 | cheap | — | math moby sound |
| U152 | 432, 1052 | 04 | 4/4 | L04:2c6858 | 1/214 | cheap | — | anim math moby sound |
| U155 | 481 | 04 | 4/4 | L04:2cdda0 | 1/176 | ported (2026-09-30, `units::spinner_float`) | — | anim math moby platform |
| U160 | 617 | 04 | 4/4 | L04:2d6f68 | 1/82 | cheap | — | math |
| U176 | 625 | 05 | 4/4 | L05:304320 | 4/1492 | missing | missing: creature,fxmoby | anim coll creature death hit math moby particles shadow sound |
| U177 | 717 | 05 | 4/4 | L05:307910 | 10/1446 | missing | missing: cheat,path; partly: anim | anim coll creature group hit math moby path shadow |
| U178 | 810 | 05 | 4/4 | L05:30bdd8 | 1/112 | cheap | — | math platform sound |
| U192 | 893 | 05 | 4/4 | L05:316258 | 1/274 | cheap | — | anim creature math sound |
| U193 | 895 | 05 | 4/4 | L05:3166a0 | 1/92 | cheap | — | creature math sound |
| U210 | 857 | 06 | 4/4 | L06:2f0040 | 22/4370 | missing | missing: path | anim coll creature fxdraw group hit math moby particles shadow sound target trig |
| U216 | 1039 | 06 08 13 18 | 4/4 | L06:2f7930, L08:302a78, L13:3036f0 +1 | 1/98 | partly | partly: herostate | cine herostate math trig |
| U230 | 1123 | 06 | 4/4 | L06:3049f8 | 2/478 | cheap | — | coll fxdraw math particles sound |
| U244 | 871 | 07 | 4/4 | L07:30b000 | 2/382 | missing | missing: shadow | creature group hit math moby path target |
| U257 | 1113, 1114, 1115, 1116 | 07 | 4/4 | L07:319f48 | 2/316 | cheap | — | creature hit math moby |
| U269 | 253 | 08 | 4/4 | L08:2d42d8 | 8/2306 | cheap | — | anim creature death help hit hud lsound math moby particles platform sound |
| U280 | 467, 472 | 08 | 4/4 | L08:2ea398 | 1/74 | cheap | — | math sound |
| U328 | 1031 | 10 | 4/4 | L10:2d92b8 | 1/166 | cheap | — | coll creature math moby sound |
| U337 | 1240 | 10 | 4/4 | L10:2e5be0 | 1/184 | cheap | — | math particles |
| U366 | 1178 | 11 | 4/4 | L11:30eb90 | 1/156 | partly | partly: herostate | math platform |
| U373 | 1264 | 11 | 4/4 | L11:3172c0 | 6/998 | cheap | — | creature hit math moby sound |
| U396 | 1281 | 12 | 4/4 | L12:307840 | 3/310 | missing | missing: creature,ptype | anim math moby target |
| U403 | 6 | 13 | 4/4 | L13:2b0e80 | 1/264 | cheap | — | group moby trig |
| U404 | 21, 244 | 13 | 4/4 | L13:2b39e0 | 2/502 | missing | missing: camera; partly: herostate | cine math platform sound |
| U414 | 231 | 13 | 4/4 | L13:2e4448 | 2/396 | cheap | — | creature group hit math moby sound |
| U420 | 674, 677, 680 | 13 | 4/4 | L13:2f8a78 | 1/192 | cheap | — | anim moby |
| U429 | 1577 | 13 | 4/4 | L13:30be60 | 1/84 | cheap | — | moby |
| U437 | 31 | 14 | 4/4 | L14:2b46e8 | 14/1784 | cheap | — | anim creature death fxmoby hit math moby particles path shadow sound |
| U440 | 309 | 14 | 4/4 | L14:2de1f8 | 2/286 | cheap | — | math platform sound |
| U442 | 557 | 14 | 4/4 | L14:2e7bd8 | 14/2084 | cheap | — | anim creature group hit math moby particles platform sound trig |
| U443 | 610 | 14 | 4/4 | L14:2eaf88 | 4/682 | cheap | — | fxdraw math moby trig |
| U471 | 92 | 15 | 4/4 | L15:2a36d0 | 1/310 | partly | partly: herostate | sound trig |
| U507 | 654 | 16 | 4/4 | L16:2d5ef8 | 6/474 | missing | missing: camera | cine hit math moby sound trig |
| U513 | 1387 | 16 | 4/4 | L16:2e36e8 | 1/34 | cheap | — | math moby |
| U519 | 1443, 1890 | 16 18 | 4/4 | L16:2e5708, L18:2fae48 | 3/194 | cheap | — | fxdraw math sound |
| U541 | 1380 | 17 | 4/4 | L17:2ee2b0 | 3/558 | missing | missing: camera; partly: herostate | cine math moby platform sound |
| U554 | 583 | 18 | 4/4 | L18:2d6600 | 1/342 | cheap | — | creature math moby sound |
| U131 | 578 | 03 | 3/5 | L03:2c9eb8 | 3/922 | missing | missing: cheat,creature | anim creature death hit math moby shadow sound |
| U93 | 331 | 02 | 3/3 | L02:2cb978 | 1/12 | cheap | — | — |
| U97 | 651 | 02 | 3/3 | L02:2dbd38 | 2/606 | cheap | — | fxdraw math platform sound trig |
| U104 | 732 | 02 | 3/3 | L02:2df1a8 | 1/140 | cheap | — | anim math sound |
| U134 | 816 | 03 | 3/3 | L03:2d3198 | 5/566 | missing | missing: ptype; untagged ×1 | cine creature help herostate math moby platform trig |
| U148 | 86 | 04 | 3/3 | L04:29ecf8 | 8/1436 | cheap | — | coll math moby |
| U153 | 434 | 04 | 3/3 | L04:2c6bb8 | 1/258 | cheap | — | anim math moby sound |
| U156 | 484 | 04 | 3/3 | L04:2ce060 | 1/8 | cheap | — | moby |
| U159 | 584 | 04 | 3/3 | L04:2d3580 | 1/42 | cheap | — | math platform |
| U186 | 844 | 05 | 3/3 | L05:30e7f8 | 5/1036 | cheap | — | coll creature hit math moby particles platform sound |
| U187 | 846 | 05 | 3/3 | L05:30f5c8 | 3/406 | cheap | — | hit math moby particles sound |
| U196 | 447, 920 | 05 07 13 | 3/3 | L05:317aa0, L07:2f61c0, L13:2edbe8 | 2/388 | cheap (2026-09-30: `0x2777d8` ported) | — | anim cine math moby shadow talk trig |
| U242 | 529 | 07 | 3/3 | L07:2fbdb8 | 1/14 | cheap | — | fxdraw |
| U277 | 452 | 08 | 3/3 | L08:2e2df0 | 6/2680 | missing | missing: cheat; partly: platform; untagged ×1 | anim camera coll creature death fxdraw hit hud math moby particles shadow sound target |
| U286 | 671 | 08 | 3/3 | L08:2f70a0 | 4/1558 | cheap | — | fxdraw light math moby particles |
| U308 | 1201 | 09 | 3/3 | L09:304c80 | 4/1078 | missing | missing: camera; partly: herostate | anim cine light math moby particles sound target |
| U310 | 1285, 1286, 1287, 1288 | 09 | 3/3 | L09:3082d8 | 2/380 | missing | missing: fxmoby | creature hit math moby sound |
| U322 | 702 | 10 | 3/3 | L10:2c7a20 | 1/120 | missing | missing: ptype | math sound |
| U365 | 1159 | 11 | 3/3 | L11:30e978 | 1/134 | cheap | — | math moby sound | **ported: `units::lock_doors` (2026-10-01 gadgets)**
| U367 | 1179 | 11 15 | 3/3 | L11:30ee00, L15:2d9100 | 1/284 | cheap | — | help math sound |
| U389 | 384 | 12 | 3/3 | L12:2ec720 | 1/232 | cheap | — | fxdraw math particles |
| U397 | 1345 | 12 | 3/3 | L12:3081b0 | 2/304 | cheap | — | fxdraw math mission moby sound |
| U438 | 211 | 14 | 3/3 | L14:2d5f40 | 6/1204 | missing | missing: ptype | anim coll creature fxdraw light math moby path trig |
| U516 | 1439 | 16 | 3/3 | L16:2e5010 | 1/146 | cheap | — | math sound |
| U517 | 1441 | 16 | 3/3 | L16:2e5258 | 1/146 | cheap | — | math sound |
| U518 | 1442 | 16 | 3/3 | L16:2e54a0 | 2/154 | cheap | — | hit math sound |
| U538 | 1021 | 17 18 | 3/3 | L17:2e2c58, L18:2ec3e8 | 1/284 | cheap | — | creature math moby sound |
| U563 | 1402 | 18 | 3/3 | L18:2f2310 | 2/196 | cheap | — | fxdraw math |
| U566 | 1434, 1435 | 18 | 3/3 | L18:2f7ad8 | 1/90 | cheap | — | math |
| U567 | 1454 | 18 | 3/3 | L18:2f7c40 | 5/1050 | missing | missing: fxmoby | anim creature death hit math moby particles sound |
| U99 | 656 | 02 | 2/2 | L02:2dca10 | 1/74 | cheap | — | anim math |
| U106 | 735, 736 | 02 | 2/2 | L02:2df948 | 1/164 | cheap | — | creature math |
| U115 | 1005, 1016 | 02 06 | 2/2 | L02:2ea210, L06:2f4638 | 3/562 | missing | missing: camera,save | cine creature dialog help hud math mission moby particles |
| U123 | 822 | 03 | 2/2 | L03:292e98 | 9/2558 | missing | missing: path | anim cine creature group hit math moby music particles path platform sound trig |
| U144 | 1012 | 03 | 2/2 | L03:2dccc0 | 5/406 | unknown | untagged ×1 | cine help herostate hud math platform trig |
| U174 | 439 | 05 16 | 2/2 | L05:2f87a8, L16:2c7788 | 1/230 | partly | partly: herostate | anim help math particles |
| U207 | 55 | 06 | 2/2 | L06:2b4770 | 1/168 | cheap | — | creature math |
| U224 | 1083 | 06 | 2/2 | L06:2ffcb0 | 7/952 | missing | missing: camera,fxmoby; partly: herostate; untagged ×1 | cine fxdraw math moby sound |
| U272 | 424 | 08 | 2/2 | L08:2dbdb0 | 7/874 | missing | missing: path | anim help math platform shadow |
| U273 | 435 | 08 | 2/2 | L08:2dd3c0 | 2/1232 | cheap | — | creature hit math moby particles sound water |
| U278 | 462 | 08 | 2/2 | L08:2e8cd0 | 4/884 | missing | missing: fxmoby | anim creature hit math moby sound |
| U294 | 1629 | 08 14 | 2/2 | L08:3085f0, L14:3088b0 | 1/136 | partly | partly: particles | math |
| U295 | 1641 | 08 | 2/2 | L08:308c90 | 2/132 | cheap | — | math particles |
| U317 | 1117 | 10 | 2/2 | L10:295c20 | 2/216 | cheap | — | math platform sound |
| U326 | 947 | 10 | 2/2 | L10:2d8d30 | 2/270 | missing | missing: path | light math moby particles sound |
| U344 | 1424 | 10 | 2/2 | L10:2e9990 | 1/290 | unknown | untagged ×1 | math |
| U360 | 1075 | 11 | 2/2 | L11:309ac0 | 3/874 | cheap | — | anim camera creature math particles platform sound |
| U398 | 1400 | 12 14 | 2/2 | L12:308be0, L14:306390 | 5/830 | missing | missing: heightmap,ptype; partly: particles | math particles water |
| U399 | 1404 | 12 | 2/2 | L12:3093c8 | 1/54 | cheap | — | dialog moby trig |
| U432 | 903 | 14 | 2/2 | L14:2aba80 | 3/616 | cheap | — | math platform sound trig |
| U453 | 923 | 14 | 2/2 | L14:2fe2a0 | 7/862 | cheap | — | anim creature death hit math moby particles path trig |
| U457 | 1331 | 14 15 | 2/2 | L14:3039e0, L15:2e8910 | 6/1510 | missing | missing: ptype | fxdraw light math |
| U462 | 1416 | 14 | 2/2 | L14:306b78 | 1/218 | cheap | — | math sound |
| U481 | 655 | 15 17 | 2/2 | L15:2d6810, L17:2d4e88 | 9/2552 | cheap | — | coll creature fxdraw herostate hit light math moby particles sound trig |
| U486 | 1394 | 15 | 2/2 | L15:2eabd8 | 2/82 | cheap | — | math moby |
| U492 | 1451, 1899 | 15 18 | 2/2 | L15:2ed068, L18:2fb868 | 1/204 | missing | missing: herostate | anim cine creature dialog help moby music |
| U515 | 1410 | 16 | 2/2 | L16:2e43e0 | 10/798 | missing | missing: path | cine help math platform sound |
| U525 | 1826 | 16 | 2/2 | L16:2e7f80 | 1/346 | cheap | — | fxdraw math platform sound |
| U526 | 1891 | 16 | 2/2 | L16:2e88d0 | 1/40 | cheap | — | math |
| U543 | 1426 | 17 | 2/2 | L17:2f1418 | 1/28 | cheap | — | fxdraw moby |
| U548 | 1772 | 17 | 2/2 | L17:2f3848 | 1/200 | partly | partly: herostate | fxdraw math trig |
| U555 | 586 | 18 | 2/2 | L18:2d7b20 | 1/274 | cheap | — | fxdraw help math sound |
| U562 | 1392 | 18 | 2/2 | L18:2f1e68 | 2/298 | cheap | — | fxdraw math sound |
| U23 | 530 | 00 | 1/1 | L00:2d1e80 | 1/44 | cheap | — | anim fxdraw |
| U30 | 834 | 00 | 1/1 | L00:2d9dc8 | 3/386 | missing | missing: levelexit | cine dialog hud math trig |
| U32 | 1413 | 00 | 1/1 | L00:2e0988 | 1/128 | cheap | — | help trig |
| U34 | 1471 | 00 | 1/1 | L00:2e1df0 | 6/1504 | missing | missing: ptype | fxdraw light math |
| U35 | 1545 | 00 | 1/1 | L00:2e3800 | 3/458 | cheap | — | anim math particles |
| U54 | 676 | 01 | 1/1 | L01:2f6128 | 2/22 | cheap | — | fxdraw water |
| U55 | 678 | 01 | 1/1 | L01:2f6180 | 1/106 | cheap | — | fxdraw math |
| U74 | 761 | 01 | 1/1 | L01:2feb58 | 2/144 | cheap | — | fxdraw math water |
| U85 | 1225 | 01 | 1/1 | L01:309c98 | 1/14 | cheap | — | fxdraw |
| U92 | 1848 | 01 | 1/1 | L01:30f208 | 3/264 | partly | partly: fxdraw | fxdraw math |
| U101 | 675 | 02 | 1/1 | L02:2dd370 | 1/88 | cheap | — | anim math |
| U103 | 713 | 02 | 1/1 | L02:2ddc88 | 7/1502 | missing | missing: camera,path; partly: herostate | cine help herostate math moby particles platform sound |
| U105 | 733 | 02 | 1/1 | L02:2df3d8 | 4/378 | cheap | — | anim creature math moby sound |
| U107 | 743 | 02 | 1/1 | L02:2dffc8 | 3/20 | cheap | — | — | **ported: `units::lock_doors` (2026-10-01 gadgets)**
| U108 | 744 | 02 | 1/1 | L02:2e0138 | 1/82 | cheap | — | math | **ported: `units::lock_doors` (2026-10-01 gadgets)**
| U110 | 762 | 02 | 1/1 | L02:2e0720 | 2/370 | cheap | — | hit math moby particles sound |
| U111 | 786 | 02 | 1/1 | L02:2e0dc0 | 3/778 | missing | missing: camera,save; partly: anim | anim cine creature help herostate lsound math mission moby trig |
| U112 | 788 | 02 | 1/1 | L02:2e1950 | 4/870 | missing | missing: save; partly: anim | anim cine creature help hud lsound math mission moby trig |
| U113 | 792 | 02 | 1/1 | L02:2e2228 | 1/310 | cheap | — | anim math platform sound |
| U114 | 854 | 02 | 1/1 | L02:2ea198 | 5/96 | cheap | — | — |
| U119 | 1324 | 02 | 1/1 | L02:2ee890 | 1/484 | cheap | — | help hud lsound math trig |
| U124 | 845 | 03 | 1/1 | L03:293c78 | 3/1144 | missing | missing: path | anim creature group hit math moby particles path sound |
| U132 | 631 | 03 | 1/1 | L03:2cca00 | 3/1734 | missing | missing: cheat,creature; partly: anim | anim creature death hit math moby path shadow sound trig |
| U136 | 825 | 03 | 1/1 | L03:2d3e58 | 1/32 | cheap | — | math platform |
| U137 | 890 | 03 | 1/1 | L03:2da870 | 3/652 | missing | missing: save; partly: anim | anim cine hud lsound math mission moby |
| U139 | 909 | 03 | 1/1 | L03:2db558 | 2/580 | missing | missing: save; partly: anim | anim cine hud lsound math mission |
| U140 | 914 | 03 | 1/1 | L03:2dbd90 | 1/352 | cheap | — | anim cine creature hit hud lsound math particles shadow |
| U143 | 997 | 03 | 1/1 | L03:2dca30 | 1/64 | cheap | — | — |
| U145 | 1342 | 03 | 1/1 | L03:2df520 | 1/812 | cheap | — | gadget help hud lsound math trig |
| U146 | 1548 | 03 | 1/1 | L03:2e01d0 | 2/312 | cheap | — | anim math particles |
| U161 | 642 | 04 | 1/1 | L04:2d7e90 | 1/394 | cheap | — | coll math |
| U163 | 1120 | 04 | 1/1 | L04:2e1a10 | 3/330 | missing | missing: save | dialog hud lsound math mission moby particles |
| U164 | 1190 | 04 | 1/1 | L04:2e3078 | 4/474 | missing | missing: save | anim cine dialog hud math mission shadow trig |
| U165 | 1343 | 04 | 1/1 | L04:2e4418 | 1/754 | cheap | — | help math trig |
| U166 | 1549 | 04 | 1/1 | L04:2e53a0 | 2/220 | cheap | — | anim math particles |
| U184 | 841 | 05 | 1/1 | L05:30e1f8 | 1/196 | cheap | — | math trig |
| U194 | 918 | 05 | 1/1 | L05:316ab8 | 3/738 | missing | missing: save; partly: anim | anim cine creature hud math music |
| U195 | 919 | 05 | 1/1 | L05:317470 | 2/388 | missing | missing: save; partly: anim | anim cine math mission moby |
| U197 | 925 | 05 | 1/1 | L05:3180a0 | 2/420 | missing | missing: save; partly: anim | anim cine creature hud math mission moby shadow |
| U200 | 984 | 05 | 1/1 | L05:318b98 | 1/56 | cheap | — | math |
| U202 | 1099 | 05 | 1/1 | L05:319c28 | 1/14 | cheap | — | — |
| U204 | 1347 | 05 | 1/1 | L05:31bf40 | 2/702 | cheap | — | help math sound trig |
| U206 | 1550 | 05 | 1/1 | L05:31d5c8 | 2/214 | cheap | — | anim math particles |
| U213 | 1028 | 06 | 1/1 | L06:2f55a0 | 2/948 | missing | missing: camera,save; partly: herostate | cine help hud math mission moby sound |
| U218 | 1051 | 06 | 1/1 | L06:2f9a28 | 21/3324 | missing | missing: camera,cheat,path; partly: breakfx,group,herostate | anim camera cine coll creature death group hit hud lsound math moby shadow trig |
| U220 | 1061 | 06 | 1/1 | L06:2fc640 | 2/722 | missing | missing: herostate; partly: herostate | cine dialog help math moby music platform sound trig |
| U221 | 1062 | 06 | 1/1 | L06:2fd088 | 2/246 | cheap | — | fxdraw math trig |
| U226 | 1105 | 06 | 1/1 | L06:301070 | 6/1224 | missing | missing: camera,save | anim cine creature dialog help hud math mission moby shadow trig |
| U227 | 1108 | 06 | 1/1 | L06:301b90 | 11/690 | missing | missing: camera; partly: group,herostate,lsound; untagged ×1 | cine dialog fxdraw group herostate math moby music sound |
| U228 | 1109 | 06 | 1/1 | L06:302578 | 9/1794 | missing | missing: camera,save; partly: herostate,lsound; untagged ×2 | anim cine creature dialog help hud lsound math mission moby music particles platform sound trig |
| U229 | 1118 | 06 | 1/1 | L06:304098 | 3/580 | missing | missing: camera; partly: lsound; untagged ×1 | cine help math moby particles sound |
| U231 | 1302 | 06 | 1/1 | L06:307d30 | 5/990 | partly | partly: herostate | fxdraw math mission particles sound |
| U232 | 1348 | 06 | 1/1 | L06:3083b0 | 1/302 | cheap | — | group help math trig |
| U234 | 1551 | 06 | 1/1 | L06:309348 | 2/218 | cheap | — | anim math particles |
| U238 | 38 | 07 | 1/1 | L07:2cd2b0 | 2/390 | partly | partly: herostate | math platform sound |
| U240 | 436 | 07 | 1/1 | L07:2f5ba0 | 2/406 | missing | missing: levelexit,save | cine dialog map math mission moby music trig |
| U254 | 1106 | 07 | 1/1 | L07:314150 | 32/7704 | missing | missing: cheat,creature,ptype,shadow; partly: anim,herostate,math; untagged ×1 | anim camera coll creature death fxdraw hit hud light lsound math mission moby particles path pickup shadow sound talk trig |
| U259 | 1128 | 07 | 1/1 | L07:31a9e8 | 1/318 | cheap | — | math sound |
| U261 | 1133 | 07 | 1/1 | L07:31b3f0 | 1/128 | cheap | — | math sound |
| U262 | 1142 | 07 | 1/1 | L07:31d2e0 | 1/300 | cheap | — | math moby path pickup trig |
| U263 | 1552 | 07 | 1/1 | L07:31eba8 | 2/322 | cheap | — | anim math particles |
| U267 | 1789 | 07 | 1/1 | L07:31f900 | 2/236 | cheap | — | anim math particles |
| U275 | 440 | 08 | 1/1 | L08:2e0328 | 5/1606 | partly | partly: herostate | anim cine fxdraw herostate math moby particles sound |
| U276 | 444 | 08 | 1/1 | L08:2e24e8 | 4/1036 | missing | missing: fxmoby | anim creature hit math moby sound |
| U279 | 463 | 08 | 1/1 | L08:2e9b70 | 4/912 | missing | missing: fxmoby | anim creature hit math moby sound |
| U283 | 600 | 08 | 1/1 | L08:2f1548 | 10/862 | partly | partly: fxdraw | fxdraw math moby |
| U288 | 1130 | 08 | 1/1 | L08:302ce8 | 2/476 | missing | missing: save; partly: anim | anim cine hud lsound math mission moby shadow |
| U289 | 1144 | 08 | 1/1 | L08:305270 | 2/460 | missing | missing: save; partly: anim | anim cine math mission moby shadow |
| U290 | 1283 | 08 | 1/1 | L08:3065d8 | 4/512 | missing | missing: save; partly: anim,herostate | anim cine herostate lsound math mission moby |
| U291 | 1349 | 08 | 1/1 | L08:307540 | 1/404 | cheap | — | help hud lsound math trig |
| U292 | 1400 | 08 | 1/1 | L08:307cf0 | 5/702 | missing | missing: heightmap,ptype; partly: particles | math particles water |
| U293 | 1553 | 08 | 1/1 | L08:3084d8 | 2/250 | cheap | — | anim math particles |
| U296 | 1649 | 08 | 1/1 | L08:3097b0 | 10/856 | partly | partly: fxdraw | fxdraw math moby |
| U301 | 263, 264, 265, 266, 267 | 09 | 1/1 | L09:2eb970 | 8/1882 | partly | partly: herostate | coll creature math moby particles sound |
| U304 | 1000 | 09 | 1/1 | L09:300888 | 1/82 | cheap | — | help hud lsound math trig |
| U311 | 1290 | 09 | 1/1 | L09:308818 | 3/378 | missing | missing: save | dialog help hud math mission moby particles |
| U313 | 1766 | 09 | 1/1 | L09:309c08 | 3/668 | cheap | — | anim math particles platform sound trig |
| U316 | 1033 | 10 | 1/1 | L10:295a38 | 1/100 | cheap | — | math moby sound |
| U318 | 18 | 10 | 1/1 | L10:298668 | 3/320 | missing | missing: save | dialog hud lsound math mission moby particles |
| U319 | 22 | 10 | 1/1 | L10:298b68 | 2/330 | missing | missing: herostate; partly: herostate | cine help math moby |
| U321 | 353 | 10 | 1/1 | L10:2befe8 | 1/102 | cheap | — | — |
| U329 | 1047 | 10 | 1/1 | L10:2d9eb8 | 4/454 | cheap | — | creature hit math moby sound |
| U331 | 1073 | 10 | 1/1 | L10:2dcc58 | 1/238 | partly | partly: herostate | math moby |
| U339 | 1326 | 10 | 1/1 | L10:2e8358 | 1/152 | missing | missing: save | dialog help hud math talk |
| U340 | 1344 | 10 | 1/1 | L10:2e85b8 | 1/318 | cheap | — | help math trig |
| U341 | 1346 | 10 | 1/1 | L10:2e8ab0 | 2/462 | cheap | — | creature math moby particles |
| U343 | 1421 | 10 | 1/1 | L10:2e9648 | 1/210 | unknown | untagged ×1 | math trig |
| U346 | 1555 | 10 | 1/1 | L10:2eacd8 | 2/232 | cheap | — | anim math particles |
| U349 | 23 | 11 | 1/1 | L11:2cb668 | 1/74 | missing | missing: save | help math moby |
| U350 | 65 | 11 | 1/1 | L11:2cb810 | 1/96 | cheap | — | creature math moby |
| U352 | 90 | 11 | 1/1 | L11:2d0710 | 2/362 | missing | missing: save; partly: anim | anim cine help lsound math mission trig |
| U353 | 114 | 11 | 1/1 | L11:2d0fa8 | 17/2084 | missing | missing: save; partly: anim,herostate,platform | anim cine coll creature help math mission moby shadow trig |
| U354 | 298 | 11 | 1/1 | L11:2f0e40 | 2/498 | missing | missing: save; partly: anim | anim cine help hud lsound math mission moby shadow |
| U356 | 361 | 11 | 1/1 | L11:2f3350 | 1/40 | partly | partly: herostate | moby |
| U362 | 1156 | 11 | 1/1 | L11:30c788 | 2/1054 | missing | missing: camera | cine creature math moby sound |
| U363 | 1157 | 11 | 1/1 | L11:30d800 | 2/542 | missing | missing: camera | cine math moby sound |
| U368 | 1180 | 11 | 1/1 | L11:30f2a8 | 1/92 | cheap | — | creature math sound |
| U370 | 1242 | 11 | 1/1 | L11:313290 | 21/4018 | missing | missing: camera,ptype; partly: herostate; untagged ×3 | anim cine coll creature fxdraw help herostate hit hud math mission moby music sound target |
| U375 | 1350 | 11 | 1/1 | L11:31aa80 | 2/68 | cheap | — | hud lsound moby |
| U377 | 1903 | 11 | 1/1 | L11:31e2f0 | 10/884 | partly | partly: fxdraw | fxdraw math moby |
| U378 | 1919 | 11 | 1/1 | L11:31f150 | 10/922 | partly | partly: fxdraw | fxdraw math moby trig |
| U382 | 282 | 12 | 1/1 | L12:2e6c48 | 2/404 | missing | missing: save; partly: anim | anim cine hud lsound math mission moby |
| U383 | 293 | 12 | 1/1 | L12:2e7208 | 5/388 | partly | partly: fxdraw | fxdraw moby trig |
| U386 | 328 | 12 | 1/1 | L12:2eb570 | 2/516 | missing | missing: save; partly: anim | anim cine hud lsound math mission moby |
| U390 | 422 | 12 | 1/1 | L12:2ed280 | 1/196 | cheap | — | group help hud lsound math trig |
| U393 | 1267 | 12 | 1/1 | L12:303540 | 5/1242 | missing | missing: camera,save; partly: herostate | anim cine creature dialog fxdraw herostate hit math mission moby particles sound water |
| U395 | 1274 | 12 | 1/1 | L12:3069d0 | 7/2316 | missing | missing: camera; partly: anim | anim camera cine creature dialog hit math moby particles sound trig |
| U400 | 1557 | 12 | 1/1 | L12:3094a0 | 2/218 | cheap | — | anim math particles |
| U407 | 69 | 13 | 1/1 | L13:2bb068 | 12/3030 | missing | missing: camera,shadow; partly: herostate; untagged ×3 | anim cine coll creature fxdraw help herostate hit hud lsound math mission moby music particles sound target trig |
| U415 | 388 | 13 | 1/1 | L13:2eb098 | 23/4776 | partly | partly: herostate | anim coll creature death dialog fxdraw hit hud light math moby particles path sound |
| U417 | 533 | 13 | 1/1 | L13:2f3750 | 1/10 | cheap | — | — |
| U418 | 558 | 13 | 1/1 | L13:2f3778 | 1/118 | cheap | — | help math trig |
| U419 | 667 | 13 | 1/1 | L13:2f8880 | 1/126 | cheap | — | moby |
| U424 | 1270 | 13 | 1/1 | L13:30a6d8 | 2/484 | missing | missing: camera; partly: herostate | cine math platform sound |
| U425 | 1271 | 13 | 1/1 | L13:30af50 | 2/116 | partly | partly: herostate | react trig |
| U426 | 1353 | 13 | 1/1 | L13:30b628 | 1/338 | missing | missing: save; partly: cine | cine dialog mission moby music trig |
| U427 | 1403 | 13 | 1/1 | L13:30bb70 | 2/276 | cheap | — | math moby path |
| U428 | 1558 | 13 | 1/1 | L13:30bdf0 | 2/208 | cheap | — | anim math particles |
| U445 | 684 | 14 | 1/1 | L14:2ed280 | 6/1482 | cheap | — | cine dialog fxdraw math moby particles sound |
| U446 | 685 | 14 | 1/1 | L14:2ee8d8 | 6/940 | cheap | — | math moby path platform sound trig |
| U448 | 712 | 14 | 1/1 | L14:2f0538 | 9/1144 | missing | missing: path | camera cine help math platform sound trig |
| U449 | 851 | 14 | 1/1 | L14:2fba20 | 5/582 | missing | missing: save; partly: anim | anim cine hud lsound math mission moby shadow trig |
| U451 | 921 | 14 | 1/1 | L14:2fcbb0 | 7/1070 | unknown | untagged ×1 | coll creature hit hud lsound math moby particles path sound trig |
| U452 | 922 | 14 | 1/1 | L14:2fdbb8 | 2/412 | cheap | — | creature math moby particles sound |
| U454 | 924 | 14 | 1/1 | L14:2fefe0 | 2/466 | missing | missing: save; partly: anim | anim cine hud lsound math mission moby trig |
| U458 | 1352 | 14 | 1/1 | L14:305408 | 3/212 | unknown | untagged ×1 | math moby |
| U459 | 1354 | 14 | 1/1 | L14:305758 | 3/372 | missing | missing: save | creature dialog help hud math mission moby particles |
| U460 | 1395 | 14 | 1/1 | L14:305d28 | 2/298 | cheap | — | math moby sound |
| U464 | 1418 | 14 | 1/1 | L14:307a80 | 2/42 | cheap | — | — |
| U466 | 1559 | 14 | 1/1 | L14:3087c0 | 3/280 | cheap | — | anim hud lsound math particles |
| U485 | 1388 | 15 | 1/1 | L15:2ea748 | 3/292 | missing | missing: save | dialog hud math mission moby particles |
| U488 | 1419 | 15 | 1/1 | L15:2eb4c0 | 1/126 | missing | missing: save | cine dialog fxdraw herostate hud lsound mission trig |
| U490 | 1430 | 15 | 1/1 | L15:2ec030 | 6/544 | cheap | — | cine creature math moby |
| U491 | 1446 | 15 | 1/1 | L15:2ec760 | 5/926 | missing | missing: herostate,save; partly: anim,herostate | anim cine dialog group hud lsound math mission moby music trig |
| U493 | 1469 | 15 | 1/1 | L15:2ed398 | 1/482 | partly | partly: herostate | help herostate math trig |
| U494 | 1560 | 15 | 1/1 | L15:2edb20 | 2/228 | cheap | — | anim math particles |
| U512 | 1377 | 16 | 1/1 | L16:2e3190 | 2/400 | missing | missing: save; partly: anim | anim cine help lsound math mission trig |
| U521 | 1455 | 16 | 1/1 | L16:2e6808 | 8/834 | missing | missing: save; partly: anim | anim cine help herostate hud lsound math mission music shadow |
| U522 | 1561 | 16 | 1/1 | L16:2e7208 | 1/54 | cheap | — | hud lsound trig |
| U524 | 1812 | 16 | 1/1 | L16:2e7e00 | 2/212 | cheap | — | anim math particles |
| U527 | 1923 | 16 | 1/1 | L16:2e8970 | 6/318 | cheap | — | anim coll creature fxmoby math sound trig |
| U528 | 1943 | 16 | 1/1 | L16:2e8e80 | 1/146 | cheap | — | fxdraw math |
| U529 | 1947 | 16 | 1/1 | L16:2e93b0 | 1/146 | cheap | — | fxdraw math |
| U530 | 1948 | 16 | 1/1 | L16:2e98e0 | 1/150 | cheap | — | fxdraw math |
| U532 | 1953 | 16 | 1/1 | L16:2ea128 | 1/150 | cheap | — | fxdraw math |
| U540 | 1379 | 17 | 1/1 | L17:2ed018 | 12/2894 | missing | missing: camera,shadow; partly: herostate; untagged ×3 | anim cine coll creature fxdraw help herostate hit hud lsound math mission moby music particles sound target |
| U544 | 1428 | 17 | 1/1 | L17:2f1790 | 3/302 | missing | missing: save; partly: herostate | dialog hud math moby particles |
| U545 | 1448 | 17 | 1/1 | L17:2f1940 | 4/880 | missing | missing: camera | anim cine creature help math platform sound |
| U546 | 1470 | 17 | 1/1 | L17:2f26d0 | 1/490 | missing | missing: camera; partly: herostate | help herostate hud lsound math trig |
| U547 | 1562 | 17 | 1/1 | L17:2f2e78 | 2/220 | cheap | — | anim math particles |
| U553 | 582 | 18 | 1/1 | L18:2d62e8 | 2/232 | cheap | — | math moby |
| U557 | 644 | 18 | 1/1 | L18:2df608 | 1/294 | missing | missing: camera | cine help math |
| U564 | 1422 | 18 | 1/1 | L18:2f2bf0 | 50/7136 | missing | missing: camera,cheat,cine,herostate,platform; partly: anim; untagged ×1 | anim camera cine creature dialog fxdraw herostate hit hud lsound math mission moby particles shadow sound target trig |
| U568 | 1563 | 18 | 1/1 | L18:2f88e8 | 4/1314 | unknown | untagged ×1 | anim creature fxdraw math particles |
| U570 | 1750 | 18 | 1/1 | L18:2fad08 | 2/248 | ported (2026-10-01 b6 saves, `units::ending_save`) | — | math |
| U571 | 1799 | 18 | 1/1 | L18:2fad28 | 1/72 | cheap | — | anim math particles |
| U239 | 1474 | 07 | 0/1 | L07:2cdb28 | 2/416 | partly | partly: herostate | math platform sound |

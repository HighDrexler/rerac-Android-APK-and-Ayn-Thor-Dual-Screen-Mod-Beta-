---
status: open
job: forum-fixes
date: 2026-10-19
commit: 9fb4c40, 49651f6, 3974efd, fc51251, 6126b01, 9d114f1 (Comet-Strike part), 4968a3f, cf7ebda, c80b9b3, accd9d3, 23a24cf, 14c8b55, 4f34672, 142909a, 8447e57, d6c8cd6
areas: [hero, weapons, classes, ui, world]
---

# Fixes from the Discord bugs-and-qa forum (2026-10-16 .. 2026-10-19)

## 1. Summary

Sixteen fixes to bugs players reported, each ported from the original's code (addresses below), plus one test made
stricter. None adds a class; most correct a shared seam, so §4 matters as much as §3.

- Menus: Port Options in the front end's Options with scroll arrows; the underwater tint and the classes' 2-D
  countdowns no longer drawn over the page menus; the card icon while the memory card is in use.
- Hero and weapons: the aim frame 0x13f990 (grind-rail firing along the rail, swept by the stick); the wrench states'
  target search (auto-aim); Comet-Strike against doors; the hand items hidden at a gold-bolt pickup; Giant Clank's
  pilot on the body.
- Classes and scripts: Quartu's Codebot slot 1430 waiting for its fly-by; Kalebo's race host reading the Hoverboard's
  score; the cheat tables relocated on every level; a class teleport turning Ratchet's rows before the camera reset;
  the scene end restoring the gameplay FOV; the Glove of Doom canister / gold-bolt glints using defs 8 / 53.

## 2. Where it lives

| fix | files :: functions | original |
|---|---|---|
| Port Options in the front end | `rc-game/src/menus/pause/port.rs` (`port_page`, `install_front_port_page`, the arrows `scroll_marks`), `pause.rs` | front Options lists 0x1b8d08 |
| Underwater tint over menus | `rc-engine/src/fog_state.rs::update_fog_state` (`Hud2dHook::replace_hud`) | `PageMenuModeRender` 0x21ab50 |
| Comet-Strike vs doors | `rc-game/src/hero/comet.rs` (`coll_line_m` with `ItemEnv::mobys`, flags 2), `hero/items.rs`, `tick.rs` | `CollLine` flags 2 |
| Gold-bolt hand items | `moby_update/classes/bomb.rs` (hide on 0x1413ff), `rc-engine/src/moby_attach.rs` (the hook) | 0x2c3300, 0x2dccd8 |
| Particle defs 32 / 60 | `particles/type32.rs` `def_first(8)`, `type60.rs` `def_first(53)` | def table 0x1b2500 |
| Giant Clank's pilot | `hero/bodies/giant.rs::after_update` (`World::joint_matrix(body, 5)`, `rows_euler`) | 0x2061f0 |
| Codebot slot | `moby_update/classes/units/quartu_small.rs::dispenser_update` state 2; `World::camera_flyby_phase`; `follow_camera.rs` `CameraView::flyby_phase` | 0x2ec030, `0x2f78e0` |
| Magician skill point | `units/kalebo_race.rs::update` (states 3+): `w.hero.board.score`, `w.svc.board.records.best_score[1]` | 0x2e6808 |
| Cheat tables | `menus/mod.rs::DATA_LABELS` (+ `cheats::label::{PATTERNS, LIST_SLOTS, LIST_LABELS, CODES}`); test `cheats::level_tables` | 0x179b80, 0x20a228, 0x20a248, 0x1ba020 |
| Grind aim | `hero/weapons.rs::aim_frame` (end of `arm_upkeep`), `Hero::aim_rows`; `hero/boots.rs::phys_grind` (the aim lean); readers: `blaster`, `pyrocitor`, `tesla` (beam, light), `devastator`, `suck_cannon`, `ryno`, `gloves::facing`, `classes/bomb.rs` release, `creature/react.rs` | 0x22f068 tail, 0x217970 (L00) |
| Wrench auto-aim | `hero/melee.rs` (`aim_assist` in the 0x13 / 0x14 entries and physics, `AIM_45/60/30`), `hero/comet.rs` | `0x2351d0`, `0x22e238` |
| Timers over menus | `rc-engine/src/scene_render.rs::subtitle_layer` (class texts and rings skipped while `replace_hud`) | 0x21ab50 |
| Warp-pad camera | `moby_update/services.rs::HeroFields::apply` (rows from the pose when a `HeroCall::Teleport` is queued); test `a_class_teleport_turns_the_moby_rows_before_the_camera_reset` | `HeroTeleport` 0x2368e0 |
| FOV after a scene | `rc-engine/src/scene_render.rs::leave_mode2` (`view_tan_x = 0.63`) | `FUN_002ac608` |
| Card icon | `memcard.rs::MemCard::in_use`, `menus/freeze.rs::busy_icon`, `rc-engine/src/hud_render.rs::tick_and_build`; test `busy_icon_tests` | `entry` 0x259c40 |
| Wrench as attacker | `moby_update/services.rs` (`HitRecord::wrench`, `attacker_class`, `attacker_pos`, `ServiceHits::mark_wrench`); readers in `classes/` (`orxon_lava`, `mine`, `kerwan_layer`, `kerwan_bystander`, `qwark_ship_parts`, `hop_gunner`, `pokitaru_biter`, `light_fixture`, `amoeboid`, `rilgar_biter`, `rilgar_flame_tank`, `batalia_tank`, `fleet_turret`, `gemlik_tower`, `gemlik_turret`, `drone`); test `hero_hits_with_no_attacker_are_the_wrenchs` | `FUN_0026e808`, 0x2be1c0 |
| Scout test | `tests/classes/orxon_gemlik_enemies.rs::scout_run` (O2 Mask owned) | — |

## 3. Behaviours to verify

### B1. Port Options from the main menu
- **Claim:** the front end's Options list (NTSC and PAL lists) ends with the port's page; it opens the same page as the
  pause menu's entry. When the list has more rows than fit (4), up / down arrows (icon 0xe99e frame 6 turned ±90°,
  6 × 10) show at the top / bottom while rows are hidden there.
- **Setup:** front end (`RC_FRONTEND=1`), Options.
- **Expect:** the entry present once; the page's rows; the bottom arrow on the first screen, the top one after scrolling
  down, both in the middle; none when everything fits.
- **Suggested method:** menu unit test on `install_front_port_page` (idempotent) and `scroll_marks(top, shown)`; QA.

### B2. No underwater tint over the page menus
- **Claim:** while a page menu replaces the HUD (pause, map, Options) the underwater tint is off; it returns on closing.
- **Setup:** any level with water deep enough to swim (Hoven, Pokitaru), Ratchet underwater.
- **Expect:** `UnderwaterTint` absent while the menu is open, present before and after.
- **Suggested method:** engine system test of `update_fog_state` with `Hud2dHook::replace_hud` both ways.

### B3. Comet-Strike bounces off doors
- **Claim:** the thrown wrench's ground probe and bounce use `CollLine` flags 2 against the mobys too, so a closed door
  (a moby) stops and bounces it instead of letting it pass.
- **Setup:** a closed door in reach (e.g. Blarg's or Quartu's sliding doors 67); crouch + □.
- **Expect:** the wrench turns at the door; with the door open it flies through.
- **Suggested method:** hero test with a `MobyScene` holding a box moby across the flight; QA.

### B4. Hand items hidden at a gold-bolt pickup
- **Claim:** while 0x1413ff is set (the gold-bolt pickup's hold) the bomb in the Bomb Glove (class bomb, `0x2c3300`)
  and the Swingshot's hook (`0x2dccd8`) are hidden, as the hand item is.
- **Setup:** Bomb Glove (or Swingshot) in hand; pick up a gold bolt.
- **Expect:** no bomb / hook floating during the pickup; back after.
- **Suggested method:** unit test of the bomb's hide condition; QA on Novalis's gold bolt.

### B5. Particle defs of types 32 and 60
- **Claim:** type 32 (the Glove of Doom canister's glow) uses def[8] (`*0x1b2520`), type 60 (the gold-bolt glints) def[53]
  (`*0x1b25d4`), not their own type's def.
- **Expect:** no large yellow halo round the canister on Rilgar and the other planets; the glints' sprite as before.
- **Suggested method:** unit test of the def index; visual compare on Rilgar.

### B6. Giant Clank's pilot
- **Claim:** while Ratchet rides Giant Clank he is placed at the body's joint list 5 matrix each tick (position from row
  3, Euler from the rows: `rows_euler`) and his matrix rebuilt (`0x2061f0`), not left where he got on.
- **Setup:** Quartu, the Giant Clank section.
- **Expect:** Ratchet on Clank's head/back through walking, turning and attacks.
- **Suggested method:** level harness on level 15 with body 2 (Ratchet's moby position = joint 5 translation); QA.

### B7. Quartu's Codebot slot waits for its fly-by (softlock)
- **Claim:** state 2 of class 1430 goes on only when `0x2f78e0` is true: the camera is not class 19, **or** the fly-by is
  in phase 1 with the fade 0x15f3fc ≠ 0 (the first ticks after the fade to black), **or** it is in phase 3. Then the
  piece 1428 is made 2 to the slot's side, Ratchet is teleported 3 away facing it in state 0x72, the first group shown,
  → 3. The fly-by's phase 2 → 3 clears 0x72 (`SetState(0, 1)`).
- **Setup:** level 15, flag 0x13d38a set (the Codebot), Ratchet on foot within 3 of the slot.
- **Trigger:** walk to the slot.
- **Expect:** sequence: arm (state 2) → camera class 19 (phase 0 fade in) → phase 1 → the slot's state 3 within a tick
  or two of phase 1, while the screen is still black; the Codebot slides out on camera; at the fly-by's end Ratchet is in
  state 0 and controllable (pause works); the door / group open; 15000 bolts after ten of the group are gone.
- **Edge cases:** no fly-by record (+0x00 = −1: class never 19 → goes on at once); a revisit (the slot's death bit →
  state 4: piece in place, no fly-by).
- **Suggested method:** level-harness test on level 15 driving the camera (`World::camera_class`, `camera_flyby_phase`,
  `cinematic.fade`) through the phases and asserting the order: teleport-to-0x72 before the release to 0. QA: the forum's
  gold-bolt door.

### B8. The Magician skill point (Kalebo III)
- **Claim:** in states 3+ the race host 1455 reads the race score 0x13fbf8 from the Hoverboard (`hero.board.score`);
  above 4499 it awards skill point 0x1a (0x13d422: sound, banner) once, and when Kalebo's best score 0x15ee6c
  (`records.best_score[1]`) is below 4500 it becomes the score.
- **Setup:** level 16, the host's race running (state 3+), `hero.board.score` 4500.
- **Expect:** skill point set, one banner; best score 4500; at 4499 nothing.
- **Edge cases:** best already ≥ 4500 (unchanged); the point already earned (no second banner).
- **Suggested method:** level-harness test; QA (a trick-heavy race).

### B9. Cheat tables on every level
- **Claim:** the move patterns (0x179b80), the Cheats page's slots / labels (0x20a228 / 0x20a248) and the debug codes
  (0x1ba020) read the same on all 19 levels (relocated through `DATA_LABELS`). On Veldin a lone left side-flip no longer
  toggles the mirror, nor a double jump a cheat; the Cheats page lists the cheats ever enabled.
- **Expect:** `cheats::level_tables::every_level_reads_level_01s_cheat_tables` passes (already written); a `MoveEntry`
  fed a single code 1 or 5 on any level toggles nothing; the mirror pattern (1 1 1 1 12 13 14 6 2 2 2 2 5 7) toggles slot 4.
- **Suggested method:** the existing test plus a per-level `MoveEntry::step` test; QA in New Game+ (double jumps, side
  flips: no "Cheat Enabled").

### B10. Ratchet's aim frame and the grind's aim lean
- **Claim (aim frame, `0x22f068` tail):** each tick the items' upkeep writes 0x13f990: in group 0xf (grinding) the Euler
  (0x13f3e0 x, −0x13f8d0, 0x13f8cc + 0x13f920) as rows; otherwise Ratchet's moby rows. The early returns of the arm
  upkeep (a layer freed, the arm taken down) skip the write.
- **Claim (aim lean, L00 0x217970):** on the rail, with a gun out (slot id ≠ 0xc, its def +0x18 ≠ 0, 0x1413fa) and the
  stick past 0.2: target = stick yaw − rail yaw, clamped ±45°, sprung (0.015, 0.3) into 0x13f920; without a gun or with
  the stick under 0.2 it springs to 0, and the stick (> 0.5) picks the side-lean animations only without a gun.
- **Expect:** firing on a rail: shots along the rail (not sideways); the stick sweeps them up to 45° either way (the
  Pyrocitor flame, Blaster shots, gloves' throws, Tesla beam, Devastator, Suck Cannon, R.Y.N.O.); off a rail everything
  as before (aim frame = moby rows).
- **Edge cases:** the wrench (id 0xc) never leans the aim; the lean spring's return when the gun is put away.
- **Suggested method:** hero tests on `aim_frame` (grinding / not) and `phys_grind`'s lean; QA on Kalebo III's rail
  (the mines and gate switches while steering).

### B11. Wrench auto-aim
- **Claim:** the wrench states call `0x2351d0` with its target search `0x22e238`: combo 0x13 entry and physics (5,
  45°, 45°); jump attack 0x14 entry (8, 60°, none) and physics (8, 60°, 60°); comet strike 0x15 entry (14, 30°, none)
  and physics (14, 30°, 45°). The best-scoring target (distance + yaw offset · distance, + 7 for a crate) turns the swing.
- **Setup:** a crate 3 ahead and 30° to the side; an enemy at 4 and 40°.
- **Expect:** the swing turns toward them (combo), not toward one outside the cone or range; with the stick held the
  stick's direction is the aim and the search runs from it.
- **Suggested method:** hero tests with `Carriers::melee` targets (`melee_aim_search_scores` is the model); QA with the
  PS2 video's crate rows.

### B12. Countdowns not drawn over the page menus
- **Claim:** the classes' 2-D draws (`draw_callbacks.texts`: Blarg's escape, the electrified water's air timer on
  Quartu / the Fleet, Veldin's countdown; `draw_callbacks.rings`: the Trespasser) are skipped while
  `Hud2dHook::replace_hud`.
- **Expect:** pause during Blarg's escape: the timer gone on the pause / map pages, back on closing.
- **Suggested method:** engine system test of `subtitle_layer`; QA.

### B13. Warp pads snap the camera behind Ratchet
- **Claim:** a class's `HeroTeleport` (`cinematic::hero_teleport`) builds Ratchet's rows from the new yaw when its stores
  land (`HeroFields::apply` with a queued `HeroCall::Teleport`), before the camera calls; `CameraResetBehindHero` then
  snaps behind the new facing. Pose stores without a teleport leave the rows alone.
- **Setup:** Gemlik (level 13) teleporter pads 1135; also Pokitaru's 318 and every other class teleport with
  `reset_cam`.
- **Expect:** after the beam, the camera behind Ratchet at the partner pad, looking the way the pad faces.
- **Suggested method:** the unit test exists; add a level-harness run of a pad pair with the camera and assert the
  camera's forward ≈ Ratchet's. QA on Gemlik.

### B14. The FOV after a scene
- **Claim:** a scene's end (`leave_mode2` = `FUN_002ac608`) sets the gameplay `view_tan_x` to 0.63.
- **Setup:** Pokitaru, the dogfight ending with ✕ held (the jet's FOV 85°); the next scene (any talk with a scene).
- **Expect:** wide FOV until that scene ends, normal after (the original also keeps the jet's FOV until then).
- **Suggested method:** engine test of `leave_mode2`; QA.

### B15. The card icon while the card is in use
- **Claim:** every frame `MemCard::in_use()` (state 0x13d364 > 2 or a requested state 0x13d36c ≥ 0) the 2-D pass ends
  with icon 0x755d frame 0 (64 × 64 at (50, 300), alpha 0x80) and frame 1 centred on it (82, 332), half size 17, turned
  −2π·(frame mod 55)/55; in every mode, menus included.
- **Expect:** `busy_icon_tests` passes (written); visible bottom left during an autosave (a gold bolt, a planet's
  arrival save), a save and a load; gone when the card is idle.
- **Suggested method:** the test; QA with `RC_SAVE_DIR` pointing at a scratch folder.

### B16. Orxon's path scouts wake their brawlers (stricter test)
- **Claim:** with the O2 Mask owned (Ratchet the hero) the scout #622 flies and its calls wake #651..#653 (states 2..9,
  not 0xfd). The other scouts: #623 → #658, #624 → #659, #625 → #662, #626 → #676 / #677 (then zapped by barrier
  generator 794 #488 on its path, as the original's zap lists any moby with collision), #627 → #689 / #690 / #693,
  #628 → #694 / #697.
- **Suggested method:** extend `scout_run` to the seven scouts.

### B17. The wrench knocks Orxon's bombs away and sets off placed mines
- **Claim:** a wrench swing or throw that hits a resting lava rock 938 on Orxon bats it (state 2: red, flown, burst), and
  one that hits a placed Mine Glove mine sets it off (`cmd = 1`), as the original's records name the wrench moby (class
  0x47). Clank's punch already named Clank and is unchanged.
- **Setup:** Orxon, the Clank area's rocks for the first; any level with the Mine Glove for the second.
- **Expect:** each reacts to one hit; the Blaster still bats the rocks too.
- **Suggested method:** harness test swinging at a rock and at a mine; QA against the PS2 clip of the forum post.

## 4. Shared code touched (regression risk)

- **`Hero::aim_rows`** replaces `moby_rows` in the weapons listed in B10. Off a rail it is a copy of the moby rows taken in
  the items' upkeep, so off-rail behaviour must be unchanged: re-run the weapons and gloves tests (`test-job weapons`).
  The Tesla Claw's body-point hit test and its arcs still read the moby rows (other functions; not confirmed either way).
- **`HeroFields::apply`** now rebuilds `rows` / `moby_rows` / `moby_rot` when a teleport is queued with a pose: every
  class teleport (story scenes, Hoverboard respawns, shuttles, pads) — the hero update's own `teleport` then rebuilds
  them again. Re-run the hero tests that teleport (`test-job hero`).
- **`World::camera_flyby_phase`** is new (0 off class 19); `CameraView` gained `flyby_phase` (its other constructors pass
  0).
- **Wrench auto-aim** changes every wrench swing's facing when a target is in the cone: the hero digest (NO_IDLE) and
  melee tests that place mobys near Ratchet may differ — intended.
- **`DATA_LABELS`** gained the cheat tables: the media page and `cheat_patterns` now read relocated addresses off level 01.
- **Wrench as attacker:** every class that tests the hit's attacker now sees class 0x47 (and Ratchet's hand item's
  position) for the hero's swings, where it saw none: Kerwan's bystanders flee, the Gemlik turret / tower test the side
  hit, the drones read the wrench's class type, the flame tank ignores the wrench, the fleet turret drops it. All are
  the original's reads; re-run `test-job classes`.
- **`leave_mode2`** writes `view_tan_x` — any class FOV set during a scene is reset at its end (faithful).
- **The card icon** is drawn in the HUD pass whenever the card is busy, menus included; the freeze dialog keeps its own.
- **Existing requests whose claims changed** (not edited, per the README):
  - `2026-10-01-minigames.md` B13 says "score word 0x13fbf8": the host now reads the Hoverboard's score, not a unit word
    (a test writing the unit word would no longer trigger the point).
  - `2026-10-04-quartu.md` Q4 "the dispenser slides out its piece": the piece now comes out during the fly-by.
  - `2026-10-01-media-cheats.md` B2 is level 01 only; the tables now hold on every level.
  - The Orxon scout test (`orxon_gemlik_enemies`) now loads with the O2 Mask.

## 5. Not ported (don't test as working)

- The wrench hits still name no attacker (the game names the wrench moby, `FUN_0026e808`): classes that need an attacker
  ignore the wrench — Orxon's lava rocks 938 cannot be batted, the Mine Glove's mines may not go off from a wrench hit.
  On hold (a patch exists; not committed).
- The aim frame's look-at branch (0x13f928, set by no ported class).
- The card icon on the frame the page menu opens (the game skips it, 0x1ba2ac) [L].

## 6. In-game QA spots (for the user)

- Quartu's gold-bolt door with the Codebot (B7): the scene plays, the Codebot comes out on camera, control and pause
  come back after.
- `RC_LEVEL=16 RC_HERO_AT=118,287,120,3.14 RC_GIVE_ITEMS=29,30` — the race host: a race with 4500+ trick points gives
  "Magician" (B8). On the same planet's grind rail with the Pyrocitor or Blaster: shots along the rail, sweeping with
  the stick (B10).
- New Game+ on Veldin: double jumps and side flips without "Cheat Enabled"; the Cheats page lists what was enabled (B9).
- Wrench swings beside crates and enemies turn toward them (B11).
- Pause during Blarg's escape or on Quartu's electrified water: no timer on the pause / map pages (B12).
- Gemlik's teleporter pads: the camera behind Ratchet after the beam (B13).
- Pokitaru's dogfight ended with ✕ held, then any scene: normal FOV after it (B14).
- `RC_SAVE_DIR=<scratch folder>`: the spinning card icon bottom left while saving / autosaving (B15).
- Front end → Options: Port Options last, the arrows while the list scrolls (B1). Underwater pause: no tint over the
  menu (B2). Comet-Strike at a closed door (B3). Gold bolt with the Bomb Glove out (B4). Rilgar's Glove of Doom canister
  (B5). Quartu's Giant Clank (B6).

## 7. Results (the test expert fills this in)

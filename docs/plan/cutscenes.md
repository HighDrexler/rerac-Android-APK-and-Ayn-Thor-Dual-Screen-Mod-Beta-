# Cutscenes: the kinds, how they are driven, and what the port plays (RAC1 NTSC)

Addresses are **level01.elf** unless marked. Confidence: **[H]** read in the code and checked on disc data or in a
headless / engine run, **[M]** read in the code only, **[L]** inference. The detailed format and timing of the
mode-2 scenes and of the level transitions stay in `cutscenes_transitions.md`; this page answers "what kinds of
cutscene are there, and are they one system" and documents the in-level cinematic port (2026-09-27).

## 1. Answer in brief

The game has **three distinct kinds** of cutscene, and **no script system**. Two of them share nothing but the
call that starts them. (The same holds for level logic in general: no script VM or data, only each level's own class code
calling engine helpers; docs/plan/level_scripting.md.)

| kind | examples | started by | data | game mode | camera | player | skip |
|---|---|---|---|---|---|---|---|
| **A. Scene** (in-engine, pre-authored) | Novalis arrival (scene 5), the Plumber (scenes 0 / 1), the Novalis mission's scenes 3 / 4 | `DialogStreamStart(k)` 0x2ac330 from a class (mission NPC 730/790, talking NPC 774, infobot 750, ship, menu replay) | level header scene record k: 30 Hz animation chunks with a per-tick **camera track**, **actor animation + position tracks**, subtitles, and a speech VAG per language | **2** (the gameplay tick stops; `CutsceneModeUpdate` 0x2aca80 runs instead) | the chunk's camera record, 60 Hz | state 100, hidden; `SetState(0,1)` at the end | Start + L1 L2 R1 R2 after 18 ticks (Start alone once the game is beaten / from the replay menu / level 0) |
| **B. Cinematic cutaway** (live gameplay, scripted by one class) | door / bridge cutaways, the gunship strafe over the bridge, the troop drop-in, the gold bolt, the bolt crank's door view, the vendor | a moby class's own update, calling the camera and hero functions directly | **none of its own**: the class's pvars, cuboids (camera placement = a cuboid's centre + Euler angles), timers | **0** (the world keeps running: enemies, shells, doors move) | the **script camera** (camera type 5): `CameraScript` / `CameraScript2` | hero state **0x72** (held, no control), letterbox bars, HUD hidden (global `0x15f404`) | **none** |
| **C. Movie** (FMV, PSS) | new-game opening, Veldin → Novalis flagship scenes, in-level holofilms (the Novalis mission's movie 3, infobot movies), extras, title attract | in level: `DialogStreamUpdate(n)` 0x2acf50 → `StartPssMovie` 0x2ad0c0; between levels: `fun_00231608` from `DoSpaceTransition` 0x2a68f8; title: boot 0x1e9488 | `mpegs[88]` in the TOC: MPEG-2 program stream, 512×416 30 fps video + SShd SPU-ADPCM 48 kHz stereo (docs/formats/pss.md; played natively, §5) | **1** (`MovieModeUpdate` 0x2ad498 blocks the game loop) | – | – | the same Start / Start+shoulders rule (inside `readMpeg` 0x31a308); any button in replay mode 2 |

**Your examples:**
* *Ships swoop in and shoot holes in the bridge* → kind **B**: the gunship 688 instance 695's own scripted state
  (`GunshipUpdate` 0x2f7728), with the script camera on cuboid 23 (§3.3). Not a scene file.
* *Reaching the Plumber* → kind **A**: scenes 0 (2318 ticks) and 1 of Novalis, started by the talking NPC 774's dialogue
  (`FUN_0027b028` / `0x27b550`, node scene id); the same player as the arrival scene.
* *The random door-opening cutscenes* → kind **B**: camera trigger 737 commanded by another class (on Novalis the
  mission NPC 790 lowers the hinged bridge 746 and shows it through trigger 863 for 3 s, §4).
* *Connected to the Novalis intro?* Only kind A is: the arrival is scene 5, played by exactly the same code as the
  Plumber's. Kinds B and C do not use the scene files at all.
* *Movies like the first Drek holofilm* → kind **C**, special in every way: a separate game mode, a separate file
  format (PSS on the disc root table, not in the level), no in-engine rendering. In-level holofilms are played
  between two scenes by the class that owns the story beat: the infobot 750 (`InfobotUpdate` 0x2fbf80 states 8–10:
  scene pvar+0xc → movie pvar+0xd → scene pvar+0xe) and the Novalis mission NPC (scene 3 → movie 3 → scene 4).

**Shared code (clusters.tsv):** the scene player (`DialogStreamStart` / `CutsceneModeUpdate` / end `0x2ac608`)
and the movie player are engine code in all 19 overlays; the script camera (`CameraScript` cbb1cc1f, update
4b806e22, `CameraScript2` d0b32970) is in 19 / 19 / 16 overlays; the camera trigger 737 (5c26590d) is on levels 1, 2,
3, 7, 10, 13; the gunship (7751f2bf) and the mission NPC (2c0aee80) exist only on Novalis.

## 2. Kind A: scenes (mode 2) — the arrival scene's system

Unchanged from `cutscenes_transitions.md` §1–§4 (format, player, audio, skip) and already ported
(`rc_formats::scene`, `rc_game::scene_player`, `rc-engine/src/scene_render.rs`, `rc_game::audio::scene`). What
changed now is **who starts them**: the engine no longer hard-codes the Novalis trigger; it plays the scenes the
classes ask for (`rc_game::cinematic::EngineRequest::StartScene`, and the talkers' `interact::Handoff::Scene`) on
the frame after the tick that asked, which for the arrival is the same frame as before (app frame 3; scene tick 1 on
frame 14; control back on frame 1532) [H: engine log]. Scenes asked for while one runs are queued.

End of a scene (`0x2ac608`), now applied by the engine: Ratchet `SetState(0, 1)` (queued on the hero-block channel for
the next tick), and for a talker's scene the teleport in front of it (`Interact::scene_end_place`, 0x16cd26) and its
dialogue refresh (`Interact::scene_ended`, `FUN_0027b550(npc, rec, 1)`).

Novalis scenes: 0 the Plumber / Water Pump Worker 774 (auto node, 2318 ticks), then after buying the Infobot scene 1
→ movie 2 (`mpegs[4]`) → scene 2 (his talk tree, `interaction.md`); 3 the mission's hand-over (811 / 750 / 731, 1078), 4
the Qwark-ad reaction with ship 531 (1536), 5 the arrival (1508), 6 = scene 0 without audio.

**`RC_SCENE`** only concerns the arrival: `RC_SCENE=0` drops the arrival request, `RC_SCENE=<k>` plays scene k after
gameplay tick 1 instead; every other hand-off (talkers, the mission's scenes 3 / 4) always plays, and a scene that
cannot be loaded is reported as ended (skipped) to its talker. Checked: `RC_SCENE=0` in front of the Water Pump
Worker plays scene 0 from frame 3 (2342 frames), puts Ratchet in front of him and his dialogue advances to
"△ Buy Infobot for 500 bolts" (frame 2450).

## 3. Kind B: in-level cinematics — the script camera and its callers

### 3.1 The engine functions every caller uses [H]
| function | effect |
|---|---|
| `CameraScript(pos, euler, mode, ticks, collide)` 0x316ef8 | the type-5 camera switched in with `FUN_0020d110` (a cut) and its init 0x3171b8: position spring k 0.3 / damping 0.3 / max 1 per axis, Euler spring k 0.05 / 0.3 / 1; camera and targets = (pos, euler); mode +0x88; timer D+0xe0/e4 (modes 2, 3). Mode 3 instead keeps the current view and records its (yaw, elevation, distance) about Ratchet |
| update 0x317670 → 0x317278 | mode 0 spring toward the targets; 1 snap; 2 cosine-eased move applied to the current pose each tick; 3 timed swing about Ratchet (anchor eased to him, yaw eased — the long way round when the short way and the target view disagree by > 20° —, elevation eased, distance on the Hermite curve `0x26cc00(D+0x124, 0, 1, D+0x128, t)`, Euler eased), then hold the targets. Rows = `EulerToMatrix(euler)`. `collide` → sphere push `0x20f2a8` r 0.5 (no Novalis caller) |
| `0x316dd0` / `0x316e28` / `0x316e88(a, b)` | target position / target Euler / mode 3's curve slopes |
| `CameraScript2(kind)` 0x317070 | release: 0 = the script camera jumps 1 behind / 1.2 above Ratchet, then the follow camera is switched in **cut** (`+0x7e = 1` → `+0x8e = 1`) and snaps behind him; 2 = **blend** (0x167370, rates 0.018); 4 = blend from the copied pose (0.018, 0.01 on level 1); 1 = pose copy, no blend [1 / 4: M] |
| `0x15f404` | 1 = letterbox + HUD off. `DrawScreenFade` 0x21b7d8 (in the HUD layer, after `HudDraw` and `DrawHud2D_B`): bar height 0x15f408 +1 px per frame to 24 while set, −1 per frame after; two opaque black bars (PRIM 0x104 strip, RGBA 0x80000000) at the top and bottom of the frame. `HudDraw` 0x24fb50 skips every slot and the banners while set. Also read by the gunship shells (`0x2f6a30`: fewer particles, shells hit mobys) |
| `SetState(0x72, …)` / `HeroTeleport(pos, euler, 0x72, 1)` 0x2368e0 | the hold: group 9, 0x1413fc = 1 (no control), idle anim over 10 ticks, idle physics, no transitions. HeroTeleport also clears the motion block and resets the follow camera behind him |

The camera types (table 0x20c480 {type, activation, init, update, release}): 0 follow, 3 (0x315358), 4 first person,
**5 script**, 6, 7 (region cameras), 0x11. Type 5's activation 0x317668 returns 0: only `CameraScript` brings it up.

### 3.2 Camera trigger 737 (`CameraTriggerUpdate` 0x2fb5b0) [H] — the generic cutaway
Pvars: P[0..7] target pos / Euler, P[8] trigger cuboid, P[9] camera cuboid, P[10] Ratchet's cuboid (−1: stays), P[11]
hold ticks, P[12] 0 = cut to P[9] / else move there over P[13] ticks (mode 2). Fires on Ratchet's feet in P[8] **or
moby+0xbc = 1** (another class's command, usually with P[11] set); holds while the timer runs and +0xbc = 1
(re-targeting the camera from P[0..7] each tick, so a commander can aim it); releases with `CameraScript2(0 / 2)`,
`SetState(0, 1)` (`FUN_002405a0`; 0x53 in hero mode 3), `0x15f404 = 0`; then only a new command re-arms it. Spent
when its spawn id is collected or its death bit set. Novalis: 860 (cuboid 42 → camera 43, Ratchet to 42), 863
(camera 51), 859 / 861 / 862 (no cuboid, no commander on the disc: never fire).

### 3.3 Gunship 688 fly-by (`GunshipUpdate` 0x2f7728) [H] — the bridge strafe
Instance 695 (P+0x170 = cuboid 22 at the bridge's west end, P+0x174 = camera cuboid 23). On Ratchet's feet in cuboid
22 (and not seen before: spawn-id / death bit): shown, `SetState(0x72, 0)`, `CameraScript(cuboid 23, 0, 0, 0)`,
`0x15f404 = 1`, timer ticks(510); the update body re-runs within the same tick until the timer is at 390 (the ship
jumps 120 ticks ahead on its path), then flies and fires at the target cuboids 19 / 20 (the bridge); with 60 ticks left
it wakes the troopers; at 0: `CameraScript2(0)`, `SetState(0, 1)`, flag 0, killed + death bits, deleted. **The strafe
is the gunship's own scripted state**: the enemies agent owns the class (`classes/gunship.rs`); it queues
`creature::ScriptRequest::{Start, End}` and sets `creature::Globals::cutscene` (= 0x15f404). The cinematic layer turns
the requests into exactly the calls above (`cinematic::from_creature`) and keeps them in `Cinematic::creature_log`
(the last 64, for reports and tests: `creatures_enemies_novalis::novalis_gunship_bridge_fly_by` checks one Start
{0x72, cuboid 23} then one End by the gunship). What the camera side expects of the gunship:
Start once on the trigger tick with the cuboid 23 centre / Euler and hero state 0x72; End once when the timer runs
out; the flag set / cleared with them. Instance 694 (P+0x170 = −1) is an ambient gunship with no cutscene.

### 3.4 Other in-level cinematics (not on the Novalis critical path)
* **Gold bolt 1134** `GoldBoltUpdate` 0x307ca0: pickup cutaway, `FadeToBlack(ticks(10))`, `CameraScript(beside
  it, 1 = snap, …)`, the targets re-set each tick (an orbit about Ratchet or a move between two cuboids),
  `HeroTeleport` + `CameraScript2(0)` at the end, flag 0x15f404 [H]. Ported 2026-09-28 (hero_gameplay.md §6).
* **Bolt crank 280** 0x2e0c68: `CameraScript(…, 3, ticks(180))` + `0x316e88`: the swing to the door / rotator it
  drives, with `0x316e88(pvar+0x28, +0x2c)` (`cinematic::camera_curve`); `CameraScript2(4)` on leaving. The class is
  the crank agent's; it calls `rc_game::cinematic`. Engine check (the crank agent's `engine_script_stop180.txt`,
  frames 45 / 80 / 140 / 215): the camera swings from behind Ratchet round to the door over the 180 ticks, then
  blends back.
* **Path enemy 459** P+0x1c4 camera cuboid (no Novalis trooper has one), **vendor 11** (`OpenVendorMenu`), 
  **collapsing platform 701** state 4 (`CameraScript2(0)` + `SetState(0,1)` when P+0x8c ≠ −1: a leftover with no
  starting call; instance 711 has P+0x8c = 24, so the game resets Ratchet to idle once its debris timer runs out
  after the fall [M]; the class belongs to the world agent).

## 4. The Novalis mission (classes 730 / 790, `MissionNpcUpdate` 0x2fad68) [H]
Class 790 (instance 905 at (189.2, 203.8, 40.6), mission 0) is the **Blarg troop dropship** of the first Novalis
mission (the catalogue's "mission NPC" name is Lombyte's); 730 (instance 858, pvar +0x04 = −1) runs the same code
without a drop-in camera. Sequence (§ table in `mission_npc.rs`):
1. Load pass: hidden 30 units up. **Tick 1**: mission open, mode 0, flag 0x13d397 clear → `DialogStreamStart(5)`
   (the arrival scene), flag set. Every tick until triggered: the ship on the pad hidden (`FUN_002a2450`).
2. Ratchet in cuboid 42 → trigger 860 fires (Ratchet to cuboid 42's centre, 0x72, camera at cuboid 43) and the ship
   commands it for ticks(420), aiming cuboid 43's Euler at itself; it drops at 5 u/s (≈ 6 s) while 860 re-aims the
   camera every tick (spring mode 0: the camera turns to follow it), lands (anim 1 → 2), and the troopers 325–327
   attack. After 420 ticks: control back.
3. All three troopers dead → 45 ticks → scene 3 → movie 3 (`mpegs[5]`) → scene 4 → `UnlockPlanet(3)` (Kerwan) +
   banner, the bridge trigger 863 commanded for ticks(180) and the hinged bridge halves 869 / 870 (class 746,
   `HingedBridgeUpdate` 0x2fb8a8) commanded (+0xbc = 2): they swing from 36° to 0° in 2 s about a pivot 11 below their
   origin (into the arch) while the camera shows them; `SetMissionDone(0)`, checkpoint at cuboid 55 (the landing
   pad), save, the ship shown.

## 5. Kind C: movies (FMV) — as built (U10 decided: native, from the original files)

**Files [H].** `mpegs[88]` (TOC 0x17f8, `{lsn, bytes}`), archived raw as `global/mpegs/NNN.bin` (84 files, 2.7 GB,
NTSC and PAL copies) and read as such at play time: nothing is converted at extraction and the product has no
ffmpeg (decisions.md, 2026-09-27). Format, audio channels per language, the header survey and the decoder:
`docs/formats/pss.md`. In short: MPEG-2 MP@ML 512×416 (30 fps NTSC, 25 fps PAL), 4:2:0 frame pictures (82 progressive
files; the two 437 s extras 73 / 78 interlaced with field prediction, field DCT and the alternate scan), audio in
private stream 1 (`FF A1 00 cc`, cc = language channel) as `SShd`/`SSbd` stereo SPU ADPCM, 48 kHz (44.1 kHz in 0, 1,
45, 57), interleave 0x20.

**The game's flow (level01) [M unless noted].**
1. `DialogStreamUpdate(n)` 0x2acf50 (n ≥ 0; from a class update, inside the moby loop): HP 0x1415f8 = 0 → the death
   fade `0x2319b0` instead; else if not dying (0x141401 = 0) → `StartPssMovie(mpegs[2 + n].lsn, .bytes, language
   0x15ed88)` (PAL: `mpegs[21 + n]`). The menus use `PlayMovieB` 0x2ad050 (transitions table) and `PlayMovieC`
   0x2acfe8 (extras, language 0).
2. `StartPssMovie` 0x2ad0c0: flag 0x13e5bb |= 8, `sound_StopAllSounds` 0x2a1b08 (989snd `snd_StopAllSounds`, every
   sound slot 0x13e550 + 0x70·i freed), `music_Stop` 0x27a2a0 (players / streams reset, a pending music-box track
   becomes the track), `FadeToBlack(4)` (blocking: black quads over the last image), `FUN_002258b0`, **mode 1**
   (0x15f5c4), VRAM saved (13 + 17 DMA chunks, GS syncs, no vsync). The rest of the tick runs.
3. Main loop, mode 1 → `MovieModeUpdate` 0x2ad498: stops sounds and music again, `snd_stream_safe_cd_sync`, waits
   for the memory card, then `fun_0023a3b8` 0x31a260 → `initAll` 0x31a668 (sceMpeg, video decoder thread, ADPCM
   channel = language, SPU movie sound `snd_init_movie_sound(0x400, 0x1000, vol 0x400, pan 0, group 5, 3)`) →
   **`readMpeg` 0x31a308** (blocking): reads the file 64 KiB at a time, decodes (IPU) into a 2-picture output
   buffer; when the buffer is full **and** 4 KiB of audio are buffered it calls `startDisplay` (waits a vsync) and
   `audioDecStart` together; the vblank handler 0x31b280 shows each decoded picture for one field pair (two vsyncs)
   — so pictures and audio start on the same vsync and the picture rate is the vsync rate / 2 (no PTS is used).
   Every loop it reads the pad and tests the skip rule; skip or end of data → flush, `endDisplay`,
   `audioDecReset`, master volume group 5 restored.
4. `MovieExitToGameplay` 0x2ad2b8: VRAM restored, (replay mode 2: 0x15eed8-related flag cleared),
   `FadeToBlack(4)` over the last picture, `music_start_track(0x151708, 1, 0x400)` (the level track from the start),
   **mode 0**, the talker (0x179588) refreshed with `FUN_0027b550(npc, rec, 1)`, flag 0x13e5bb |= 0x10.
   Between levels, `DoSpaceTransition` plays `mpegs[40..50]` through `fun_00231608` → the same `fun_0023a3b8`
   (cutscenes_transitions.md §4.3, §6).

**Skip [H: code].** In `readMpeg`: never while 0x15eed8 = −1; replay mode 2 (0x15eed8 = 2): any newly pressed
button; Start (pressed 0x13cae4 & 0x800) alone when the game is beaten (0x15eea0), completed (0x15ee20), replaying
(0x15eed8 ≠ 0) or on level 0; otherwise Start pressed with L1 L2 R1 R2 held (0x13cae0 & 0xf). No minimum time
(unlike the scene's 18 ticks).

**Screen, audio, mode.** Screen: `setImageTag` 0x31b0b8 uploads each 512×416 picture at (0, 0) of the display
buffer, so the movie fills exactly the picture the game's frames fill. Audio: all game sounds and the music stop
for the whole mode; the movie's stereo ADPCM plays on two SPU voices (group 5 = the sfx option). Mode 1: no game
tick, no HUD, no pause menu.

**In the port (2026-09-27).**
* `rc_formats::pss` (demuxer, `SShd` audio, decode with the existing `vag::decode_frame`); `crates/rc-video` (our
  MPEG-2 decoder, IEEE 1180 IDCT, BT.601 conversion, `Movie::open`); `rc_game::movie_player` (the frame sequence,
  the audio clock, the skip rule); `rc_game::audio::AudioSystem::{movie_stop, movie_exit, mix_movie}`;
  `rc-engine/src/movie_render.rs` (request queue, decoder thread, UI picture, fades, audio push);
  `scene_render::play_movie(n, npc)` hands the classes' / talkers' requests to it.
* Frames: the request is taken the frame after the asking tick (like scenes) and the tick is suspended from that
  frame (`GameTick` `run_if`, menu mode `Mode::Movie`); 4 `FadeToBlack(4)` frames (coverages of
  `scene_player::fade_to_black_coverage`) + 1 black; the movie; 4 + 1 exit fade frames over the last picture; the
  next frame runs `MovieExitToGameplay` before the tick, which runs again that frame. The audio is the clock: vsync
  k of the movie outputs samples `800k .. 800k + 800` and shows picture `(800k − latency)·fps/48000` (latency = the
  audio ring's fill in live runs, 0 in frame-exact runs), i.e. picture k/2 at 30 fps; both come from the vsync
  count, so they cannot drift, and the device clock is absorbed by the audio ring.
* Audio level [L]: each channel on its own voice panned hard left / right with the stream law at vol 0x400, group 5
  at the sfx option: unity gain at the default sfx 0x400 (measured 0.9999 against the decoded PCM).
* Picture: a UI image (nearest sampled) over the main camera's letterboxed viewport, the same 512:416 rectangle as
  the game frame and the HUD, so the movie keeps the game's proportions and placement whatever the window; the
  window outside it is the letterbox. (The port shows the draw buffer with square pixels, as for gameplay; if the
  display moves to a 4:3 non-square pixel aspect, the movie follows because it uses the same viewport.) The fade is
  a black UI quad with alpha `1 − (1 − c)^2.2` so the linear-light blend gives the GS's `C·(1 − c)` in display bytes.
* Decoding runs on a worker thread (read, demux, audio, then pictures as RGBA through a 4-deep channel); the frame
  that needs a picture waits for it, so frame-exact runs are identical.
* Dev hooks: `RC_PLAY_MOVIE=<n>` (in-level movie n after gameplay tick 1), `RC_MOVIE_SKIP=<k>` (skip combination on
  movie frame k), `RC_MOVIE_TRACE=1`.
* **Checked (engine, frame-exact, scratchpad `movies/`):** `RC_SCENE=0 RC_PLAY_MOVIE=3` (the Novalis holofilm,
  `mpegs[5]`): request on app frame 2, fade 2–6, picture 0 and audio sample 0 on app frame 7 (WAV: the movie's PCM
  found at exactly 6 × 800 samples, gain 0.9999, residual 3e-8), pictures at 2 vsyncs each; frames 60 / 1200 / 2400
  show pictures 26 / 596 / 1196; two runs give byte-identical PNG and WAV. `RC_PLAY_MOVIE=15` (`mpegs[17]`, 17.4 s)
  plays through: 5 + 1044 + 5 movie frames, 522 of 522 pictures, gameplay and the level music back on frame 1056.
  `RC_MOVIE_SKIP=300`: exit fade from the next frame, gameplay on frame 307. Headless: `movie_player` tests (skip
  rule, timeline, clock), rc-video tests.
* **Not yet:** the between-level transitions (no `DoSpaceTransition` in the port yet: `mpegs[40..50]` wait for it;
  the player takes any file index), the menu replay (`PlayMovieB/C`, replay modes), the title attract loop; PAL
  file selection (the port is NTSC); a scene request arriving while a movie plays is not held back by the scene
  player (never happens on the Novalis path).

## 6. In the port (2026-09-27)

**rc-game**
* `follow_camera/script.rs`: the script camera (type 5) — `Camera::camera_script`, `camera_script_targets`,
  `camera_script_curve`, `camera_script2`, `script_active`; modes 0–3; release kinds 0 / 2 / 4 / 1. Hooked into
  `Camera::update` (the script camera is the active camera while up; no activation check runs). Native `f32`.
* `cinematic.rs`: the call layer for classes (`camera_script`, `camera_targets`, `camera_curve`, `camera_script2`,
  `letterbox`, `hero_state`, `hero_teleport`, `start_scene`, `start_movie`) and `EngineRequest`; the tick applies the
  camera calls right after the moby loop (`MobySystem::take_cinematic`), the hero calls go through
  `HeroFields::call` / `pose` / `clear_motion` (HeroTeleport keeps only the yaw of the Euler; every Novalis caller
  passes 0 pitch / roll).
* `hero/scripted.rs`: state 0x72 (entry, idle physics, no transitions).
* Classes: `camera_trigger.rs` (737), `mission_npc.rs` (730 / 790), `hinged_bridge.rs` (746).
**rc-engine** (`scene_render.rs`): scene / movie requests from the classes and the talkers, the scene end's
`SetState(0, 1)` / teleport / dialogue refresh, `SetMissionDone`, the ship hide / show, the movie hand-over to `movie_render` (§5), the letterbox
bars and the HUD off while `0x15f404` (`creature::Globals::cutscene`). `RC_DEBUG_KILL=t` (dev check): the mission's
three troopers deleted before tick t. With `RC_SCENE=0 RC_HERO_AT=159.5,208.44,40 RC_DEBUG_KILL=500` the whole mission
runs in the engine: drop-in cutaway from the first ticks for 420 ticks, scene 3 from frame 548, the movie (then a stub: now §5 plays `mpegs[5]`, 48 s, which moves scene 4 later), scene 4 from 1652, the bridge
cutaway ≈ 3215–3395 (frame 3290: the halves mid-swing under the bars).

**Verified (headless, `crates/rc-game/tests/world/cutscene_novalis.rs`, Novalis from `extracted/`):**
* the arrival scene is requested once, in tick counter 1, with the ship hidden;
* drop-in: trigger on the tick Ratchet is in cuboid 42; Ratchet at the cuboid centre, state 0x72, **420 ticks** held
  with the stick pushed (he does not move), letterbox flag, camera exactly at cuboid 43 and turning down after the
  dropship (forward z 0.61 → −0.07); released: state 0 (then walking), flag off, trigger state 3;
* after the kills: scene 3 → movie 3 → scene 4 → mission done; bridge cutaway 863 for **180 ticks**, camera at cuboid
  51, the bridge flat after 120 ticks, Ratchet held throughout, then released;
* gunship: cuboid 22 → script camera at cuboid 23 for **390 ticks**, 0x72 and letterbox throughout, then released;
* two runs identical tick for tick; the `novalis_hero_digest` guard byte-identical.
**Engine** (frame-exact, captures in the agent scratchpad `cutscenes/`): arrival run unchanged (frames 3 / 14 / 194 /
1532), music held through the scene and resumed at 1564, speech present (WAV correlation ≥ 0.99 per second with the
previous scene capture); drop-in / bridge / gunship frames show the bars, no HUD, the camera placements; two runs give
identical PNGs and WAVs.

**Not yet:** kind C between-level / menu movies (§5); memcard save (logged; the in-memory game state holds every
write). Done since (hero_gameplay.md §6): the gold bolt cutaway, `UnlockPlanet` + `ShowPlanetBanner` into the game
state and the HUD, a gameplay class's `FadeToBlack` (`scene_render::FadeHold`), the talker scene → movie → scene
chain in the engine (the Water Pump Worker's Infobot); the collapsing platform's stray release; `CameraScript2` kinds 1 / 4
pose copy details. Still not ported: the script camera's collision push `0x20f2a8` (no Novalis caller passes `collide`,
`follow_camera/script.rs`; a consumer would file it under G-HERO-026). (The FX driver 1546 and the rest of the mode-2 world: §7.)

## 7. The scene pass (2026-09-27): data coverage, what runs in mode 2, the one general player

### 7.1 Scene data coverage [H]

Every scene on the disc parsed and checked byte by byte (`crates/rc-formats/tests/formats/scene_coverage.rs`, behind
`test_data::root()`): **138 level scenes** in 19 levels (15 records each; 275 NTSC / PAL regions) plus the **48 global
lumps** (TOC `anim_looking_thing_2` = space take-off / landing scenes, `things` = item scenes, mode 6, 0x800-byte
`{offset, size}` table: `scene::lump_chunks`, `Scene::from_lump`): 323 regions, 4,315 chunks, 366,100 ticks,
370,415 camera records (1,938 cuts, 2,242 FOV steps), 17,829 actor records, 780,229 animation frames, 4,284
subtitle entries.

**There is no command, event or particle stream in a scene.** A chunk is exactly: header, one camera record per tick,
the actor records, the subtitle table; every byte of every chunk is inside one of them, zero padding, or the authoring
tool's trailing filler.

| structure / field | values on the disc | read by the game | port (`rc_formats::scene` → player) | before this pass |
|---|---|---|---|---|
| header +0x00 end tick | per scene | yes (0x16cd20) | end rule | handled |
| header +0x02, +0x0e | always 0 | no | ignored (test pins 0) | parsed, unchecked |
| header +0x04 subtitle offset | < 0x400 = none, else table | yes | subtitles | handled |
| header +0x08 audio start | −6 or −8 | yes (0x16cd28) | speech lead (the start overwrites −3) | handled |
| header +0x0a | always −1 | no | ignored (pinned) | parsed |
| header +0x0c actor count, +0x10 camera offset | | yes | | handled |
| camera eye / angles X Y Z / tan(hfov/2) | tan: 1 value in most scenes, **animated FOV** in 11 (e.g. L03 S01 41 values, L05 S10 619) | yes (`FUN_002ac8d8`) | per tick, main **and sky** camera | FOV applied to the main camera only (sky warp) |
| camera +0x0c word | 0 or 1 (cut flag) | yes (byte) | the odd-tick snap | handled |
| actor +0x00 class | stable across a scene's chunks | yes (`CreateMoby`) | actor spawn | chunk-0 actors only (same set) |
| actor +0x04 | one value per scene: its chunk count as authored (NTSC count, also in most PAL copies) | no | ignored (pinned) | "unknown" |
| actor +0x08 | the chunk index | no | ignored (pinned) | "0" |
| actor +0x0c position track | one vec4 per frame, w = 0 | yes | position lerp | handled |
| actor sequence header | loop sound 0, trigger count 0xff (no trigger words), +0x13 0xff, no trigger data, no rate override | only frame count (actors are never advanced: mode 2) | parsed, ignored | handled |
| actor frames | joint count constant per sequence; ≠ class joint count in 3 cases (L01 S06 Clank 72 / 75, L11 S07 1246 47 / 44, L16 S03 1509 6 / 2) | yes | evaluator | handled |
| subtitle entries + terminator (a whole 16-byte entry, start = end = −1) + strings | 4,284 entries, pad 0 | yes | first match drawn | handled |
| trailing filler (296 chunks, KiB-sized, fixed junk pattern `e4 d5 d9 36 …`) | never pointed at | no | ignored (pinned pattern) | not known |
| actor classes not in the level | 11 scenes, all without speech: L00 S06, L04 S03, L06 S04, L07 S05, L09 S01, L12 S05, L12 S08, L14 S07, L15 S06, L16 S03, L17 S03 | — (no code starts them) | listed; the other actors still drawn | the whole scene's actors dropped |
| actor class blend | no scene actor class has mode bit 0x200 or 8 | — | plain draw (`MobyBlend::pick`, checked) | plain |

The conformance test fails on any unknown value, stray byte, class change, joint-count change, a missing class in a
scene with speech, or a blend mode bit on an actor class. It found the subtitle terminator's size, the PAL +0x04 rule
and L07 S05 on its first runs.

### 7.2 What runs during a mode-2 scene [H]

`CutsceneModeUpdate` 0x2aca80, gated by the debug mask 0x16c4e0 (0xf from `entry` 0x259c40, so all on), in order:
`FUN_002ab920` (clears the frame's draw-callback lists), **`MobyUpdateLoop` 0x2793d8** (every active moby: anim advance,
its update, `MobyBuildMatrix`; the active list 0x265548 measures update distances from 0x167240 = the scene camera),
`RunLevelCallbacks` 0x2a1a18, **the hero update** 0x228870 (Ghidra name `PatchShrubGifs`; state 100), **`UpdateParts`**
0x27c7e8; then the scene: fade, tick++, audio, end / skip, chunk roll, camera `FUN_002ac8d8`, the actors (frames, blend,
position, +0x71 = 0xff, `MobyBuildMatrix`, `FUN_0026f0e0` shadow probe if +0x7f, class 0 `FUN_0024a1d0`, classes
10 / 419 / 1365 `FUN_00228bc0`); then `sound_update`, `UpdateAllPointLights`, `UpdateShadowDir` 0x264988,
`IncrementTickCounter`. **Not** run (only `InLevelFrameUpdate` 0x2aba68 has them): `MobyFreeSlotBookkeeping`,
`LevelAmbienceUpdate`, `FUN_00220928` (glints), `CameraUpdate`, `HudUpdate`, `Help_Update`.

* **Actors** are real mobys: `FUN_00259288` `CreateMoby(class)` per record into 0x16ce58[k] (only once per slot),
  mode |= 6 (bit 2: skipped by the moby loop, so no `MobyAnimAdvance` and no class update; bit 4: the loop keeps its
  matrix), +0x32 = 0x1ff, +0x72 = 0xff, +0x94 = 0, lights +0x38 from Ratchet; the streamed sequence is a new class slot
  (class+0xc++). Deleted at the end (`FUN_002ac608`). Existing level mobys are **not** bound: a scene's Ratchet, Clank,
  Plumber are separate actor mobys; the game hides the real ones: Ratchet and his items (`FUN_002486c0`; the always-on
  extras 33 / 34 / 35 excepted: `FUN_0024a0e8` clears their hidden bit each tick so they ride the actor), the talker
  0x179588 and 0x1ba25c (mode |= 1), every moby of classes 74 / 203 (mode |= 0x80). Nothing is streamed but the chunks:
  every actor class must be in the level (or a spaceship class).
* **Visibility / spawn / despawn**: none in the data; actors exist for the whole scene (parked out of view when not
  in a shot: e.g. scene 3's Ratchet starts at (82.8, 95.5, 18.7)).
* **Effects** are not in the scene: an ordinary class, the **cutscene FX driver 1546 `CutsceneFxUpdate` 0x30c190**
  (Novalis instance 976), reads the scene id 0x16cd10, tick 0x16cd14 and the actor slots and calls the general
  spawners: scene 5 (arrival) the ship's glowing trail (`FUN_00278810` type-23 puffs on joint lists 1 / 2, ticks < 120)
  and **at tick 118 the crash: two `SpawnBeamExplosion` 0x273310 with the ship actor as their moby** (5 / 2.5 / 9 / 1 /
  60, 3 streaks, 200 sparks, 200 puffs; and 1 / 0.5, 300 streaks, 2, 2); scene 4 the courier ship's thrusters
  (type 23, joint lists 1..6, ticks 850–1160); scenes 1–4 the infobot's thrusters (`FUN_00278450`); scene 1 the
  Plumber's splash (types 46 / 35, `FUN_002ff768`). So scene data reaches the particle and effect systems only
  through classes that the moby loop keeps running in mode 2, exactly as in gameplay.
* **Camera / FOV**: `FUN_002ac8d8` writes 0x167240 (eye), 0x167450.. (rows) and **0x16cf70 = tan(hfov/2)**; the sky is
  drawn with the world projection (sky_render_notes.md), so it takes the same tangent. The end restores 0.63.

### 7.3 The general player, as built

One path for every scene, driven only by the data (adding a scene is no code):
* **Actors**: every record's class (level core, or a spaceship class from `spaceships`), a missing class kept as an
  empty slot; a table moby per actor (`CreateMoby`, mode 6 + the port's hidden bit, since the scene renderer draws it),
  posed and `rebuild_matrix`ed each scene tick (bounding sphere from the streamed sequence), deleted at the end.
* **World in mode 2**: the gameplay tick runs on the world-running scene frames in its mode-2 form
  (`Game::camera_paused`: no follow camera, no free-slot pass; no glints), `Services::game_mode` = 2 (kept while
  another scene is queued), the moby loop's camera = the last scene record, `SceneState` for the classes, hero
  `SetState(100, 2)`, talker / classes 74 / 203 hidden; the audio frame comes from the tick's sound step then.
* **Class 1546** ported (`classes/cutscene_fx.rs`); particle type 23 spawn / update (`particles/type23.rs`).
* **Sky**: the sky camera copies the main camera's projection (tan(hfov/2)) every frame.
* **Shadows**: the actors cast (crate::shadow_render: `ShadowProbeAlongDir` slabs after each tick, posed on the
  streamed sequence).

Root causes of the reported symptoms:
| symptom | cause | fix (general) |
|---|---|---|
| Agnogg scene: frozen characters, missing / see-through ship, empty frames | a scene played **after another** (the arrival) uploaded the previous scene's actor records and palette (other sizes) into the new scene's buffers during its fade frames; the new actors then drew from stale buffers (frozen poses, parts at the old positions, the ship 731 in the wrong pose, the characters off screen). Forcing scene 3 alone (`RC_SCENE=3`) looked right, which hid it | the records / palette are reset at every scene start and the upload refuses data of another size |
| arrival: sky warps | the sky camera kept tan 0.63 while the scene camera used 0.414 / 0.554 | the sky camera follows the main projection |
| arrival: background mobys frozen | the gameplay tick was suspended for the whole scene | the mode-2 tick (7.2) |
| arrival: no crash explosion | the explosion is class 1546's `SpawnBeamExplosion`, and the moby loop never ran; then its flash shells need a moby (the ship actor) | 1546 ported, the moby loop runs, the actors are table mobys |

**Verified** (engine, scratchpad `scene_pass/`): scene 3 reached by play after the arrival (`RC_HERO_AT=159.5,208.44,40
RC_DEBUG_KILL=1560`; the counter now advances during scenes, so the kill tick is after the arrival) matches the
original first shot (`cmp_scene3_first_shot.png`); the crash at ticks 119 / 128 / 136 / 148 against the original
frames (`cmp_crash.png`: the trail, the nested shells, the yellow shell, the ember burst); scenes 0 (via its copy 6), 1,
2, 3, 4, 5 and L02 S01, L03 S01 (animated FOV), L05 S05, L08 S01, L10 S06 render with animated actors; actor shadows
(`cmp_shadow_s4.png`); two runs byte-identical (PNG and WAV); the arrival's audio against the pre-pass capture:
per-second correlation ≥ 0.95 for seconds 0–24 (speech unchanged), differences only after control returns (the world
state differs because it ran). Headless: rc-formats conformance, rc-game (digest unchanged), rc-engine tests.

**Left** (effects hand-off `scratchpad/scene_pass/effects_handoff.md`): type-23 orbs render dim; the explosion light
does not relight the world (tfrag / tie / shrub point lights: `LightTfrags` 0x2a8e40, `LightTies` 0x2ab218,
`LightShrubs` 0x29e7e8); particle types 46 / 35, the splash 0x2ff768, the infobot thrusters 0x278450, the ship draw
callback 0x2a70a8; ~~the hero's state 100 body~~ (ported 2026-09-28, hero_gameplay.md §7); the infobot's projection screen in scene 4 draws black (the original is black too: breakables.md §4) [L: an effect of
its own]; the actors' collision; mode-6 space scenes (parsed and checked, not played).

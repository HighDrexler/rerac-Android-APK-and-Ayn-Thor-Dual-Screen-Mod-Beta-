# Forum triage (bugs-and-qa)

Status: **2026-10-19.** The open posts of the Discord forum, sorted by what fixing them needs. Players are on runtime
0.3.4; items fixed on main since then and items with an open PR are listed at the end and not repeated above.

## 1. Doable from the posts and Ghidra alone

Each has a description (and usually a video or screenshot) precise enough to find the original's code and compare.

**Blocks progress or changes gameplay**
1. **Quartu: softlock at the gold-bolt door after the Codebot.** After the door-unlock scene no input works but the
   camera (not even pause); the Codebot's model is missing during the scene and its animation plays afterwards.
   Blocks 100%. (idunnomate, Spoober)
2. **Magician skill point never triggers.** (roxas)
3. **Mirror World switches on by itself.** In New Game+, "cheat enabled" on every double jump and a left side-flip
   toggles Mirror World; the cheats menu is empty. The cheats' input-sequence test. (idunnomate; roxas saw it on Umbris)
4. **Weapons on the grind rail only fire straight to the right**; leaning while firing does not sweep the aim, so
   mines and gate switches cannot be hit while steering. (idunnomate, with old footage of the original)
5. **No wrench auto-aim on crates and enemies** (PS2 video posted). One player asks for it as an option.
6. **Blarg soldiers do not notice Ratchet** on Orxon and on Blarg Station (they do not run at him).
7. **Orxon's bombs (Clank area) cannot be knocked away** by Ratchet's wrench or Clank's punch (PS2 video posted).
8. **The Mine Glove's mines are hard to destroy** once placed.
9. **Damage that does not land:** the crashing ship's lasers do not hurt Ratchet; enemy missiles pass through Giant
   Clank without hurting him. (roxas)
10. **Enemy attacks do not break the green explosive canisters** (Gemlik; best seen with the Space Commandos).
11. **Kalebo III: dying on the grind rail** plays the ground death instead of falling off.
12. **The escape / underwater timers draw over the pause menu** (Blarg warship, Quartu, Drek's fleet, Deplanetizer).
13. **The camera does not snap behind Ratchet after a warp pad** (Oltanis Orbit; checked on PS2 by the reporter).
14. **The autosave icon never appears.**

**Cutscenes and camera**
15. **Clank shows two antennas in his mission scenes**: the extra one stays where the player stood before the scene
    (Blarg: Hydrodisplacer; Orxon: Magneboots, Infobot).
16. **Clank's footsteps keep playing during and after the Hydrodisplacer scene** (Blarg).
17. **The camera dips under the floor when the Hydrodisplacer is used** (Rilgar and Pokitaru sewers, Hoven; not on PS2
    per the reporter; videos posted).
18. **The field of view stays zoomed after Pokitaru's dogfight** when the booster was in use at the cutaway.
19. **Space looks too close to the camera during travel**, snapping to its place on arrival (screenshots posted).
20. **Giant Clank turns all red during his triangle attack** (PS2 footage posted).

**Sound**
21. **Clank on Orxon plays Ratchet's track** instead of "Clank's Second Foray" (needs the `ratchet1` Ghidra project:
    the track choice is not in the exported decompile).
22. **Blarg warship's self-destruct music never starts** (videos posted).
23. **No music during the ending credits.** (roxas)
24. **The equipped weapon's sound plays during the ship landing** (every planet; Blaster video).
25. **Static when a Blarg helicopter dies** under the Hoven turret (not in the original per HMDSOLO).
26. **The reverb stays after teleporting out of Pokitaru's sewers** (walking out of a reverb area clears it).

**Visuals with a reference or a clear description**
27. **The wrench reticle is opaque**; on PS2 it is glassy (PS2 screenshot posted).
28. **The thrown wrench's trail hides geometry behind it** (also Blarg Station's windows).
29. **The Walloper's effect glitches** where it crosses Ratchet's body, for half the punch directions (yaw dependent,
    zshrocks).
30. **The aerial turret's HUD is blurry / pixelated** (screenshot posted).
31. **Thruster-Pack and Hoverboard turbo effects are broken** (videos posted).
32. **Orxon's Clank area textures are broken** (PS2 screenshots posted for each spot).
33. **The Glove of Doom canister is slightly out of step with Ratchet's hand** (Spoober; may apply to every glove).

**Larger pieces (known, longer)**
34. Ratchet's worn items in scenes (Sonic Summoner and O2 mask clipping; G-CUT-009) — the ship part is PR #29.
35. Quartu: the monitor text in Clank's reunion scene is missing.
36. The Hologuise: Clank's transition, the antenna staying green when the alarm goes, its place in Ratchet's hand, the
    unstable transform (roxas).

## 2. Needs more input, screenshots or data

**Needs a reproduction or details from the reporter**
1. **Music and speech too fast** (roxas, on stream) / **music in slow motion after opening the vendor right after a
   music zone change** (Kerwan, Novalis). Track rates, the stream pitch through a vendor open and per-tick audio
   frames are ruled out. Needs: where it happened, the display's refresh rate, the audio device, and a log.
2. **Planet transitions hitch ("not responding")** before the planet loads. Needs a log and the PC's specs.
3. **Oltanis Orbit's ship disappeared** from both ends once (back after a restart). Needs the save and what was done
   before.
4. **Hoven "looks like it's being rebuilt"** after the planet is done. Needs a screenshot and where.
5. **Oltanis ice: an invisible wall / momentum loss** near the PDA, and the remaining ice items (melee drift, the
   wrench throw not pushing Ratchet, weapons pointing down while moving). Retest after PR #21.
6. **Hoven: a TNT crate and a crate inside each other** near the path to the Hydro-Pack. The placement data has no
   overlap (it happens at run time). Needs a screenshot of the spot.
7. **The Blarg Elite Commandos' shots are too dark** (Kalebo III). Needs a screenshot to tell which enemy.
8. **Metal Detector rings** — the post has no description. Needs what is wrong.
9. **Gaspar's lava "white structures"** — also on PS2, but more frequent here. Needs a PS2 capture of the same spot
   to compare the rate.

**Needs PS2 / PCSX2 references**
10. **Helmets (all) a little bigger, and their animation glitches** when Ratchet looks at enemies, draws a weapon,
    fires, or at low health (videos posted; the original's side is missing).
11. **Pokitaru's water turns purple** as the camera gets close (players like it; may be the original).
12. **Texture errors on Batalia** ("a UV error on almost every level"), **Pokitaru's sewers and waterfalls**
    (like Batalia's), **Rilgar and Blarg**, **Gaspar's and Orxon's skies**. Needs PS2 captures of the same views.
13. **The gold sheen on the gold weapons**, and the normal sheen on all weapons (Spoober compared with an emulator).
14. Earlier asks still open: the Mine Glove in hand, the helmets' size, pausing underwater, the Glove of Doom's
    canister timing.

**Launcher, setup and hardware**
15. **"Could not move the build into …"** when setting a runtime on Windows. Needs the launcher log and the install
    path.
16. **Black screen with sound on Windows** (Vulkan); `WGPU_BACKEND=dx12` works around it. Needs the GPU and driver
    to make DX12 the fallback where it is needed.
17. **Controllers:** a PS5 pad works on macOS by cable only (not Bluetooth); DS4Windows pads are not detected; an
    analog keyboard registering as a gamepad hid an Xbox pad (resolved by the player). Needs the hardware to test;
    players ask for a controller picker.

**Probably the original's behaviour**
18. The Heli/Thruster-Pack sound continuing after touching Hoven's water until a respawn (two players say the original
    does it).

## 3. Already done (not in the lists above)

**Fixed 2026-10-19 (from §1):** the Quartu Codebot door softlock (4968a3f: the slot waits for its fly-by like
`0x2f78e0`); the Magician skill point (cf7ebda: Kalebo's race host reads the Hoverboard's score); the cheats switching
on by themselves and the empty Cheats page (c80b9b3: the cheat tables were not relocated off level 01); weapons on the
grind rail (accd9d3: Ratchet's aim frame 0x13f990, the rail's yaw plus the aim lean steered by the stick); the wrench
auto-aim (23a24cf: the wrench states run the target search `0x22e238`); the escape / air timers over the pause menu
(14c8b55).

**Fixed 2026-10-20:** Clank on Orxon plays Ratchet's track (§1.21): the Clank section (class 22) requests
`MusicRequestTrack(2, 4)` from the first tick's moby loop, before the port's sound step made the level start's
`music_start_track(0)` (`entry` 0x259c40 makes it before its main loop): the idle player took track 2, then the start
overwrote it with 0. The start now runs before a class's first request (`AudioSystem::level_music_start`).
No music during the ending credits (§1.23): the last scene's end arms the music resume 30 ticks later
(`FUN_0027a460`), and `music_UpdateStream` counts it down inside `sound_update`; the boss enters the slideshow (mode 7)
at once, whose frames run only `sound_update`, while the port counted the delay in the gameplay frame only, so the music
stayed held through the credits. The countdown is now in `sound_update` (`music_resume_countdown`).
Clank's second antenna in his mission scenes (§1.15): the red light left where Clank stood is his antenna glow moby
0x140974; `FUN_002486c0` (the scene start's hide) hides it when Clank is the hero (0x1413f4 = 1) and `FUN_002487a8`
shows it again. The port's scene start now hides it with the other scene-hidden mobys.
The opaque wrench reticle (§1.27): the look stance's marker is `0xff917267` (alpha 0xff ≈ 2.0 on the GS), so the GS
blend `(Cs − Cd)·As + Cd` gives `2·Cs − Cd`: bright and tinted by the frame under it. The HUD layer kept the alpha but its
composite clamped it to 1 and drew the marker opaque. A second composite node takes the layer's pixels above 1 with a
subtract blend (`HudCompositeOver`).

**Looked at 2026-10-20, needs a PS2 reference:** the weapon sound during the ship landing (§1.24). The clip's sounds
are the hand item's take-out (the Blaster's class sounds 2 / 3, its sequence 0's triggers): `HeroItemsCreate`
0x22f3c0 creates the saved hand item at level start, and the landing's frame (0x2a4080 subs 0 / 8: `MobyUpdateLoop`,
which runs Ratchet's update as his moby's) runs the item pass as the gameplay frame does, so the original creates it
during the landing too. The port matches that code; whether the PS2 takes the sound out somewhere else needs a PCSX2
capture of a landing with the Blaster in hand. Clank's footsteps through the Hydrodisplacer scene (§1.16) need a run of
that scene as Clank (text-only report). Giant Clank red during his △ attack (§1.20): the emulator clip shows the beam's screen record
(0x15f330, `draw_fogged_fullscreen_sprite` after the shrubs in `DrawWorld`) reddening the terrain, ties and shrubs only;
the mobys and particles are drawn after it. The port's `screen_tint` pass runs after all the 3-D (its [L] note), so
Clank and his shot are tinted too: needs the tint inside the world pass, before the mobys (a renderer change). The blurry aerial turret HUD (§1.30): the gauges are 2-D HUD
art at the game's 512×416, scaled to the window and smoothed by the HUD option's Original setting (the TV's look);
the Sharp setting shows the pixels. The fix the post wants is the planned HUD High resolution option, not a bug. The
Kalebo III grind-rail death (§1.11): level 16's grind transitions (0x2270c8's case 0x28: the grind hit at health < 2
knocks Ratchet off backwards into the hurt 0x16) match the port; a headless run on rail 1 with a hit at 1 health goes
0x16 → the fall 6 → below the death height → the death fall 0x77, as the report says the PS2 does. Not reproduced:
needs the spot (a knock-off that lands back on a rail would end in the ground death 0x3d in both). The thrown
wrench's trail hiding the water (§1.28): the trail is the wrench's after-images (mode 0x80a: bit 8 fading, TEST_1
0x5308b = As ≥ 8 writes colour and Z; bit 0x800 deferred), drawn in `DrawMobys` before `RunDrawCallbacks_1`, the list
the ocean 1111 and the strip water draw from — the port's order too, so their Z hides the water behind them in both
unless the game masks Z for faded mobys (ZBUF ZMSK, not read). Needs a PS2 capture of a throw over water. The canisters
and enemy attacks (§1.10): the tanks 1261 take any record through `MobyGetHitMessage(0x330001)` and the resolver gives
out5 = 2 for every attack type, and enemy shots with a template (the turret's 1238, flags 0x10001) line-test into
mobys with mode 0x4000 as the hero's do; the Space Commandos' class is not identified (needs the spot). Hoven's helicopter static (§1.25): the clip
clips only a handful of samples; the noise is a broadband 4–7 kHz block before the first explosion and two clicks on
later ones; the copter 1255's own code plays no death sound (its burst's sound is −1), so the source is another
voice (its rotor loop or the explosions' shared sounds): needs an `RC_AUDIO_TRACE` run of the turret section. Orxon's
Clank-area textures (§1.32): the PS2 shots show lit square panels on the walls and the door, and vent grilles on the
raised blocks, that the port's shots lack — missing objects or a layer rather than wrong textures; needs a run there
to name the classes at those spots. Space too close during travel (§1.19): in the clips (16:9) the flight's
stars are larger and stretched sideways, the arrival's (the level's sky) round and fine. The flight sky is the
transition lump's six shells (`flight_render::update_sky`, one scalar scale each) with the sky camera's tangents raised to
at least `view_tans(sky_tan)` (Hor+ included), so the stretch is not in those two places; next: capture a flight at 4:3
and 16:9 (take off from Novalis's pad, choose a planet) and compare the sky camera's projection and the shells'
texture against the level's.

**Fixed later 2026-10-19:** the warp pads' camera (4f34672: a class teleport turns Ratchet's moby rows before the
camera reset snaps behind them); the FOV after Pokitaru's dogfight (142909a: a scene's end restores the gameplay
view, FUN_002ac608); the autosave icon (8447e57: the main loop's card icon 0x755d while the card is in use).

**Verified 2026-10-19 as the original's behaviour (code, data and a run of the classes):**
- §1.6 soldiers. Blarg Station's troopers 1048 wake and close in from 30 inside their arena (19 of 20 in a run; the
  leapers wait for their trigger cuboid, group 23 is parked in 0x12 until the boss / escape unparks it); they never
  leave the arena polygon, so the one behind the force field in the clip stays there. Orxon's brawlers 1202: the ten
  lone ones (idle +0x1b4 = 1) chase from 30 when Ratchet is within 3 in height; the 47 in packs sleep (state 1, sight 0)
  until a path scout calls them, a member is hit or touched (the group wakes), the Taunter or the Morph-o-Ray — the
  original's `0x2e1d38` / `0x2e3ef0` / `0x2e47d8` line for line; the seven scouts fly and wake their brawlers (one,
  #626, is zapped by the barrier 794 it flies through after its calls, as the original's zap lists any moby with
  collision). The clips show a soldier on a ledge above Ratchet (more than 3 in height) and packs not yet called.
- §1.26 the reverb after the sewer teleport: Pokitaru has no reverb boxes; its two reverb env points sit at the sewer
  pads, and the pad up top has no point within 8, while `HeroTeleport` is the only caller of `EnvNearestSamplePoint` on
  level 11 — the original keeps the sewer reverb too.
- §1.7 / §1.8 Orxon's bombs are the lava rocks 938 (the PS2 clip: hit, red, flown, burst). They ignored the wrench
  because the port's wrench hits named no attacker (the game's name the wrench moby, class 0x47, `FUN_0026e808`); the
  Mine Glove's mines test the same `attacker->class != own`. Fixed: the hit records the hero's swings write carry a
  wrench marker (`HitRecord::wrench`, `attacker_class` / `attacker_pos`), read by every class that tests the attacker
  (lava rocks, mines, Kerwan's layers and bystanders, the Qwark ship parts, the gunners and biters, the lights, the
  tanks and turrets, the drones, the amoeboids).

**Fixed on main, not yet released:** the Morph-o-Ray while moving (849f873); the Glove of Doom glow blue or yellow by
planet (fc51251); Ratchet on the ground during Giant Clank (6126b01); Kalebo III's race water (f8b3016); Clank's antenna
light at his feet in scenes (a9f976c); the ship star in the map's corner (c3ee600); the ammo counter after death
(8b2c9a9); the underwater tint over the pause and map pages (49651f6); Comet-Strike through doors (in 9d114f1); the
bomb in hand at a gold-bolt pickup (3974efd); Port Options in the main menu and the scroll arrows (9fb4c40).

**Open PRs:** trees sway (#39); Veldin Orbit's music after the fleet mission (#38); the O2 mask / helmet and Clank's
glow on the ship's take-off and landing (#29); always-on items such as the Persuader drawn on Ratchet (#28); the Mine
Glove's mine in hand and its smoke (#24, likely also the pause-menu floating mine); Clank's hover and Heli-Pack sounds
(#23); momentum on slippery surfaces (#21); Port Options in the main menu (#30, duplicates 9fb4c40). keepcount reports
the jet fighter's reticle and missile HUD and the Platinum Zoomerator's turbo bar fixed in an upcoming batch.

## 4. Retest replies on 0.3.5 (read 2026-10-20; new attachments in `work/forum`, `urls.tsv`)

**Confirmed fixed:** footprints in Hoven's snow, trees sway, the O2 mask at the ship's landings, Orxon's bombs, the
Mirror World / cheats (and the Cheats page shows them), grind-rail firing (idunnomate's clip), the Mine Glove smoke, the
mines set off by the wrench, Ratchet's head in the pause menu, Gaspar's turret pitch, ice jump momentum (idunnomate),
the Swingshot claws, the Morph-o-Ray while moving, Clank's antenna light, Clank's Heli-Pack sound, the Kalebo III race
water and HUD, the Magician skill point, the timers over the pause menu, the Glove of Doom glow (one colour on two
planets, image only).

**Partly fixed / still open (new detail):**
- Wrench auto-aim: the swing combos (□) on enemies and crates still miss; the Hyper-Strike is better but not right; the
  throw is much better (HMDSOLO).
- Always-on items (Persuader, Map-o-Matic, Bolt Grabber): fine in gameplay and the menus, missing on Ratchet in the
  scenes, the ship's included (PCSX2 comparison posted).
- FOV after the dogfight: fixed for the scene; crashing the jet while boosting keeps the FOV at the respawn (clip).
- The map's ship star: right on the current planet; on other planets' maps it sits in the corner or is missing (PCSX2
  shows it at each planet's pad). Also: Umbris's map does not fill yellow on completion.
- Mine Glove in hand: in place now, but the mine pokes through the glove (PCSX2 comparison).
- Ice: weapons still point at the ground while moving; the wrench throw and catch do not push Ratchet across the ice
  (VP44, VP45); roxas still gets stuck in place on some ice jumps (Oltanis, Hoven).
- Grind rail / Glove of Doom: roxas says Ratchet still does not face straight when firing (images).
- Giant Clank: Ratchet rides on Clank's back now but not in the holding pose.
- Skill points page: no check marks on earned skill points.
- Mines: odd hit box for the wrench; the Taunter does not set them off.
- The Blarg Elite Commandos' shots too transparent (image).
- Audio crackling and music / speech speed-ups (roxas).
- The pause menu still shows stray panels when Start is pressed repeatedly, more often on Kerwan (Spoober suspects the
  frame rate).

**New:** in-level movies replayed from the Goodies show the level's own NPC next to the scene's actor (two Big Als, a
second Qwark); the camera dips under Aridia's quicksand (image).

**No reply yet:** the Codebot softlock, Spoober's other items (he is replaying; has reached Blarg Station).

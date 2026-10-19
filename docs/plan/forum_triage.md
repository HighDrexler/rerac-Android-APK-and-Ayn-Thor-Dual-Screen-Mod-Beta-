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

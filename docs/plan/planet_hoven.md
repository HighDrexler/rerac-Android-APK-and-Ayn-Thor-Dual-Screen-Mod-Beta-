# Hoven (level 12): the batch C checklist

Batch C's fourteenth planet, done 2026-10-03 after Orxon. Before this pass Hoven ran ported code for its help
director 422, the animated idlers 339, the story NPCs 282 / 328 / 1404, the weather 1400, the turret mini-game 1267
with its carrier 1274, gun drones 326 and their riders 1371, the shells 458 / 184 and the shared crates, bolts and
checkpoints. The census of 2026-10-03 left these classes without a port; this page tracks them.

## Classes without a port (census 2026-10-03)

| class | what | status |
|---|---|---|
| 238 | the burrowers: out of the ground in dust, the sidestepping chase and bite, the wander, the jump-out, the Suck Cannon's hold | **ported** (`units::hoven_burrower`) |
| 240 | the hover platforms: path, bob and hover kinds, the tilt under Ratchet, the jets | **ported** (`units::hoven_platform`) |
| 294 | the gunners: the ambush, the path, the sidestepping approach, the aimed gun shot, the crate smash | **ported** (`units::hoven_gunner`) |
| 336 | the helicopters: circling, blown up and back out of view, hidden in the carrier battle, the skill point | **ported** (`units::hoven_copter`) |
| 384 | the falls: the pulsing curtain raised and lowered by commands, its drips | **ported** (`units::hoven_fall`) |
| 1259 | the arc posts: sliding pairs with four crackling bolts that hurt | **ported** (`units::hoven_arc`) |
| 1269 | the seeker mines | **ported** (`units::hoven_mine`) |
| 1281 | the mine dispensers | **ported** (`units::hoven_dispenser`) |
| 1345 | the power beams, held until their generator is destroyed (the mission) | **ported** (`units::hoven_beam`) |
| 1557 | the scene thrusters | **ported** (`units::hoven_story`) |

## Shared-system changes in this pass

- **Suck Cannon table 238** (`react::HOVEN_238`, `Hooks::Hoven238`): refused while burrowed, the group woken on a take.
- **`fx::clump_sparks`**: the melee creatures' death sparks (Orxon's brawler and the burrowers share it).
- **`fx::part23` / `fx::part28`**: the plain type-23 puff and the type-28 mote ring spawns.
- **`eudora_gunner::hold_gun_at`**: the gun 499 on joint 1 for any holder (the gunners 294).
- The Hydro-Pack giver 328 now gets its joint lists (its look-at had none).

## Notes

- **Snow footprints** (2026-10-19): level 12's footstep wrapper `0x228e58` spawns the type-70 flat decal at the
  foot joints (the walk / run steps, the landing, and the idle → walk transition's two feet), 0.15 ahead along the
  yaw and 0.04 above the ground, with the left / right frames 52 / 53. Ported as `hero::fx::footprint`
  (`PartSpawn::Footprint`); the class-0 / level-ground gates are the wrapper's. The idle → walk pair
  (`SpawnFootstep(..., 0)`, level 12's transitions case 0) is `hero::fx::footprints`, called before the state's
  entry animation.

- [L] Ratchet's anim speed write in the burrowers' bite goes to his table moby, which the hero code drives.
- [L] The burrowers' awake counter points at address 0 in the game (a pointer never set): a shared word here.
- [L] The gunners' `MobyAnimBlendEx(…, 5)` blends are plain blends; the arc posts' two frames are the post's yaw.
- [L] The helicopters' explosion shake flag is a leftover register in the game: no shake.
- The power beams start and release their hum every tick, as the game does.

## Open

- The user's QA: docs/test-requests/2026-10-03-hoven.md.

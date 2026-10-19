# Test request: Going Commando — Ratchet on Aranos (2026-10-16)

Run: `RC_DATA_DIR=<repo>/extracted-rac2 cargo dev` → title → Start → New Game, or straight in with `RC_LEVEL=0`.
`RC_PLAY_TRACE=1` prints one line per tick (state, position, velocity, air ticks, sequence, frame).

| Claim | How to check |
| --- | --- |
| Ratchet stands at his start (247, 194, 49.9) on the platform, idle sequence playing | look at him; the trace shows state 0, seq 0 |
| Left stick walks (seq 3) then runs (seq 4) above 2.35 u/s; releasing slows to idle | push the stick gently, then fully |
| He turns toward the stick relative to the camera; a sharp reversal does the quick turn | flick the stick back |
| ✕ jumps (7); ✕ while running near full stick does the run jump (9); holding ✕ longer jumps higher (to 2.3 / 2.7) | tap vs hold ✕ |
| ✕ again in the air (after about a quarter second, on the way up or near the top) double jumps (0xe, sequence 0x16); the higher he already is, the smaller the second jump | tap ✕ twice |
| Holding R1 with the stick (or d-pad) left, right or back and pressing ✕ flips that way (0xb, sequences 0x1c / 0x1d / 0x1f); no double jump from a flip; on landing R1 + direction + ✕ flips again | crouch, then direction + ✕ |
| Walking off an edge falls (6); a high fall plays sequence 0xb and lands with a short lag | walk off the platform |
| R1 crouches (4); releasing stands up | hold R1 |
| He stays on the floor and steps down small ledges, collides with walls | run into the rails / steps |
| The camera sits 4.64 behind and 2 above him at the start and follows with the game's lag (it trails when he runs, catches up when he stops) | run forward, stop |
| Right stick left / right turns the camera about him at about 1° a tick; up / down tilts it (up to 40°) and it eases back when released | use the right stick |
| Running toward the camera pulls it out (up to 6) instead of letting him run into it | run at the camera |
| In a jump the camera holds its height until he comes down; then it follows | jump on the spot and off a ledge |
| Walls and the dome pull the camera in, and it eases back out over time once clear; it never ends up inside a wall | back him into a wall, turn the camera into the dome's pillars |

Not expected to work yet: any other move (the pack jumps, wrench, swingshot, gadgets, ledges, swimming,
damage), Ratchet's lighting (drawn unlit), his sounds, Clank on his back, mobys reacting to him. A state that is not
ported freezes him and prints `rc_game2::hero: state 0x.. is not ported` once.

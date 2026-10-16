# Test request: Going Commando — Ratchet on Aranos (2026-10-16)

Run: `RC_DATA_DIR=<repo>/extracted-rac2 cargo dev` → title → Start → New Game, or straight in with `RC_LEVEL=0`.
`RC_PLAY_TRACE=1` prints one line per tick (state, position, velocity, air ticks, sequence, frame).

| Claim | How to check |
| --- | --- |
| Ratchet stands at his start (247, 194, 49.9) on the platform, idle sequence playing | look at him; the trace shows state 0, seq 0 |
| Left stick walks (seq 3) then runs (seq 4) above 2.35 u/s; releasing slows to idle | push the stick gently, then fully |
| He turns toward the stick relative to the camera; a sharp reversal does the quick turn | flick the stick back |
| ✕ jumps (7); ✕ while running near full stick does the run jump (9); holding ✕ longer jumps higher (to 2.3 / 2.7) | tap vs hold ✕ |
| Walking off an edge falls (6); a high fall plays sequence 0xb and lands with a short lag | walk off the platform |
| R1 crouches (4); releasing stands up | hold R1 |
| He stays on the floor and steps down small ledges, collides with walls | run into the rails / steps |
| The camera follows behind; the right stick turns it; walls pull it in | **interim camera, not the game's** |

Not expected to work yet: any other move (long jump, double jump, wrench, swingshot, gadgets, ledges, swimming,
damage), Ratchet's lighting (drawn unlit), his sounds, Clank on his back, mobys reacting to him. A state that is not
ported freezes him and prints `rc_game2::hero: state 0x.. is not ported` once.

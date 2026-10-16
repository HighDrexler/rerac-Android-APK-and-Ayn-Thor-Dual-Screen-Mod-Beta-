//! Game logic of Ratchet & Clank: Going Commando (R&C2), ported from the game's code like `rc-game` is for the first
//! game (docs/plan/rac2_bringup.md): the boot's front end ([`front_end`], [`gui`]) and Ratchet ([`hero`]). The engine
//! (`rc-engine`, its `rac2` modules) loads the data, plays the movies, reads the pad and draws what this crate lays
//! out. The systems that are the same code in both games (the pad record, the collision queries, Ratchet's animation
//! player) are `rc-game`'s, used as they are.

pub mod camera;
pub mod front_end;
pub mod gui;
pub mod hero;

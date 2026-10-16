//! Game logic of Ratchet & Clank: Going Commando (R&C2), ported from the game's code like `rc-game` is for the first
//! game. Only the front end so far (docs/plan/rac2_bringup.md): the boot's splash screens, the title and the main
//! menu. The engine (`rc-engine`, its `rac2` module) loads the data, plays the movies, reads the pad and draws
//! what [`gui`] lays out.

pub mod front_end;
pub mod gui;

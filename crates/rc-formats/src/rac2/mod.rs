//! Readers for Ratchet & Clank: Going Commando (R&C2, NTSC-U `SCUS_972.68` v2.00). The disc keeps its assets as
//! ISO files (`RC2.HDR`, `G/*.WAD`), which the extractor copies as they are (docs/plan/rac2_bringup.md §2). The
//! layouts here are the ones the boot ELF reads (§5 of that plan, addresses of `SCUS_972.68`).
//!
//! - [`toc`]: `RC2.HDR`, the table of contents (global WAD headers: the movies, `MISC.WAD`'s parts, the GUI block).
//! - [`front_end`]: the front-end blob (`MISC.WAD` part 4): splash screens, the menu backdrop, the texture set
//!   (the font), the text tables.
//! - [`gui_sprite`]: the GUI sprite block (`BE_H`, `HUD.WAD`): the menus' vector sprites.
//! - [`level`]: a level file `G/LEVELn.WAD` and its lumps; [`gameplay`]: its gameplay file.
//! - [`armor`]: `ARMOR.WAD`, Ratchet's mesh and textures for level class 0.
//! - [`boot`]: reads from the boot ELF's loaded image (the font's glyph table, the menu definitions).

pub mod armor;
pub mod boot;
pub mod front_end;
pub mod gameplay;
pub mod gui_sprite;
pub mod level;
pub mod toc;

/// A picture as the GS holds it: RGBA bytes, alpha 0x80 = opaque (not scaled).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Picture {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

//! Reads from the boot ELF's loaded image (`SCUS_972.68`): the bytes at a virtual address of a `PT_LOAD`
//! segment's file data. The front end keeps its constant tables there: the font's glyph table (0x263b10) and the
//! menu definitions (the main menu: 0x261520).

use crate::buf::{invalid, Buf, Result};

/// The glyph table of the font in texture slot 1 (`FUN_003377b0`: the menus' text objects use 0x263b10).
pub const GLYPHS: u32 = 0x263b10;
/// The glyph table of the font in texture slot 2 (`FUN_0027fc28`: "Press START", the subtitles).
pub const GLYPHS_2: u32 = 0x263eb0;
/// The main menu's definition (`FUN_002d63b0` → `FUN_00342dc0(gui, 0x261520)`).
pub const MAIN_MENU: u32 = 0x261520;

pub struct BootElf<'a> {
    elf: &'a [u8],
    /// `(vaddr, file offset, file size)` of each `PT_LOAD` segment.
    loads: Vec<(u32, usize, usize)>,
}

impl<'a> BootElf<'a> {
    pub fn parse(elf: &'a [u8]) -> Result<BootElf<'a>> {
        let b = Buf(elf);
        if b.u32(0)? != 0x464c_457f { return invalid("boot ELF: no ELF magic"); }
        let (phoff, phnum) = (b.u32(0x1c)? as usize, b.u16(0x2c)? as usize);
        let mut loads = Vec::new();
        for i in 0..phnum {
            let o = phoff + i * 32;
            if b.u32(o)? == 1 { loads.push((b.u32(o + 8)?, b.u32(o + 4)? as usize, b.u32(o + 16)? as usize)); }
        }
        Ok(BootElf { elf, loads })
    }

    /// `n` bytes at `vaddr` (they must lie in one segment's file data).
    pub fn read(&self, vaddr: u32, n: usize) -> Result<&'a [u8]> {
        for &(va, off, size) in &self.loads {
            if vaddr >= va && (vaddr - va) as usize + n <= size {
                let o = off + (vaddr - va) as usize;
                return Buf(self.elf).sub(o, n, "boot ELF read").map(|s| s.bytes());
            }
        }
        invalid(format!("boot ELF: {vaddr:#x}+{n:#x} is not in a loaded segment"))
    }

    /// A glyph table (`GLYPHS`, `GLYPHS_2`): 256 × `{u, v, y offset (signed), advance}` (`FUN_0027fcb0`). The tables
    /// lie 0x3a0 bytes apart, so the first one's characters past 0xe7 read the second's (no string uses them).
    pub fn glyphs(&self, table: u32) -> Result<[Glyph; 256]> {
        let t = self.read(table, 256 * 4)?;
        Ok(std::array::from_fn(|c| Glyph { u: t[c * 4], v: t[c * 4 + 1], y: t[c * 4 + 2] as i8, advance: t[c * 4 + 3] }))
    }

    /// A menu definition: rows of 0x14 bytes `{f32 scale, text id, kind, target, 0}` up to the first row whose
    /// scale is not positive, at most 16 (`FUN_00348da0`).
    pub fn menu(&self, vaddr: u32) -> Result<Vec<MenuRow>> {
        let b = Buf(self.read(vaddr, 16 * 0x14)?);
        let mut rows = Vec::new();
        for i in 0..16 {
            let scale = b.f32(i * 0x14)?;
            if scale <= 0.0 { break; }
            rows.push(MenuRow { scale, text: b.u32(i * 0x14 + 4)?, kind: b.u32(i * 0x14 + 8)?, target: b.u32(i * 0x14 + 12)? });
        }
        Ok(rows)
    }
}

/// One character of the font.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Glyph {
    /// Top-left texel of the 16×16 cell.
    pub u: u8,
    pub v: u8,
    /// Drawn this many pixels (× scale) lower.
    pub y: i8,
    /// Pixels (× scale) to the next character; 0 = not drawn.
    pub advance: u8,
}

/// A menu row.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MenuRow {
    pub scale: f32,
    /// Text id ([`super::front_end::Strings`]).
    pub text: u32,
    pub kind: u32,
    /// What the row opens (a page or a menu definition; by kind).
    pub target: u32,
}

//! `RC2.HDR`: the table of contents at disc sector 1001 (docs/formats/disc_layout.md §2.10). It starts with the
//! global WAD headers back to back, each `{s32 header size, s32 absolute sector, …}`, in the order of
//! [`Global`]; the boot ELF reads it to 0x14b540 and addresses the headers there (`MPEG` at 0x14b540, `MISC` at
//! 0x14b868, `HUD` at 0x14b8a8).

use crate::buf::{invalid, Buf, Result};

/// The global WAD headers, in their order in `RC2.HDR`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Global { Mpeg, Misc, Hud, Bonus, Audio, Space, Scene, Gadget, Armor }

/// `MISC.WAD`'s part holding the front end (`FUN_0026ede8`: sector `0x14b890` + `0x14b86c`, 0x14b894 sectors).
pub const MISC_FRONT_END: usize = 4;

/// The GUI sprite block's `{sector, sectors}` pair: `HUD` header + 0x1350 (`FUN_0034f308`: `0x14cbf8` / `0x14cbfc`).
const HUD_GUI_BLOCK: usize = 0x1350;

/// The disc's movie files, back to back on the disc: a movie's sector counts from the first one's start.
pub const MOVIE_FILES: [&str; 3] = ["G/MPEG.WAD", "G/MPEG1.WAD", "G/MPEG2.WAD"];

/// A run of sectors inside one global WAD.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Part {
    /// Sectors from the WAD's start.
    pub sector: u32,
    pub sectors: u32,
}

impl Part {
    pub fn bytes(&self) -> std::ops::Range<usize> { self.sector as usize * 2048..(self.sector + self.sectors) as usize * 2048 }
}

/// A movie: its bytes from the start of [`MOVIE_FILES`] taken as one file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Movie {
    pub sector: u32,
    pub bytes: u32,
}

pub struct Toc<'a> {
    hdr: Buf<'a>,
    /// Offset of each global header in `RC2.HDR`.
    globals: Vec<usize>,
}

impl<'a> Toc<'a> {
    pub fn parse(hdr: &'a [u8]) -> Result<Toc<'a>> {
        let b = Buf(hdr);
        let mut globals = Vec::new();
        let mut o = 0;
        for _ in 0..9 {
            let size = b.i32(o)?;
            if size <= 8 || o + size as usize > hdr.len() { return invalid(format!("RC2.HDR: global header {} at {o:#x} has size {size:#x}", globals.len())); }
            globals.push(o);
            o += size as usize;
        }
        Ok(Toc { hdr: b, globals })
    }

    /// The header of a global WAD.
    pub fn header(&self, g: Global) -> Result<Buf<'a>> {
        let o = self.globals[g as usize];
        self.hdr.sub(o, self.hdr.u32(o)? as usize, "RC2.HDR global header")
    }

    /// `MISC.WAD` part `i`: the header's `{sector, sectors}` pairs from +8.
    pub fn misc_part(&self, i: usize) -> Result<Part> {
        let h = self.header(Global::Misc)?;
        Ok(Part { sector: h.u32(8 + i * 8)?, sectors: h.u32(12 + i * 8)? })
    }

    /// The GUI sprite block in `HUD.WAD`.
    pub fn gui_block(&self) -> Result<Part> {
        let h = self.header(Global::Hud)?;
        Ok(Part { sector: h.u32(HUD_GUI_BLOCK)?, sectors: h.u32(HUD_GUI_BLOCK + 4)? })
    }

    /// Movie row `row` (`FUN_002f6e10`: `0x14b550 + 16·row` = `{sector, bytes}`); `None` for an empty row.
    pub fn movie(&self, row: usize) -> Result<Option<Movie>> {
        let h = self.header(Global::Mpeg)?;
        let o = 0x10 + row * 16;
        if o + 8 > h.len() { return Ok(None); }
        let (sector, bytes) = (h.u32(o)?, h.u32(o + 4)?);
        Ok((bytes > 0).then_some(Movie { sector, bytes }))
    }
}

/// `FUN_002efe68`: the movie row of a movie id (ids above 0xb0 count from 0x91; the story movies' language tables
/// below that are not read here).
pub fn movie_row(id: u32) -> Option<usize> { (id > 0xb0).then(|| (id - 0x91) as usize) }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn movie_ids() {
        assert_eq!(movie_row(0xb2), Some(33));
        assert_eq!(movie_row(0xb9), Some(40));
        assert_eq!(movie_row(0x20), None);
    }

    #[test]
    fn walks_the_global_headers() {
        let mut hdr = Vec::new();
        for (i, size) in [0x328usize, 0x40, 0x1870, 0x10, 0x10, 0x10, 0x10, 0x10, 0x10].into_iter().enumerate() {
            let mut h = vec![0u8; size];
            h[..4].copy_from_slice(&(size as u32).to_le_bytes());
            h[4..8].copy_from_slice(&(1000 + i as u32).to_le_bytes());
            hdr.extend(h);
        }
        // MISC part 4 and the GUI block.
        hdr[0x328 + 8 + 32..0x328 + 8 + 40].copy_from_slice(&[0x03, 0x05, 0, 0, 0x55, 0x04, 0, 0]);
        hdr[0x368 + 0x1350..0x368 + 0x1358].copy_from_slice(&[0x1f, 0x40, 0, 0, 0x40, 0, 0, 0]);
        // Movie row 33.
        hdr[0x10 + 33 * 16..0x18 + 33 * 16].copy_from_slice(&[1, 0, 0, 0, 2, 0, 0, 0]);
        let t = Toc::parse(&hdr).unwrap();
        assert_eq!(t.misc_part(MISC_FRONT_END).unwrap(), Part { sector: 1283, sectors: 1109 });
        assert_eq!(t.gui_block().unwrap(), Part { sector: 16415, sectors: 64 });
        assert_eq!(t.movie(33).unwrap(), Some(Movie { sector: 1, bytes: 2 }));
        assert_eq!(t.movie(32).unwrap(), None);
    }
}

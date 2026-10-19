//! A Going Commando level file, `G/LEVELn.WAD`: the 0x60-byte level header, then its parts by sector. The data part
//! starts with its own table of `{offset, size}` byte ranges (0x80 bytes), the lumps the level overlay and
//! `LoadLevelCoreData` read. The lumps that R&C1 kept as separate files (`levels/NN/core_index.bin`, `gs_ram.bin`,
//! `core_data.bin`, `gameplay_ntsc.bin`, …) are slices of this one file, and where their layout matches R&C1's the
//! shared readers take them as they are (docs/plan/rac2_bringup.md §6).

use crate::buf::{invalid, Buf, Result};
use std::ops::Range;

/// The level header's parts (`{s32 sector, s32 sectors}` from +0x10, sectors from the file's start).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Part { Data = 0, CoreBank = 1, Gameplay = 2, Occlusion = 3 }

/// The data part's lumps, in its table's order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lump {
    Overlay = 0,
    CoreIndex = 1,
    GsRam = 2,
    /// 0xfdc bytes on level 0; not mapped yet.
    Lump3 = 3,
    HudHeader = 4,
    HudBank0 = 5,
    HudBank1 = 6,
    HudBank2 = 7,
    HudBank3 = 8,
    /// WAD-compressed (the core index's `assets_compressed_size`).
    CoreData = 9,
    Transition = 10,
}

pub struct LevelWad<'a> {
    file: &'a [u8],
    /// The level number the header carries (+8).
    pub id: i32,
    data: Range<usize>,
}

impl<'a> LevelWad<'a> {
    pub fn parse(file: &'a [u8]) -> Result<LevelWad<'a>> {
        let b = Buf(file);
        if b.u32(0)? != 0x60 { return invalid(format!("level WAD header size {:#x}, 0x60 expected", b.u32(0)?)); }
        let mut w = LevelWad { file, id: b.i32(8)?, data: 0..0 };
        w.data = w.part(Part::Data)?.ok_or_else(|| crate::buf::FormatError::Invalid("level WAD without a data part".into()))?;
        Ok(w)
    }

    /// A part's byte range in the file (sector sized), `None` when empty.
    pub fn part(&self, p: Part) -> Result<Option<Range<usize>>> {
        let b = Buf(self.file);
        let o = 0x10 + 8 * p as usize;
        let (sector, sectors) = (b.u32(o)? as usize, b.u32(o + 4)? as usize);
        if sectors == 0 { return Ok(None); }
        let r = sector * 2048..((sector + sectors) * 2048).min(self.file.len());
        if r.start >= self.file.len() { return invalid(format!("level WAD part {p:?} starts past the file")); }
        Ok(Some(r))
    }

    pub fn part_bytes(&self, p: Part) -> Result<Option<&'a [u8]>> { Ok(self.part(p)?.map(|r| &self.file[r])) }

    /// A lump of the data part (exact size), `None` when empty.
    pub fn lump(&self, l: Lump) -> Result<Option<&'a [u8]>> {
        let data = &self.file[self.data.clone()];
        let b = Buf(data);
        let o = 8 * l as usize;
        let (off, size) = (b.i32(o)?, b.i32(o + 4)?);
        if off < 0 || size <= 0 { return Ok(None); }
        let (off, size) = (off as usize, size as usize);
        data.get(off..off + size).map(Some).ok_or_else(|| crate::buf::FormatError::Invalid(format!("level lump {l:?} runs past the data part")))
    }
}

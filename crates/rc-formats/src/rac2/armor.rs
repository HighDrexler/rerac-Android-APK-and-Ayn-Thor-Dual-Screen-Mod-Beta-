//! `G/ARMOR.WAD`: Ratchet's body. Going Commando's level class 0 (Ratchet) has a header, skeleton, joints and
//! sequences but no packets (packet table 0); the worn armour supplies the mesh and its textures. The armour loader
//! (L00 `0x2fe988`, called from the level entry when the worn armour changed) reads, for armour `i`, the
//! `{mesh sector, mesh sectors, texture sector, texture sectors}` row at `ARMOR` header + 8 + 16·i (`RC2.HDR`; the
//! sectors count from `ARMOR.WAD`'s start), the textures first, then the mesh into the same buffer.
//!
//! * **Textures**: `s32 count`, then `count` offsets; each is a PIF2 (`crate::pif`: palette at +0x20, pixels at
//!   +0x420), uploaded with its own TEX0 (`0x1a72a0[i]`). The packets' texture slot `k` is the armour's texture `k`.
//! * **Mesh** (`0x2ffd48` relocates it): `u8 high, low, metal, metal_begin` (the counts class headers hold at +4),
//!   `+4` the packet table, `+8` the GIF usage table, offsets from the mesh's start ([`crate::moby::attach_packets`]).

use super::toc::{Global, Toc};
use crate::buf::{invalid, Buf, Result};
use crate::texture::Texture;

/// An armour's two parts, as byte ranges of `ARMOR.WAD`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ArmorParts {
    pub mesh: (usize, usize),
    pub textures: (usize, usize),
}

/// The armour table of `RC2.HDR`.
pub fn armors(toc: &Toc) -> Result<Vec<ArmorParts>> {
    let h = toc.header(Global::Armor)?;
    let n = (h.len() - 8) / 16;
    (0..n)
        .map(|i| {
            let o = 8 + 16 * i;
            let r = |k: usize| -> Result<(usize, usize)> { Ok((h.u32(o + 8 * k)? as usize * 2048, h.u32(o + 8 * k + 4)? as usize * 2048)) };
            Ok(ArmorParts { mesh: r(0)?, textures: r(1)? })
        })
        .collect()
}

/// An armour read from `ARMOR.WAD`: its mesh bytes and decoded textures.
pub struct Armor {
    pub mesh: Vec<u8>,
    pub textures: Vec<Texture>,
}

pub fn read(wad: &[u8], p: ArmorParts) -> Result<Armor> {
    let part = |(at, len): (usize, usize), what: &'static str| Buf(wad).sub(at, len.min(wad.len().saturating_sub(at)), what);
    let tex = part(p.textures, "armour textures")?;
    let count = tex.i32(0)?;
    if !(0..=16).contains(&count) { return invalid(format!("armour: {count} textures")); }
    let textures = (0..count as usize)
        .map(|i| crate::pif::Pif::parse_at(tex.bytes(), tex.u32(4 + 4 * i)? as usize)?.decode())
        .collect::<Result<Vec<_>>>()?;
    Ok(Armor { mesh: part(p.mesh, "armour mesh")?.bytes().to_vec(), textures })
}

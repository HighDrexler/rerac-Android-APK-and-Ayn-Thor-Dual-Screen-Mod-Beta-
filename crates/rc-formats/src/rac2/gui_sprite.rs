//! The GUI sprite block (`BE_H`, [`super::toc::Toc::gui_block`]): the menus' vector sprites (the main menu's
//! buttons are 0x73 and 0x74), relocated by `FUN_00337b90` / `FUN_003014a8` and drawn by `FUN_003018a0`.
//!
//! Block: `+0x18` count, then `{id, offset}` rows. Sprite header: `u16 vertices, prims, bones, textures, frames,
//! frame stride; f32 scale; offsets of the vertices, prims, textures, ?, frames; u32 packed; u32 colour`.
//! - Vertices: packed (one word: bone = bits 28..31, x = sext(bits 0..13)·4, z = bits 12..27 as s16; colour =
//!   the header's) or 16 bytes `{s16 x, y, z, bone; s16 u, v (/4096); RGBA}` (`FUN_0029e1d0` / `FUN_0029e090`).
//! - Prims: four vertex indices, a quad drawn as the strip a, b, d, c; a word whose top 24 bits are all ones
//!   selects texture `byte 0` instead.
//! - Frames: per bone 0x18 bytes, a quaternion (s16 / 32768), a scale (u16 / 4096), a translation (s16) and the
//!   parent (s16, −1 = none) (`FUN_002844a0`).

use crate::buf::{invalid, Buf, Result};
use std::collections::HashMap;

/// `FUN_003014a8`: the header's scale is multiplied by this at load.
pub const SCALE_UNIT: f32 = 0.003_211_601_6;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vertex {
    pub pos: [f32; 3],
    pub bone: usize,
    pub uv: [f32; 2],
    /// GS RGBA bytes (alpha 0x80 = 1.0).
    pub rgba: [u8; 4],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Prim {
    /// Vertex indices a, b, c, d of a quad (drawn as the strip a, b, d, c).
    Quad([u8; 4]),
    /// The following quads sample this texture.
    Texture(u8),
}

/// A bone's key: rotation (x, y, z, w), scale, translation, parent.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BoneKey {
    pub rotation: [f32; 4],
    pub scale: [f32; 3],
    pub translation: [f32; 3],
    pub parent: Option<usize>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Sprite {
    /// The header's scale × [`SCALE_UNIT`].
    pub scale: f32,
    pub vertices: Vec<Vertex>,
    pub prims: Vec<Prim>,
    /// Per frame, per bone.
    pub frames: Vec<Vec<BoneKey>>,
    pub textures: usize,
}

/// Every sprite of the block by id.
pub fn parse(block: &[u8]) -> Result<HashMap<u32, Sprite>> {
    let b = Buf(block);
    if block.get(..4) != Some(b"BE_H".as_slice()) { return invalid("GUI block: no BE_H magic"); }
    let n = b.u32(0x18)? as usize;
    let mut out = HashMap::with_capacity(n);
    for i in 0..n {
        let (id, off) = (b.u32(0x1c + i * 8)?, b.u32(0x20 + i * 8)? as usize);
        out.insert(id, sprite(&b.tail(off, "GUI sprite")?)?);
    }
    Ok(out)
}

fn sprite(b: &Buf) -> Result<Sprite> {
    let (nv, nq, nb, nt, nf, stride) = (b.u16(0)? as usize, b.u16(2)? as usize, b.u16(4)? as usize, b.u16(6)? as usize, b.u16(8)? as usize, b.u16(10)? as usize);
    let scale = b.f32(0xc)? * SCALE_UNIT;
    let (pv, pq, pf) = (b.u32(0x10)? as usize, b.u32(0x14)? as usize, b.u32(0x20)? as usize);
    let packed = b.u32(0x24)? != 0;
    let colour = b.u32(0x28)?.to_le_bytes();
    let mut vertices = Vec::with_capacity(nv);
    for i in 0..nv {
        vertices.push(if packed {
            let w = b.i32(pv + i * 4)?;
            let x = ((w << 18) >> 16) as f32;
            let z = ((w << 4) >> 16) as f32;
            // `pcpyld` puts the sign of x in y (the GUI projection ignores y).
            Vertex { pos: [x, if x < 0.0 { -1.0 } else { 0.0 }, z], bone: (w >> 28) as usize & 0xf, uv: [0.0; 2], rgba: colour }
        } else {
            let o = pv + i * 16;
            Vertex {
                pos: [b.i16(o)? as f32, b.i16(o + 2)? as f32, b.i16(o + 4)? as f32],
                bone: b.u16(o + 6)? as usize,
                uv: [b.i16(o + 8)? as f32 / 4096.0, b.i16(o + 10)? as f32 / 4096.0],
                rgba: b.u32(o + 12)?.to_le_bytes(),
            }
        });
    }
    let mut prims = Vec::with_capacity(nq);
    for i in 0..nq {
        let w = b.u32(pq + i * 4)?;
        prims.push(if w >> 8 == 0x00ff_ffff { Prim::Texture(w as u8) } else { Prim::Quad(w.to_le_bytes()) });
    }
    if let Some(Prim::Quad(q)) = prims.iter().find(|p| matches!(p, Prim::Quad(q) if q.iter().any(|&v| v as usize >= nv))) {
        return invalid(format!("GUI sprite: quad {q:?} past its {nv} vertices"));
    }
    let mut frames = Vec::with_capacity(nf);
    for f in 0..nf {
        let mut bones = Vec::with_capacity(nb);
        for k in 0..nb {
            let o = pf + f * stride + k * 0x18;
            let q = |j: usize| -> Result<f32> { Ok(b.i16(o + j * 2)? as f32 / 32768.0) };
            let s = |j: usize| -> Result<f32> { Ok(b.u16(o + 8 + j * 2)? as f32 / 4096.0) };
            let t = |j: usize| -> Result<f32> { Ok(b.i16(o + 16 + j * 2)? as f32) };
            let parent = b.i16(o + 22)?;
            bones.push(BoneKey {
                rotation: [q(0)?, q(1)?, q(2)?, q(3)?],
                scale: [s(0)?, s(1)?, s(2)?],
                translation: [t(0)?, t(1)?, t(2)?],
                parent: (parent >= 0).then_some(parent as usize),
            });
        }
        frames.push(bones);
    }
    Ok(Sprite { scale, vertices, prims, frames, textures: nt })
}

#[cfg(test)]
mod tests {
    #[test]
    fn packed_vertex_fields() {
        // bone 1, x = −64 (bits 0..13 = 0x3ff0), z = −225 (bits 12..27).
        let x_bits = (-16i32 & 0x3fff) as u32;
        let z = -225i32 as u32 & 0xffff;
        let w = (1u32 << 28) | ((z << 12) & 0x0fff_f000) | x_bits;
        let w = w as i32;
        assert_eq!((w << 18) >> 16, -64);
        assert_eq!((w >> 28) & 0xf, 1);
        // z shares bits 12, 13 with x's top bits (the game reads them so); here they agree.
        assert_eq!((w << 4) >> 16, -225);
    }
}

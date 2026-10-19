//! The front-end blob: `MISC.WAD` part 4 ([`super::toc::MISC_FRONT_END`]), WAD-compressed, unpacked by the boot
//! (`FUN_0026ede8`). It starts with 13 `{offset, size}` pairs:
//!
//! | Part | Contents |
//! | --- | --- |
//! | 0 | the texture set (`FUN_00292650`): `{data offset, count, table offset, data base}`, rows `{palette, pixels, w, h}` |
//! | 1 | GS data for `FUN_002b7700` (not read here) |
//! | 2 | the menu backdrop: 16-byte header `{w, h}`, then w×h RGBA32 |
//! | 3–8 | not read here |
//! | 9 | the text: six language tables (`FUN_0026ede8`'s relocation loop) |
//! | 10, 11, 12 | splash screens, each WAD-compressed `{w, h}` (16-byte header) + RGBA32 (`FUN_0026ea00`) |

use super::Picture;
use crate::buf::{invalid, Buf, Result};
use std::collections::HashMap;

/// Parts of the blob.
pub const TEXTURES: usize = 0;
pub const BACKDROP: usize = 2;
pub const TEXT: usize = 9;
/// "Sony Computer Entertainment America presents", "A game created and developed by", the copyright page.
pub const SPLASHES: [usize; 3] = [10, 11, 12];

pub struct FrontEnd {
    blob: Vec<u8>,
    parts: [(usize, usize); 13],
}

impl FrontEnd {
    /// The unpacked blob.
    pub fn parse(blob: Vec<u8>) -> Result<FrontEnd> {
        let b = Buf(&blob);
        let mut parts = [(0, 0); 13];
        for (i, p) in parts.iter_mut().enumerate() {
            let (o, n) = (b.u32(i * 8)? as usize, b.u32(i * 8 + 4)? as usize);
            if o + n > blob.len() { return invalid(format!("front end: part {i} ({o:#x}+{n:#x}) past the blob ({:#x})", blob.len())); }
            *p = (o, n);
        }
        Ok(FrontEnd { blob, parts })
    }

    pub fn part(&self, i: usize) -> &[u8] {
        let (o, n) = self.parts[i];
        &self.blob[o..o + n]
    }

    /// A `{w, h}` + RGBA32 picture (16-byte header).
    fn picture(b: &[u8]) -> Result<Picture> {
        let buf = Buf(b);
        let (w, h) = (buf.u32(0)?, buf.u32(4)?);
        let px = buf.sub(16, w as usize * h as usize * 4, "front-end picture")?;
        Ok(Picture { width: w, height: h, rgba: px.bytes().to_vec() })
    }

    /// The menu backdrop (the station interior, 512×416).
    pub fn backdrop(&self) -> Result<Picture> { Self::picture(self.part(BACKDROP)) }

    /// Splash `i` of [`SPLASHES`].
    pub fn splash(&self, i: usize) -> Result<Picture> { Self::picture(&crate::wad::decompress(self.part(SPLASHES[i]))?) }

    /// The texture set: 8-bit pictures with 256-colour CSM1 palettes (slot 1 is the font, 256×128).
    pub fn textures(&self) -> Result<Vec<Picture>> {
        let (o, _) = self.parts[TEXTURES];
        let b = Buf(&self.blob);
        let (count, table) = (b.u32(o + 4)? as usize, o + b.u32(o + 8)? as usize);
        let data = o + b.u32(o + 12)? as usize + b.u32(o)? as usize;
        let mut out = Vec::with_capacity(count);
        for i in 0..count {
            let r = table + i * 16;
            let (pal, pix, w, h) = (b.u32(r)? as usize, b.u32(r + 4)? as usize, b.i32(r + 8)?, b.i32(r + 12)?);
            // A negative size marks a 4-bit texture (`FUN_00292650`, PSMT4); the front end's set has none.
            if w <= 0 || h <= 0 { return invalid(format!("front end: texture {i} is {w}×{h} (4-bit textures are not read)")); }
            let (w, h) = (w as u32, h as u32);
            let clut = b.sub(data + pal, 1024, "front-end palette")?.bytes();
            let idx = b.sub(data + pix, (w * h) as usize, "front-end pixels")?.bytes();
            let mut rgba = Vec::with_capacity((w * h * 4) as usize);
            for &ix in idx {
                let e = crate::texture::clut_index(ix as u32) as usize * 4;
                rgba.extend_from_slice(&clut[e..e + 4]);
            }
            out.push(Picture { width: w, height: h, rgba });
        }
        Ok(out)
    }

    /// Language table `lang` (0 English; 1 is empty).
    pub fn strings(&self, lang: usize) -> Result<Strings> {
        let (t, _) = self.parts[TEXT];
        let b = Buf(&self.blob);
        let table = t + b.u32(t + lang * 4)? as usize;
        let count = b.u32(table)? as usize;
        let mut map = HashMap::with_capacity(count);
        for i in 0..count {
            let row = table + 8 + i * 16;
            let (off, id) = (b.u32(row)? as usize, b.u32(row + 4)?);
            let s = self.blob.get(table + off..).ok_or_else(|| crate::FormatError::Invalid("front end: string past the blob".into()))?;
            map.insert(id, s[..s.iter().position(|&c| c == 0).unwrap_or(s.len())].to_vec());
        }
        Ok(Strings(map))
    }
}

/// A language's strings by text id (raw bytes: characters below 0x20 are pad icons, 8..15 colour codes).
#[derive(Clone, Debug, Default)]
pub struct Strings(pub HashMap<u32, Vec<u8>>);

impl Strings {
    /// `FUN_002899f8`: the text of an id (empty when missing).
    pub fn get(&self, id: u32) -> &[u8] { self.0.get(&id).map_or(&[], |s| s.as_slice()) }
}

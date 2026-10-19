//! The front end's 2D draws as screen quads (512×416 pixels, strip order v0 v1 v2 v3: triangles 012 and 123), in
//! draw order: the GUI sprites (`FUN_003018a0`), the font (`FUN_0027fcb0`, `FUN_0027f920`) and full-screen pictures
//! and fades. The engine draws the list as the GS would (alpha 0x80 = 1.0, `(Cs − Cd)·As/128 + Cd`).

use rc_formats::rac2::boot::Glyph;
use rc_formats::rac2::gui_sprite::{BoneKey, Prim, Sprite};

/// The screen (`FUN_0027baa0`, NTSC: `FUN_00284c18(0x200, 0x1a0, …)`).
pub const SCREEN_W: f32 = 512.0;
pub const SCREEN_H: f32 = 416.0;

/// The GUI projection's pixels per sprite unit × 16 (`FUN_0027af50`: half the screen over 0x1b9070 = 0.62; y ×
/// 0.775 for NTSC, `FUN_0027b0a0`).
const X_SCALE: f32 = SCREEN_W * 0.5 / 0.62;
const Y_SCALE: f32 = SCREEN_H * 0.5 / (0.62 * 0.775);

/// What a quad samples.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tex {
    None,
    /// The front end's texture set, slot n (1, 2: the fonts).
    Slot(usize),
    /// A full-screen picture.
    Picture(Picture),
}

/// The pictures the front end puts in the frame buffer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Picture {
    /// `rc_formats::rac2::front_end::SPLASHES[i]`.
    Splash(usize),
    /// The menu backdrop.
    Backdrop,
    /// The movie's current picture.
    Movie,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Quad {
    /// Pixels; a corner at x covers pixels x.. like the GS's `x·16 + OF − 8` (crate docs).
    pub pos: [[f32; 2]; 4],
    /// Texels.
    pub uv: [[f32; 2]; 4],
    pub tex: Tex,
    /// GS RGBA (alpha 0x80 = 1.0); a textured quad modulates: `C = Ct·Cf >> 7`.
    pub rgba: [u8; 4],
}

/// A picture over the whole screen at its own size (texel = pixel), unmodulated (`0x80808080`).
pub fn picture(out: &mut Vec<Quad>, p: Picture) {
    let (w, h) = (SCREEN_W, SCREEN_H);
    out.push(Quad { pos: [[0.0, 0.0], [w, 0.0], [0.0, h], [w, h]], uv: [[0.0, 0.0], [w, 0.0], [0.0, h], [w, h]], tex: Tex::Picture(p), rgba: [0x80; 4] });
}

/// `FUN_0027e440(r, g, b, a)`: a full-screen untextured quad.
pub fn fill(out: &mut Vec<Quad>, rgba: [u8; 4]) {
    let (w, h) = (SCREEN_W, SCREEN_H);
    out.push(Quad { pos: [[0.0, 0.0], [w, 0.0], [0.0, h], [w, h]], uv: [[0.0; 2]; 4], tex: Tex::None, rgba });
}

/// A GS colour word (`0xAABBGGRR`) as bytes.
pub const fn rgba(w: u32) -> [u8; 4] { w.to_le_bytes() }

/// `FUN_002846e8(t, a, b)`: `a·(1 − t) + b·t` per byte, truncated.
pub fn lerp_colour(t: f32, a: u32, b: u32) -> [u8; 4] {
    let (a, b) = (rgba(a), rgba(b));
    std::array::from_fn(|i| (a[i] as f32 * (1.0 - t) + b[i] as f32 * t) as i32 as u8)
}

/// A bone matrix: rows x, y, z and the translation (row-vector convention: `v' = v.x·r0 + v.y·r1 + v.z·r2 + t`).
type Mat = [[f32; 3]; 4];

fn apply(m: &Mat, v: [f32; 3]) -> [f32; 3] { std::array::from_fn(|j| v[0] * m[0][j] + v[1] * m[1][j] + v[2] * m[2][j] + m[3][j]) }

/// `FUN_00284408`: rotation (quaternion) × scale, then the translation.
fn key_matrix(k: &BoneKey) -> Mat {
    let [x, y, z, w] = k.rotation;
    let r = [
        [1.0 - 2.0 * (y * y + z * z), 2.0 * (x * y + z * w), 2.0 * (x * z - y * w)],
        [2.0 * (x * y - z * w), 1.0 - 2.0 * (x * x + z * z), 2.0 * (y * z + x * w)],
        [2.0 * (x * z + y * w), 2.0 * (y * z - x * w), 1.0 - 2.0 * (x * x + y * y)],
    ];
    let s = k.scale;
    [r[0].map(|c| c * s[0]), r[1].map(|c| c * s[1]), r[2].map(|c| c * s[2]), k.translation]
}

/// `FUN_003015d8`: the bones at `frame` (keys `⌊frame⌋` and the next, wrapping, blended linearly when the fraction is
/// above 0.001), each under its parent.
pub fn pose(s: &Sprite, frame: f32) -> Vec<Mat> {
    let n = s.frames.len();
    if n == 0 { return Vec::new(); }
    let i = frame.max(0.0) as usize % n;
    let t = frame - frame.floor();
    let (a, b) = (&s.frames[i], &s.frames[if i + 1 >= n { 0 } else { i + 1 }]);
    let mut out: Vec<Mat> = Vec::with_capacity(a.len());
    for (k, ka) in a.iter().enumerate() {
        let key = if t > 0.001 {
            let kb = &b[k];
            let l = |p: f32, q: f32| p * (1.0 - t) + q * t;
            BoneKey {
                rotation: std::array::from_fn(|j| l(ka.rotation[j], kb.rotation[j])),
                scale: std::array::from_fn(|j| l(ka.scale[j], kb.scale[j])),
                translation: std::array::from_fn(|j| l(ka.translation[j], kb.translation[j])),
                parent: ka.parent,
            }
        } else {
            *ka
        };
        let m = key_matrix(&key);
        out.push(match key.parent.and_then(|p| out.get(p)) {
            Some(p) => [apply(&[p[0], p[1], p[2], [0.0; 3]], m[0]), apply(&[p[0], p[1], p[2], [0.0; 3]], m[1]), apply(&[p[0], p[1], p[2], [0.0; 3]], m[2]), apply(p, m[3])],
            None => m,
        });
    }
    out
}

/// `FUN_003017f8(x, y, sx, sy, frame, sprite, colour)`: the sprite's quads. A vertex lands at
/// `pos + (X_SCALE·sx·scale·x, −Y_SCALE·sy·scale·z) / 16` (the GS coordinate, half a pixel right of a [`Quad`]
/// corner); its colour is `vertex × colour >> 7`. Quads after a texture select of a texture the sprite has are not
/// drawn (no textured GUI sprite is used yet). Nothing is drawn when the colour's alpha is 0 (`FUN_003017f8`).
pub fn sprite(out: &mut Vec<Quad>, s: &Sprite, pos: [f32; 2], frame: f32, scale: [f32; 2], colour: [u8; 4]) {
    if colour[3] == 0 { return; }
    let bones = pose(s, frame);
    let pts: Vec<([f32; 2], [u8; 4])> = s.vertices.iter().map(|v| {
        let p = bones.get(v.bone).map_or(v.pos, |m| apply(m, v.pos));
        let x = pos[0] + X_SCALE * scale[0] * s.scale * p[0] / 16.0 + 0.5;
        let y = pos[1] - Y_SCALE * scale[1] * s.scale * p[2] / 16.0 + 0.5;
        let c = std::array::from_fn(|i| ((v.rgba[i] as u32 * colour[i] as u32) >> 7).min(0xff) as u8);
        ([x, y], c)
    }).collect();
    let mut textured = false;
    for p in &s.prims {
        match *p {
            Prim::Texture(t) => textured = (t as usize) < s.textures,
            Prim::Quad([a, b, c, d]) if !textured => {
                let v = [a, b, d, c].map(|i| pts[i as usize]);
                // Flat shading: the GS takes the last vertex's colour (every GUI sprite in use has one colour).
                out.push(Quad { pos: v.map(|q| q.0), uv: [[0.0; 2]; 4], tex: Tex::None, rgba: v[3].1 });
            }
            Prim::Quad(_) => {}
        }
    }
}

/// `FUN_0027f858`: the width of `text` at `scale` (Σ advance·scale, truncated).
pub fn text_width(glyphs: &[Glyph; 256], text: &[u8], scale: f32) -> i32 {
    text.iter().map(|&c| glyphs[c as usize].advance as f32 * scale).sum::<f32>() as i32
}

/// `FUN_0027fcb0(x, y, scale, colour, text, …, texture slot, glyphs)`: one 16×16 cell per character (16·scale
/// square) at `(x, y + y offset·scale)`, then `x += advance·scale`. Characters 8..15 are colour codes (no change
/// while the colour table is off, `0x1b15e8`); below 0x20 the pad icons, 24×16, drawn grey (the colour's average);
/// 0x80..0xa7 draw the accent of character + 0x40 first, offset by its advance and y offset; a character with
/// advance 0 is not drawn.
#[allow(clippy::too_many_arguments)]
pub fn text(out: &mut Vec<Quad>, glyphs: &[Glyph; 256], slot: usize, text: &[u8], mut x: f32, y: f32, scale: f32, colour: [u8; 4]) {
    let cell = 16.0 * scale;
    let mut quad = |x: f32, y: f32, w: f32, h: f32, u: u8, v: u8, tw: f32, rgba: [u8; 4]| {
        let (u, v) = (u as f32, v as f32);
        out.push(Quad { pos: [[x, y], [x + w, y], [x, y + h], [x + w, y + h]], uv: [[u, v], [u + tw, v], [u, v + 16.0], [u + tw, v + 16.0]], tex: Tex::Slot(slot), rgba });
    };
    for &c in text {
        if (8..16).contains(&c) { continue; }
        let g = glyphs[c as usize];
        if g.advance == 0 { continue; }
        if (0x80..0xa8).contains(&c) {
            let a = glyphs[c as usize + 0x40];
            quad(x + a.advance as f32 * scale, y + a.y as f32 * scale, cell, cell, a.u, a.v, 16.0, colour);
        }
        if c < 0x20 {
            let grey = ((colour[0] as u32 + colour[1] as u32 + colour[2] as u32) / 3) as u8;
            quad(x, y + g.y as f32 * scale, scale * 24.0, scale * 16.0, g.u, g.v, 24.0, [grey, grey, grey, colour[3]]);
        } else if c > 0x20 {
            quad(x, y + g.y as f32 * scale, cell, cell, g.u, g.v, 16.0, colour);
        }
        x += g.advance as f32 * scale;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rc_formats::rac2::gui_sprite::Vertex;

    fn key(tx: f32, parent: Option<usize>) -> BoneKey { BoneKey { rotation: [0.0, 0.0, 0.0, 1.0], scale: [1.0; 3], translation: [tx, 0.0, 0.0], parent } }

    #[test]
    fn keys_blend_and_chain() {
        let s = Sprite { scale: 1.0, vertices: vec![], prims: vec![], frames: vec![vec![key(0.0, None), key(3654.0, Some(0))], vec![key(0.0, None), key(23762.0, Some(0))]], textures: 0 };
        let m = pose(&s, 0.25);
        assert_eq!(m[1][3][0], 3654.0 + 0.25 * (23762.0 - 3654.0));
        // Frame 1 blends towards frame 0 (wrapping).
        assert_eq!(pose(&s, 1.5)[1][3][0], 0.5 * 23762.0 + 0.5 * 3654.0);
    }

    #[test]
    fn sprite_quads_are_strips_with_modulated_colour() {
        let v = |x: f32, z: f32| Vertex { pos: [x, 0.0, z], bone: 0, uv: [0.0; 2], rgba: [0x80; 4] };
        let s = Sprite { scale: 0.25 * rc_formats::rac2::gui_sprite::SCALE_UNIT, vertices: vec![v(0.0, 0.0), v(100.0, 0.0), v(100.0, -100.0), v(0.0, -100.0)], prims: vec![Prim::Texture(0xff), Prim::Quad([0, 1, 2, 3])], frames: vec![], textures: 0 };
        let mut out = Vec::new();
        sprite(&mut out, &s, [150.0, 120.0], 0.0, [1.0, 1.0], rgba(0x60442d00));
        assert_eq!(out.len(), 1);
        let q = out[0];
        assert_eq!(q.rgba, [0x00, 0x2d, 0x44, 0x60]);
        // Strip a, b, d, c; z down the screen.
        assert_eq!(q.pos[0], [150.5, 120.5]);
        assert!(q.pos[2][1] > q.pos[0][1] && q.pos[1][0] > q.pos[0][0]);
    }

    #[test]
    fn text_advances_and_skips_colour_codes() {
        let mut g = [Glyph::default(); 256];
        g[b'A' as usize] = Glyph { u: 0, v: 0, y: 0, advance: 14 };
        g[b' ' as usize] = Glyph { u: 0, v: 0, y: 0, advance: 5 };
        let mut out = Vec::new();
        text(&mut out, &g, 1, b"A\x09 A", 10.0, 20.0, 1.0, [0x80; 4]);
        assert_eq!(out.len(), 2);
        assert_eq!(out[1].pos[0], [10.0 + 14.0 + 5.0, 20.0]);
        assert_eq!(text_width(&g, b"A A", 1.0), 33);
    }

    #[test]
    fn colour_lerp() {
        assert_eq!(lerp_colour(0.0, 0x7fe0e0e0, 0x5ff0c070), rgba(0x7fe0e0e0));
        assert_eq!(lerp_colour(1.0, 0x7fe0e0e0, 0x5ff0c070), rgba(0x5ff0c070));
    }
}

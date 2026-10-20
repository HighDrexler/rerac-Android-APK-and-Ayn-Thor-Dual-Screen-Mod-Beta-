//! Optional, video-only final presentation clean-up for low-resolution PS2 FMVs.
//! The original 512x416 MPEG-2 decode stays byte-exact. This pass is intended
//! for enlarged Android playback, not the real-time game's rendering pipeline.
//!
//! The movies are mostly progressive MPEG2 but some encode thin alternating
//! horizontal bands in otherwise flat areas. A small *edge-aware* vertical
//! correction reduces those bands; unlike unconditional deinterlacing it does
//! not combine rows across an actual edge or motion boundary.
//!
//! Work is O(width * height), no full-frame copy or allocations per picture
//! (just two scanlines), and nothing runs while a movie frame is merely held.

/// Reduce fine alternating horizontal bands in encoded FMV pictures without
/// blurring strong edges. Input is tightly packed, opaque RGBA8.
/// Returns without changes for empty/malformed buffers.
pub fn remove_scanline_banding(pixels: &mut [u8], width: usize, height: usize) {
    if width < 2 || height < 3 || width.checked_mul(height)
        .and_then(|n| n.checked_mul(4)) != Some(pixels.len()) { return; }
    let stride = width * 4;
    let mut previous = pixels[..stride].to_vec();
    let mut current_copy = vec![0u8; stride];
    for y in 1..(height - 1) {
        let offset = y * stride;
        let (before, remaining) = pixels.split_at_mut(offset);
        let _ = before;
        let (row, after) = remaining.split_at_mut(stride);
        current_copy.copy_from_slice(row);
        let next = &after[..stride];
        for x in 0..width {
            let i = x * 4;
            let luma = |r: &[u8]| -> i32 {
                (77 * r[i] as i32 + 150 * r[i + 1] as i32
                    + 29 * r[i + 2] as i32 + 128) >> 8
            };
            let a = luma(&previous);
            let b = luma(&current_copy);
            let c = luma(next);
            // Strong edges (different rows) and noise under 5/255 survive.
            // An alternating stripe is a same-direction deviation from both
            // adjacent rows. Correct only moderate, coherent excursions.
            let delta = b - (a + c) / 2;
            if (a - c).abs() > 18 || delta.abs() < 5 || delta.abs() > 34 {
                continue;
            }
            if (b - a).signum() != (b - c).signum() {
                continue;
            }
            for channel in 0..3 {
                // 50% centre + 25% each neighbour (no gamma or hue shift).
                row[i + channel] = ((2 * current_copy[i + channel] as u32
                    + previous[i + channel] as u32
                    + next[i + channel] as u32 + 2) / 4) as u8;
            }
        }
        std::mem::swap(&mut previous, &mut current_copy);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn image(w: usize, h: usize, f: impl Fn(usize, usize) -> u8) -> Vec<u8> {
        let mut out = Vec::with_capacity(w * h * 4);
        for y in 0..h { for x in 0..w {
            let v = f(x,y); out.extend_from_slice(&[v, v, v, 255]);
        }}
        out
    }

    #[test]
    fn alternating_encoded_scanlines_are_less_prominent() {
        let (w,h) = (32,16);
        let mut img = image(w,h,|_,y| if y%2==0 {104} else {124});
        let before=(img[(6*w+4)*4] as i32-img[(7*w+4)*4] as i32).abs();
        remove_scanline_banding(&mut img,w,h);
        let after=(img[(6*w+4)*4] as i32-img[(7*w+4)*4] as i32).abs();
        assert!(after < before, "{before} -> {after}");
        assert_eq!(img[(6*w+4)*4+3],255);
    }
    #[test]
    fn high_contrast_edges_remain_sharp() {
        let (w,h)=(12,8);
        let mut img=image(w,h,|_,y|if y<4 {30} else {220});
        let original=img.clone();
        remove_scanline_banding(&mut img,w,h);
        assert_eq!(img,original);
    }
    #[test]
    fn clean_flat_area_is_byte_exact() {
        let mut img=image(16,16,|_,_|80);
        let old=img.clone();
        remove_scanline_banding(&mut img,16,16);
        assert_eq!(img,old);
    }
    #[test]
    fn malformed_data_does_not_panic_or_change() {
        let mut img=vec![3;37];let old=img.clone();
        remove_scanline_banding(&mut img,512,416);
        assert_eq!(img,old);
    }
    #[test]
    fn alpha_channel_is_preserved() {
        let (w,h)=(8,8);
        let mut img=image(w,h,|x,y|if y%2==0 {110} else {126+x as u8});
        for (i,pixel) in img.chunks_exact_mut(4).enumerate(){pixel[3]=(i%251) as u8;}
        let old:Vec<_>=img.chunks_exact(4).map(|p|p[3]).collect();
        remove_scanline_banding(&mut img,w,h);
        assert_eq!(old,img.chunks_exact(4).map(|p|p[3]).collect::<Vec<_>>());
    }
}
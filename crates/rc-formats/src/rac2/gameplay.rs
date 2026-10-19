//! A Going Commando gameplay file (the level file's gameplay part, WAD-compressed; `rac2::level::Part::Gameplay`):
//! 40 header words of section offsets (0 = absent), then the sections. The order is R&C1's
//! (`crate::gameplay::SECTION_NAMES`) with sections added; the records that the shared readers take are R&C1's or
//! map to R&C1's structures here (docs/plan/rac2_bringup.md §6.1).
//!
//! **How each name is known** (Aranos, level 0; [`SECTIONS`]): *content* = the payload has R&C1's layout or carries
//! counts that match the level (e.g. the occlusion mappings hold 833 tfrags and 2000 ties, the core's counts);
//! *order* = only its place in R&C1's order says what it is (to confirm in R&C2's level loader).

use crate::buf::{invalid, Buf, Result};
use crate::gameplay::MobyInstance;
use crate::shrub::{self, ShrubInstance};
use crate::tie::TieInstance;

/// Header words (0x28 × 4 = 0xa0 bytes; the last is 0 on every level seen).
pub const HEADER_WORDS: usize = 40;

/// `(name, known by)` per header word.
pub const SECTIONS: [(&str, &str); HEADER_WORDS] = [
    ("level_settings", "content"),
    ("directional_lights", "content"),
    ("cameras", "content"),
    ("sound_instances", "content"),
    ("help_us_english", "content"),
    ("help_uk_english", "content"),
    ("help_french", "content"),
    ("help_german", "content"),
    ("help_spanish", "content"),
    ("help_italian", "content"),
    ("help_japanese", "content"),
    ("help_korean", "content"),
    ("tie_classes", "content"),
    ("tie_instances", "content"),
    // New in R&C2: a count (0x1b on Aranos) and 0x800-byte records.
    ("tie_groups", "order"),
    ("shrub_classes", "content"),
    ("shrub_instances", "content"),
    // New in R&C2: a count (1 on Aranos).
    ("shrub_groups", "order"),
    ("moby_classes", "content"),
    ("moby_instances", "content"),
    ("moby_groups", "order"),
    ("shared_data", "order"),
    ("pvar_moby_links", "order"),
    ("pvar_table", "content"),
    ("pvar_data", "order"),
    ("pvar_pointer_fixups", "order"),
    ("cuboids", "content"),
    ("spheres", "order"),
    ("cylinders", "order"),
    ("pills", "order"),
    ("paths", "content"),
    ("grind_paths", "order"),
    ("unknown_32", "order"),
    ("unknown_33", "order"),
    ("unknown_34", "order"),
    ("unknown_35", "order"),
    ("occlusion_mappings", "content"),
    // New in R&C2: RGBA5551 colours for the ties' vertices (the colour indices of R&C1's tie packets are gone).
    ("tie_colors", "content"),
    ("unknown_38", "order"),
    ("unused_39", "order"),
];

/// The index of a named section in [`SECTIONS`].
pub fn index(name: &str) -> Option<usize> { SECTIONS.iter().position(|s| s.0 == name) }

/// The present sections as (name, byte range) in file order: each runs to the next section's start.
pub fn sections(gameplay: &[u8]) -> Result<Vec<(&'static str, std::ops::Range<usize>)>> {
    let g = Buf(gameplay);
    let mut starts = Vec::new();
    for (i, (name, _)) in SECTIONS.iter().enumerate() {
        let o = g.i32(4 * i)?;
        if o > 0 { starts.push((o as usize, *name)); }
    }
    starts.sort();
    let mut out = Vec::with_capacity(starts.len());
    for (k, &(start, name)) in starts.iter().enumerate() {
        let end = starts.get(k + 1).map_or(gameplay.len(), |s| s.0);
        if end > gameplay.len() { return invalid(format!("gameplay section {name} runs past the end")); }
        out.push((name, start..end));
    }
    Ok(out)
}

/// One named section's bytes, `None` when absent.
pub fn section<'a>(gameplay: &'a [u8], name: &str) -> Result<Option<&'a [u8]>> {
    Ok(sections(gameplay)?.into_iter().find(|s| s.0 == name).map(|(_, r)| &gameplay[r]))
}

/// The level-settings section (R&C1's fields where they are the same; header word 0).
pub fn level_settings(gameplay: &[u8]) -> Result<Option<&[u8]>> { section(gameplay, "level_settings") }

/// R&C2's tie instance (0x60 bytes): R&C1's record without the 64 per-light-slot colours (0x50..0xd0), so
/// `{o_class, draw distance, 0, occlusion index, matrix, directional lights, uid, 0, 0}`.
const TIE_INSTANCE: usize = 0x60;

/// The tie instances (`s32 count`, 12 bytes pad, `count` × 0x60), as R&C1's structure with no slot colours.
pub fn tie_instances(gameplay: &[u8]) -> Result<Vec<TieInstance>> {
    let Some(s) = section(gameplay, "tie_instances")? else { return Ok(Vec::new()) };
    let b = Buf(s);
    let count = b.i32(0)?;
    if !(0..=100_000).contains(&count) { return invalid("implausible tie instance count"); }
    (0..count as usize)
        .map(|i| {
            let o = 0x10 + i * TIE_INSTANCE;
            let r = b.sub(o, TIE_INSTANCE, "tie instance")?;
            Ok(TieInstance {
                o_class: r.i32(0)?,
                draw_distance: r.i32(4)?,
                pad_8: r.i32(8)?,
                occlusion_index: r.i32(0xc)?,
                matrix: r.pod(0x10, "tie matrix")?,
                ambient_rgbas: [0; 64],
                directional_lights: r.i32(0x50)?,
                uid: r.i32(0x54)?,
                pad_d8: r.i32(0x58)?,
                pad_dc: r.i32(0x5c)?,
            })
        })
        .collect()
}

/// The shrub instances: R&C1's 0x70-byte record, unchanged.
pub fn shrub_instances(gameplay: &[u8]) -> Result<Vec<ShrubInstance>> {
    match section(gameplay, "shrub_instances")? { Some(s) => shrub::parse_shrub_instance_section(s), None => Ok(Vec::new()) }
}

/// R&C2's moby instance (0x88 bytes): R&C1's 0x78-byte record with four words inserted at 0x10 (after the spawn
/// id; their meaning is not mapped yet), so the class is at 0x28 and the position at 0x40.
pub const MOBY_INSTANCE: usize = 0x88;

/// A moby instance: R&C1's fields, and the four words R&C2 adds.
#[derive(Clone, Copy, Debug)]
pub struct Rac2MobyInstance {
    pub base: MobyInstance,
    pub extra: [i32; 4],
}

/// The static moby instances (`s32 count, s32 spawnable, pad[2]`, then records stepped by their size word).
pub fn moby_instances(gameplay: &[u8]) -> Result<Vec<Rac2MobyInstance>> {
    let Some(s) = section(gameplay, "moby_instances")? else { return Ok(Vec::new()) };
    let b = Buf(s);
    let count = b.i32(0)?;
    if !(0..=10_000).contains(&count) { return invalid("implausible moby instance count"); }
    let mut out = Vec::with_capacity(count as usize);
    let mut p = 0x10;
    for i in 0..count as usize {
        let size = b.i32(p)?;
        if size as usize != MOBY_INSTANCE { return invalid(format!("moby instance {i}: size {size:#x}, expected 0x88")); }
        // R&C1's record is this one without bytes 0x10..0x20.
        let mut rec = Vec::with_capacity(crate::gameplay::MOBY_INSTANCE_SIZE);
        rec.extend_from_slice(b.sub(p, 0x10, "moby instance")?.bytes());
        rec.extend_from_slice(b.sub(p + 0x20, MOBY_INSTANCE - 0x20, "moby instance")?.bytes());
        let mut base: MobyInstance = Buf(&rec).pod(0, "moby instance")?;
        base.size = crate::gameplay::MOBY_INSTANCE_SIZE as i32;
        out.push(Rac2MobyInstance { base, extra: [b.i32(p + 0x10)?, b.i32(p + 0x14)?, b.i32(p + 0x18)?, b.i32(p + 0x1c)?] });
        p += MOBY_INSTANCE;
    }
    Ok(out)
}

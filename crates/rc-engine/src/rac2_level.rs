//! A Going Commando level for the shared renderers: `G/LEVELn.WAD`'s lumps (rc_formats::rac2::level) read into the
//! same [`LoadedLevel`] R&C1's levels fill (crate::level_load), so tfrags, ties, shrubs, the sky, the occlusion grid,
//! the collision and the textures draw through R&C1's renderers (docs/plan/rac2_bringup.md §6).
//!
//! What a Going Commando level has so far (§6.3): A, the world; C, the moby classes at their instances (R&C1's moby
//! reader with R&C2's vertex table header, `rc_formats::moby::parse_moby_class_for`). Left empty here, each for its
//! milestone: the tie / shrub / tfrag / moby lighting (D: neutral colours), the HUD, the level code's water tables,
//! the sound (E).

use crate::level_load::{LoadTimings, LoadedLevel};
use anyhow::{Context, Result};
use rc_formats::rac2::level::{LevelWad, Lump, Part};
use rc_formats::{level, texture, tfrag, Game};
use std::path::Path;
use std::time::Instant;

/// Level `index`'s file in the data folder.
pub fn file_name(index: u32) -> String { format!("G/LEVEL{index}.WAD") }

/// Ratchet's body (rc_formats::rac2::armor): level class 0's header and skeleton with armour `armor`'s packets, its
/// textures appended to the level's moby texture table (slot `k` of the armour = entry `moby_textures.len() + k`).
fn attach_ratchet(root: &Path, core: &level::LevelCore, mobys: &mut crate::moby_render::LevelMobys, textures: &mut Vec<texture::LevelTexture>, armor: usize) -> Result<()> {
    use rc_formats::rac2::{armor as ar, toc::Toc};
    let hdr = crate::disc_source::read(root, "RC2.HDR")?;
    let parts = ar::armors(&Toc::parse(&hdr)?)?;
    let part = *parts.get(armor).with_context(|| format!("no armour {armor}"))?;
    let a = ar::read(&crate::disc_source::read(root, "G/ARMOR.WAD")?, part)?;
    let class = mobys.classes.iter_mut().find(|c| c.o_class == 0).context("no class 0 in the level")?;
    rc_formats::moby::attach_packets(Game::Rac2, &mut class.class, &a.mesh)?;
    let base = core.moby_textures.len().max(textures.iter().filter(|t| t.table == texture::TextureTable::Moby).map(|t| t.index + 1).max().unwrap_or(0));
    for (k, t) in a.textures.into_iter().enumerate() {
        let index = base + k;
        let slot = u8::try_from(index).context("moby texture index past 255")?;
        if let Some(e) = class.entry.textures.get_mut(k) { *e = slot; }
        textures.push(texture::LevelTexture { table: texture::TextureTable::Moby, index, source: texture::TextureSource::Entry(bytemuck_zeroed()), texture: t });
    }
    Ok(())
}

/// An empty texture-table entry (the armour's textures have no entry in the level's table).
fn bytemuck_zeroed() -> rc_formats::level::TextureEntry { rc_formats::level::TextureEntry { data_offset: 0, width: 0, height: 0, ty: 0, palette: 0, mipmap: 0, pad: 0 } }

pub fn load(root: &Path, index: u32) -> Result<LoadedLevel> {
    let mut t = LoadTimings::default();
    let t0 = Instant::now();
    let name = file_name(index);
    let file = crate::disc_source::read(root, &name)?;
    let wad = LevelWad::parse(&file).with_context(|| name.clone())?;
    let lump = |l: Lump| -> Result<&[u8]> { wad.lump(l)?.with_context(|| format!("{name}: no {l:?} lump")) };
    let core_index = lump(Lump::CoreIndex)?;
    let gs_ram = lump(Lump::GsRam)?;
    t.read = t0.elapsed();

    let t0 = Instant::now();
    let core_data = rc_formats::wad::decompress(lump(Lump::CoreData)?).context("decompressing the core data")?;
    let gameplay = rc_formats::wad::decompress(wad.part_bytes(Part::Gameplay)?.context("no gameplay part")?).context("decompressing the gameplay file")?;
    t.decompress = t0.elapsed();

    let t0 = Instant::now();
    let core = level::parse_level_core_for(Game::Rac2, core_index, core_data.len()).context("parsing the core index")?;
    t.parse_core = t0.elapsed();

    let t0 = Instant::now();
    // The stored colours, unlit (milestone D).
    let tfrags = tfrag::parse_level_tfrags(&core, &core_data).context("parsing tfrags")?;
    t.parse_tfrags = t0.elapsed();
    let t0 = Instant::now();
    let textures = texture::parse_textures(&core, &core_data, gs_ram).context("decoding textures")?;
    t.parse_textures = t0.elapsed();
    let tfrag_lod = crate::tfrag_lod::load(&core, &core_data, gs_ram)?;

    // Level settings: the background colour (+0x00) and the fog (+0x0c..+0x20) are where R&C1 keeps them (Aranos:
    // background (0x96, 0xbe, 0xd2), fog colour (0x66, 0x66, 0x81), near 61440, far 358400).
    let fog = crate::game_camera::LevelFog::parse(&gameplay).context("parsing level settings")?;
    let background = crate::game_camera::level_background(&gameplay).context("parsing level settings")?;
    let collision = rc_formats::collision::parse_collision(&core, &core_data)
        .map_err(|e| eprintln!("rac2 level {index}: collision not parsed ({e})"))
        .ok();
    let ties = crate::tie_render::load_ties(&core, &core_data, gs_ram, &gameplay, &textures).context("loading ties")?;
    let shrubs = crate::shrub_render::load_shrubs(&core, &core_data, gs_ram, &gameplay, &textures).context("loading shrubs")?;
    let sky = crate::sky_render::load(&core, &core_data, index).context("loading the sky")?;
    let mut textures = textures;
    let mut mobys = crate::moby_render::load_mobys_with_gs(root, &core, &core_data, &gameplay, gs_ram, collision.as_ref()).context("loading mobys")?;
    if let Err(e) = attach_ratchet(root, &core, &mut mobys, &mut textures, 0) { eprintln!("rac2: Ratchet's armour not attached ({e:#}); he has no mesh"); }
    let occlusion = crate::occlusion::load(&core, &core_data, &gameplay, &tfrags, &ties.instances, &mobys.instances).context("loading occlusion")?;
    let particles = crate::particle_render::LevelParticles {
        textures: rc_formats::particle_tex::parse_particle_textures(&core, core_index, &core_data).map_err(|e| eprintln!("rac2: no particle textures ({e})")).ok(),
        owners: Vec::new(),
        level: index,
        grid: rc_formats::level::HeightGrid::parse(&core.header, &core_data),
    };
    let death_z = rc_formats::rac2::gameplay::level_settings(&gameplay)?
        .and_then(|s| s.get(0x28..0x2c))
        .map_or(f32::NEG_INFINITY, |b| f32::from_le_bytes(b.try_into().unwrap()));
    let moby_class_order = core.moby_classes.iter().map(|e| e.o_class).collect();
    println!(
        "rac2: level {index} ({name}): {} tfrags, {} textures, {} tie classes / {} instances, {} shrub classes / {} instances, {} moby classes / {} instances, sky {}, collision {}",
        tfrags.len(), textures.len(), ties.classes.len(), ties.instances.len(), shrubs.classes.len(), shrubs.instances.len(),
        mobys.classes.len(), mobys.instances.len(),
        sky.is_some(), collision.is_some()
    );
    Ok(LoadedLevel {
        tfrags, textures, fog, background, mobys, ties, shrubs, sky, occlusion, tfrag_lod, particles, hud: None,
        water: Default::default(), sea: Default::default(), collision, death_z, audio: None, gameplay, moby_class_order, timings: t,
    })
}

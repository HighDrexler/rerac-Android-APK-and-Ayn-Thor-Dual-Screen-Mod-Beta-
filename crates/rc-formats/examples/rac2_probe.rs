//! Development probe for Going Commando level files: runs the shared readers over `G/LEVELn.WAD` and prints what parses
//! (docs/plan/rac2_bringup.md §6.1). `cargo run -p rc-formats --example rac2_probe -- <LEVELn.WAD> [gameplay.bin out]`;
//! `PROBE_DUMP=<dir>` writes the first class blobs there.
use rc_formats::rac2::level::{LevelWad, Lump, Part};
use rc_formats::*;

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("LEVELn.WAD");
    if let Some(out) = std::env::args().nth(2) { return dump_gameplay(&path, &out); }
    let file = std::fs::read(&path)?;
    let w = LevelWad::parse(&file)?;
    println!("id {}", w.id);
    for p in [Part::Data, Part::CoreBank, Part::Gameplay, Part::Occlusion] { println!("part {p:?}: {:x?}", w.part(p)?); }
    let index = w.lump(Lump::CoreIndex)?.unwrap();
    let gs_ram = w.lump(Lump::GsRam)?.unwrap();
    let data = wad::decompress(w.lump(Lump::CoreData)?.unwrap())?;
    println!("core data {:#x} bytes", data.len());
    let core = level::parse_level_core_for(Game::Rac2, index, data.len())?;
    println!("header {:#x?}", core.header);
    println!("classes: moby {} tie {} shrub {}; textures tfrag {} moby {} tie {} shrub {}; gadgets {}; seqs {}",
        core.moby_classes.len(), core.tie_classes.len(), core.shrub_classes.len(), core.tfrag_textures.len(), core.moby_textures.len(), core.tie_textures.len(), core.shrub_textures.len(), core.gadgets.len(), core.ratchet_seqs.len());
    match tfrag::parse_level_tfrags(&core, &data) { Ok(t) => println!("tfrags: {}", t.len()), Err(e) => println!("tfrags: ERR {e}") }
    match texture::parse_textures(&core, &data, gs_ram) { Ok(t) => println!("textures: {}", t.len()), Err(e) => println!("textures: ERR {e}") }
    match tie::parse_level_ties(&core, &data) { Ok(t) => println!("tie classes: {} ok", t.len()), Err(e) => println!("ties: ERR {e}") }
    match shrub::parse_level_shrubs(&core, &data) { Ok(t) => println!("shrub classes: {} ok", t.len()), Err(e) => println!("shrubs: ERR {e}") }
    match sky::parse_sky(&core, &data) { Ok(t) => println!("sky: {:?}", t.map(|s| s.shells.len())), Err(e) => println!("sky: ERR {e}") }
    match collision::parse_collision(&core, &data) { Ok(_) => println!("collision ok"), Err(e) => println!("collision: ERR {e}") }
    match occlusion::parse_occlusion(&core, &data) { Ok(_) => println!("occlusion ok"), Err(e) => println!("occlusion: ERR {e}") }
    let (mut ok, mut bad) = (0, Vec::new());
    for e in &core.tie_classes {
        let Some(b) = core.block(&data, &format!("tie_class/{:04}", e.o_class)) else { continue };
        match tie::parse_tie_class_for(Game::Rac2, b) { Ok(_) => ok += 1, Err(err) => bad.push((e.o_class, e.offset_in_asset_wad, b.len(), err.to_string(), b[..0x80].chunks(4).map(|c| u32::from_le_bytes(c.try_into().unwrap())).collect::<Vec<_>>())) }
    }
    if let Ok(dir) = std::env::var("PROBE_DUMP") {
        for e in core.tie_classes.iter().take(3) { if let Some(b) = core.block(&data, &format!("tie_class/{:04}", e.o_class)) { std::fs::write(format!("{dir}/tie{:04x}.bin", e.o_class), b)?; } }
        for e in core.moby_classes.iter().take(40) { if let Some(b) = core.block(&data, &format!("moby_class/{:04}", e.o_class)) { std::fs::write(format!("{dir}/moby{:04}.bin", e.o_class), b)?; } }
    }
    println!("tie classes: {ok} ok, {} failed: {:x?}", bad.len(), bad.iter().take(3).collect::<Vec<_>>());
    let (mut ok, mut bad) = (0, Vec::new());
    for e in &core.moby_classes {
        let Some(b) = core.block(&data, &format!("moby_class/{:04}", e.o_class)) else { continue };
        match moby::parse_moby_class_for(Game::Rac2, b) { Ok(_) => ok += 1, Err(err) => bad.push((e.o_class, err.to_string())) }
    }
    println!("moby classes: {ok} ok, {} failed: {:?}", bad.len(), bad.iter().take(8).collect::<Vec<_>>());
    if let Some(g) = w.part_bytes(Part::Gameplay)? {
        println!("gameplay part starts {:02x?}", &g[..32]);
        let g = if wad::is_wad(g) { wad::decompress(g)? } else { g.to_vec() };
        println!("gameplay {:#x} bytes, first words {:x?}", g.len(), g[..0x80].chunks(4).map(|c| u32::from_le_bytes(c.try_into().unwrap())).collect::<Vec<_>>());
        use rc_formats::rac2::gameplay as g2;
        match g2::tie_instances(&g) { Ok(t) => println!("tie instances {} (first {:?})", t.len(), t.first().map(|t| (t.o_class, t.matrix[3]))), Err(e) => println!("tie inst ERR {e}") }
        match g2::shrub_instances(&g) { Ok(t) => println!("shrub instances {}", t.len()), Err(e) => println!("shrub inst ERR {e}") }
        match g2::moby_instances(&g) { Ok(t) => println!("moby instances {} (first {:?})", t.len(), t.get(1).map(|m| (m.base.o_class, m.base.position, m.extra))), Err(e) => println!("moby inst ERR {e}") }
    }
    Ok(())
}
#[allow(dead_code)]
fn dump_gameplay(path: &str, out: &str) -> anyhow::Result<()> {
    let file = std::fs::read(path)?;
    let w = LevelWad::parse(&file)?;
    let g = w.part_bytes(Part::Gameplay)?.unwrap();
    std::fs::write(out, wad::decompress(g)?)?;
    Ok(())
}

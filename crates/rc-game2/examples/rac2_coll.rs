//! Development check of the collision queries on a Going Commando level: `cargo run -p rc-game2 --example rac2_coll -- <LEVELn.WAD> x y z`.
use rc_formats::rac2::level::{LevelWad, Lump};
use rc_game::collision_query::*;

fn main() -> anyhow::Result<()> {
    let a: Vec<String> = std::env::args().collect();
    let file = std::fs::read(&a[1])?;
    let w = LevelWad::parse(&file)?;
    let data = rc_formats::wad::decompress(w.lump(Lump::CoreData)?.unwrap())?;
    let core = rc_formats::level::parse_level_core_for(rc_formats::Game::Rac2, w.lump(Lump::CoreIndex)?.unwrap(), data.len())?;
    let c = rc_formats::collision::parse_collision(&core, &data)?;
    let p: [f32; 3] = [a[2].parse()?, a[3].parse()?, a[4].parse()?];
    let up = |dz: f32| [p[0], p[1], p[2] + dz];
    println!("line down: {:?}", coll_line(&c, up(2.2), up(-48.0), QueryFlags(2)).map(|o| (o.point, o.kind)));
    for dz in [0.25f32, 0.5, 0.7, 1.0] {
        let cap = coll_capsule(&c, up(dz), 0.1, 0.45, QueryFlags(0x24));
        let sph = coll_sphere(&c, up(dz), 0.45, QueryFlags(0x24));
        let hg = coll_sphere_hero_groups(&c, up(dz), 0.45);
        println!("dz {dz}: capsule {:?} | sphere {:?} | hero groups {:?}", cap.map(|o| (o.point, o.pushed_centre)), sph.map(|o| (o.point, o.pushed_centre)), hg.map(|o| (o.point, o.pushed_centre, o.tri)));
    }
    Ok(())
}

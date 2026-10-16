//! `rac2` on the Going Commando disc files (`SCUS_972.68` v2.00): the table of contents, the front-end blob, the GUI
//! sprites and the boot ELF's tables, against the values read in Ghidra (docs/plan/rac2_bringup.md §5). Skipped
//! without `extracted-rac2/`.

use rc_formats::rac2::boot::{BootElf, GLYPHS, MAIN_MENU};
use rc_formats::rac2::front_end::FrontEnd;
use rc_formats::rac2::gui_sprite::{self, Prim};
use rc_formats::rac2::toc::{movie_row, Toc, MISC_FRONT_END};

fn read(rel: &str) -> Option<Vec<u8>> { std::fs::read(rc_formats::test_data::rac2_root().join(rel)).ok() }

#[test]
fn front_end_and_main_menu() {
    let Some(hdr) = read("RC2.HDR") else { eprintln!("skipped: no extracted-rac2/"); return };
    let toc = Toc::parse(&hdr).unwrap();
    let part = toc.misc_part(MISC_FRONT_END).unwrap();
    assert_eq!((part.sector, part.sectors), (1283, 1109));
    let misc = read("G/MISC.WAD").unwrap();
    let fe = FrontEnd::parse(rc_formats::wad::decompress(&misc[part.bytes()]).unwrap()).unwrap();
    for i in 0..3 {
        let s = fe.splash(i).unwrap();
        assert_eq!((s.width, s.height, s.rgba.len()), (512, 416, 512 * 416 * 4), "splash {i}");
    }
    assert_eq!(fe.backdrop().unwrap().width, 512);
    let tex = fe.textures().unwrap();
    assert_eq!(tex.iter().map(|t| (t.width, t.height)).collect::<Vec<_>>(), [(32, 32), (256, 128), (256, 128)]);
    let en = fe.strings(0).unwrap();
    assert_eq!(en.get(0x2c35), b"New Game");
    assert_eq!(en.get(0x2c2b), b"Load");
    assert_eq!(en.get(0x2bec), b"Options");
    assert_eq!(en.get(0x2db6), b"Press START");

    let elf = read("SCUS_972.68").unwrap();
    let boot = BootElf::parse(&elf).unwrap();
    let rows = boot.menu(MAIN_MENU).unwrap();
    assert_eq!(rows.iter().map(|r| (r.text, r.kind)).collect::<Vec<_>>(), [(0x2c35, 4), (0x2c2b, 5), (0x2bec, 3)]);
    let g = boot.glyphs(GLYPHS).unwrap();
    assert_eq!((g[b'A' as usize].u, g[b'A' as usize].v, g[b'A' as usize].advance), (0, 0, 14));
    assert_eq!((g[b'0' as usize].v, g[b'0' as usize].advance), (0x40, 13));
}

#[test]
fn gui_sprites_and_movies() {
    let Some(hdr) = read("RC2.HDR") else { eprintln!("skipped: no extracted-rac2/"); return };
    let toc = Toc::parse(&hdr).unwrap();
    let block = toc.gui_block().unwrap();
    assert_eq!((block.sector, block.sectors), (16415, 64));
    let hud = read("G/HUD.WAD").unwrap();
    let sprites = gui_sprite::parse(&hud[block.bytes()]).unwrap();
    assert_eq!(sprites.len(), 230);
    let (fill, frame) = (&sprites[&0x73], &sprites[&0x74]);
    assert_eq!((fill.vertices.len(), fill.frames.len(), fill.frames[0].len()), (6, 2, 2));
    assert_eq!(frame.vertices.len(), 28);
    assert_eq!(fill.prims, [Prim::Texture(0xff), Prim::Quad([0, 1, 2, 3])]);
    // Bone 1 slides along x between the two keys (the button's width).
    assert_eq!((fill.frames[0][1].translation[0], fill.frames[1][1].translation[0]), (3654.0, 23762.0));
    assert_eq!(fill.frames[0][1].parent, Some(0));

    let m = toc.movie(movie_row(0xb2).unwrap()).unwrap().unwrap();
    assert_eq!(m.bytes, 22_675_460);
    // The title's couch movies: rows 34..=40, all present.
    for id in 0xb3..=0xb9 { assert!(toc.movie(movie_row(id).unwrap()).unwrap().is_some(), "movie {id:#x}"); }
}

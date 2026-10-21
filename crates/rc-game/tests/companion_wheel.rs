#[path = "../../rc-engine/src/android/companion_wheel.rs"]
mod companion_wheel;
use rc_game::{game_state::Global, hud::{HudAssets,Draw}, menus::{MenuAssets,MenuDraw,Overlay,quick_select::*},ps2v::Pf};
use rc_formats::{font::{Glyph,GLYPHS},save_game::ITEM_COUNT,hud::IconEntry};
    fn consts() -> QsConsts {
        QsConsts {
            size: (210, 200),
            centre: (105, 100),
            steps: 8,
            edge: Pf::b(0x3e4c_cccd),
            radius: Pf::b(0x4294_0000),
            x_scale: Pf::b(0x3f86_6666),
            dim: Pf::b(0x3f00_0000),
            name_rgba: (0x00ff_8080, 0x80ff_8080),
            shadow_rgba: (0, 0x8000_0000),
            ammo_empty_rgba: (0x0040_40ff, 0x8040_40ff),
            ammo_rgba: (0x0040_e020, 0x8040_e020),
            period: 30,
            name_dy: [-25, -15, 0],
            ammo_format: b"%d/%d".to_vec(),
        }
    }

    fn tables() -> QsTables {
        let lrud = [[7, 1, 0, 4], [0, 2, 0, 2], [6, 2, 1, 3], [4, 2, 2, 4], [5, 3, 0, 4], [6, 4, 6, 4], [6, 2, 7, 5], [6, 0, 0, 6]];
        let entries = lrud.map(|[l, r, u, d]| Entry { left: l, right: r, up: u, down: d, ..Default::default() });
        QsTables { entries, defaults: [6, 2, 0, 4], entry_m1_icon: 314159, table_addr: 0x17df40 }
    }

    fn items() -> QsItems {
        let n = ITEM_COUNT;
        let mut q = QsItems { icon: vec![0; n], name: vec![0; n], gold_name: vec![0; n], has_ammo: vec![false; n], max_ammo: vec![0; n] };
        for i in 9..n { q.icon[i] = 60000 + i as u16; }
        q.has_ammo[10] = true;
        q.max_ammo[10] = 40;
        q
    }

    fn ring() -> QuickSelect { QuickSelect::new(consts(), tables(), items()) }


fn assets()->MenuAssets {
    let mut glyphs=[[Glyph::default();GLYPHS];3];
    for table in &mut glyphs {for glyph in &mut table[32..127] {glyph.advance=10;}}
    let icons=vec![IconEntry{id:RING_ICON,frame_count:2,first_frame:0,..Default::default()},IconEntry{id:CURSOR_ICON,frame_count:1,first_frame:2,..Default::default()},IconEntry{id:0xffff,..Default::default()}];
    MenuAssets::new(HudAssets{icons,frame_sizes:vec![(32,32);4],glyphs,messages:vec![]},Overlay::default())
}
fn sprites(q:&QuickSelect,g:&Global,tick:u64)->Vec<(usize,i32,i32,i32,i32,i32)> {
    let mut out=vec![];q.draw(&assets(),g,tick,1,&mut out);
    out.into_iter().filter_map(|d|match d {MenuDraw::Hud(Draw::Sprite{frame,x,y,w,h,alpha,..})=>Some((frame,x,y,w,h,alpha)),_=>None}).collect()
}
#[test] fn equipped_item_uses_original_cursor_pulse_and_brightness(){
    let mut g=Global::zeroed();g.quick_select=[11,10,0,0,0,0,0,0];g.ammo[10]=12;
    let(q,local)=companion_wheel::page(&ring(),&g.quick_select,&g,10,None);
    assert_eq!(q.sel,1);let a=sprites(&q,&local,0);let b=sprites(&q,&local,8);
    let cursor=a.iter().find(|s|s.0==2).unwrap();let (x,y)=q.slot_pos(1);
    assert_eq!((cursor.1,cursor.2,cursor.3,cursor.4),(x-19,y-19,38,38));
    assert!((cursor.5-112).abs()<=1);assert_ne!(cursor.5,b.iter().find(|s|s.0==2).unwrap().5);
    assert_eq!(a.iter().filter(|s|s.0==2).count(),1);
    assert_eq!(a[4].5,64);assert_eq!(a[6].5,128);
}
#[test] fn extra_page_uses_its_own_ammo_assignment_without_mutating_saved_slots(){
    let mut g=Global::zeroed();g.quick_select=[11;8];g.ammo[10]=12;
    let(q,local)=companion_wheel::page(&ring(),&[10,12],&g,10,None);
    assert_eq!(q.sel,0);assert_eq!(local.quick_select,[10,12,0,0,0,0,0,0]);assert_eq!(g.quick_select,[11;8]);
    let mut out=vec![];q.draw(&assets(),&local,0,1,&mut out);
    assert!(out.iter().any(|d|matches!(d,MenuDraw::Hud(Draw::Text{text,..}) if text==b"12/40")));
}
#[test] fn absent_equipment_and_empty_slots_have_no_false_cursor(){
    for held in [0,8,12] {let(q,g)=companion_wheel::page(&ring(),&[10,0],&Global::zeroed(),held,None);assert_eq!(q.sel,-1);assert!(!sprites(&q,&g,0).iter().any(|s|s.0==2));}
}
#[test] fn clank_command_uses_original_command_cursor(){
    let mut q=ring();let mut entries=[Entry::default();CMD_SLOTS];
    for(i,e) in entries.iter_mut().enumerate(){e.item=i as i32+1;e.icon=30035;}
    q.commands=Some(Commands{entries,defaults:[0;4],msgs:[0;4],label_rgba:(0,0x80e0c0a0),label_dy:0});
    let(q,g)=companion_wheel::page(&q,&[1,2,3,4],&Global::zeroed(),10,Some(3));
    assert!(q.command_page);assert_eq!(q.sel,2);assert_eq!(sprites(&q,&g,0).iter().filter(|s|s.0==2).count(),1);
}

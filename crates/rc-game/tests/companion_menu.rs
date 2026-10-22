//! The Android adapter runs against the real menu tick, without a game-data fixture.
#[path = "../../rc-engine/src/android/companion_menu.rs"]
mod companion_menu;
#[path = "../../rc-engine/src/touch_input.rs"]
mod touch_input;
#[path = "../../rc-engine/src/android/companion_map.rs"]
mod companion_map;
#[path = "../../rc-engine/src/android/companion_policy.rs"]
mod companion_policy;
use rc_game::{game_state::GameState, menus::{MenuAssets, MenuInput, Overlay, pause::*}, pad::button};
use rc_formats::{font::Glyph, save_game::ChunkTables, strings::Message};
use std::collections::BTreeMap;

fn widget(id:u32,label:i16,action:i16,arg:u32)->Widget {
    Widget{addr:id,update:func::LIST_UPDATE,draw:func::LIST_DRAW,enter:0,leave:0,dflags:0,moby:0,rect:[0,0,160,40],raw:[0;8],data:Data::List(List{flags:0,items_addr:0,items:vec![Item{label,action,arg,sublabel:0,hl:0}],above:0,below:0,cursor:0,scroll:0})}
}
fn fixture()->(PageMenu,MenuAssets) {
    let mut m = PageMenu {
        consts: MenuConsts::default(),
        addrs: Addrs::default(),
        pages: BTreeMap::new(),
        widgets: BTreeMap::new(),
        kind: 0,
        current: 0,
        target: 0,
        post: 0,
        trans: 0,
        ticks: 0,
        prev: 0,
        goodies: false,
        no_close: false,
        arg: 0,
        return_page: 0,
        return_kind: 0,
        replay_hook: 0,
        dest: 0,
        mobys: [(0, false); 14],
        stub_calls: BTreeMap::new(),
        active: false,
        frames: None,
        enter_b13f4: 0,
        level_names: (0..20u32).map(|l| (20153 + l, 20172 + l)).collect(),
        planet_points: vec![[0; 4]; 19],
        name_dy: 17,
        equip: [0; 4],
        items: None,
        unusable_head: false,
        unusable_back: false,
        view: gadgets::GadgetsView::default(),
        view_model: None,
        text_swap: 0,
        label_tables: BTreeMap::new(),
        help_items: Default::default(),
        movie_lists: Vec::new(),
        log_ids: Vec::new(),
        lang: 0,
        ammo_records: Vec::new(),
        gold_spin: None,
        map: Default::default(),
        media: Default::default(),
        saves: Default::default(),
    };

    let root=m.addrs.root;let mut widgets=[0;14];widgets[0]=1;widgets[6]=7;
    m.widgets.insert(1,widget(1,port::text::ENTRY as i16,3,2));
    m.widgets.insert(7,widget(7,20418,3,3));
    m.widgets.insert(2,widget(2,20258,3,3));
    m.pages.insert(root,Page{addr:root,seqs:[0;14],parent:0,kind:2,focus:1,widgets,pending:0});
    widgets=[0;14];widgets[0]=2;
    m.pages.insert(2,Page{addr:2,seqs:[0;14],parent:root,kind:8,focus:2,widgets,pending:0});
    m.current=root;m.kind=2;m.ticks=20;m.active=true;
    let hud=rc_game::hud::HudAssets{icons:Vec::new(),frame_sizes:Vec::new(),glyphs:std::array::from_fn(|_|std::array::from_fn(|_|Glyph::default())),messages:vec![Message{id:20418,text:b"Goodies".to_vec(),help_audio:-1},Message{id:20258,text:b"Sound".to_vec(),help_audio:-1},Message{id:20308,text:b"Unavailable".to_vec(),help_audio:-1}]};
    (m,MenuAssets::new(hud,Overlay::default()))
}
fn press(bits:u32)->MenuInput {MenuInput{pressed_u:bits,raw_pressed:bits,held_u:bits,connected:true,..Default::default()}}

fn map_fixture() -> (PageMenu,MenuAssets,GameState) {
    use rc_game::map::{MapFile,Mask,SIZE};use std::sync::Arc;
    let (mut menu,assets)=fixture();
    let fog=552;let revealed=fog+SIZE*SIZE;let palette=revealed+SIZE*SIZE;let end=palette+0x420;
    let mut bytes=vec![0;end];bytes[fog..revealed].fill(1);bytes[revealed..palette].fill(2);
    for i in 0..256 {bytes[palette+0x20+i*4..palette+0x24+i*4].copy_from_slice(&[i as u8,255-i as u8,0,128]);}
    let file=MapFile{bytes:Arc::new(bytes),hdr:[32,552,fog,revealed,palette,end,end,end]};
    menu.dest=1;menu.map.state.level=1;menu.map.state.file=Some(file.clone());menu.map.state.mask=Mask::empty();menu.map.available=true;
    menu.map.loader=map_page::Loader(Some(Arc::new(move |_|Some(file.clone()))));
    let game=GameState::zeroed(ChunkTables{global:Vec::new(),level:Vec::new()});
    (menu,assets,game)
}

#[test]
fn companion_map_reuses_pixels_but_keeps_camera_and_native_draws_live() {
    use std::sync::Arc;
    let (mut cached,assets,game)=map_fixture();let mut baseline=cached.clone();
    let mut cache=companion_map::ImageCache::default();
    assert!(cache.refresh(&mut cached,&game));let picture=cached.map.picture.clone().unwrap();
    let pixels=companion_map::android_pixels(&picture);
    assert_eq!(&pixels[..2],&[512,512]);assert_eq!(pixels.len(),512*512+2);assert_eq!(pixels[2] as u32,0xff02fd00);
    for frame in 0..20 {
        for m in [&mut cached,&mut baseline] {
            m.map.hero.pos=[frame as f32,frame as f32*2.,3.];m.map.hero.yaw=frame as f32/10.;
            m.map.state.view[1].pan=[frame*1000,frame*2000];m.map.state.view[1].zoom=1.+frame as f32/40.;m.map.mirror=frame%2==0;
            map_page::enter(m,&game);
        }
        baseline.map.shown=-2;map_page::compose(&mut baseline,&game);
        assert!(!cache.refresh(&mut cached,&game));assert!(Arc::ptr_eq(&picture,cached.map.picture.as_ref().unwrap()));
        baseline.map.composed=cached.map.composed;
        assert_eq!(cached.map.picture.as_ref().unwrap().rgba,baseline.map.picture.as_ref().unwrap().rgba);
        let (mut a,mut b)=(Vec::new(),Vec::new());map_page::draw(&mut cached,1,&assets,&game,&mut a);map_page::draw(&mut baseline,1,&assets,&game,&mut b);assert_eq!(a,b);
    }
}

#[test]
fn companion_map_invalidates_for_fog_palette_file_and_map_o_matic() {
    use std::sync::Arc;
    let (mut menu,_,game)=map_fixture();let mut cache=companion_map::ImageCache::default();assert!(cache.refresh(&mut menu,&game));
    let original=menu.map.picture.clone().unwrap();menu.map.state.mask.set(0,0,true);
    assert!(cache.refresh(&mut menu,&game));assert_ne!(original.rgba,menu.map.picture.as_ref().unwrap().rgba);
    let mut palette=menu.map.state.file.as_ref().unwrap().palette().to_vec();palette[4]=7;menu.map.palettes.resize(2,None);menu.map.palettes[1]=Some(palette);
    assert!(cache.refresh(&mut menu,&game));assert!(!cache.refresh(&mut menu,&game));
    let file=menu.map.state.file.as_mut().unwrap();file.bytes=Arc::new(file.bytes.as_ref().clone());
    assert!(cache.refresh(&mut menu,&game));menu.map.state.map_o_matic=true;assert!(cache.refresh(&mut menu,&game));
    menu.map.picture=None;assert!(cache.refresh(&mut menu,&game));
    assert_ne!(companion_map::image_revision(7),7);
}

#[test]
fn mobile_triangle_backs_out_of_a_page_then_resumes_from_root() {
    let (mut menu,_)=fixture();let root=menu.current;
    let mut game=GameState::zeroed(ChunkTables{global:Vec::new(),level:Vec::new()});
    let mut touch=touch_input::InputState::default();let now=std::time::Instant::now();
    assert!(companion_menu::choose(&mut menu,root,1,0));
    menu.tick(&press(button::CROSS),&mut game,&MenuEnv::default());
    for _ in 0..14{menu.tick(&press(0),&mut game,&MenuEnv::default());}
    assert_eq!(menu.current,2);
    for expected in [root,0] {
        touch.touch(button::TRIANGLE,[0.;4],0,now);
        touch.touch(0,[0.;4],0,now);
        let sample=touch.tick_with_menu(now,false,false,false,true);
        assert_eq!(sample.buttons,button::TRIANGLE);
        let mut closed=menu.tick(&press(sample.buttons),&mut game,&MenuEnv::default()).exit.is_some();
        for _ in 0..20{closed|=menu.tick(&press(touch.tick_with_menu(now,false,false,false,true).buttons),&mut game,&MenuEnv::default()).exit.is_some();}
        if expected==0 {assert!(closed,"fresh mobile Triangle must resume gameplay");}
        else {assert_eq!(menu.current,expected);assert!(!closed);}
    }
}

#[test]
fn signed_port_labels_and_the_active_message_table_match_native() {
    let (mut menu,mut assets)=fixture();
    let view=companion_menu::screen(&menu,&assets).unwrap();
    assert_eq!(view.rows[0].label,"Port Options");
    assert_eq!(view.rows[0].detail,"");
    menu.current=2;menu.text_swap=2;
    assets.all_text=vec![Message{id:20258,text:b"Swapped Sound".to_vec(),help_audio:-1}];
    assert_eq!(companion_menu::screen(&menu,&assets).unwrap().rows[0].label,"Swapped Sound");
}
#[test]
fn hidden_native_frame_slots_are_neither_drawn_nor_selectable() {
    let (mut menu,assets)=fixture();let root=menu.current;
    assert_eq!(companion_menu::screen(&menu,&assets).unwrap().rows.len(),1);
    assert!(!companion_menu::choose(&mut menu,root,7,0));
    menu.goodies=true;
    assert_eq!(companion_menu::screen(&menu,&assets).unwrap().rows.len(),2);
    assert!(companion_menu::choose(&mut menu,root,7,0));
    menu.widgets.get_mut(&7).unwrap().dflags|=4;
    assert_eq!(companion_menu::screen(&menu,&assets).unwrap().rows.len(),1);
    assert!(!companion_menu::choose(&mut menu,root,7,0));
}
#[test]
fn list_drawn_confirm_widgets_keep_their_real_key_handler() {
    let (mut menu,assets)=fixture();let root=menu.current;
    menu.widgets.get_mut(&1).unwrap().update=func::CONFIRM_UPDATE;
    assert!(!companion_menu::screen(&menu,&assets).unwrap().supported);
    assert!(!companion_menu::choose(&mut menu,root,1,0));
}
#[test]
fn disabled_rows_match_the_native_unavailable_label() {
    let (mut menu,assets)=fixture();let root=menu.current;
    if let Data::List(l)=&mut menu.widgets.get_mut(&1).unwrap().data{l.items[0].action=2;}
    let view=companion_menu::screen(&menu,&assets).unwrap();
    assert_eq!(view.rows[0].label,"Unavailable");assert!(!view.rows[0].enabled);
    assert!(!companion_menu::choose(&mut menu,root,1,0));
}
#[test]
fn a_companion_choice_opens_the_real_page_and_back_resume_still_work() {
    let (mut menu,_)=fixture();let root=menu.current;
    let mut game=GameState::zeroed(ChunkTables{global:Vec::new(),level:Vec::new()});
    assert!(companion_menu::choose(&mut menu,root,1,0));
    menu.tick(&press(button::CROSS),&mut game,&MenuEnv::default());
    assert_eq!(menu.target,2,"The real LIST_UPDATE must accept the companion choice");
    assert!(!companion_menu::choose(&mut menu,root,1,0),"No stale second tap during a pending transition");
    for _ in 0..14{menu.tick(&press(0),&mut game,&MenuEnv::default());}
    assert_eq!(menu.current,2);
    menu.tick(&press(button::TRIANGLE),&mut game,&MenuEnv::default());
    for _ in 0..14{menu.tick(&press(0),&mut game,&MenuEnv::default());}
    assert_eq!(menu.current,root);
    menu.tick(&press(button::START),&mut game,&MenuEnv::default());
    let closed=(0..3).any(|_|menu.tick(&press(0),&mut game,&MenuEnv::default()).exit.is_some());
    assert!(closed,"Resume must close the actual menu");
}

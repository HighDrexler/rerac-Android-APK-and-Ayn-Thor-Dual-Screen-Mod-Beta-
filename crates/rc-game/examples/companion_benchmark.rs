//! Headless fork-only benchmark; requires the owner's extracted game data.
//! cargo run --release -p rc-game --example companion_benchmark
//! No game data is embedded in this example or its JSON results.
#[path="../../rc-engine/src/android/companion_map.rs"] mod companion_map;
#[path="../../rc-engine/src/android/companion_wheel.rs"] mod companion_wheel;
use rc_game::{game_state::{GameState,Global},map::{self,MapFile,MapState,Tables},hud::HudAssets,menus::{MenuAssets,MenuDraw,Overlay,pause::{PageMenu,Data,map_page},quick_select::QuickSelect}};
use rc_formats::{hud,wad,save_game::{ItemTables,ChunkTables}};
use std::{alloc::{GlobalAlloc,Layout,System},sync::{Arc,atomic::{AtomicBool,AtomicU64,Ordering}},time::Instant,hint::black_box};

struct Meter;
static TRACK:AtomicBool=AtomicBool::new(false);
static BYTES:AtomicU64=AtomicU64::new(0);
static ALLOCS:AtomicU64=AtomicU64::new(0);
fn count(size:usize){if TRACK.load(Ordering::Relaxed){BYTES.fetch_add(size as u64,Ordering::Relaxed);ALLOCS.fetch_add(1,Ordering::Relaxed);}}
unsafe impl GlobalAlloc for Meter {
    unsafe fn alloc(&self,l:Layout)->*mut u8{count(l.size());System.alloc(l)}
    unsafe fn alloc_zeroed(&self,l:Layout)->*mut u8{count(l.size());System.alloc_zeroed(l)}
    unsafe fn dealloc(&self,p:*mut u8,l:Layout){System.dealloc(p,l)}
    unsafe fn realloc(&self,p:*mut u8,l:Layout,n:usize)->*mut u8{count(n);System.realloc(p,l,n)}
}
#[global_allocator] static METER:Meter=Meter;
fn begin(){BYTES.store(0,Ordering::Relaxed);ALLOCS.store(0,Ordering::Relaxed);TRACK.store(true,Ordering::Relaxed);}
fn finish()->(u64,u64){TRACK.store(false,Ordering::Relaxed);(BYTES.load(Ordering::Relaxed),ALLOCS.load(Ordering::Relaxed))}
fn file(index:usize)->Option<MapFile>{
    let raw=std::fs::read(rc_formats::test_data::root().join(format!("global/unknown_0820/{index:03}.bin"))).ok()?;
    MapFile::parse(if wad::is_wad(&raw){wad::decompress(&raw).ok()?}else{raw})
}
fn assets()->(MenuAssets,QuickSelect){
    let root=rc_formats::test_data::root();let dir=root.join("levels/01");
    let header=std::fs::read(dir.join("hud_header.bin")).expect("Set RC_EXTRACTED to extracted game data");let h=hud::parse_header(&header).unwrap();
    let banks:Vec<Vec<u8>>=(0..hud::BANKS).map(|b|if h.bank_size[b]==0{vec![]}else{wad::decompress(&std::fs::read(dir.join(format!("hud_bank_{b}.bin"))).unwrap()).unwrap()}).collect();
    let hud=hud::parse_hud(&header,std::array::from_fn(|b|banks[b].as_slice())).unwrap();
    let overlay=std::fs::read(dir.join("overlay.bin")).unwrap();let ov=Overlay::parse(&overlay).unwrap();
    let items=ItemTables::load(&std::fs::read(root.join("boot/SCUS_971.99")).unwrap(),&overlay).unwrap();
    let ring=QuickSelect::load(&ov,&items).unwrap();let glyphs=rc_formats::font::parse_glyph_tables(&overlay).unwrap().0;
    let messages=rc_formats::strings::parse_strings(&rc_formats::test_data::gameplay(1).unwrap(),0).unwrap();
    (MenuAssets::new(HudAssets::new(&hud,glyphs,messages),ov),ring)
}
fn update(menu:&mut PageMenu,live:&MapState,game:&GameState,assets:&MenuAssets,cache:&mut companion_map::ImageCache,optimized:bool,frame:usize)->(bool,Vec<MenuDraw>){
    menu.map.state=live.clone();menu.map.hero.pos=[162.5+frame as f32/16.,136.4,60.5];menu.map.hero.yaw=frame as f32/100.;menu.map.hero.group=if frame%13==0{15}else{0};menu.map.state.view[live.level as usize].zoom=1.6;
    menu.map.state.view[live.level as usize].pan=[(0.5*(1u64<<28) as f32) as i32;2];menu.map.mirror=frame%41==0;
    map_page::enter(menu,game);
    let changed=if optimized {cache.refresh(menu,game)}else{menu.map.shown=-2;map_page::compose(menu,game);true};
    if changed {menu.map.composed=if optimized{companion_map::image_revision(frame as u64)}else{frame as u64};black_box(companion_map::android_pixels(menu.map.picture.as_ref().unwrap()));}
    let id=menu.widgets.iter().find(|(_,w)|matches!(w.data,Data::Map{..})).map(|(&id,_)|id).unwrap();menu.widgets.get_mut(&id).unwrap().rect=[0,0,512,384];
    let mut draws=Vec::new();map_page::draw(menu,id,assets,game,&mut draws);(changed,draws)
}
fn trace(live:&mut MapState,frame:usize,every:usize){if every>0&&frame%every==0{let p=(frame*97)%(map::SIZE*map::SIZE);let(x,y)=(p%map::SIZE,p/map::SIZE);live.mask.set(x,y,!live.mask.get(x,y));}}
fn verify(base:&PageMenu,live:&MapState,game:&mut GameState,assets:&MenuAssets){
    let(mut a,mut b)=(base.clone(),base.clone());let(mut ca,mut cb)=(companion_map::ImageCache::default(),companion_map::ImageCache::default());let mut state=live.clone();
    for frame in 0..64 {
        game.global.flags[32]=(frame%3==0) as u8;game.global.owned[2]=(frame%2==0) as u8;
        trace(&mut state,frame,7);state.alt=state.level==6&&frame%2==0;let(_,left)=update(&mut a,&state,game,assets,&mut ca,false,frame);let(_,mut right)=update(&mut b,&state,game,assets,&mut cb,true,frame);
        assert_eq!(a.map.picture.as_ref().unwrap().rgba,b.map.picture.as_ref().unwrap().rgba,"fog pixels changed");
        for (left,right) in a.map.layers.iter().zip(&b.map.layers){assert_eq!(left.as_ref().map(|t|&t.rgba),right.as_ref().map(|t|&t.rgba),"globe layers changed");}
        // Pixel keys may differ because the optimized image is deliberately reused.
        for(draw,expected)in right.iter_mut().zip(&left){if let (MenuDraw::Quad{tex:rc_game::menus::QuadTex::Image(rc_game::menus::ImageSrc::Pixels{key,..}),..},MenuDraw::Quad{tex:rc_game::menus::QuadTex::Image(rc_game::menus::ImageSrc::Pixels{key:old,..}),..})=(draw,expected){*key=*old;}}
        assert_eq!(left,right,"camera/markers/native geometry changed");
        assert_eq!(a.map.markers,b.map.markers);assert_eq!(a.map.missions,b.map.missions);
    }
}
fn bench_map(base:&PageMenu,live:&MapState,game:&GameState,assets:&MenuAssets,every:usize,label:&str){
    let mut menus=[base.clone(),base.clone()];let mut live=live.clone();let mut caches=[companion_map::ImageCache::default(),companion_map::ImageCache::default()];
    for i in 0..2 {update(&mut menus[i],&live,game,assets,&mut caches[i],i==1,0);}
    let n=600;let mut times=[Vec::with_capacity(n),Vec::with_capacity(n)];let mut changes=[0;2];let mut bytes=[0;2];let mut allocs=[0;2];
    // Alternate old/new order every update to limit thermal/order/heap bias.
    for frame in 1..=n {trace(&mut live,frame,every);for i in if frame%2==0{[0,1]}else{[1,0]} {
        begin();let at=Instant::now();let(changed,draw)=update(&mut menus[i],&live,game,assets,&mut caches[i],i==1,frame);black_box(draw);let elapsed=at.elapsed().as_nanos() as u64;let(b,a)=finish();
        times[i].push(elapsed);changes[i]+=changed as usize;bytes[i]+=b;allocs[i]+=a;
    }}
    for i in 0..2 {times[i].sort_unstable();println!("{{\"kind\":\"map\",\"level\":{},\"workload\":\"{}\",\"optimized\":{},\"updates\":{},\"median_us\":{:.3},\"p95_us\":{:.3},\"allocated_bytes_per_update\":{},\"allocations_per_update\":{:.2},\"texture_rebuilds\":{}}}",live.level,label,i==1,n,times[i][n/2] as f64/1000.,times[i][n*95/100] as f64/1000.,bytes[i]/n as u64,allocs[i] as f64/n as f64,changes[i]);}
}
fn bench_wheel(ring:&QuickSelect,assets:&MenuAssets,optimized:bool){
    let mut global=Global::zeroed();global.quick_select=[10,11,12,13,14,15,16,17];global.ammo[10]=12;
    let pages=[[10,11,12,13,14,15,16,17],[18,19,20,21,22,23,24,25],[26,27,28,29,30,31,32,33]];let n=3000;let mut times=Vec::with_capacity(n);
    begin();for frame in 0..n {let at=Instant::now();let mut draw=Vec::new();if optimized{let(mut q,mut g)=companion_wheel::page(ring,&pages[0],&global,10,None);for(i,page)in pages.iter().enumerate(){if i!=0{companion_wheel::prepare(&mut q,page,&mut g,10,None);}q.draw(assets,&g,frame as u64,0,&mut draw);}}else{for page in &pages{let(q,g)=companion_wheel::page(ring,page,&global,10,None);q.draw(assets,&g,frame as u64,0,&mut draw);}}black_box(draw);times.push(at.elapsed().as_nanos() as u64);}
    let(bytes,allocs)=finish();times.sort_unstable();println!("{{\"kind\":\"three_page_wheel\",\"optimized\":{},\"updates\":{},\"median_us\":{:.3},\"p95_us\":{:.3},\"allocated_bytes_per_update\":{},\"allocations_per_update\":{:.2}}}",optimized,n,times[n/2] as f64/1000.,times[n*95/100] as f64/1000.,bytes/n as u64,allocs as f64/n as f64);
}
fn main(){
    let order=if std::env::var_os("PERF_REVERSE").is_some(){[true,false]}else{[false,true]};
    let(assets,ring)=assets();let files:Vec<_>=(0..38).map(file).collect();assert!(files.iter().all(Option::is_some));
    for level in 0..19 {
        for map_o_matic in [false,true] {
            let mut game=GameState::zeroed(ChunkTables{global:Vec::new(),level:Vec::new()});game.global.level=level;game.levels[level as usize].visited=1;
            let mut menu=PageMenu::load(&assets.overlay).unwrap();menu.dest=level;menu.map.available=true;
            let cached=files.clone();menu.map.loader=map_page::Loader(Some(Arc::new(move |i|cached.get(i).cloned().flatten())));
            let live=MapState::enter(&Tables::read(&assets.overlay).unwrap(),level,map_o_matic,files[map::file_index(level as usize,map_o_matic)].clone(),&[]);
            verify(&menu,&live,&mut game,&assets);
        }
    }
    for level in [1,6,12] {
        let mut game=GameState::zeroed(ChunkTables{global:Vec::new(),level:Vec::new()});game.global.level=level;game.levels[level as usize].visited=1;
        let mut menu=PageMenu::load(&assets.overlay).unwrap();menu.dest=level;menu.map.available=true;
        let cached=files.clone();menu.map.loader=map_page::Loader(Some(Arc::new(move |i|cached.get(i).cloned().flatten())));
        let live=MapState::enter(&Tables::read(&assets.overlay).unwrap(),level,false,files[level as usize].clone(),&[]);
        for(every,label)in [(0,"moving_in_revealed_area"),(20,"fog_changes_every_20_updates"),(1,"fog_changes_every_update")] {bench_map(&menu,&live,&game,&assets,every,label);}
    }
    for optimized in order{bench_wheel(&ring,&assets,optimized);}
    println!("{{\"validation\":\"pixel and native draw equivalence passed on all 19 real maps, both picture sets, 64 transitions each (2432 comparisons)\",\"scope\":\"desktop release-build companion CPU/allocation microbenchmarks; allocator instrumentation enabled; excludes primary renderer, GPU, JNI and Android Bitmap allocation\"}}");
}

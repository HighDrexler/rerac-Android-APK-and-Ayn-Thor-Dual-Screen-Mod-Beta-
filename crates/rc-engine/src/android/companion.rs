//! v16 companion protocol backed by the real 0.3.5 menu/map/inventory state.
//! Uses the same PS2 draw conversion as the primary display, including all three
//! screen-static passes. Rendering/snapshots never advance the gameplay RNG.
use super::*;
use crate::gameplay::{Persistent, Play, Session};
use crate::hud_render::{Hud2d, LevelHud, Prim, Tex};
use crate::menu_render::{MenuMode, MenuRt};
use crate::text_render::TextState;
use rc_formats::texture::Texture;
use rc_game::game_state::GameState;
use rc_game::menus::{ImageSrc, MenuAssets, MenuDraw, MenuInput};
use rc_game::menus::mode::Mode;
use rc_game::menus::pause::{Data, PageMenu, lf, map_page};
use rc_game::menus::quick_select::{HeroGate, QuickSelect};
use rc_game::pad::button;
use serde_json::{Value, json};
use std::collections::HashSet;
use std::path::Path;
use std::time::Duration;

#[derive(Default)]
pub(crate) struct Cache {
    generation: u64,
    language: u32,
    at: Option<Instant>,
    frame: u64,
    next_dynamic: usize,
    icons: HashMap<i32,Arc<[i32]>>,
    dynamics: HashMap<(u32,u64),(usize,u64)>,
    map_menu: Option<PageMenu>,
}
fn text(raw:&[u8])->String { raw.iter().copied().take_while(|&c|c!=0).filter(|&c|c>=32||c==10).map(char::from).collect() }
fn pixels(t:&Texture)->Arc<[i32]> {
    let n=(t.width as usize).saturating_mul(t.height as usize);
    if n>1024*1024 || t.rgba.len()<n*4 {return Arc::from([0,0]);}
    let mut out=Vec::with_capacity(n+2);out.push(t.width as i32);out.push(t.height as i32);
    out.extend(t.rgba.chunks_exact(4).take(n).map(|p| {
        let a=(u32::from(p[3])*255/128).min(255);
        ((a<<24)|(u32::from(p[0])<<16)|(u32::from(p[1])<<8)|u32::from(p[2])) as i32
    })); out.into()
}
impl Cache {
    fn texture(&mut self, tex:Tex, hud:&LevelHud)->i32 {
        let (key,t)=match tex {
            Tex::None=>return 0,
            Tex::Frame(i)=>(i as i32+1,hud.frames.get(i)),
            Tex::Fx(i)=>(-(i as i32)-1,hud.fx.get(i).and_then(|x|x.as_ref())),
            Tex::Dyn(i)=>return 20000+i as i32,
        };
        if let Some(t)=t {self.icons.entry(key).or_insert_with(||pixels(t)); key} else {0}
    }
    fn resolve(&mut self, src:&ImageSrc)->Option<usize> {
        let key=src.key();
        if let Some((i,frame))=self.dynamics.get_mut(&key) {*frame=self.frame;return Some(*i)}
        let tex=match src { ImageSrc::Pixels{tex,..}=>tex.clone(), ImageSrc::Lump{field,index}=>Arc::new(crate::hud_images::read_lump_picture(*field,*index)?) };
        let i=self.next_dynamic; self.next_dynamic=self.next_dynamic.wrapping_add(1)%100_000_000;
        self.icons.insert(20000+i as i32,pixels(&tex));self.dynamics.insert(key,(i,self.frame));Some(i)
    }
    fn convert(&mut self, draws:&[MenuDraw], a:&MenuAssets, hud:&LevelHud)->Vec<Prim> {
        let mut h=Hud2d::default(); h.frame_sizes=a.hud.frame_sizes.clone();
        let mut statics:[Vec<Prim>;3]=Default::default();let mut st=TextState::default();
        crate::menu_render::convert(draws,&mut h,&mut statics,&mut st,&hud.glyphs,&mut |src|self.resolve(src));
        // v16 Java reconstructs GS noise/glass blending, but still needs the
        // exact native static primitives and original textures to identify panels.
        for pass in statics {h.prims.extend(pass);}
        h.prims
    }
    fn prims(&mut self, p:&[Prim], hud:&LevelHud)->Vec<Value> {
        p.iter().map(|p|json!([self.texture(p.tex,hud),p.pos,p.uv,p.rgba,p.scissor,p.repeat,p.uv16])).collect()
    }
    fn wheel_draws(&mut self,p:&[Prim],hud:&LevelHud)->Vec<Value> {
        p.iter().filter(|p|p.tex!=Tex::None).map(|p| {
            let key=self.texture(p.tex,hud);let div=if p.uv16 {16}else{1};
            json!([key,p.pos[0][0],p.pos[0][1],p.pos[1][0]-p.pos[0][0],p.pos[2][1]-p.pos[0][1],p.uv[0][0]/div,p.uv[0][1]/div,(p.uv[1][0]-p.uv[0][0]).abs()/div,(p.uv[2][1]-p.uv[0][1]).abs()/div,p.rgba&255,(p.rgba>>8)&255,(p.rgba>>16)&255,(p.rgba>>24)&255])
        }).collect()
    }
    fn prune(&mut self) {
        let frame=self.frame;
        let stale:Vec<_>=self.dynamics.iter().filter(|(_,(_,at))|frame.saturating_sub(*at)>2).map(|(k,(i,_))|(*k,*i)).collect();
        for (key,i) in stale {self.dynamics.remove(&key);self.icons.remove(&(20000+i as i32));}
    }
}
fn gate(play:&Play,gs:&GameState,session:&rc_game::game_state::SessionState)->HeroGate {
    let h=&play.game.hero;
    HeroGate {early_exit:session.hp<1,swap_state:h.items.slot.swap,f594:play.svc.interact.owner_at_hero,b13fc:h.items.f13fc,b13f7:h.items.f13f7,b13f4:h.mode,s3f502:h.f502,held_item:if gs.global.wrench_held!=0 {8}else{gs.global.equipped[0]}}
}
fn allowed(g:&HeroGate)->bool { !g.early_exit&&g.swap_state!=2&&g.f594==0&&(g.b13fc==0||g.b13f4==1||g.s3f502!=0)&&(g.b13f7==0||g.b13f4==1||g.s3f502!=0)&&matches!(g.b13f4,0|1|3) }

pub(crate) fn apply_commands(mut rt:Option<ResMut<MenuRt>>, mut play:Option<ResMut<Play>>, mut gs:Option<ResMut<Persistent>>,mut sess:Option<ResMut<Session>>,mm:Option<Res<MenuMode>>) {
    if paused(){return;}
    let (Some(rt),Some(play),Some(gs),Some(sess),Some(mm))=(rt.as_deref_mut(),play.as_deref_mut(),gs.as_deref_mut(),sess.as_deref_mut(),mm.as_deref()) else{return;};
    let work:Vec<_>=commands().lock().unwrap_or_else(|e|e.into_inner()).drain(..).collect();
    for cmd in work {match cmd {
        Command::Select{slot,item,generation}=>{
            if generation!=GENERATION.load(Ordering::Acquire)||mm.state.mode!=Mode::Gameplay||crate::saves::front_end_active(){continue;}
            let g=gate(play,&gs.0,&sess.0);if !allowed(&g){continue;}
            let Some(q)=rt.qs.as_ref() else{continue};
            let mut q=q.clone();q.open=true;q.sel=(slot%8) as i32;q.arm=0x0100_00ff;q.fade=0;q.lock_timer=0;
            if g.b13f4==1 {
                let Some(c)=q.commands.as_ref() else{continue};
                if c.entries.get(slot).is_none_or(|e|e.item!=item){continue;}
                q.command_page=true;
            }else{
                if item<=0||gs.0.global.owned.get(item as usize).is_none_or(|&b|b==0){continue;}
                q.command_page=false;q.entries[slot%8].item=item;
                q.entries[slot%8].icon=q.items.icon.get(item as usize).copied().unwrap_or(0) as i32;
                let part=rt.menu.as_ref().and_then(|m|m.items.as_ref()).and_then(|items|items.get(item)).map(|i|i.slot);
                match part {
                    Some(1)=>{sess.0.temp_feet=item;continue;},
                    Some(2)=>{if !rt.menu.as_ref().is_some_and(|m|m.unusable_head){sess.0.temp_head=item;}continue;},
                    Some(3)=>{if !rt.menu.as_ref().is_some_and(|m|m.unusable_back){sess.0.temp_back=item;}continue;},
                    Some(0)=>{},_=>continue,
                }
            }
            let out=q.update(&MenuInput::default(),&g,&mut gs.0.global,&mut sess.0,mm.loop_frame as u32,play.svc.units.bot_listeners);
            if let Some(command)=out.command {play.game.hero.bodies.command=command;}
            // The ordinary quick-select's release writes 0x141408. The live
            // gameplay tick reads item_globals.request; keeping both in sync
            // makes touch selection identical to a physical △ release.
            if let Some(request)=out.request {
                sess.0.temp_hand=request;
                play.game.item_globals.request=request;
            }
        },
        Command::Touch{page,x,y}=>{
            let Some(menu)=rt.menu.as_mut() else{continue};
            if mm.state.mode==Mode::Menu&&menu.current==page&&menu.trans==0 {if touch(menu,x,y){pulse(button::CROSS);}}
        },
        Command::Gesture{x,y,dx,dy,scale}=>{
            let Some(menu)=rt.menu.as_mut() else{continue};if mm.state.mode!=Mode::Menu||menu.trans!=0 {continue;}
            let Some(page)=menu.pages.get(&menu.current) else{continue};
            let ids=page.widgets;
            for id in ids {let Some(w)=menu.widgets.get_mut(&id) else{continue};let [a,b,c,d]=w.rect;
                if x<a as f32||y<b as f32||x>=(a+c) as f32||y>=(b+d) as f32{continue;}
                match &mut w.data {
                    Data::Preview(p)=>p.angle=(p.angle+dx*0.015).rem_euclid(std::f32::consts::TAU),
                    Data::Map{..}=>{if let Some(v)=menu.map.state.view.get_mut(menu.dest.max(0) as usize){v.zoom=(v.zoom*scale).clamp(0.65,4.);let f=(1u64<<28) as f32/(512.*v.zoom);v.pan[0]=(v.pan[0] as f32-dx*f).clamp(0.,(1u64<<28) as f32) as i32;v.pan[1]=(v.pan[1] as f32-dy*f).clamp(0.,(1u64<<28) as f32) as i32;}},
                    Data::Label(l)=>l.scroll=l.scroll.saturating_sub((dy*16.) as i32).max(0),
                    _=>if dy.abs()>12.{pulse(if dy<0.{button::DOWN}else{button::UP});},
                }
            }
        },
    }}
}
/// Match the widget's on-disc draw geometry; let its original update execute
/// validation and action logic after the cursor/focus is selected.
fn touch(menu:&mut PageMenu,x:f32,y:f32)->bool {
    let Some(page)=menu.pages.get(&menu.current) else{return false};let ids=page.widgets;
    for id in ids {
        let Some(w)=menu.widgets.get_mut(&id) else{continue};let [a,b,c,d]=w.rect;
        if w.dflags&4!=0||c<=0||d<=0||x<a as f32||y<b as f32||x>=(a+c) as f32||y>=(b+d) as f32{continue;}
        let u=(x-a as f32)/c as f32;let v=(y-b as f32)/d as f32;
        let nearest=|n:usize|->i32 {((v*(n as f32+1.)-1.).round() as i32).clamp(0,n.saturating_sub(1) as i32)};
        let activate=match &mut w.data {
            Data::List(l) if !l.items.is_empty()=>{
                if l.flags&lf::STEP_FROM_FONT!=0 {let size=if l.flags&lf::LARGE!=0 {32.}else if l.flags&lf::SMALL!=0 {12.}else{16.};l.cursor=(((y-b as f32)/(size+3.)-1.).round() as i32).clamp(0,l.items.len() as i32-1);}else{l.cursor=nearest(l.items.len());}
                l.items[l.cursor as usize].action!=0
            },
            Data::Grid(g) if !g.cells.is_empty()=>{let col=(u*g.cols.max(1) as f32).floor() as i32;let row=(v*g.rows.max(1) as f32).floor() as i32;g.cursor=(row*g.cols+col).clamp(0,g.cells.len() as i32-1);true},
            Data::Toggle(t) if !t.entries.is_empty()=>{t.cursor=nearest(t.entries.len());true},
            Data::Camera(t) if !t.entries.is_empty()=>{t.cursor=nearest(t.entries.len());true},
            Data::Port(p)=>{p.cursor=(p.top+nearest(rc_game::menus::pause::port::VISIBLE)).clamp(0,rc_game::menus::pause::port::ENTRIES.len() as i32-1);true},
            Data::Sound{cursor}=>{*cursor=nearest(3);if *cursor<2 {pulse(if u>0.5 {button::RIGHT}else{button::LEFT});false}else{true}},
            Data::Slots{cursor,..}=>{*cursor=(u*8.).floor() as i32;true},
            Data::IconList(l) if !l.cells.is_empty()=>{l.cursor=nearest(l.cells.len());true},
            Data::Map{..}|Data::Missions(_)|Data::Confirm=>true,
            _=>false,
        };
        if activate {if let Some(p)=menu.pages.get_mut(&menu.current){p.focus=id;p.pending=0;}return true;}
    } false
}
fn map_draws(cache:&mut Cache,source:&PageMenu,play:&Play,gs:&GameState,a:&MenuAssets)->(Vec<MenuDraw>,Value) {
    let m=cache.map_menu.get_or_insert_with(||source.clone());
    let live=if play.svc.map.file.is_some(){&play.svc.map}else{&source.map.state};
    if live.file.is_none(){return (Vec::new(),json!({}));}
    m.map.state=live.clone();m.dest=live.level;m.map.available=true;
    m.map.loader=map_page::Loader(Some(Arc::new(crate::gameplay::map_file)));
    let h=&play.game.hero;m.map.hero=map_page::HeroMark{pos:[h.pos[0].to_f32(),h.pos[1].to_f32(),h.pos[2].to_f32()],yaw:h.rot[2].to_f32(),group:h.group};
    m.map.mirror=gs.global.cheats_active[rc_game::cheats::slot::MIRROR]!=0;
    m.map.hooks=play.svc.interact.talk_slots.iter().filter_map(|(&i,&k)|play.game.mobys.mobys.get(i).map(|o|(k,(o.position[0],o.position[1],o.rotation[2])))).collect();
    let (hu,hv)=rc_game::map::world_to_map(&live.transforms,live.level,live.alt,h.pos[0].to_f32(),h.pos[1].to_f32());
    let v=*view().lock().unwrap_or_else(|e|e.into_inner());let (u,w)=if v.follow {(hu,hv)}else{(v.u,v.v)};
    if let Some(view)=m.map.state.view.get_mut(m.dest.max(0) as usize){view.zoom=v.zoom;view.pan=[(u*(1u64<<28) as f32) as i32,(w*(1u64<<28) as f32) as i32];}
    m.map.shown=-2;map_page::enter(m,gs);map_page::compose(m,gs);
    // Unique keys across cloned map compositions prevent Java from retaining
    // stale fog textures when the user explores or returns from the pause map.
    m.map.composed=cache.frame;
    let Some(id)=m.widgets.iter().find(|(_,w)|matches!(w.data,Data::Map{..})).map(|(&i,_)|i) else{return(Vec::new(),json!({}));};
    if let Some(w)=m.widgets.get_mut(&id){w.rect=[0,0,512,384];}
    let mut out=Vec::new();map_page::draw(m,id,a,gs,&mut out);
    let (missions,_)=map_page::mission_list(&m.map.missions,true);
    let name=m.level_names.get(gs.global.level.max(0) as usize).copied().unwrap_or((0,0));
    let metadata=json!({"mapTitle":text(a.msg(name.1 as i32)),"objectives":missions.iter().filter(|m|!m.done).map(|m|text(a.msg(m.text))).collect::<Vec<_>>(),"mapCentre":[u,w],"mapZoom":v.zoom,"mapHero":[hu,hv],"mapMirror":m.map.mirror});
    (out,metadata)
}
/// The HUD fallback icon is frame zero even when the requested ID does not
/// exist. That is fine for the game draw path, but not for a named companion
/// icon; zero must mean "no valid image" rather than an unrelated sprite.
fn checked_icon(cache:&mut Cache,a:&MenuAssets,hud:&LevelHud,id:u16,frame:i32)->i32 {
    let Some(entry)=a.hud.icons.iter().find(|entry|entry.id==id) else{return 0};
    if frame<0||frame>=entry.frame_count as i32{return 0}
    let index=(entry.first_frame as usize).saturating_add(frame as usize);
    if index>=hud.frames.len(){return 0}
    cache.texture(Tex::Frame(index),hud)
}
fn collectibles(menu:&PageMenu,gs:&GameState,a:&MenuAssets,cache:&mut Cache,hud:&LevelHud)->Value {
    let level=gs.global.level.max(0) as usize;
    let found=gs.levels.get(level).map_or(0,|l|l.gold_bolts.iter().filter(|&&b|b!=0).count());
    let total=menu.map.tables.gold_totals.get(level).copied().unwrap_or(0);
    // Locate the actual Skills list by its neighbouring Skill widget, rather
    // than hard-code overlay addresses that differ between planets.
    let mut skills=Vec::new();
    for page in menu.pages.values() {
        if !page.widgets.iter().any(|id|menu.widgets.get(id).is_some_and(|w|matches!(w.data,Data::Skill))){continue;}
        for id in page.widgets {let Some(Data::List(l))=menu.widgets.get(&id).map(|w|&w.data) else{continue};
            if l.items.len()<20 {continue;}
            for (id,it) in l.items.iter().enumerate().take(30){if it.arg as i32!=gs.global.level&&it.arg as i32!=-1{continue;}
                skills.push(json!({"id":id,"name":text(a.msg(it.label as i32)),"instruction":if it.sublabel!=0{text(a.msg(it.sublabel as i32))}else{String::new()},"complete":gs.global.skill_points.get(id).is_some_and(|&b|b!=0)}));
            }
        }
        if !skills.is_empty(){break;}
    }
    let skill_found=skills.iter().filter(|v|v["complete"]==true).count();
    // These are HUD icon IDs, not overlay addresses. Previously 0xe998
    // and 0xea24 were treated as icon IDs; absent IDs silently mapped to
    // frame zero (an unrelated sprite). Never publish a false texture.
    let bolt=checked_icon(cache,a,hud,rc_game::hud::icon::BOLT,0);
    let skill=checked_icon(cache,a,hud,0xea24,0);
    let check=checked_icon(cache,a,hud,0xea24,1);
    json!({"level":gs.global.level,"goldFound":found,"goldTotal":total,"skillFound":skill_found,"skillTotal":skills.len(),"skills":skills,"boltIcon":bolt,"skillIcon":skill,"checkIcon":check})
}

pub(crate) fn publish(rt:Option<Res<MenuRt>>,level:Res<crate::Level>,play:Option<Res<Play>>,gs:Option<Res<Persistent>>,sess:Option<Res<Session>>,mm:Option<Res<MenuMode>>,scene:Option<Res<crate::hud_render::SceneLayer>>,mut cache:Local<Cache>) {
    let (Some(rt),Some(play),Some(gs),Some(sess),Some(mm),Some(hud))=(rt.as_deref(),play.as_deref(),gs.as_deref(),sess.as_deref(),mm.as_deref(),level.0.hud.as_ref()) else{return;};
    let frontend=crate::saves::front_end_active();
    let mode=mm.state.mode;
    let (in_menu,touch_mode)=super::companion_policy::display_mode(mode.raw(),frontend);
    MENU.store(in_menu,Ordering::Release);
    TOUCH_MODE.store(touch_mode,Ordering::Release);
    let now=Instant::now();let interval=if in_menu {Duration::from_millis(40)}else{Duration::from_millis(90)};
    if cache.at.is_some_and(|last|now.saturating_duration_since(last)<interval){return;}
    let mut generation=GENERATION.load(Ordering::Acquire);
    if cache.generation==generation&&cache.language!=hud.lang {generation=GENERATION.fetch_add(1,Ordering::AcqRel)+1;}
    if cache.generation!=generation {*cache=Cache{generation,language:hud.lang,..Default::default()};}
    cache.at=Some(now);cache.frame+=1;
    let mut selection=Vec::new();let mut pages=Vec::new();let mut bounds=[0,0,256,256];
    let allow=mode==Mode::Gameplay&&!frontend&&allowed(&gate(play,&gs.0,&sess.0));
    if let Some(q)=rt.qs.as_ref(){
        let (ox,oy)=q.origin();bounds=[ox,oy,q.consts.size.0,q.consts.size.1];
        let mut ids=gs.0.global.quick_select.to_vec();
        for (id,owned) in gs.0.global.owned.iter().enumerate(){
            if *owned!=0&&!ids.contains(&(id as i32))&&q.items.icon.get(id).is_some_and(|&i|i!=0)&&rt.menu.as_ref().and_then(|m|m.items.as_ref()).and_then(|t|t.get(id as i32)).is_some_and(|i|(0..4).contains(&i.slot)){ids.push(id as i32);}
        }
        ids.resize(ids.len().max(16).div_ceil(8)*8,0);
        if play.game.hero.mode==1 {ids=q.commands.as_ref().map(|c|c.entries.iter().map(|e|e.item).collect()).unwrap_or_default();ids.resize(8,0);}
        for group in ids.chunks(8){
            let mut ring=q.clone();ring.open=true;ring.command_page=play.game.hero.mode==1;ring.fade=ring.consts.steps.clamp(0,255) as u8;ring.text_fade=ring.fade;ring.sel=-1;
            let mut slots=Vec::new();
            for (i,&item) in group.iter().enumerate(){
                ring.entries[i].item=item;ring.entries[i].icon=if item>0{ring.items.icon.get(item as usize).copied().unwrap_or(0) as i32}else{0};
                let (x,y)=ring.slot_pos(if play.game.hero.mode==1 {i*2}else{i});slots.push(json!({"id":item,"x":x,"y":y,"enabled":allow&&item!=0&&(play.game.hero.mode!=1||play.svc.units.bot_listeners>0)}));selection.push(item);
            }
            let mut draw=Vec::new();ring.draw(&rt.assets,&gs.0.global,play.game.counter,play.svc.units.bot_listeners,&mut draw);
            let p=cache.convert(&draw,&rt.assets,hud);let draws=cache.wheel_draws(&p,hud);
            pages.push(json!({"slots":slots,"draws":draws}));
        }
    }
    while pages.len()<2 {pages.push(json!({"slots":[],"draws":[]}));}
    // The small font (FX1) is v16's key -2.
    cache.texture(Tex::Fx(1),hud);
    let wheel=json!({"generation":generation,"enabled":allow,"box":bounds,"glyphs":hud.glyphs[0].iter().map(|g|[g.u as i32,g.v as i32,g.y_off as i32,g.advance as i32]).collect::<Vec<_>>(),"pages":pages}).to_string();
    // The original Android title already draws Start/Load text; duplicating
    // the game's font quads overlaps its buttons. Preserve the panels/art.
    // In-engine cutscenes publish SceneLayer subtitles (rt.draws is empty
    // there) so the bottom display remains useful during dialogue.
    let prims=if mode==Mode::Cutscene {
        scene.as_ref().map_or_else(Vec::new, |layer|cache.prims(&layer.prims,hud))
    }else{
        let p=cache.convert(&rt.draws,&rt.assets,hud);
        let p=p.into_iter().filter(|p|super::companion_policy::native_frontend_primitive(frontend,matches!(p.tex,Tex::Fx(_)))).collect::<Vec<_>>();
        cache.prims(&p,hud)
    };
    let panels:Vec<_>=rt.draws.iter().filter_map(|d|if let MenuDraw::PanelBegin{x,y,w,h,..}=d {Some([*x,*y,*w,*h])}else{None}).collect();
    let mut comp=json!({"menu":in_menu,"mode":mode.raw(),"page":rt.menu.as_ref().map_or(0,|m|m.current),"transition":rt.menu.as_ref().is_some_and(|m|m.trans!=0),"continuousArrows":false,"focusedPanels":false,"panels":panels,"prims":prims,"map":[],"textures":cache.frame});
    if let Some(menu)=rt.menu.as_ref(){
        comp["collectibles"]=collectibles(menu,&gs.0,&rt.assets,&mut cache,hud);
        if MAP_VISIBLE.load(Ordering::Acquire)&&secondary(){let (draw,meta)=map_draws(&mut cache,menu,play,&gs.0,&rt.assets);let p=cache.convert(&draw,&rt.assets,hud);comp["map"]=json!(cache.prims(&p,hud));if let Some(m)=meta.as_object(){for(k,v)in m{comp[k]=v.clone();}}}
    }
    cache.prune();
    let snapshot=Snapshot{wheel:Arc::from(wheel),companion:Arc::from(comp.to_string()),icons:cache.icons.clone(),selection,generation};
    *published().write().unwrap_or_else(|e|e.into_inner())=snapshot;
}

pub(super) fn planet_icon(root:&Path)->Option<Arc<[i32]>> {
    // Disc-authentic globe layer from the map pack; never a downloaded image.
    let raw=crate::disc_source::read(root,"global/unknown_0820/000.bin").ok()?;
    let bytes=if rc_formats::wad::is_wad(&raw){rc_formats::wad::decompress(&raw).ok()?}else{raw.to_vec()};
    let file=rc_game::map::MapFile::parse(bytes)?;
    let p=file.pif(0)?;
    p.decode_raw().ok().map(|t|pixels(&t))
}

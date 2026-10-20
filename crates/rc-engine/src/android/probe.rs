//! Opt-in native diagnostics for a disposable Android test or support session.
//! A valid nonce must be written into this app's private files directory; it is
//! consumed once. There is no exported Android activity or network trigger.
//! This tests native startup/GPU/input/lifecycle, NOT Ratchet gameplay or saves.
use super::*;
use bevy::render::{view::screenshot::{Screenshot, ScreenshotCaptured}, renderer::RenderAdapterInfo};
use bevy::shader::Shader;
use std::path::{Path,PathBuf};
use serde_json::json;

#[derive(Resource)] struct Probe { root:PathBuf, nonce:String, frames:u64, ticks:u64, buttons:u16, axes:[u8;4], capture_requested:bool, captured:bool, handles:Vec<Handle<Shader>>, report_at:Instant }
const SHADERS:&[&str]=&["aa_blit","display_blend","effect_blit","fade","fx_prim","hud","menu_snapshot","mirror","missile_view","moby","moby_metal","particle","screen_tint","shadow_resolve","shadow_volume","shrub","shrub_billboard","sky","sky_stars","tfrag","tie","underwater_tint","water","world_lights"];
pub(super) fn requested(root:&Path)->Option<String> {
    let path=root.join("native-probe.request");
    let meta=std::fs::metadata(&path).ok()?;
    if meta.len()>128{return None;}
    let nonce=std::fs::read_to_string(&path).ok()?.trim().to_owned();
    if nonce.len()!=32||!nonce.bytes().all(|b|b.is_ascii_hexdigit()){return None;}
    std::fs::remove_file(path).ok()?;Some(nonce)
}
pub(super) fn run(root:PathBuf,nonce:String) {
    TOUCH_MODE.store(0,Ordering::Release);
    let mut app=App::new();
    app.add_plugins(DefaultPlugins.set(AssetPlugin{file_path:String::new(),..default()}).set(WindowPlugin{primary_window:Some(Window{title:"ReRAC native runtime test".into(),..default()}),..default()}))
       .insert_resource(ClearColor(Color::srgb(0.025,0.035,0.055)))
       .insert_resource(Time::<Fixed>::from_hz(60.0))
       .init_resource::<crate::input_map::PadFrame>()
       .insert_resource(Probe{root,nonce,frames:0,ticks:0,buttons:0,axes:[127;4],capture_requested:false,captured:false,handles:Vec::new(),report_at:Instant::now()})
       .add_plugins(crate::frame_pace::FramePacePlugin)
       .add_systems(Startup,setup)
       .add_systems(FixedUpdate,(super::sample_tick,sample).chain())
       .add_systems(Update,report);
    app.run();
}
fn setup(mut commands:Commands,assets:Res<AssetServer>,mut probe:ResMut<Probe>) {
    commands.spawn(Camera2d);
    for (i,color) in [Color::srgb(0.1,0.65,0.9),Color::srgb(0.95,0.65,0.12),Color::srgb(0.15,0.8,0.4)].into_iter().enumerate(){commands.spawn((Sprite::from_color(color,Vec2::new(150.,160.)),Transform::from_xyz((i as f32-1.)*175.,0.,0.)));}
    commands.spawn((Text2d::new(format!("ReRAC {} | native Android runtime",env!("CARGO_PKG_VERSION"))),TextFont{font_size:28.,..default()},Transform::from_xyz(0.,150.,1.)));
    probe.handles=SHADERS.iter().map(|name|assets.load(format!("shaders/{name}.wgsl"))).collect();
}
fn sample(pad:Res<crate::input_map::PadFrame>,mut p:ResMut<Probe>){if !paused(){p.ticks+=1;p.buttons=pad.0.buttons;p.axes=[pad.0.lx,pad.0.ly,pad.0.rx,pad.0.ry];}}
fn report(mut commands:Commands,assets:Res<AssetServer>,adapter:Option<Res<RenderAdapterInfo>>,mut p:ResMut<Probe>){
    p.frames+=1;
    let loaded=p.handles.iter().filter(|h|assets.is_loaded_with_dependencies(h.id())).count();
    if p.frames>=60&&loaded==SHADERS.len()&&!p.capture_requested {
        p.capture_requested=true;
        commands.spawn(Screenshot::primary_window()).observe(|shot:On<ScreenshotCaptured>,mut p:ResMut<Probe>|{
            if let Ok(image)=shot.image.clone().try_into_dynamic(){p.captured=image.to_rgb8().save(p.root.join("native-probe.png")).is_ok();}
        });
    }
    if p.report_at.elapsed().as_millis()<500{return;}p.report_at=Instant::now();
    let core=std::str::from_utf8(&super::CORE_STAMP[b"RERAC_ANDROID_CORE_JSON:".len()..super::CORE_STAMP.len()-1]).unwrap();
    let data=json!({"schema":1,"nonce":p.nonce,"core":serde_json::from_str::<serde_json::Value>(core).unwrap(),"frames":p.frames,"ticks":p.ticks,"paused":paused(),"buttons":p.buttons,"axes":p.axes,"display_mhz":display_mhz(),"shader_files_loaded":loaded,"shader_files_expected":SHADERS.len(),"gpu_capture":p.captured,"adapter":adapter.as_ref().map(|a|a.name.clone()),"backend":adapter.as_ref().map(|a|format!("{:?}",a.backend)),"native_activity_test":true,"gameplay_test":false});
    let temp=p.root.join("native-probe.report.tmp");
    if std::fs::write(&temp,data.to_string()).is_ok(){let _=std::fs::rename(temp,p.root.join("native-probe.report.json"));}
}
#[no_mangle]
pub extern "system" fn Java_org_rerac_android_NativeCore_buildInfo(env:JNIEnv,_class:JClass)->jstring {
    let data=&super::CORE_STAMP[b"RERAC_ANDROID_CORE_JSON:".len()..super::CORE_STAMP.len()-1];
    super::java_string(env,std::str::from_utf8(data).unwrap_or("{}"))
}

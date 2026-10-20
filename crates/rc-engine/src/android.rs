//! Source-built Android NativeActivity adapter. The public Java ABI is v16's ABI.
//! JNI threads only exchange immutable snapshots or bounded commands. All game
//! mutations happen on Bevy's simulation thread; no Rust references cross JNI.
mod companion;
include!(concat!(env!("OUT_DIR"), "/android_core_build.rs"));
mod companion_policy;
mod skip_policy;

/// Android-only accessibility skip for both in-engine scenes and PSS movies.
/// It supplies the standard original shoulder combination on a Start edge,
/// so all existing scene/movie safety gates and fade-out logic still execute.
pub(crate) fn cinematic_skip_pad(pad:&mut rc_game::pad::PadState) {
    use rc_game::pad::button;
    let shoulders=button::L1|button::L2|button::R1|button::R2;
    pad.held=skip_policy::shoulder_chord(pad.held,pad.pressed,button::START,shoulders);
}
include!(concat!(env!("OUT_DIR"), "/android_core_build.rs"));
mod probe;
use crate::touch_input::InputState;
use bevy::prelude::*;
use jni::{JNIEnv, objects::{JClass, JString}, sys::{jboolean, jfloat, jint, jintArray, jlong, jstring, JNI_TRUE, JNI_FALSE}};
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex, OnceLock, RwLock, atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering}};
use std::time::Instant;

static PAUSED: AtomicBool = AtomicBool::new(false);
static DUAL: AtomicBool = AtomicBool::new(false);
static SECONDARY: AtomicBool = AtomicBool::new(false);
static MAP_VISIBLE: AtomicBool = AtomicBool::new(true);
static DISPLAY_MHZ: AtomicU32 = AtomicU32::new(60_000);
static GENERATION: AtomicU64 = AtomicU64::new(1);
static TOUCH_MODE: AtomicU32 = AtomicU32::new(3);
static MENU: AtomicBool = AtomicBool::new(false);
static INPUT: OnceLock<Mutex<InputState>> = OnceLock::new();
static COMMANDS: OnceLock<Mutex<VecDeque<Command>>> = OnceLock::new();
static PUBLISHED: OnceLock<RwLock<Snapshot>> = OnceLock::new();
static VIEW: OnceLock<Mutex<MapView>> = OnceLock::new();

#[derive(Clone)]
pub(super) struct Snapshot {
    wheel: Arc<str>, companion: Arc<str>, icons: HashMap<i32, Arc<[i32]>>,
    selection: Vec<i32>, generation: u64,
}
impl Default for Snapshot {
    fn default() -> Self {
        Self { wheel: Arc::from(r#"{"generation":0,"enabled":false,"box":[0,0,256,256],"glyphs":[],"pages":[{"slots":[],"draws":[]},{"slots":[],"draws":[]}]}"#), companion: Arc::from(r#"{"menu":false,"mode":0,"transition":false,"map":[],"prims":[],"panels":[],"objectives":[],"textures":0}"#), icons: HashMap::new(), selection: Vec::new(), generation: 0 }
    }
}
#[derive(Clone, Copy, Debug)]
pub(super) enum Command {
    Select { slot: usize, item: i32, generation: u64 },
    Touch { page: u32, x: f32, y: f32 },
    Gesture { x: f32, y: f32, dx: f32, dy: f32, scale: f32 },
}
#[derive(Clone, Copy)]
pub(super) struct MapView { zoom: f32, u: f32, v: f32, follow: bool }
impl Default for MapView { fn default() -> Self { Self { zoom: 1.6, u: 0.5, v: 0.5, follow: true } } }
fn input() -> &'static Mutex<InputState> { INPUT.get_or_init(|| Mutex::new(InputState::default())) }
fn published() -> &'static RwLock<Snapshot> { PUBLISHED.get_or_init(|| RwLock::new(Snapshot::default())) }
fn commands() -> &'static Mutex<VecDeque<Command>> { COMMANDS.get_or_init(|| Mutex::new(VecDeque::new())) }
fn view() -> &'static Mutex<MapView> { VIEW.get_or_init(|| Mutex::new(MapView::default())) }
fn queue(command: Command) { let mut q=commands().lock().unwrap_or_else(|e| e.into_inner()); if q.len()<64 && !paused() { q.push_back(command); } }
pub fn paused() -> bool { PAUSED.load(Ordering::Acquire) }
pub fn display_mhz() -> u32 { DISPLAY_MHZ.load(Ordering::Relaxed) }
pub fn secondary() -> bool { DUAL.load(Ordering::Relaxed) && SECONDARY.load(Ordering::Relaxed) }
pub(super) fn pulse(button: u32) { input().lock().unwrap_or_else(|e| e.into_inner()).pulse(button); }
fn reset_level() {
    GENERATION.fetch_add(1, Ordering::AcqRel);
    *published().write().unwrap_or_else(|e| e.into_inner())=Snapshot::default();
    commands().lock().unwrap_or_else(|e| e.into_inner()).clear();
    *view().lock().unwrap_or_else(|e| e.into_inner())=MapView::default();
    TOUCH_MODE.store(3,Ordering::Relaxed);
}

pub struct AndroidPlugin;
impl Plugin for AndroidPlugin {
    fn build(&self, app:&mut App) {
        app.add_systems(crate::level_switch::LevelUnload, reset_level)
           .add_systems(FixedUpdate, (companion::apply_commands, sample_tick).chain().before(crate::gameplay::GameTick).before(crate::menu_render::MenuFrameSet))
           .add_systems(PostUpdate, companion::publish.after(crate::hud_render::HudBuild));
    }
}
fn sample_tick(mut pad: Option<ResMut<crate::input_map::PadFrame>>) {
    let Some(pad)=pad.as_deref_mut() else { return };
    let sample=input().lock().unwrap_or_else(|e| e.into_inner()).tick(Instant::now());
    let mut p=rc_game::pad::PadInput::neutral();
    let raw=|v:f32| (127.5+127.5*(v*crate::input_map::STICK_SCALE).clamp(-1.,1.)).round() as u8;
    let axis=|i:usize| if sample.touch_axes & (if i<2 {1} else {2}) !=0 {rc_game::pad::PadInput::axis_byte(sample.axes[i])} else {raw(sample.axes[i])};
    p.lx=axis(0);p.ly=axis(1);p.rx=axis(2);p.ry=axis(3);
    pad.0=p.press(sample.buttons);
}

/// The same ABI generated by Bevy's `bevy_main`, with v16's private data paths.
#[no_mangle]
fn android_main(app: bevy::android::android_activity::AndroidApp) {
    let Some(files)=app.internal_data_path() else { eprintln!("ReRAC Android: files directory unavailable"); return };
    std::env::set_var("RC_DATA_DIR",files.join("gamedata"));
    std::env::set_var("RC_SAVE_DIR",files.join("memcard"));
    std::env::set_var("RC_SETTINGS_FILE",files.join("settings.toml"));
    std::env::set_var("RC_ANDROID_FILES",&files);
    let diagnostic=probe::requested(&files);
    let _=bevy::android::ANDROID_APP.set(app);
    if let Some(nonce)=diagnostic { probe::run(files,nonce); return; }
    if let Err(error)=crate::main() { eprintln!("ReRAC Android engine startup failed: {error:#}"); }
}

fn java_string(mut env: JNIEnv, value: &str) -> jstring { env.new_string(value).map(|s|s.into_raw()).unwrap_or(std::ptr::null_mut()) }
fn java_pixels(env: JNIEnv, data: Option<Arc<[i32]>>) -> jintArray {
    let Some(data)=data else {return std::ptr::null_mut()};
    let Ok(array)=env.new_int_array(data.len() as jint) else {return std::ptr::null_mut()};
    if env.set_int_array_region(&array,0,&data).is_err(){return std::ptr::null_mut()}
    array.into_raw()
}
#[no_mangle] pub extern "system" fn Java_org_rerac_android_GameActivity_nativeInput(_env:JNIEnv,_class:JClass,b:jint,lx:jfloat,ly:jfloat,rx:jfloat,ry:jfloat) { input().lock().unwrap_or_else(|e| e.into_inner()).physical(b as u32,[lx,ly,rx,ry]); }
#[no_mangle] pub extern "system" fn Java_org_rerac_android_GameActivity_nativeTouchInput(_env:JNIEnv,_class:JClass,b:jint,lx:jfloat,ly:jfloat,rx:jfloat,ry:jfloat,active:jint) { input().lock().unwrap_or_else(|e| e.into_inner()).touch(b as u32,[lx,ly,rx,ry],active as u8,Instant::now()); }
#[no_mangle] pub extern "system" fn Java_org_rerac_android_GameActivity_nativePause(_env:JNIEnv,_class:JClass,v:jboolean) {
    let pause=v!=JNI_FALSE; PAUSED.store(pause,Ordering::Release);
    input().lock().unwrap_or_else(|e| e.into_inner()).pause(pause);
    if pause {commands().lock().unwrap_or_else(|e| e.into_inner()).clear();}
}
#[no_mangle] pub extern "system" fn Java_org_rerac_android_GameActivity_nativeDisplayRate(_env:JNIEnv,_class:JClass,hz:jfloat) { if hz.is_finite() && (20.0..=500.0).contains(&hz) { DISPLAY_MHZ.store((hz*1000.).round() as u32,Ordering::Relaxed); } }
#[no_mangle] pub extern "system" fn Java_org_rerac_android_GameActivity_nativeSetDual(_env:JNIEnv,_class:JClass,v:jboolean) { DUAL.store(v!=JNI_FALSE,Ordering::Release); }
#[no_mangle] pub extern "system" fn Java_org_rerac_android_GameActivity_nativeDualEnabled(_env:JNIEnv,_class:JClass)->jboolean { if DUAL.load(Ordering::Acquire) {JNI_TRUE}else{JNI_FALSE} }
#[no_mangle] pub extern "system" fn Java_org_rerac_android_GameActivity_nativeSecondary(_env:JNIEnv,_class:JClass,v:jboolean) { SECONDARY.store(v!=JNI_FALSE,Ordering::Release); }
#[no_mangle] pub extern "system" fn Java_org_rerac_android_GameActivity_nativeTouchMode(_env:JNIEnv,_class:JClass)->jint { TOUCH_MODE.load(Ordering::Acquire) as jint }
#[no_mangle] pub extern "system" fn Java_org_rerac_android_GameActivity_nativeMenuMode(_env:JNIEnv,_class:JClass)->jboolean { if MENU.load(Ordering::Acquire){JNI_TRUE}else{JNI_FALSE} }
#[no_mangle] pub extern "system" fn Java_org_rerac_android_GameActivity_nativeWheel(env:JNIEnv,_class:JClass)->jstring { let s=published().read().unwrap_or_else(|e|e.into_inner()).wheel.clone();java_string(env,&s) }
#[no_mangle] pub extern "system" fn Java_org_rerac_android_GameActivity_nativeCompanion(env:JNIEnv,_class:JClass)->jstring { let s=published().read().unwrap_or_else(|e|e.into_inner()).companion.clone();java_string(env,&s) }
#[no_mangle] pub extern "system" fn Java_org_rerac_android_GameActivity_nativeIcon(env:JNIEnv,_class:JClass,id:jint)->jintArray { let p=published().read().unwrap_or_else(|e|e.into_inner()).icons.get(&id).cloned();java_pixels(env,p) }
#[no_mangle] pub extern "system" fn Java_org_rerac_android_GameActivity_nativeSelect(_env:JNIEnv,_class:JClass,slot:jint,item:jint,generation:jlong) {
    if slot<0 || item<0 || generation<0 {return;}
    let snap=published().read().unwrap_or_else(|e|e.into_inner());
    if snap.generation!=generation as u64 {return;}
    let Some(index)=companion_policy::selected_slot(&snap.selection,slot as usize,item) else{return};
    drop(snap);
    queue(Command::Select{slot:index,item,generation:generation as u64});
}
#[no_mangle] pub extern "system" fn Java_org_rerac_android_GameActivity_nativeMenuButton(_env:JNIEnv,_class:JClass,b:jint) { pulse(b as u32); }
#[no_mangle] pub extern "system" fn Java_org_rerac_android_GameActivity_nativeMenuTouch(_env:JNIEnv,_class:JClass,page:jlong,x:jfloat,y:jfloat) { if (0..=u32::MAX as i64).contains(&page)&&x.is_finite()&&y.is_finite() {queue(Command::Touch{page:page as u32,x,y});} }
#[no_mangle] pub extern "system" fn Java_org_rerac_android_GameActivity_nativeMenuGesture(_env:JNIEnv,_class:JClass,x:jfloat,y:jfloat,dx:jfloat,dy:jfloat,scale:jfloat) { if [x,y,dx,dy,scale].iter().all(|v|v.is_finite()) {queue(Command::Gesture{x,y,dx:dx.clamp(-512.,512.),dy:dy.clamp(-416.,416.),scale:scale.clamp(0.25,4.)});} }
#[no_mangle] pub extern "system" fn Java_org_rerac_android_GameActivity_nativeMapVisible(_env:JNIEnv,_class:JClass,v:jboolean) {MAP_VISIBLE.store(v!=JNI_FALSE,Ordering::Release);}
#[no_mangle] pub extern "system" fn Java_org_rerac_android_GameActivity_nativeMapView(_env:JNIEnv,_class:JClass,z:jfloat,u:jfloat,v:jfloat,follow:jboolean) {
    if [z,u,v].iter().all(|f|f.is_finite()) { *view().lock().unwrap_or_else(|e|e.into_inner())=MapView {zoom:z.clamp(0.65,4.),u:u.clamp(0.,1.),v:v.clamp(0.,1.),follow:follow!=JNI_FALSE}; }
}
#[no_mangle] pub extern "system" fn Java_org_rerac_android_GameActivity_nativePlanetIcon(mut env:JNIEnv,_class:JClass,path:JString)->jintArray {
    // The Java caller gives the already-imported data path. Do not load arbitrary
    // content outside that directory, and never retain JNIEnv on a worker.
    let Ok(path)=env.get_string(&path).map(|s|s.to_string_lossy().into_owned()) else{return std::ptr::null_mut()};
    let expected=std::env::var("RC_DATA_DIR").unwrap_or_default();
    if !expected.is_empty() && path!=expected {return std::ptr::null_mut()}
    java_pixels(env,companion::planet_icon(std::path::Path::new(&path)))
}

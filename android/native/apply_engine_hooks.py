#!/usr/bin/env python3
"""Idempotent, reviewed hooks into upstream; fail closed on changed anchors.
This initial bootstrap commits the resulting Rust source on the integration branch.
Subsequent versions retain normal tracked source and use Git merges, not byte offsets.
"""
from pathlib import Path
ROOT = Path(__file__).resolve().parents[2]
def replace(path, old, new):
    p = ROOT / path
    text = p.read_text()
    if new in text: return
    if text.count(old) != 1:
        raise RuntimeError(f"Expected one integration anchor in {path}: {old[:70]!r}")
    p.write_text(text.replace(old, new, 1))
replace('crates/rc-engine/Cargo.toml', '[features]', '[lib]\nname = "rc_engine"\npath = "src/main.rs"\ncrate-type = ["cdylib"]\n\n[features]')
p = ROOT / 'crates/rc-engine/Cargo.toml'
s = p.read_text()
android_deps = '\n[target.\'cfg(target_os = "android")\'.dependencies]\nbevy = { version = "0.19", features = ["android-native-activity"] }\njni = "0.21.1"\nserde_json = "1"\n'
if 'android-native-activity' not in s: p.write_text(s + android_deps)
replace('crates/rc-engine/src/main.rs', 'mod crash_log;', '#[cfg(target_os = "android")]\nmod android;\n#[cfg(any(target_os = "android", test))]\nmod touch_input;\n\nmod crash_log;')
replace('crates/rc-engine/src/main.rs', '    let mut app = App::new();', '    let mut app = App::new();\n    #[cfg(target_os = "android")]\n    app.add_plugins(android::AndroidPlugin);')
replace('crates/rc-engine/src/main.rs', 'fn asset_dir() -> String {', 'fn asset_dir() -> String {\n    #[cfg(target_os = "android")]\n    { return String::new(); }')
replace('crates/rc-engine/src/input_map.rs', '    frame.0 = sample(&keys, &mouse, &motion, gamepads.iter().next());', '    #[cfg(not(target_os = "android"))]\n    { frame.0 = sample(&keys, &mouse, &motion, gamepads.iter().next()); }')
replace('crates/rc-engine/src/frame_pace.rs', '    let now = Instant::now();\n    let Some(last)', '    let now = Instant::now();\n    #[cfg(target_os = "android")]\n    if crate::android::paused() {\n        pace.last = Some(now); pace.acc = 0.0; pace.next = None;\n        *strategy = TimeUpdateStrategy::FixedTimesteps(0);\n        return;\n    }\n    let Some(last)')
# Upstream v0.3.6 adds an adaptive 60-Hz fallback and the Android port
# must use a mutable period, so the original bootstrap replacement is already
# superseded by the tracked, reviewed native source in this branch.
pace = (ROOT / 'crates/rc-engine/src/frame_pace.rs').read_text()
if 'let mut period = period_for(Some(crate::android::display_mhz()));' not in pace:
    replace('crates/rc-engine/src/frame_pace.rs', '    let period = period_for(monitors.iter().next().and_then(|m| m.refresh_rate_millihertz));', '    #[cfg(not(target_os = "android"))]\n    let period = period_for(monitors.iter().next().and_then(|m| m.refresh_rate_millihertz));\n    #[cfg(target_os = "android")]\n    let period = period_for(Some(crate::android::display_mhz()));')
replace('crates/rc-engine/src/menu_render.rs', 'struct MenuRt {', 'pub(crate) struct MenuRt {')
for field in ('assets', 'qs', 'menu', 'draws', 'render_mode', 'last_counter'):
    p = ROOT / 'crates/rc-engine/src/menu_render.rs'
    s = p.read_text()
    new = '\n    pub(crate) ' + field + ':'
    if new not in s:
        old = '\n    ' + field + ':'
        if old not in s: raise RuntimeError('Missing MenuRt field: ' + field)
        p.write_text(s.replace(old, new, 1))
replace('crates/rc-engine/src/menu_render.rs', '\nfn convert(', '\npub(crate) fn convert(')
# Embed the actual compiled version and source provenance, not a launcher label.
# The editable adapter now also declares companion_policy/skip_policy; the
# build stamp may no longer directly follow mod companion. Check globally.
if 'include!(concat!(env!("OUT_DIR"), "/android_core_build.rs"));' not in (ROOT/'crates/rc-engine/src/android.rs').read_text():
    replace('crates/rc-engine/src/android.rs', 'mod companion;', 'mod companion;\ninclude!(concat!(env!("OUT_DIR"), "/android_core_build.rs"));')
# Preserve the v16 Adreno skinning workaround while retaining every other .3.5
# shader change (notably tie wind animation and the additive HUD blend).
for name in ('moby', 'moby_metal'):
    path = f'crates/rc-engine/assets/shaders/{name}.wgsl'
    replace(path, 'palette: array<mat4x4<f32>>;', 'palette: array<vec4<f32>>;')
    replace(path, 'fn s16(x: i32) -> i32', '''// Explicit 16-byte column loads preserve v16's Adreno storage-matrix workaround.
// The CPU still uploads the same 64-byte matrices; this changes no asset layout.
fn joint(index: u32) -> mat4x4<f32> {
    let p = index * 4u;
    return mat4x4<f32>(palette[p], palette[p + 1u], palette[p + 2u], palette[p + 3u]);
}

fn s16(x: i32) -> i32''')
    p=ROOT/path; s=p.read_text()
    for axis,count in [('x',2),('y',1),('z',1)]:
        old=f'palette[base + j.{axis}]'; new=f'joint(base + j.{axis})'
        if old not in s and s.count(new)==count: continue
        if s.count(old)!=count: raise RuntimeError(f'Changed skinning anchor in {path}: {axis}')
        s=s.replace(old,new)
    p.write_text(s)

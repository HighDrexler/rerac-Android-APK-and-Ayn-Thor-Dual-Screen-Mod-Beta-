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

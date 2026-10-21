#!/usr/bin/env python3
"""Wire reviewed diagnostic entry points and retain the resolved Android lock."""
from pathlib import Path
import re
ROOT=Path(__file__).resolve().parents[2]
def replace(name,old,new):
    p=ROOT/name;s=p.read_text()
    if new in s:return
    if s.count(old)!=1:raise RuntimeError('Changed source integration anchor: '+name)
    p.write_text(s.replace(old,new,1))
replace('crates/rc-engine/src/android.rs','include!(concat!(env!("OUT_DIR"), "/android_core_build.rs"));','include!(concat!(env!("OUT_DIR"), "/android_core_build.rs"));\nmod probe;')
replace('crates/rc-engine/src/android.rs','    let _=bevy::android::ANDROID_APP.set(app);','    let diagnostic=probe::requested(&files);\n    let _=bevy::android::ANDROID_APP.set(app);\n    if let Some(nonce)=diagnostic { probe::run(files,nonce); return; }')
replace('crates/rc-engine/src/android/companion.rs','if play.game.hero.mode==1 {ids=q.commands.as_ref().map(|c|c.entries.iter().map(|e|e.item).collect()).unwrap_or_default();ids.resize(16,0);}','if play.game.hero.mode==1 {ids=q.commands.as_ref().map(|c|c.entries.iter().map(|e|e.item).collect()).unwrap_or_default();ids.resize(8,0);}')
# The v1.0 parity adapter retains Clank layout and adds real item availability.
# Accept that exact tracked successor; unknown changes still fail closed.
if 'let (x,y)=ring.slot_pos(if play.game.hero.mode==1 {i*2}else{i});slots.push(json!({"id":item,"x":x,"y":y,"enabled":allow&&item!=0&&if play.game.hero.mode==1 {play.svc.units.bot_listeners>0}else{companion_select::available(&q.items,&gs.0.global,item)}}));selection.push(item);' not in (ROOT/'crates/rc-engine/src/android/companion.rs').read_text():
    replace('crates/rc-engine/src/android/companion.rs','let (x,y)=ring.slot_pos(i);slots.push(json!({"id":item,"x":x,"y":y,"enabled":allow&&item!=0}));selection.push(item);','let (x,y)=ring.slot_pos(if play.game.hero.mode==1 {i*2}else{i});slots.push(json!({"id":item,"x":x,"y":y,"enabled":allow&&item!=0&&(play.game.hero.mode!=1||play.svc.units.bot_listeners>0)}));selection.push(item);')
p=ROOT/'Cargo.lock';s=p.read_text()
m=re.search(r'\[\[package\]\]\nname = "rc-engine"\n.*?(?=\n\[\[package\]\])',s,re.S)
if not m:raise RuntimeError('Native engine lock entry missing')
entry=m.group()
for after,dependency in [('bevy','jni 0.21.1'),('rc-video','serde_json')]:
    line=' "'+dependency+'",\n'
    if line not in entry:
        anchor=' "'+after+'",\n'
        if entry.count(anchor)!=1:raise RuntimeError('Changed native dependency lock')
        entry=entry.replace(anchor,anchor+line,1)
p.write_text(s[:m.start()]+entry+s[m.end():])

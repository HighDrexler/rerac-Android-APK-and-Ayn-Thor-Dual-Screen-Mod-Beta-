#!/usr/bin/env python3
"""Apply reviewed source-level audio/movie hooks, not binary-offset patches."""
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]
def change(name,old,new):
    p=ROOT/name
    s=p.read_text()
    if new in s: return
    if s.count(old)!=1: raise RuntimeError(f"Changed Android media integration anchor: {name}: {old[:60]!r}")
    p.write_text(s.replace(old,new,1))
a='crates/rc-engine/src/audio_out.rs'
m='crates/rc-engine/src/movie_render.rs'
change('crates/rc-engine/src/main.rs','mod audio_out;','mod audio_out;\nmod audio_queue;\nmod movie_pipe;')
change(a,'use std::collections::VecDeque;','use crate::audio_queue::{Ring, Cursor, MovieOwner};')
change(a,'/// Frames the reader waits for before (re)starting: 50 ms.\nconst PREFILL: usize = 2400;\n/// Ring cap: 0.25 s (older frames are dropped when the game runs ahead of the device).\nconst RING_CAP: usize = 12_000;\n','// Ring sizing and movie ownership live in the independently tested audio_queue.\n')
change(a,'''#[derive(Default)]
struct RingState {
    frames: VecDeque<[i16; 2]>,
    running: bool,
    underruns: u64,
}

/// Rendered frames waiting for the audio device.
#[derive(Default)]
pub struct Ring(Mutex<RingState>);

impl Ring {
    fn push(&self, frames: &[[i16; 2]]) {
        let mut r = self.0.lock().unwrap_or_else(|e| e.into_inner());
        r.frames.extend(frames.iter().copied());
        let over = r.frames.len().saturating_sub(RING_CAP);
        r.frames.drain(..over);
    }

    fn pop(&self) -> [i16; 2] {
        let mut r = self.0.lock().unwrap_or_else(|e| e.into_inner());
        if !r.running {
            if r.frames.len() < PREFILL { return [0, 0]; }
            r.running = true;
        }
        match r.frames.pop_front() {
            Some(f) => f,
            None => {
                r.running = false;
                r.underruns += 1;
                [0, 0]
            }
        }
    }
}

''','// Audio device buffering is owned by audio_queue::Ring.\n')
change(a,'    right: Option<f32>,','    cursor: Cursor,')
change(a,'''        if let Some(r) = self.right.take() { return Some(r); }
        let [l, r] = self.ring.pop();
        self.right = Some(r as f32 / 32768.0);
        Some(l as f32 / 32768.0)''','''        #[cfg(target_os = "android")]
        let paused = crate::android::paused();
        #[cfg(not(target_os = "android"))]
        let paused = false;
        Some(self.cursor.next(&self.ring, paused))''')
change(a,'MixerDecoder { ring: Arc::clone(&self.ring), right: None }','MixerDecoder { ring: Arc::clone(&self.ring), cursor: Cursor::default() }')
change(a,'pub fn queued(&self) -> usize { self.ring.0.lock().map(|r| r.frames.len()).unwrap_or(0) }','pub fn queued(&self) -> usize { self.ring.queued() }\n\n    pub fn begin_movie(&self) -> MovieOwner { MovieOwner::new(&self.ring) }')
change(a,'''    pub fn push_frame(&mut self) {
        self.done += 1;
        self.ring.push(&self.buf);''','''    pub fn push_frame(&mut self) { self.push_owned_frame(0); }

    pub fn push_movie_frame(&mut self, owner: u64) { self.push_owned_frame(owner); }

    fn push_owned_frame(&mut self, owner: u64) {
        self.done += 1;
        self.ring.push(&self.buf, owner);''')
change(a,'let underruns = out.ring.0.lock().map(|r| r.underruns).unwrap_or(0);','let underruns = out.ring.underruns();')
change(m,'use std::sync::mpsc::{sync_channel, Receiver, SyncSender};','use crate::movie_pipe::{channel as sync_channel, Receiver, Sender as SyncSender, Empty};')
change(m,'    last_phase: Option<Phase>,','    last_phase: Option<Phase>,\n    owner: Option<crate::audio_queue::MovieOwner>,\n    media_cancelled: bool,')
change(m,'    loop {\n        match m.next_frame()','    loop {\n        if tx.closed() { return; }\n        match m.next_frame()')
change(m,'            Ok(Some(f)) => {\n                let mut rgba','            Ok(Some(f)) => {\n                if !tx.should_convert(n) { n += 1; continue; }\n                let mut rgba')
change(m,'    fn receive(&mut self) -> bool {','    fn receive(&mut self, blocking: bool) -> bool {')
change(m,'        match rx.recv() {','        match rx.receive(blocking) {')
change(m,'            Err(_) => {\n                self.opened = true;','            Err(Empty::Pending) => return false,\n            Err(Empty::Closed) => {\n                self.opened = true;')
change(m,'''    fn fetch(&mut self, p: u32) {
        while !self.opened { if !self.receive() { break; } }
        while self.received <= p && self.total.is_none() { if !self.receive() { break; } }
    }''','''    fn fetch(&mut self, p: u32, blocking: bool) {
        self.rx.get_mut().unwrap_or_else(|e| e.into_inner()).wanted(p, !blocking);
        while !self.opened { if !self.receive(blocking) { break; } }
        while self.received <= p && self.total.is_none() { if !self.receive(blocking) { break; } }
    }''')
change(m,'            last_phase: None,','            last_phase: None,\n            owner: audio.as_ref().map(|a| a.begin_movie()),\n            media_cancelled: false,')
change(m,'''    if let Some(p) = rt.player.wanted_picture(latency) { rt.fetch(p); }
    let out: MovieFrame = rt.player.tick(&rt.pad, rt.total, latency);''','''    // Keep the native event loop responsive while file/audio decoding opens.
    // Frame-exact tooling still waits for every original picture.
    if !rt.opened {
        rt.fetch(0, det.0);
        if !rt.opened { set_visible(&mut vis, st.root, true); return; }
    }
    if let Some(p) = rt.player.wanted_picture(latency) { rt.fetch(p, det.0); }
    let out: MovieFrame = rt.player.tick(&rt.pad, rt.total, latency);
    if rt.player.skipped && !rt.media_cancelled {
        rt.rx.get_mut().unwrap_or_else(|e|e.into_inner()).close();
        if let Some(owner) = &rt.owner { owner.cancel(); }
        rt.media_cancelled = true;
    }''')
change(m,'    if let Some(p) = out.picture {\n        if rt.shown != Some(p) && rt.received > 0 {','    if out.picture.is_some() {\n        let p = rt.received.saturating_sub(1);\n        if rt.shown != Some(p) && rt.received > 0 {')
change(m,'        a.push_frame();','        a.push_movie_frame(rt.owner.as_ref().map_or(0, |o| o.id));')

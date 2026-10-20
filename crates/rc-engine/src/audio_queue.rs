//! Bounded stereo queue with movie-specific cancellation. Pure Rust and tested
//! independently of the Android device. Owner zero is game audio; a movie owns a
//! monotonically increasing epoch. Cancelling an old movie never removes new
//! movie/game audio, including the right half held by the device's interleaver.
use std::collections::VecDeque;
use std::sync::{Arc, Mutex, atomic::{AtomicU64, Ordering}};

#[derive(Clone, Copy)]
struct Frame { pcm: [i16; 2], owner: u64 }
struct State { frames: VecDeque<Frame>, running: bool, underruns: u64, dropped: u64 }
pub struct Ring { state: Mutex<State>, active: AtomicU64, next: AtomicU64, prefill: usize, capacity: usize }
impl Default for Ring { fn default() -> Self { Self::new(2400, 12_000) } }
impl Ring {
    fn new(prefill: usize, capacity: usize) -> Self {
        assert!(prefill > 0 && capacity >= prefill);
        Self { state: Mutex::new(State { frames: VecDeque::with_capacity(capacity), running: false, underruns: 0, dropped: 0 }), active: AtomicU64::new(0), next: AtomicU64::new(1), prefill, capacity }
    }
    fn valid(&self, owner: u64) -> bool { owner == 0 || self.active.load(Ordering::Acquire) == owner }
    pub fn begin_movie(&self) -> u64 {
        let id = self.next.fetch_add(1, Ordering::Relaxed);
        assert_ne!(id, 0, "movie epoch exhausted");
        let mut s = self.state.lock().unwrap_or_else(|e|e.into_inner());
        self.active.store(id, Ordering::Release);
        s.frames.retain(|f| f.owner == 0);
        id
    }
    pub fn end_movie(&self, id: u64) {
        if id == 0 { return; }
        let mut s = self.state.lock().unwrap_or_else(|e|e.into_inner());
        let _ = self.active.compare_exchange(id, 0, Ordering::AcqRel, Ordering::Acquire);
        s.frames.retain(|f| f.owner != id);
    }
    pub fn push(&self, frames: &[[i16; 2]], owner: u64) {
        let mut s = self.state.lock().unwrap_or_else(|e|e.into_inner());
        if !self.valid(owner) { return; }
        let frames = &frames[frames.len().saturating_sub(self.capacity)..];
        let over = (s.frames.len() + frames.len()).saturating_sub(self.capacity);
        s.frames.drain(..over);
        // Upstream v0.3.6 accounts for samples discarded when the producer
        // outpaces the device. Preserve that metric in Android's tagged ring.
        s.dropped = s.dropped.saturating_add(over as u64);
        s.frames.extend(frames.iter().map(|&pcm| Frame { pcm, owner }));
    }
    pub fn queued(&self) -> usize { self.state.lock().unwrap_or_else(|e|e.into_inner()).frames.len() }
    pub fn underruns(&self) -> u64 { self.state.lock().unwrap_or_else(|e|e.into_inner()).underruns }
    pub fn dropped(&self) -> u64 { self.state.lock().unwrap_or_else(|e|e.into_inner()).dropped }
    fn pop(&self) -> Frame {
        let mut s = self.state.lock().unwrap_or_else(|e|e.into_inner());
        let silence = Frame { pcm: [0, 0], owner: 0 };
        if !s.running {
            if s.frames.len() < self.prefill { return silence; }
            s.running = true;
        }
        while let Some(frame) = s.frames.pop_front() { if self.valid(frame.owner) { return frame; } }
        s.running = false; s.underruns += 1; silence
    }
}
/// RAII ownership: dropping a movie invalidates its queued and cached samples.
pub struct MovieOwner { ring: Arc<Ring>, pub id: u64 }
impl MovieOwner {
    pub fn new(ring: &Arc<Ring>) -> Self { Self { ring: ring.clone(), id: ring.begin_movie() } }
    pub fn cancel(&self) { self.ring.end_movie(self.id); }
}
impl Drop for MovieOwner { fn drop(&mut self) { self.cancel(); } }
#[derive(Default)]
pub struct Cursor { right: Option<(i16, u64)> }
impl Cursor {
    pub fn next(&mut self, ring: &Ring, paused: bool) -> f32 {
        if let Some((right, owner)) = self.right.take() {
            return if paused || !ring.valid(owner) {0.0} else {right as f32 / 32768.0};
        }
        let frame = if paused {Frame { pcm: [0,0], owner:0 }} else {ring.pop()};
        self.right = Some((frame.pcm[1], frame.owner));
        if !ring.valid(frame.owner) {0.0} else {frame.pcm[0] as f32 / 32768.0}
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn take(c:&mut Cursor,r:&Ring)->[i16;2] {[(c.next(r,false)*32768.) as i16,(c.next(r,false)*32768.) as i16]}
    #[test] fn game_audio_survives_movie_cancel() {let r=Ring::new(1,8);let id=r.begin_movie();r.push(&[[10,11]],0);r.push(&[[20,21]],id);r.push(&[[30,31]],0);r.end_movie(id);let mut c=Cursor::default();assert_eq!(take(&mut c,&r),[10,11]);assert_eq!(take(&mut c,&r),[30,31]);}
    #[test] fn newer_movie_is_not_cancelled_by_old_owner() {let r=Ring::new(1,8);let old=r.begin_movie();r.push(&[[1,2]],old);let new=r.begin_movie();r.push(&[[3,4]],new);r.end_movie(old);assert_eq!(take(&mut Cursor::default(),&r),[3,4]);}
    #[test] fn cancelled_cached_right_is_silent() {let r=Ring::new(1,8);let id=r.begin_movie();r.push(&[[100,200]],id);let mut c=Cursor::default();assert_eq!(c.next(&r,false),100./32768.);r.end_movie(id);assert_eq!(c.next(&r,false),0.);}
    #[test] fn cancelling_movie_keeps_cached_game_right() {let r=Ring::new(1,8);let id=r.begin_movie();r.push(&[[100,200]],0);let mut c=Cursor::default();c.next(&r,false);r.end_movie(id);assert_eq!(c.next(&r,false),200./32768.);}
    #[test] fn pause_does_not_consume_queued_frames() {let r=Ring::new(1,8);r.push(&[[3,4]],0);let mut c=Cursor::default();for _ in 0..100 {assert_eq!(c.next(&r,true),0.);}assert_eq!(r.queued(),1);assert_eq!(take(&mut c,&r),[3,4]);}
    #[test] fn stale_producer_is_ignored() {let r=Ring::new(1,8);let id=r.begin_movie();r.end_movie(id);r.push(&[[3,4]],id);assert_eq!(r.queued(),0);}
    #[test] fn refill_and_capacity_are_bounded() {let r=Ring::new(2,3);let mut c=Cursor::default();r.push(&[[1,2]],0);assert_eq!(take(&mut c,&r),[0,0]);r.push(&[[3,4],[5,6],[7,8]],0);assert_eq!(r.queued(),3);assert_eq!(take(&mut c,&r),[3,4]);assert_eq!(take(&mut c,&r),[5,6]);assert_eq!(take(&mut c,&r),[7,8]);assert_eq!(take(&mut c,&r),[0,0]);assert_eq!(r.underruns(),1);}
    #[test] fn oversized_push_keeps_only_latest() {let r=Ring::new(1,2);r.push(&[[1,2],[3,4],[5,6]],0);let mut c=Cursor::default();assert_eq!(take(&mut c,&r),[3,4]);assert_eq!(take(&mut c,&r),[5,6]);}
}

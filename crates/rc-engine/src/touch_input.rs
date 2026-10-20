//! Android input state, independent of JNI, Bevy and render frequency.
//! Java sends normalized touch axes (already dead-zoned) and raw physical axes.
//! A down edge is retained until an actual emulation/game tick acknowledges it.
use std::collections::VecDeque;
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Sample { pub buttons: u32, pub axes: [f32; 4], pub touch_axes: u8 }
#[derive(Debug)]
pub struct InputState {
    physical: Sample,
    touch: Sample,
    pending: u32,
    touch_at: Option<Instant>,
    pulses: VecDeque<u32>,
    pulse_release: bool,
    paused: bool,
}
impl Default for InputState {
    fn default() -> Self { Self { physical: Sample::default(), touch: Sample::default(), pending: 0, touch_at: None, pulses: VecDeque::new(), pulse_release: false, paused: false } }
}
fn clean(x: f32) -> f32 { if x.is_finite() { x.clamp(-1.0, 1.0) } else { 0.0 } }
impl InputState {
    pub fn physical(&mut self, buttons: u32, axes: [f32; 4]) {
        if self.paused { return; }
        self.pending |= (buttons & 0xffff) & !self.physical.buttons;
        self.physical = Sample { buttons: buttons & 0xffff, axes: axes.map(clean), touch_axes: 0 };
    }
    pub fn touch(&mut self, buttons: u32, axes: [f32; 4], active: u8, now: Instant) {
        if self.paused { return; }
        self.pending |= (buttons & 0xffff) & !self.touch.buttons;
        self.touch = Sample { buttons: buttons & 0xffff, axes: axes.map(clean), touch_axes: active & 3 };
        self.touch_at = Some(now);
    }
    pub fn pulse(&mut self, buttons: u32) {
        if !self.paused && self.pulses.len() < 64 && buttons & 0xffff != 0 { self.pulses.push_back(buttons & 0xffff); }
    }
    pub fn pause(&mut self, value: bool) {
        if value != self.paused {
            *self = Self { paused: value, ..Self::default() };
        }
    }
    pub fn paused(&self) -> bool { self.paused }
    /// Called on a simulation tick, never a faster UI/display refresh.
    pub fn tick(&mut self, now: Instant) -> Sample {
        if self.paused { return Sample::default(); }
        let mut out = self.physical;
        out.buttons |= self.touch.buttons | std::mem::take(&mut self.pending);
        let fresh = self.touch_at.is_some_and(|t| now.saturating_duration_since(t) < Duration::from_millis(250));
        for (bit, range) in [(1, 0..2), (2, 2..4)] {
            if self.touch.touch_axes & bit != 0 && (bit == 1 || fresh) {
                for i in range { out.axes[i] = self.touch.axes[i]; }
                out.touch_axes |= bit;
            }
        }
        if self.pulse_release { self.pulse_release = false; }
        else if let Some(b) = self.pulses.pop_front() { out.buttons |= b; self.pulse_release = true; }
        out
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn tap_survives_between_renders() { let mut s=InputState::default(); s.physical(64,[0.;4]); s.physical(0,[0.;4]); assert_eq!(s.tick(Instant::now()).buttons,64); assert_eq!(s.tick(Instant::now()).buttons,0); }
    #[test] fn touch_does_not_erase_physical_buttons() { let mut s=InputState::default(); let n=Instant::now(); s.physical(8,[0.7,0.3,0.2,0.4]); s.touch(64,[-0.5,0.0,0.8,0.0],1,n); let p=s.tick(n); assert_eq!(p.buttons,72); assert_eq!(p.axes,[-0.5,0.,0.2,0.4]); assert_eq!(p.touch_axes,1); }
    #[test] fn stale_camera_does_not_spin_forever() { let mut s=InputState::default(); let n=Instant::now(); s.touch(0,[0.,0.,1.,1.],2,n); assert_eq!(s.tick(n).axes[2],1.); assert_eq!(s.tick(n+Duration::from_secs(1)).axes[2],0.); }
    #[test] fn pause_clears_and_rejects_input() { let mut s=InputState::default(); s.physical(64,[1.;4]); s.pause(true); s.physical(8,[1.;4]); s.pause(false); assert_eq!(s.tick(Instant::now()),Sample::default()); }
    #[test] fn repeated_menu_taps_have_release_ticks() { let mut s=InputState::default(); s.pulse(64);s.pulse(64); let n=Instant::now(); assert_eq!([s.tick(n).buttons,s.tick(n).buttons,s.tick(n).buttons,s.tick(n).buttons],[64,0,64,0]); }
    #[test] fn invalid_axes_are_sanitized() { let mut s=InputState::default(); s.physical(0,[f32::NAN,f32::INFINITY,-2.,2.]); assert_eq!(s.tick(Instant::now()).axes,[0.,0.,-1.,1.]); }
}

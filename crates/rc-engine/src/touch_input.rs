//! Android input state, independent of JNI, Bevy and render frequency.
//! Java sends normalized touch axes and raw physical axes. Each down edge is
//! retained until a real game tick acknowledges it. The original v16 overlay
//! also emits two *non-PS2* action bits: PACK and THROW. They must not be
//! accidentally discarded by the 16-bit DualShock button mask.
use std::collections::VecDeque;
use std::time::{Duration, Instant};

// Exact PS2 pad bit encodings; kept local so standalone rustc tests work.
pub const TRIANGLE: u32 = 0x10;
pub const CROSS: u32 = 0x40;
pub const SQUARE: u32 = 0x80;
pub const L1: u32 = 0x04;
// From the original v16 TouchLayout: PACK = 0x10000, THROW = 0x20000.
pub const PACK_REQUEST: u32 = 0x1_0000;
pub const THROW_REQUEST: u32 = 0x2_0000;
const THROW_AIM_TICKS: u8 = 34;
const THROW_RELEASE_TICKS: u8 = THROW_AIM_TICKS + 2;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Sample { pub buttons: u32, pub axes: [f32; 4], pub touch_axes: u8 }

#[derive(Debug)]
pub struct InputState {
    physical: Sample,
    touch: Sample,
    pending_physical: u32,
    pending_touch: u32,
    touch_special: u32,
    // An existing weapon-wheel hold must not turn into contextual Triangle
    // when Ratchet walks into a prompt. Only a NEW down edge at the prompt arms it.
    triangle_armed: bool,
    // A short tap starts one auto-chord: L1 enters the original aiming
    // stance; after its 20-tick minimum, Square throws the wrench.
    throw_ticks: Option<u8>,
    touch_at: Option<Instant>,
    pulses: VecDeque<u32>,
    pulse_release: bool,
    paused: bool,
}
impl Default for InputState {
    fn default() -> Self {
        Self { physical: Sample::default(), touch: Sample::default(),
            pending_physical: 0, pending_touch: 0, touch_special: 0,
            triangle_armed: false,
            throw_ticks: None, touch_at: None,
            pulses: VecDeque::new(), pulse_release: false, paused: false }
    }
}
fn clean(x: f32) -> f32 { if x.is_finite() { x.clamp(-1.0, 1.0) } else { 0.0 } }
impl InputState {
    pub fn physical(&mut self, buttons: u32, axes: [f32; 4]) {
        if self.paused { return; }
        let buttons = buttons & 0xffff;
        self.pending_physical |= buttons & !self.physical.buttons;
        self.physical = Sample { buttons, axes: axes.map(clean), touch_axes: 0 };
    }
    pub fn touch(&mut self, buttons: u32, axes: [f32; 4], active: u8, now: Instant) {
        if self.paused { return; }
        let special = buttons & (PACK_REQUEST | THROW_REQUEST);
        if special & THROW_REQUEST != 0 && self.touch_special & THROW_REQUEST == 0 {
            self.throw_ticks = Some(0);
        }
        // PACK is a distinct Java-only shortcut (0x10000), NOT a normal
        // Cross press. Don't guess at gameplay behavior while repairing the
        // requested THROW and WEAPONS paths; never leak 0x10000 into PS2
        // pad input, where that bit means the unrelated stick FLICK.
        self.touch_special = special;
        let buttons = buttons & 0xffff;
        if buttons & TRIANGLE == 0 { self.triangle_armed = false; }
        self.pending_touch |= buttons & !self.touch.buttons;
        self.touch = Sample { buttons, axes: axes.map(clean), touch_axes: active & 3 };
        self.touch_at = Some(now);
    }
    pub fn pulse(&mut self, buttons: u32) {
        if !self.paused && self.pulses.len() < 64 && buttons & 0xffff != 0 {
            self.pulses.push_back(buttons & 0xffff);
        }
    }
    pub fn pause(&mut self, value: bool) {
        if value != self.paused {
            *self = Self { paused: value, ..Self::default() };
        }
    }
    pub fn paused(&self) -> bool { self.paused }

    /// Exactly once per game simulation tick. `gameplay` gates special
    /// actions from the overlay. `prompt` makes WEAPONS also act as the
    /// PS2 Triangle *only* when the game has an actual interaction prompt;
    /// otherwise its original Java weapon-wheel gesture stays independent.
    pub fn tick(&mut self, now: Instant, gameplay: bool, prompt: bool) -> Sample {
        if self.paused { return Sample::default(); }
        let mut out = self.physical;
        let physical_buttons = self.physical.buttons | std::mem::take(&mut self.pending_physical);
        let touch_edges = std::mem::take(&mut self.pending_touch);
        // Only a fresh touch while a context prompt is actually rendered may
        // provide Triangle; a wheel gesture begun elsewhere cannot be reused.
        if gameplay && prompt && touch_edges & TRIANGLE != 0 { self.triangle_armed = true; }
        // A disappearing interaction prompt consumes the old gesture. It must
        // never reactivate if a vendor or ship prompt reappears while held.
        if !gameplay || !prompt { self.triangle_armed = false; }
        let mut touch_buttons = self.touch.buttons | touch_edges;
        // The built-in Java quick-select wheel forwards Triangle/axes to
        // native even without a contextual prompt; dropping that bit breaks
        // the wheel. Only suppress a pre-existing gesture *while* a prompt
        // is displayed. Contextual Triangle still requires a fresh DOWN.
        if !gameplay || (prompt && !self.triangle_armed) { touch_buttons &= !TRIANGLE; }
        // Unlike a finger hover, physical Triangle must always be retained.
        out.buttons = physical_buttons | touch_buttons;
        let fresh = self.touch_at.is_some_and(|t|
            now.saturating_duration_since(t) < Duration::from_millis(250));
        for (bit, range) in [(1, 0..2), (2, 2..4)] {
            if self.touch.touch_axes & bit != 0 && (bit == 1 || fresh) {
                for i in range { out.axes[i] = self.touch.axes[i]; }
                out.touch_axes |= bit;
            }
        }
        if !gameplay {
            self.throw_ticks = None;
        } else if let Some(t) = self.throw_ticks {
            // R1 crouches: L1/L2 is the actual look/strafe stance used
            // by rc-game's Hero::weapon_check() wrench throw. Holding L1
            // first is essential: a same-frame L1+Square causes a swing.
            out.buttons |= L1;
            if t == THROW_AIM_TICKS { out.buttons |= SQUARE; }
            self.throw_ticks = if t >= THROW_RELEASE_TICKS { None } else { Some(t + 1) };
        }
        if self.pulse_release { self.pulse_release = false; }
        else if let Some(b) = self.pulses.pop_front() { out.buttons |= b; self.pulse_release = true; }
        out
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    const ZERO: [f32;4] = [0.;4];
    fn tick(s:&mut InputState,now:Instant)->Sample {s.tick(now,true,false)}
    #[test] fn tap_survives_between_renders() {
        let mut s=InputState::default();s.physical(64,ZERO);s.physical(0,ZERO);
        assert_eq!(tick(&mut s,Instant::now()).buttons,64);
        assert_eq!(tick(&mut s,Instant::now()).buttons,0);
    }
    #[test] fn touch_does_not_erase_physical_buttons() {
        let mut s=InputState::default();let n=Instant::now();
        s.physical(8,[0.7,0.3,0.2,0.4]);
        s.touch(64,[-0.5,0.0,0.8,0.0],1,n);
        let p=tick(&mut s,n);
        assert_eq!(p.buttons,72);assert_eq!(p.axes,[-0.5,0.,0.2,0.4]);assert_eq!(p.touch_axes,1);
    }
    #[test] fn stale_camera_does_not_spin_forever() {
        let mut s=InputState::default();let n=Instant::now();
        s.touch(0,[0.,0.,1.,1.],2,n);
        assert_eq!(tick(&mut s,n).axes[2],1.);
        assert_eq!(tick(&mut s,n+Duration::from_secs(1)).axes[2],0.);
    }
    #[test] fn pause_clears_and_rejects_input() {
        let mut s=InputState::default();s.physical(64,[1.;4]);s.pause(true);
        s.physical(8,[1.;4]);s.pause(false);
        assert_eq!(tick(&mut s,Instant::now()),Sample::default());
    }
    #[test] fn repeated_menu_taps_have_release_ticks() {
        let mut s=InputState::default();s.pulse(64);s.pulse(64);let n=Instant::now();
        assert_eq!([tick(&mut s,n).buttons,tick(&mut s,n).buttons,
            tick(&mut s,n).buttons,tick(&mut s,n).buttons],[64,0,64,0]);
    }
    #[test] fn invalid_axes_are_sanitized() {
        let mut s=InputState::default();s.physical(0,[f32::NAN,f32::INFINITY,-2.,2.]);
        assert_eq!(tick(&mut s,Instant::now()).axes,[0.,0.,-1.,1.]);
    }
    #[test] fn wheel_triangle_only_acts_on_real_prompts() {
        let mut s=InputState::default();let n=Instant::now();
        s.touch(TRIANGLE,ZERO,0,n);
        assert_eq!(s.tick(n,true,false).buttons&TRIANGLE,TRIANGLE);
        // Preserve native wheel opening outside prompts. An old wheel hold
        // cannot activate the vendor or ship when a prompt appears.
        assert_eq!(s.tick(n,true,true).buttons&TRIANGLE,0);
        s.touch(0,ZERO,0,n);
        s.touch(TRIANGLE,ZERO,0,n);
        assert_ne!(s.tick(n,true,true).buttons&TRIANGLE,0);
        s.touch(0,ZERO,0,n);
        assert_eq!(s.tick(n,true,true).buttons&TRIANGLE,0);
    }
    #[test] fn wheel_held_before_a_prompt_does_not_become_interaction() {
        let mut s=InputState::default();let n=Instant::now();
        s.touch(TRIANGLE,ZERO,0,n);
        assert_eq!(s.tick(n,true,false).buttons & TRIANGLE,TRIANGLE);
        for _ in 0..5 {assert_eq!(s.tick(n,true,true).buttons & TRIANGLE,0);}
        s.touch(0,ZERO,0,n);
        s.touch(TRIANGLE,ZERO,0,n);
        assert_eq!(s.tick(n,true,true).buttons & TRIANGLE,TRIANGLE);
        s.touch(0,ZERO,0,n);
        assert_eq!(s.tick(n,true,true).buttons & TRIANGLE,0);
    }
    #[test] fn hidden_then_returned_prompt_needs_fresh_triangle_down_edge() {
        let mut s=InputState::default();let n=Instant::now();
        s.touch(TRIANGLE,ZERO,0,n);
        assert_eq!(s.tick(n,true,true).buttons & TRIANGLE,TRIANGLE);
        assert_eq!(s.tick(n,true,false).buttons & TRIANGLE,TRIANGLE);
        assert_eq!(s.tick(n,true,true).buttons & TRIANGLE,0);
        s.touch(0,ZERO,0,n);s.tick(n,true,true);
        s.touch(TRIANGLE,ZERO,0,n);
        assert_eq!(s.tick(n,true,true).buttons & TRIANGLE,TRIANGLE);
    }
    #[test] fn physical_triangle_still_works_without_prompt() {
        let mut s=InputState::default();let n=Instant::now();
        s.physical(TRIANGLE,ZERO);s.touch(TRIANGLE,ZERO,0,n);
        assert_eq!(s.tick(n,true,false).buttons&TRIANGLE,TRIANGLE);
    }
    #[test] fn throw_touch_request_survives_short_tap_and_aims_before_firing() {
        let mut s=InputState::default();let n=Instant::now();
        s.touch(THROW_REQUEST,ZERO,0,n);
        s.touch(0,ZERO,0,n);
        for _ in 0..THROW_AIM_TICKS {
            assert_eq!(tick(&mut s,n).buttons&(L1|SQUARE),L1);
        }
        assert_eq!(tick(&mut s,n).buttons&(L1|SQUARE),L1|SQUARE);
        assert_eq!(tick(&mut s,n).buttons&(L1|SQUARE),L1);
        assert_eq!(tick(&mut s,n).buttons&(L1|SQUARE),L1);
        assert_eq!(tick(&mut s,n).buttons&(L1|SQUARE),0);
    }
    #[test] fn throw_cannot_leak_into_menus_or_retrigger_while_held() {
        let mut s=InputState::default();let n=Instant::now();
        s.touch(THROW_REQUEST,ZERO,0,n);
        assert_eq!(s.tick(n,false,false).buttons,0);
        assert_eq!(s.tick(n,true,false).buttons,0);
        s.touch(0,ZERO,0,n);
        s.touch(THROW_REQUEST,ZERO,0,n);
        assert_eq!(s.tick(n,true,false).buttons,L1);
        for _ in 0..(THROW_RELEASE_TICKS as usize+2){ s.tick(n,true,false); }
        s.touch(THROW_REQUEST,ZERO,0,n);
        assert_eq!(s.tick(n,true,false).buttons,0);
    }
    #[test] fn pack_does_not_fake_cross_or_ps2_flick() {
        let mut s=InputState::default();let n=Instant::now();
        s.touch(PACK_REQUEST,ZERO,0,n);s.touch(0,ZERO,0,n);
        assert_eq!(tick(&mut s,n).buttons,0);
    }
}

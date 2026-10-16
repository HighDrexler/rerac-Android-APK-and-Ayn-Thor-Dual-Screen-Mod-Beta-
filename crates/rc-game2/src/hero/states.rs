//! The state driver: SetState (L00 `0x2c4560`), the per-state physics `0x2ba8e0` and the per-state transitions
//! `0x2cb650`, routed to the group modules by [`super::registry`].
//!
//! **SetState(id, play)**, the parts every state shares:
//! * refusals: in group 0x14 only 0x61 with play 2; with no health left (`0x18c2ec` = 0) in groups 2 / 7 only the
//!   death and knock states (6, 0x39, 0x43, 0x5f, 0x67, 0x74, 0x78, 0x79, 0x7c, 0x7d, 0x7f, 0x91);
//! * bookkeeping: the previous two states / groups (`0x18c0c0..0x18c0d0`), the previous timer `0x18c0c8`, substate
//!   0, state = id, the per-state entry (its module), then SetAnim(blend, seq, frame) when the entry asks for it;
//! * epilogue: a group change resets the group timer `0x189fdc`; the state timer `0x189fd8` and the sequence timers
//!   `0x189fe0` / `0x189fe4` reset; SetState returns true.
//!
//! Not ported in the shared part: the vehicle / body branches (`0x18c0d4` ≠ 0), the state-change hook `0x2af450`
//! (sounds, the held item's pose), the Clank / item side effects of the epilogue.

use super::physics::{rot_dist, sub_rot, Env, PI};
use rc_game::pad::button;

/// `0x1f000`: the d-pad and the stick flick, the flip's direction presses.
pub(super) const FLIP_DIRS: u32 = button::UP | button::RIGHT | button::DOWN | button::LEFT | button::FLICK;
use super::registry::{module_of, Module};
use super::Hero;
use rc_game::hero::anim::AnimCtl;

/// SetState's entry result: an animation to set after the entry (`SetAnim(blend, seq, frame)` at the switch's end),
/// nothing more, or a SetState return value of its own (the entry ran a nested SetState or refused).
pub enum Entry {
    Anim(f32, u8, i32),
    Done,
    Return(bool),
}

impl Hero {
    /// SetState (`0x2c4560`). `play` = the call's second argument (1: set the state's animation).
    pub fn set_state(&mut self, anim: &mut dyn AnimCtl, env: &Env, id: i32, play: bool) -> bool {
        if self.group == 0x14 && id != 0x61 { return false; }
        if self.hp == 0 && (self.group == 2 || self.group == 7) && !matches!(id, 6 | 0x39 | 0x43 | 0x91 | 0x5f | 0x74 | 0x78 | 0x79 | 0x7c | 0x67 | 0x7d | 0x7f) {
            return false;
        }
        self.prev2_state = self.prev_state;
        self.prev2_group = self.prev_group;
        self.prev_timer = self.timer;
        self.prev_state = self.state;
        self.prev_group = self.group;
        self.substate = 0;
        self.state = id;
        self.cam_mode = 0;
        let entry = match module_of(id) {
            Module::Ground => self.ground_entry(anim, env, id, play),
            Module::Walk => self.walk_entry(anim, env, id, play),
            Module::Air => self.fall_entry(anim, env, id, play),
            Module::Jump => self.jump_entry(anim, env, id, play),
            Module::Unported => Entry::Done,
        };
        match entry {
            Entry::Return(r) => return r,
            Entry::Anim(blend, seq, frame) => if play { self.set_anim(anim, blend, seq, frame) },
            Entry::Done => {}
        }
        if self.group != self.prev_group { self.group_timer = 0; }
        self.timer = 0;
        self.substate_timer = 0;
        self.seq_timer = 0;
        true
    }

    /// `0x2ba8e0`: the current state's physics. `false` for a state the port has not got.
    pub fn state_physics(&mut self, env: &Env, anim: &mut dyn AnimCtl) -> bool {
        let (substate, seq) = (self.substate, anim.view().seq_b);
        match module_of(self.state) {
            Module::Ground => self.phys_ground(env, anim),
            Module::Walk => self.phys_walk(env, anim),
            Module::Air => self.phys_fall(env, anim),
            Module::Jump => self.phys_jump(env, anim),
            Module::Unported => return false,
        }
        if substate != self.substate { self.substate_timer = 0; }
        if seq != anim.view().seq_b { self.seq_timer = 0; }
        true
    }

    /// `0x2cb650`: the current state's transitions (the prologue's checks that apply to every state: below the
    /// level's death height `0x1b1a68` with more than 2 above the ground → 0x74, not ported).
    pub fn transitions(&mut self, env: &Env, anim: &mut dyn AnimCtl) {
        let next = match module_of(self.state) {
            Module::Ground => self.trans_ground(env, anim),
            Module::Walk => self.trans_walk(env, anim),
            Module::Air => self.trans_fall(env, anim),
            Module::Jump => self.trans_jump(env, anim),
            Module::Unported => None,
        };
        // `break` with a state: `SetState(next, 1)` at the dispatcher's end.
        if let Some(n) = next { self.set_state(anim, env, n, true); }
    }

    /// `0x330240(0x40, n, 0)`: ✕ pressed within the last `n` ticks.
    pub fn cross_within(env: &Env, n: i32) -> bool { env.pad.pressed_within(button::CROSS, n).is_some() }

    /// `0x2ca270(n, no_plain)`: the jump chooser. ✕ within `n` ticks: with R1 held and the stick not forward, ✕ and a
    /// d-pad press or stick flick within 12 ticks of each other (both within `n` + 12) → the flip 0xb; the strafing
    /// flip (L1 / L2, `0x18b274`) is not ported; else the plain jump 7 unless `no_plain` (the caller picks 7 or 9
    /// itself). Returns whether a jump was set.
    pub fn try_jump(&mut self, anim: &mut dyn AnimCtl, env: &Env, n: i32, no_plain: bool) -> bool {
        if !Self::cross_within(env, n) { return false; }
        let gap = self.flip_gap(env);
        if env.pad.held & button::R1 != 0 && 0 < gap && env.pad.combo(button::CROSS, FLIP_DIRS, n + gap, gap) {
            return self.set_state(anim, env, 0xb, true);
        }
        if no_plain { return false; }
        self.set_state(anim, env, 7, true)
    }

    /// `0x2c3cf8`: the stick's direction against the facing, after `0x2b3af8(1, 0)` (strafing: the strafe's own, not
    /// ported): 3 behind (within 45° of it), 1 right, 0 left, else 2 ahead.
    pub fn flip_direction(&mut self, env: &Env) -> i32 {
        self.stick_target(env, 1.0, 0);
        let d = sub_rot(self.target_yaw, self.rot[2]);
        let q = std::f32::consts::FRAC_PI_4;
        if q <= rot_dist(d, PI) {
            if rot_dist(d, -PI / 2.0) < q { return 1; }
            if rot_dist(d, PI / 2.0) < q { return 0; }
            return 2;
        }
        3
    }

    /// `0x2c3cc0`: the flip's press window, 12 ticks, or 0 with the stick ahead (no flip).
    pub fn flip_gap(&mut self, env: &Env) -> i32 { if self.flip_direction(env) == 2 { 0 } else { 0xc } }
}

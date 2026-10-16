//! The standing ground states: idle 0 (group 0), stop / skid 3 (group 1) and crouch 4 (group 0xc) — their SetState
//! entries (L00 `0x2c4560`), physics (`0x2ba8e0`, the case shared by 0, 1, 3, 4, 0x1c, 0x1d, 0x1e, 0x39, 0x6d, 0x6f,
//! 0x7d) and transitions (`0x2cb650`).
//!
//! Not ported (the branches are noted in place): the sliding floors (`0x18b2b2` → 0x2e / 0x2f, `0x18b2ba` → 0x76),
//! the look / strafe stance 1 (L1), the weapon stances (`0x2cb1c8`), the idle fidgets and back items, the water
//! effects, the crouch's pack jumps (in `0x2cadb0`; its flip and plain jump are ported) and the ledges (0x27).

use super::physics::*;
use super::states::{Entry, FLIP_DIRS};
use super::Hero;
use rc_game::hero::anim::AnimCtl;
use rc_game::pad::button;

impl Hero {
    pub(super) fn ground_entry(&mut self, anim: &mut dyn AnimCtl, _env: &Env, id: i32, _play: bool) -> Entry {
        match id {
            0 => {
                self.group = 0;
                self.momentum = self.eff;
                let seq = self.idle_seq();
                Entry::Anim(if seq == 0x51 { 18.0 } else { -1.0 }, seq, 0)
            }
            3 => {
                self.group = 1;
                self.momentum = self.eff;
                // The stop's sequence 6, or 5 right after the landing sequence 4 (frames 3..16).
                let v = anim.view();
                let seq = if v.seq_a == 4 && 3.0 < v.frame && v.frame < 16.0 { 5 } else { 6 };
                Entry::Anim(9.0, seq, 4)
            }
            4 => {
                self.group = 0xc;
                self.speed = len2(self.eff).min(7.0 * DT);
                self.momentum = self.eff;
                if 0.116_666_67 < len(self.momentum) { self.momentum = set_len(self.momentum, 0.116_666_67); }
                let v = anim.view();
                let blend = if v.seq_a == self.idle_seq() { 9.0 } else { 14.0 };
                self.set_anim(anim, blend, 0xd, 2);
                anim.set_loop(3, 0x12);
                Entry::Done
            }
            _ => Entry::Done,
        }
    }

    /// The ground case of `0x2ba8e0`.
    pub(super) fn phys_ground(&mut self, env: &Env, anim: &mut dyn AnimCtl) {
        self.drag(0.5, 1.0 / 30.0);
        self.edge_brake(env, 3.7, 0.0);
        self.stick_target(env, 1.0, 0);
        if self.state == 4 {
            let seq = anim.view().seq_b;
            if 0.2 < self.stick_mag || seq == 0xe || seq == 0xf {
                self.turn_to(0.002, 0.07, 0.116_355_3, 0);
                if 0.005_817_765 < self.yaw_vel.abs() {
                    let s = 0xe + (self.yaw_vel < 0.0) as u8;
                    if anim.view().seq_b != s { self.set_anim(anim, 5.0, s, 0); }
                    self.anim_speed = (self.yaw_vel.abs() * 18.093_4).clamp(0.55, 2.5);
                } else if self.yaw_vel == 0.0 && anim.view().seq_b != 0xd {
                    self.set_anim(anim, 7.0, 0xd, 0);
                }
            }
        }
        self.target_speed = 0.0;
        let dec = match self.state {
            3 => {
                let dec = (self.slope_ratio + 1.0).min(1.0) * 0.0075;
                self.stop_decel = dec;
                let v = anim.view();
                if v.seq_a == v.seq_b {
                    let r = self.eff_len_xy / dec;
                    if 5.0 < r && (v.frame_b as f32) < 5.5 {
                        self.anim_speed_for_ticks(&v, 13.0, r, 0.5, -1.0);
                        self.anim_speed = self.anim_speed.min(1.1);
                    } else if 6.5 < v.frame_b as f32 {
                        self.anim_speed = 1.0;
                    }
                }
                dec
            }
            4 => 0.0075,
            _ => 0.004_916_667,
        };
        self.speed_step(0.0, dec);
        self.vel = [0.0; 3];
        self.momentum_decay(dec);
        self.wall_check(env, false);
        if self.air_ticks != 0 {
            self.gravity_from(self.eff_v[2], 0.006_944_44);
        } else {
            self.gravity(0.015);
        }
    }

    /// The ground cases 0, 3, 4 of `0x2cb650`. Returns the state the dispatcher sets with `SetState(n, 1)`.
    pub(super) fn trans_ground(&mut self, env: &Env, anim: &mut dyn AnimCtl) -> Option<i32> {
        let held = env.pad.held;
        match self.state {
            0 => {
                self.stick_target(env, 1.0, 0);
                if 5 <= self.air_ticks && 0.4 < self.height {
                    if self.prev_state == 8 && self.timer < 0xf { return None; }
                    return Some(6);
                }
                if self.land_lag != 0 { return None; }
                let from_look = (self.prev_state == 1 || self.prev_state == 0x1d) && self.timer < 5;
                if !from_look && self.try_jump(anim, env, 7, false) { return None; }
                if held & button::R1 != 0 && self.grounded_ticks != 0 { return Some(4); }
                let from_look = (self.prev_state == 1 || self.prev_state == 0x1d) && self.timer < 0x18;
                if from_look { return None; }
                // `0x2b3370(0.1745)`: the stick moved past 10° this tick (the flick); or a magnitude above 0.22.
                if 0.22 < self.stick_mag { return Some(2); }
                None
            }
            3 => {
                if self.land_lag == 0 && env.pad.pressed & button::CROSS != 0 { return Some(7); }
                if self.land_lag == 0 && held & button::R1 != 0 { return Some(4); }
                if 9 <= self.air_ticks && 0.3 < self.height { return Some(6); }
                if anim.view().flags & 2 != 0 { return Some(0); }
                if self.stick_mag <= 0.2 || self.land_lag != 0 { return None; }
                if self.speed == 0.0 { return Some(0); }
                (0.5 < self.stick_mag).then_some(2)
            }
            4 => {
                if 0xb < self.timer && held & button::R1 == 0 {
                    self.set_state(anim, env, 0, false);
                    let seq = self.idle_seq();
                    self.set_anim(anim, -2.0, seq, 0);
                    return None;
                }
                if self.crouch_jump(anim, env) { return None; }
                if 9 <= self.air_ticks {
                    if self.prev_state == 8 && self.timer < 0xf { return None; }
                    return Some(6);
                }
                None
            }
            _ => None,
        }
    }

    /// `0x2cadb0`: the crouch's jumps. R1 held, the stick not ahead, ✕ and a direction press within 12 ticks of each
    /// other (both within 17) → the flip 0xb; after 7 crouched ticks ✕ within 9 → the jump 7 (with the Heli-Pack /
    /// Thruster-Pack the long jumps 10, 0xd, 0xf, 0x10: not ported, Ratchet has no pack on Aranos's opening).
    fn crouch_jump(&mut self, anim: &mut dyn AnimCtl, env: &Env) -> bool {
        let gap = self.flip_gap(env);
        if env.pad.held & button::R1 != 0 && 0 < gap && env.pad.combo(button::CROSS, FLIP_DIRS, gap + 5, gap) {
            self.set_state(anim, env, 0xb, true);
            return true;
        }
        if self.timer < 7 || !Self::cross_within(env, 9) { return false; }
        self.set_state(anim, env, 7, true);
        true
    }

    /// `0x2a9ea8(target, ticks, k, step)`: the playback speed that reaches frame `target` in `ticks` ticks;
    /// `step` < 0 sets it, otherwise the speed approaches it by `step`.
    pub fn anim_speed_for_ticks(&mut self, v: &rc_game::hero::anim::AnimView, target: f32, ticks: f32, k: f32, step: f32) {
        let cur = v.frame;
        let dist = if cur < target { target - cur } else { target + (v.frame_count_b as f32 - cur) };
        let rate = if v.rate != 0.0 { v.rate } else { 1.0 };
        let want = (k * dist) / (ticks * rate).max(1e-6);
        if step < 0.0 { self.anim_speed = want } else { approach(want, step, &mut self.anim_speed); }
    }
}

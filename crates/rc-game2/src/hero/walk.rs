//! Walk / run 2 (group 1): its SetState entry (L00 `0x2c4560` case 2), physics (`0x2ba8e0` case 2 / 0x70) and
//! transitions (`0x2cb650` case 2 / 0x2e / 0x70), with the walk ↔ run animation selector `0x2ca358`.
//!
//! The animation: substate 0 walks (sequence 3, playback speed · 157 in 0.6..4), substate 1 runs (sequence 4,
//! speed · 14 in 0.6..2.2). The substate moves up above the edge `0x1a7dac + 16·s` (2.35 / 10 u/s) and down below
//! `$gp − 0x7248 + 16·s` (0 / 1.9 u/s, `$gp` = 0x1aeff0); the new sequence starts at the old one's phase plus
//! `$gp − 0x7228[new + 2·old]` (1 walk → run, 5 run → walk).
//!
//! Not ported: the strafe (`0x18b274`, L1/L2), the wading walk 0x70 and the slide 0x2e, the held item's run speeds,
//! the water effects, the body lean (`0x2b7c88`, `0x2b8dc0`), the step-down test `0x2cb038` (always "no step"),
//! the flick test `0x2b3370`, the edge slide `0x2b4410` and the weapon stances on stopping (`0x2cb1c8`).

use super::physics::*;
use super::states::Entry;
use super::Hero;
use rc_game::hero::anim::AnimCtl;
use rc_game::pad::button;

/// Walk → run / run → walk speed edges (u/s): `$gp − 0x7248 + 16·s` (low), `0x1a7dac + 16·s` (high).
const WALK_LOW: [f32; 2] = [0.0, 1.9];
const WALK_HIGH: [f32; 2] = [2.35, 10.0];
/// The phase offset when the substate changes, `[new + 2·old]` (`$gp − 0x7228`).
const PHASE: [i32; 4] = [0, 1, 5, 0];
/// The walk entry's start frame per substate (`$gp − 0x7218`).
const ENTRY_FRAME: [i32; 2] = [8, 0x10];

impl Hero {
    pub(super) fn walk_entry(&mut self, _anim: &mut dyn AnimCtl, env: &Env, _id: i32, _play: bool) -> Entry {
        self.group = 1;
        if self.prev_group != 4 && self.prev2_group != 4 { self.speed = len2(self.eff); }
        self.run_dir = self.rows[0];
        self.run_keep = 0;
        self.run_yaw = self.rot[2];
        self.speed = self.speed.min(7.0 * DT);
        self.quick_turn = 0;
        self.stick_target(env, 1.0, 0);
        if PI / 2.0 < rot_dist(self.rot[2], self.target_yaw) && (self.prev_group == 4 || self.prev_state == 3 || self.prev2_state == 3) {
            self.speed = 0.0;
        }
        self.substate = 0;
        self.run_hold = 0;
        self.walk_gate = 0;
        let mut blend = 8;
        if self.prev2_state == 0x22 || self.prev_state == 0x22 || self.prev2_state == 0x83 || self.prev_state == 0x83 { blend = 0xf; }
        if self.prev_state == 0x13 { blend = 0xd; }
        if self.prev2_state == 6 || self.prev2_state == 0x2c { blend = 0x12; }
        if self.prev2_group == 4 && 5 < self.jump.landed { blend = 0xd; }
        Entry::Anim(blend as f32, (self.substate + 3) as u8, ENTRY_FRAME[self.substate as usize & 1])
    }

    /// `0x2ba8e0` case 2.
    pub(super) fn phys_walk(&mut self, env: &Env, _anim: &mut dyn AnimCtl) {
        if self.eff_len_xy < 0.028_333_336 {
            self.run_hold = 1;
            if 2.094_395_4 < self.yaw_residual.abs() && self.stick_mag == 1.0 { self.quick_turn = 1; }
        }
        if self.yaw_residual.abs() < 0.087_266_47 { self.run_hold = 0; }
        if self.quick_turn != 0 && self.yaw_residual.abs() < 0.069_813_18 { self.quick_turn = 0; }
        if self.stick_mag < 0.2 {
            if self.run_keep != 0 { self.quick_turn = 1; }
            self.target_speed = 0.0;
            self.stick_dir = self.run_dir;
            self.target_yaw = self.run_yaw;
        } else {
            self.run_target(env);
            self.run_dir = self.rows[0];
            self.run_yaw = self.rot[2];
        }
        let mut acc_k = 1.0;
        if self.substate == 1 && self.quick_turn == 0 {
            self.turn_to(0.02, 0.17, 0.165_806_28, 0);
        } else {
            if self.stick_mag < 0.9 && PI / 4.0 < self.yaw_residual.abs() { acc_k = 0.0; }
            self.run_turn();
        }
        if self.quick_turn != 0 {
            let v = (0.099_166_67 - self.yaw_residual.abs() * 0.029_178_405).max(0.0);
            self.target_speed = self.target_speed.min(v);
        }
        let acc = acc_k * 0.005;
        let mut dec = 0.005_555_556;
        if self.quick_turn != 0 {
            let f = (2.617_994 - self.yaw_residual).clamp(0.2, 1.0);
            dec = 0.004_722_222;
            self.target_speed *= f;
        }
        let r = self.yaw_residual.abs();
        if self.stick_mag < 0.75 && 0.610_865_3 < r { self.target_speed *= (1.0 - r * 0.4).max(0.0); }
        self.speed_step(acc, dec);
        if self.run_hold == 0 { self.set_planar_vel(f32::MAX) } else { self.set_planar_vel(self.target_yaw) }
        self.wall_check(env, true);
        if self.air_ticks != 0 {
            self.gravity_from(self.eff_v[2], 0.006_944_44);
        } else {
            self.gravity(0.015);
        }
    }

    /// `0x2cb650` case 2.
    pub(super) fn trans_walk(&mut self, env: &Env, anim: &mut dyn AnimCtl) -> Option<i32> {
        if self.walk_gate != 0 {
            let v = anim.view();
            if 0.9 < v.t || v.seq_a != 0xff { self.walk_gate = 0; }
        }
        if !(self.air_ticks < 5 || self.height <= 1.35) { return Some(6); }
        let n = if self.land_lag != 0 { 1 } else { 9 };
        if Self::cross_within(env, n) && !(4 < self.air_ticks) {
            self.stick_target(env, 1.0, 0);
            if self.try_jump(anim, env, n, true) { return None; }
            return Some(if 0.85 < self.stick_mag { 9 } else { 7 });
        }
        if self.air_ticks != 0 { return None; }
        if env.pad.held & button::R1 != 0 { return Some(4); }
        let keep = (0.17 <= self.stick_mag && self.land_lag == 0) || (self.run_keep != 0 && 0.017_453_294 <= self.yaw_residual.abs());
        if keep {
            self.walk_anim(anim);
            return None;
        }
        if self.set_state(anim, env, 0, false) {
            if 0.1 < len(self.momentum) { self.momentum = set_len(self.momentum, 0.1); }
            self.speed = self.speed.min(0.025);
            let seq = self.idle_seq();
            self.set_anim(anim, if seq == 0x51 { 15.0 } else { -1.0 }, seq, 0);
        }
        None
    }

    /// `0x2ca358`: the walk / run sequence and its playback speed.
    fn walk_anim(&mut self, anim: &mut dyn AnimCtl) {
        let v = anim.view();
        if v.seq_b == 0x14 {
            self.set_anim(anim, 7.0, (self.substate + 3) as u8, 0);
        } else if 4 < self.substate_timer && self.walk_gate == 0 {
            let old = self.substate.clamp(0, 1);
            let mut s = old;
            let sp = self.speed / DT;
            if sp < WALK_LOW[s as usize] || WALK_HIGH[s as usize] < sp {
                while 0 < s && sp < WALK_LOW[s as usize] { s -= 1; }
                while s < 1 && WALK_HIGH[s as usize] < sp { s += 1; }
                if s != old {
                    let (n_old, n_new) = (anim.frame_count(old as u8 + 3) as i32, anim.frame_count(s as u8 + 3) as i32);
                    let frame = if n_old == 0 || n_new == 0 { 0 } else { (v.frame_b as i32 * n_new / n_old + PHASE[(s + 2 * old) as usize]) % n_new };
                    self.substate = s;
                    self.set_anim(anim, 8.0, (s + 3) as u8, frame);
                }
            }
        }
        self.anim_speed = if self.substate == 0 { (self.speed * 157.0).clamp(0.6, 4.0) } else { (self.speed * 14.0).clamp(0.6, 2.2) };
    }
}

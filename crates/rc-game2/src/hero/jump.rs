//! The jump group (group 4): SetState's shared jump entry (L00 `0x2c4560`, the case of 7, 9, 0xa..0x12, …), the
//! physics (`0x2ba8e0` case 7…, the vertical `0x2b6668`, the air control `0x2b6cd0`) and the transitions
//! (`0x2cb650` case 7…, the landing `0x2cab40`). Ported: the jump 7, the run jump 9, the flip 0xb and the double
//! jump 0xe.
//!
//! **Height.** The entry sets a minimum and maximum height (7: 1.28 / 2.3, 9: 1.0 / 2.7, 0xb: 3.2 / 3.25), the
//! gravity 0.00825 (descending: 0.009652 for 7), the takeoff tick (5) and the ramp (15; 0xb 14). While ✕ is held
//! (and for the first tick), up to the ramp's end, the height aimed at grows by `(max − min) / ramp` per tick and the
//! missing vertical speed `sqrt(2·h·g) − applied` is added from the takeoff on: the longer ✕ is held, the higher the
//! jump. The double jump instead raises the vertical speed toward 7/60 for 8 ticks, less the higher above the first
//! jump's takeoff.
//!
//! **Animation.** 7: sequence 7, 9: sequence 8 or 9 by the run's foot (`$gp − 0x7210`[run frame]); blend 5. One tick
//! after the takeoff the playback is timed so the hold frame (18) comes at the landing ETA; descending, the apex
//! frame (7: 30, 9: 29, 0xb: 31, 0xe: 43) is timed to the landing; at touchdown the sequence jumps to the landing
//! frame (7: 12, 9: 11, 0xb: 24, 0xe: 19) + 2. The flip plays 0x1c..0x1f by its direction (left, right, ahead,
//! back); the double jump 0x16 from frame 3 at 0.8, and cannot touch down before frame 35.
//!
//! **The flip 0xb** (R&C1's id for its long jump; R&C2's long jumps need a pack): R1 held, the stick not ahead, ✕
//! and a direction press together (`states::try_jump`, the crouch's `0x2cadb0`, or chained on landing). It flies
//! along the stick's yaw (`flip_air`) and allows no double jump.
//!
//! Not ported: the pack jumps (10, 0xd, 0xf, 0x10) and the glide 8, the other jump states (the gadget / water /
//! wall / scripted jumps), the scripted vertical curves (`0x18a2bd`), the strafe flips, the ledges, the landing
//! effects and camera jolt, the jump sounds, the body lean.

use super::physics::*;
use super::states::{Entry, FLIP_DIRS};
use super::Hero;
use rc_game::hero::anim::AnimCtl;
use rc_game::pad::button;

/// `$gp − 0x7210`: the run jump's sequence by the run's frame (the leading foot).
const RUN_JUMP_SEQ: [u8; 24] = [9, 9, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9];
/// `0x1a7db4` (0.82): the target speed above which a re-jump can become the run jump. The target is the one the flip
/// test `0x2c3cc0` left this tick (`0x2b3af8(1, 0)`: the stick's magnitude), so a re-jump at nearly full stick toward
/// the facing is the run jump.
const RUN_JUMP_SPEED: f32 = 0.82;

impl Hero {
    pub(super) fn jump_entry(&mut self, anim: &mut dyn AnimCtl, env: &Env, id: i32, play: bool) -> Entry {
        self.group = 4;
        self.cam_mode = 0x50;
        self.vel = self.eff;
        let height = self.height;
        let j = &mut self.jump;
        // The block's fields the entry does not write keep their values (the takeoff position: the double jump
        // measures from the first jump's).
        *j = super::JumpBlock {
            g: 0.008_25,
            acc: 0.005_555_556,
            dec: 0.003_055_555_7,
            keep_speed: 0,
            max_air: 0x50,
            f34a: 0x46,
            air_turn: 0.250_011_4,
            terminal: 0.833_333_3,
            ref_z: self.pos[2],
            fallover_h: 2.5,
            f334: 0.5,
            air_speed: 0.099_166_67,
            takeoff_pos: j.takeoff_pos,
            land_eta: j.land_eta,
            ..Default::default()
        };
        match id {
            7 => {
                j.g_down = 0.009_652_62;
                (j.f_apex, j.f_land, j.hmax, j.hmin) = (30.0, 12.0, 2.3, 1.28);
            }
            9 => (j.f_apex, j.f_land, j.hmax, j.hmin) = (29.0, 11.0, 2.7, 1.0),
            0xe => {
                // Lower the higher he already is: 0.7, plus half of what is missing to 2.5 below a height of 2.
                let h = if height < 2.0 { (2.5 - height) * 0.5 + 0.7 } else { 0.7 };
                (j.hmin, j.hmax, j.f346, j.takeoff, j.bottom) = (h, h + 0.05, 0x1e, -1, 0.6);
                (j.f_hold, j.f_apex, j.f_land, j.ramp, j.f334, j.air_speed) = (32.0, 43.0, 19.0, 0xe, 1.0, 0.079_166_67);
            }
            _ => {
                // 0xb, the flip.
                (j.f34a, j.takeoff, j.bottom, j.f_hold, j.f_apex, j.f_land) = (0x32, 5, 0.6, 18.0, 31.0, 24.0);
                (j.hmax, j.hmin, j.ramp, j.fallover_h, j.f334, j.no_double) = (3.25, 3.2, 0xe, 4.5, 1.0, 1);
            }
        }
        j.h = j.hmin;
        if id == 7 || id == 9 {
            (j.can_double, j.ramp, j.f_hold, j.bottom, j.takeoff, j.f34c, j.f34e) = (1, 0xf, 18.0, 0.6, 5, 1, 1);
        }
        if id == 0xb {
            self.jump.flip_yaw = self.yaw_ago(10);
            self.jump.flip_fwd = self.rows[0];
            self.jump.flip_dir = self.flip_direction(env);
            let j = &mut self.jump;
            (j.flip_target_yaw, j.flip_stick_dir) = (self.target_yaw, self.stick_dir);
            j.flip_speed = if 0.025 < self.eff_len_xy { self.eff_len_xy } else { 0.0 };
            (j.flip_push, j.f34c, j.f34e) = (0.0, 1, 1);
        }
        let j = &mut self.jump;
        if j.fallover_h == 0.0 { j.fallover_h = j.h * 1.5; }
        match id {
            7 => Entry::Anim(5.0, 7, 0),
            9 => {
                let v = anim.view();
                Entry::Anim(5.0, if v.seq_a == 4 { RUN_JUMP_SEQ.get(v.frame as usize).copied().unwrap_or(8) } else { 8 }, 0)
            }
            0xe => {
                if play {
                    self.set_anim(anim, 7.0, 0x16, 3);
                    self.anim_speed = 0.8;
                }
                Entry::Done
            }
            _ => {
                // The flip's sequence by its direction (0x1c left, 0x1d right, 0x1e ahead, 0x1f back; the option
                // `0x1a7a4d` swaps left and right: not ported); chained from a jump it starts at frame 1, takeoff 1.
                let seq = 0x1c + self.jump.flip_dir as u8;
                if play && self.prev_group == 4 {
                    self.set_anim(anim, 4.0, seq, 1);
                    self.jump.takeoff = 1;
                    Entry::Done
                } else {
                    Entry::Anim(5.0, seq, 0)
                }
            }
        }
    }

    pub(super) fn phys_jump(&mut self, env: &Env, _anim: &mut dyn AnimCtl) {
        if self.jump.landed != 0 { self.stick_target(env, 0.099_166_67, 0); }
        self.stick_target(env, self.jump.air_speed, 0);
        if self.timer == self.jump.takeoff { self.jump.takeoff_pos = self.pos; }
        if self.jump.landed == 0 && (self.jump.takeoff <= self.timer || self.state == 0xb) {
            if self.state == 0xb { self.flip_air() } else { self.air_control() }
        } else {
            self.turn_to(0.015, 0.2, 0.250_011_4, 0);
            let s = self.speed;
            if 1 < self.jump.landed { self.speed *= 0.4; }
            self.set_planar_vel(f32::MAX);
            self.speed = s;
            if self.jump.landed != 1 { self.speed_step(self.jump.acc, self.jump.dec); }
            if self.jump.landed != 0 { self.momentum_decay(self.jump.dec); }
            self.drag(0.5, 1.0 / 30.0);
            // `0x2d3b38(k, v, v)`: v + the facing · k — the run jump's windup pushes forward.
            if self.state == 9 && self.timer < self.jump.takeoff + 4 { self.vel = self.along_facing(self.vel, self.timer as f32 * 0.004_444_444_6); }
            // Blocked ahead in the first 10 ticks: a line (flags 4) from 0.5 up to 0.95 ahead.
            if self.timer < 10 && 0.2 < self.stick_mag {
                let a = add(self.pos, [0.0, 0.0, 0.5]);
                if env.line(a, self.along_facing(a, 0.95), 4).is_some() { self.jump.blocked = 1; }
            }
        }
        if self.jump.ref_z < self.ground_z {
            self.jump.ref_z = (self.jump.ref_z + 0.1).min(self.ground_z);
        }
        if self.pos[2] < self.jump.ref_z { self.jump.ref_z = self.pos[2]; }
        if self.jump.takeoff < self.timer && self.vel[2] < 0.001 && (self.state != 0xe || 0x11 < self.timer) {
            self.jump.descending = 1;
            if self.jump.g_down != 0.0 { self.jump.g = self.jump.g_down; }
        }
        self.jump_vertical(env);
    }

    /// `0x2b6668` for the plain jumps: before the takeoff a small push down (1/75); landed, 0.015 down; in the air,
    /// gravity with the fall speed capped at `0x18a2c0`. Then the height ramp (module docs).
    fn jump_vertical(&mut self, env: &Env) {
        let j = self.jump;
        if self.timer < j.takeoff {
            self.vel[2] = -0.013_333_334;
        } else if j.landed != 0 {
            self.vel[2] = -0.015;
        } else {
            self.vel[2] = (self.vel[2] - j.g).max(-j.terminal);
        }
        if self.state == 0xe {
            self.double_jump_rise();
            return;
        }
        let first = j.pending == 0.0 && j.applied == 0.0;
        if (env.pad.held & button::CROSS != 0 || first) && j.h < j.hmax && self.timer <= j.ramp {
            let h = (j.h + (j.hmax - j.hmin) / j.ramp as f32).min(j.hmax);
            self.jump.h = h;
            let v = ((h + h) * j.g).sqrt();
            self.jump.pending = v - self.jump.applied;
        }
        if j.takeoff <= self.timer && 0.0 < self.jump.pending {
            self.vel[2] += self.jump.pending;
            self.jump.applied += self.jump.pending;
            self.jump.pending = 0.0;
        }
    }

    /// `0x2b6668`'s double jump (no pack): for the first 8 ticks the vertical speed rises toward 7/60 by 1/24 a
    /// tick, ×0.7 more than 1.7 above the first jump's takeoff and ×0.45 more than 2.1 above it. (With the
    /// Thruster-Pack: other limits and the pack's effects; not ported.)
    fn double_jump_rise(&mut self) {
        let mut k = 0.116_666_675;
        let d = self.pos[2] - self.jump.takeoff_pos[2];
        if 2.1 < d {
            k *= 0.45;
        } else if 1.7 < d {
            k *= 0.7;
        }
        if self.timer < 8 {
            let mut v = self.vel[2];
            if v < k { approach(k, 0.041_666_668, &mut v); }
            self.vel[2] = v;
        }
    }

    /// `0x2b6cd0`'s flip: the facing turns toward the yaw of 10 ticks before (0.04, 0.2, at most 0.1164 a tick); the
    /// push along the stick's yaw grows toward 0.07 by 1/450 (toward 0 once descending within 11 ticks of the
    /// ground); the ground speed carried goes along the old facing, scaled by the sine of the angle between it and the
    /// stick (the back flip carries none); then the drag (0.7) as momentum. (Strafing and the camera-relative mode
    /// `0x18c0e3`: not ported.)
    fn flip_air(&mut self) {
        let j = self.jump;
        self.target_yaw = j.flip_yaw;
        self.stick_dir = j.flip_fwd;
        self.turn_to(0.04, 0.2, 0.116_355_3, 0);
        self.speed = if j.flip_dir == 3 { 0.0 } else { j.flip_speed };
        let t = if j.descending == 0 || 0xb < j.land_eta { 0.07 } else { 0.0 };
        approach(t, 0.002_222_222_3, &mut self.jump.flip_push);
        let (s, c) = j.flip_target_yaw.sin_cos();
        self.vel[0] = c * self.jump.flip_push;
        self.vel[1] = s * self.jump.flip_push;
        let carry = j.flip_speed * rot_dist(j.flip_yaw, j.flip_target_yaw).sin();
        let (s, c) = j.flip_yaw.sin_cos();
        self.vel[0] += c * carry;
        self.vel[1] += s * carry;
        self.drag(0.7, 0.0);
        self.momentum = self.drag;
    }

    /// `0x2cb650` case 7…: the flip chained on landing, the fall-over when airborne too long, the animation timing,
    /// the touchdown, the re-jump and the landing, the double jump.
    pub(super) fn trans_jump(&mut self, env: &Env, anim: &mut dyn AnimCtl) -> Option<i32> {
        // The flip chained from a jump: the stick not ahead, in the first 12 ticks or once landed, on the ground; once
        // landed ✕ within 7 (the flip itself only once landed); a direction press within 5 with R1 held.
        if self.jump.f34e != 0 {
            let gap = self.flip_gap(env);
            let j = self.jump;
            if 0 < gap && (self.timer < gap || j.landed != 0) && self.grounded_ticks != 0 && (self.state != 0xb || Self::cross_within(env, 7)) {
                let mut ok = true;
                if j.landed != 0 { ok = Self::cross_within(env, 7); }
                if self.state == 0xb && j.landed == 0 { ok = false; }
                if ok && env.pad.pressed_within(FLIP_DIRS, 5).is_some() && env.pad.held & button::R1 != 0 { return Some(0xb); }
            }
        }
        let j = self.jump;
        if j.max_air < self.timer && j.landed == 0 { return Some(6); }
        // The double jump's first 35 frames cannot touch down.
        let early_double = self.state == 0xe && anim.view().frame < 35.0;
        if self.timer <= j.takeoff { return None; }
        if self.timer == j.takeoff + 1 {
            if self.state != 0xe {
                let eta = self.land_eta(j.g);
                let v = anim.view();
                self.anim_speed_for_ticks(&v, j.f_hold, eta, j.f334, -1.0);
                self.anim_speed = self.anim_speed.min(1.5);
            }
            return None;
        }
        let v = anim.view();
        if j.landed == 0 && self.air_ticks == 0 && j.descending != 0 && !early_double {
            self.jump.landed = 1;
            if self.stick_mag < 0.2 && 0.041_666_668 < self.speed { self.speed = 0.041_666_668; }
            if v.frame < j.f_apex - 1.5 { self.set_anim(anim, 7.0, v.seq_b, j.f_land as i32 + 2); }
        }
        if self.jump.landed == 0 {
            if j.descending != 0 {
                if (v.frame_b as f32) < j.f_land {
                    if self.state != 0xe || 37.0 <= v.frame {
                        let eta = self.land_eta(j.g).min(60.0);
                        self.jump.land_eta = eta as i16;
                        self.anim_speed_for_ticks(&v, j.f_apex, eta, j.f334, 0.2);
                    } else {
                        approach(1.0, 0.15, &mut self.anim_speed);
                    }
                    if !early_double && 1.8 < self.anim_speed { self.set_anim(anim, self.jump.land_eta as f32 + 3.0, v.seq_b, j.f_land as i32 + 2); }
                    self.anim_speed = self.anim_speed.clamp(0.2, 2.7);
                }
                if v.frame.round() == j.f_apex.round() { self.anim_speed = 0.0; }
            }
        } else {
            self.anim_speed = 1.0;
        }
        if self.jump.landed != 0 && self.land_lag == 0 && Self::cross_within(env, 8) && self.air_ticks < 4 {
            let mut n = 7;
            if RUN_JUMP_SPEED < self.target_speed
                && rot_dist(self.target_yaw, self.rot[2]) < std::f32::consts::FRAC_PI_3
                && RUN_SPEED * 0.014_166_668 < self.eff_len_xy
            {
                n = 9;
            }
            return Some(n);
        }
        if self.jump.landed < 2 {
            if j.descending != 0 && j.f34a <= self.timer && j.fallover_h < self.height { return Some(6); }
        } else {
            self.land(env, anim);
            if self.group != 4 { return None; }
        }
        // The double jump: in the double jump high up, the rise's playback toward 1.3; ✕ pressed since the jump began
        // (4 ticks in, 2..30), after 14 ticks, still in the air, from a jump that allows it: on the way up, or near the
        // top (more than 0.7 up, vertical speed under 0.0967).
        if self.state == 0xe && 4.0 < self.height {
            if self.jump.descending == 0 && self.anim_speed < 1.3 { approach(1.3, 0.1, &mut self.anim_speed); }
            self.jump.f346 = 0x11;
        }
        let window = (self.timer - 4).clamp(2, 0x1e);
        let j = self.jump;
        let up_or_top = j.descending == 0 || (0.7 < self.height && self.vel[2].abs() < 0.096_666_67);
        if j.landed == 0 && j.no_double == 0 && self.state != 0xe && Self::cross_within(env, window) && j.can_double != 0 && 0xe < self.timer && up_or_top {
            return Some(0xe);
        }
        None
    }

    /// `0x2cab40`: the landing — crouch 4 with R1, the stop 3 with speed (after 3 landed ticks) or idle 0 when the
    /// landing sequence ends (13 ticks or the wrap), else the walk 2 with the landing sequence 4.
    fn land(&mut self, env: &Env, anim: &mut dyn AnimCtl) {
        self.stick_target(env, 1.0, 0);
        if env.pad.held & button::R1 != 0 {
            self.set_state(anim, env, 4, false);
            self.set_anim(anim, -2.0, 0xd, 2);
            anim.set_loop(3, 0x12);
            return;
        }
        if self.stick_mag <= 0.3 || self.land_lag != 0 {
            if 0.05 < self.fwd_speed && 3 < self.jump.landed {
                if self.set_state(anim, env, 3, false) {
                    self.set_anim(anim, -1.0, 5, 0);
                    if 1.0 / 12.0 < len(self.momentum) { self.momentum = set_len(self.momentum, 1.0 / 12.0); }
                }
                return;
            }
            let wrapped = anim.view().flags & 2 != 0;
            if self.jump.landed < 0xd && !wrapped { return; }
            if !self.set_state(anim, env, 0, false) { return; }
            let seq = self.idle_seq();
            if seq == 0x51 { self.set_anim(anim, 15.0, seq, 0) } else if wrapped { self.set_anim(anim, -2.0, seq, 0) }
            return;
        }
        if !self.set_state(anim, env, 2, false) { return; }
        let blend = if 5 < self.jump.landed { 0xe } else { 8 };
        self.substate = 1;
        self.set_anim(anim, blend as f32, 4, 0x10);
        self.walk_gate = 1;
    }

    /// The ticks until the feet reach the ground at gravity `g` (`0x2d4138`, here from the current height).
    pub fn land_eta(&self, g: f32) -> f32 {
        let (vz, h) = (self.vel[2], (self.pos[2] - self.ground_z).max(0.0));
        if g <= 0.0 { return 1.0; }
        ((vz + (vz * vz + 2.0 * g * h).sqrt()) / g).max(1.0)
    }
}

//! The jump group (group 4): SetState's shared jump entry (L00 `0x2c4560`, the case of 7, 9, 0xa..0x12, …), the
//! physics (`0x2ba8e0` case 7…, the vertical `0x2b6668`, the air control `0x2b6cd0`) and the transitions
//! (`0x2cb650` case 7…, the landing `0x2cab40`). Ported: the jump 7 and the run jump 9.
//!
//! **Height.** The entry sets a minimum and maximum height (7: 1.28 / 2.3, 9: 1.0 / 2.7), the gravity 0.00825
//! (descending: 0.009652 for 7), the takeoff tick 5 and the ramp 15. While ✕ is held (and for the first tick), up to
//! the ramp's end, the height aimed at grows by `(max − min) / ramp` per tick and the missing vertical speed
//! `sqrt(2·h·g) − applied` is added from the takeoff on: the longer ✕ is held, the higher the jump.
//!
//! **Animation.** 7: sequence 7, 9: sequence 8 or 9 by the run's foot (`$gp − 0x7210`[run frame]); blend 5. One tick
//! after the takeoff the playback is timed so the hold frame (18) comes at the landing ETA; descending, the apex
//! frame (7: 30, 9: 29) is timed to the landing; at touchdown the sequence jumps to the landing frame (7: 12, 9: 11)
//! + 2.
//!
//! Not ported: the other jump states (long jump 0xb, double jump 0xe, the gadget / water / crouch jumps), the
//! scripted vertical curves (`0x18a2bd`), the crouch-chain and hover branches, the ledges, the landing effects and
//! camera jolt, the body lean; the double jump's window is read but 0xe is not entered until it is ported.

use super::physics::*;
use super::states::Entry;
use super::Hero;
use rc_game::hero::anim::AnimCtl;
use rc_game::pad::button;

/// `$gp − 0x7210`: the run jump's sequence by the run's frame (the leading foot).
const RUN_JUMP_SEQ: [u8; 24] = [9, 9, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9];
/// `0x1a7db4` (0.82): the target speed above which a re-jump can become the run jump (compared with the u/tick target
/// as the code does, so in practice the re-jump is the jump 7).
const RUN_JUMP_SPEED: f32 = 0.82;

impl Hero {
    pub(super) fn jump_entry(&mut self, anim: &mut dyn AnimCtl, _env: &Env, id: i32, _play: bool) -> Entry {
        self.group = 4;
        self.cam_mode = 0x50;
        self.vel = self.eff;
        let j = &mut self.jump;
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
            ..Default::default()
        };
        let (apex, land, hmax, hmin) = match id {
            7 => { j.g_down = 0.009_652_62; (30.0, 12.0, 2.3, 1.28) }
            _ => (29.0, 11.0, 2.7, 1.0),
        };
        (j.f_apex, j.f_land, j.hmax, j.hmin, j.h) = (apex, land, hmax, hmin, hmin);
        (j.can_double, j.ramp, j.f_hold, j.bottom, j.takeoff, j.f34c, j.f34e) = (1, 0xf, 18.0, 0.6, 5, 1, 1);
        if j.fallover_h == 0.0 { j.fallover_h = j.h * 1.5; }
        let seq = if id == 7 {
            7
        } else {
            let v = anim.view();
            if v.seq_a == 4 { RUN_JUMP_SEQ.get(v.frame as usize).copied().unwrap_or(8) } else { 8 }
        };
        Entry::Anim(5.0, seq, 0)
    }

    pub(super) fn phys_jump(&mut self, env: &Env, _anim: &mut dyn AnimCtl) {
        if self.jump.landed != 0 { self.stick_target(env, 0.099_166_67, 0); }
        self.stick_target(env, self.jump.air_speed, 0);
        if self.timer == self.jump.takeoff { self.jump.takeoff_pos = self.pos; }
        if self.jump.landed == 0 && self.jump.takeoff <= self.timer {
            self.air_control();
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
        if self.jump.takeoff < self.timer && self.vel[2] < 0.001 {
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

    pub(super) fn trans_jump(&mut self, env: &Env, anim: &mut dyn AnimCtl) -> Option<i32> {
        let j = self.jump;
        if j.max_air < self.timer {
            if j.landed == 0 { return Some(6); }
        } else if self.timer <= j.takeoff {
            return None;
        } else if self.timer == j.takeoff + 1 {
            let eta = self.land_eta(j.g);
            let v = anim.view();
            self.anim_speed_for_ticks(&v, j.f_hold, eta, j.f334, -1.0);
            self.anim_speed = self.anim_speed.min(1.5);
            return None;
        }
        let v = anim.view();
        if j.landed == 0 && self.air_ticks == 0 && j.descending != 0 {
            self.jump.landed = 1;
            if self.stick_mag < 0.2 && 0.041_666_668 < self.speed { self.speed = 0.041_666_668; }
            if v.frame < j.f_apex - 1.5 { self.set_anim(anim, 7.0, v.seq_b, j.f_land as i32 + 2); }
        }
        if self.jump.landed == 0 {
            if j.descending != 0 {
                if (v.frame_b as f32) < j.f_land {
                    let eta = self.land_eta(j.g);
                    self.anim_speed_for_ticks(&v, j.f_apex, eta, j.f334, 0.2);
                    if 1.8 < self.anim_speed { self.set_anim(anim, eta + 3.0, v.seq_b, j.f_land as i32 + 2); }
                    self.anim_speed = self.anim_speed.clamp(0.2, 2.7);
                }
                if v.frame.round() == j.f_apex.round() { self.anim_speed = 0.0; }
            } else {
                self.anim_speed = 1.0;
            }
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

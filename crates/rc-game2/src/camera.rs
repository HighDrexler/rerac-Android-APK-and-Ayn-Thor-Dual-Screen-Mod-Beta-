//! **Interim** follow camera for Going Commando, until the game's type-0 camera is ported (docs/plan/rac2_bringup.md
//! §6.4). What is the game's: the target point (L00 `0x427990`: the feet + Ratchet's up axis · 1.6, Clank 0.75, Giant
//! Clank 9.5), the camera records (`0x1b9150`, 48 × 0xa0, the current one `0x1b8d90`), the type table `0x2a3a0c`
//! (type 0: init `0x428518`, update `0x4278c8`), and that the stick turns through the camera's yaw `0x1b8d58` and rows
//! `0x1b8f70`. What is not: the type-0 update itself — `0x421dc8` (the target with the platform under the feet),
//! `0x4239d0`, `0x423150` (the collision), `0x427850` (the seven sub-steps `0x427758` … `0x426a70`) and `0x423768`
//! (the springs back to the follow record's defaults) — which this stands in for with a spring follow behind
//! Ratchet: distance 6, height 2 above the target, yaw following his movement, the right stick turning it, the
//! world line from the target pulling it in.

use crate::hero::physics::{add, len2, scale, sub, unit, V3};
use crate::hero::Hero;
use rc_formats::collision::Collision;
use rc_game::collision_query::{coll_line, QueryFlags};

/// Interim tuning (not the game's).
const DIST: f32 = 6.0;
const HEIGHT: f32 = 2.0;

#[derive(Clone, Debug)]
pub struct FollowCamera {
    /// Eye position and its yaw / pitch (the yaw is `0x1b8d58`'s role: the stick turns through it).
    pub pos: V3,
    pub yaw: f32,
    pub pitch: f32,
    /// Forward, left, up (`0x1b8f70`'s role).
    pub rows: [V3; 3],
    yaw_vel: f32,
    dist: f32,
}

/// `0x427990`: the point the camera looks at.
pub fn target(hero: &Hero) -> V3 { add(hero.pos, scale(hero.rows[2], 1.6)) }

impl FollowCamera {
    /// Snapped behind the hero.
    pub fn behind(hero: &Hero) -> FollowCamera {
        let mut c = FollowCamera { pos: [0.0; 3], yaw: hero.rot[2], pitch: 0.0, rows: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]], yaw_vel: 0.0, dist: DIST };
        c.place(hero, None);
        c
    }

    /// One tick: `rstick` = the right stick (x, y), up / left negative.
    pub fn update(&mut self, hero: &Hero, coll: Option<&Collision>, rstick: [f32; 2]) {
        // The yaw drifts behind the direction Ratchet moves, faster the faster he goes; the right stick turns it.
        let v = [hero.vel[0], hero.vel[1], 0.0];
        if 0.02 < len2(v) && rstick[0].abs() < 0.1 {
            let want = v[1].atan2(v[0]);
            let d = crate::hero::physics::sub_rot(want, self.yaw);
            let k = (len2(v) * 0.6).min(0.04);
            self.yaw = crate::hero::physics::add_rot(self.yaw, d * k);
        }
        let turn = -rstick[0] * 0.05;
        self.yaw_vel += (turn - self.yaw_vel) * 0.3;
        self.yaw = crate::hero::physics::add_rot(self.yaw, self.yaw_vel);
        self.pitch = (self.pitch + rstick[1] * 0.02).clamp(-0.6, 0.9);
        self.place(hero, coll);
    }

    fn place(&mut self, hero: &Hero, coll: Option<&Collision>) {
        let t = target(hero);
        let (s, c) = self.yaw.sin_cos();
        let back = [-c, -s, 0.0];
        let up = HEIGHT + self.pitch * DIST;
        let mut want_dist = DIST;
        let want = add(add(t, scale(back, want_dist)), [0.0, 0.0, up]);
        if let Some(o) = coll.and_then(|m| coll_line(m, t, want, QueryFlags(2))) {
            let d = crate::hero::physics::dist(t, o.point);
            want_dist = (d - 0.3).clamp(0.8, DIST);
        }
        self.dist += (want_dist - self.dist) * if want_dist < self.dist { 0.5 } else { 0.1 };
        let k = self.dist / DIST;
        self.pos = add(add(t, scale(back, self.dist)), [0.0, 0.0, up * k]);
        let fwd = unit(sub(t, self.pos));
        let left = unit([-fwd[1], fwd[0], 0.0]);
        // up = forward × left.
        let upv = [fwd[1] * left[2] - fwd[2] * left[1], fwd[2] * left[0] - fwd[0] * left[2], fwd[0] * left[1] - fwd[1] * left[0]];
        self.rows = [fwd, left, upv];
    }
}

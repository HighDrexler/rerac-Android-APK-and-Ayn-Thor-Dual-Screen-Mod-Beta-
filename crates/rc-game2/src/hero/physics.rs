//! The hero's physics helpers and the move pipeline (L00 addresses; native `f32`, docs/plan/decisions.md
//! "native-first"). Each function is the R&C2 routine named in its doc; where R&C1 has the same routine the
//! R&C1 port (`rc_game::hero::physics`) is the map, never the source.
//!
//! | Routine | L00 | Here |
//! | --- | --- | --- |
//! | Approach / spring velocity / spring / angle difference / turn spring | `0x31eb18` / `0x31e9c8` / `0x31ebd8` / `0x31ef68` / `0x31f0c8` | [`approach`], [`spring_vel`], [`spring`], [`angle_diff`], [`turn_spring`] |
//! | Add / subtract rotations | `0x2efe60` / `0x2efea8` | [`add_rot`], [`sub_rot`] |
//! | Stick → target yaw and speed | `0x2b3af8` | [`Hero::stick_target`] |
//! | Run target speed | `0x2b3e18` | [`Hero::run_target`] |
//! | Turn toward the target yaw | `0x2b4000` | [`Hero::turn_to`] |
//! | Run turn rates | `0x2b4070` | [`Hero::run_turn`] |
//! | Speed approach | `0x2b42c0` | [`Hero::speed_step`] |
//! | Velocity from speed and yaw | `0x2b4310` | [`Hero::set_planar_vel`] |
//! | Wall check | `0x2b4560` | [`Hero::wall_check`] |
//! | Drag | `0x2b5600` | [`Hero::drag`] |
//! | Momentum decay | `0x2b95e0` | [`Hero::momentum_decay`] |
//! | Edge brake | `0x2ba008` | [`Hero::edge_brake`] |
//! | Gravity | `0x2d3860` (mode 0) | [`Hero::gravity`] |
//! | Capsule sizing | `0x2b3400` | [`Hero::size_capsule`] |
//! | Move: integrate, capsule, probe, snap, effective velocity, cap | `0x2b5c68` | [`Hero::move_collide`] |
//! | Capsule resolve / try | `0x2b5b90` / `0x2b56f8` | [`Hero::capsule_resolve`] |
//! | Ground probe | `0x2b4a08` | [`Hero::ground_probe`] |
//! | Step snap | `0x2b5358` (mode 0) | [`Hero::step_snap`] |
//!
//! Not ported here (noted where they are called): the gravity frames 1 and 2 (`0x18c0e3`, R&C2's gravity surfaces),
//! the platform carry (`0x189f20`), the moby collision of the queries (world only), the water surfaces of the probe.

use super::Hero;
use rc_formats::collision::Collision;
use rc_game::collision_query::{coll_capsule, coll_line, coll_sphere_hero_groups, CollOutput, QueryFlags};
use rc_game::pad::PadState;

pub type V3 = [f32; 3];
pub const PI: f32 = std::f32::consts::PI;
/// One tick at 60 Hz (the code writes speeds as `c / 60` and accelerations as `c / 3600`).
pub const DT: f32 = 1.0 / 60.0;

/// What the hero reads outside its block.
pub struct Env<'a> {
    pub coll: &'a Collision,
    pub pad: &'a PadState,
    /// The camera's yaw `0x1b8d58` (the camera update of the previous tick).
    pub cam_yaw: f32,
    /// The camera's matrix `0x1b8f70`: rows forward, left, up (the stick turns through it).
    pub cam_rows: [V3; 3],
    /// Option `0x1a7a4c`: mirrored controls.
    pub mirror: bool,
}

impl Env<'_> {
    /// `0x2de878(a, b, flags, Ratchet, 0)`: the line query (world mesh only here).
    pub fn line(&self, a: V3, b: V3, flags: u32) -> Option<CollOutput> { coll_line(self.coll, a, b, QueryFlags(flags)) }
}

// ------------------------------------------------------------------------------------------------
// Kernels.

/// `0x2efe60`: `a + b` wrapped to [−π, π).
pub fn add_rot(a: f32, b: f32) -> f32 {
    let mut s = a + b;
    let low = s < -PI;
    if PI <= s { s = s - PI - PI; }
    if low { s = s + PI + PI; }
    s
}

/// `0x2efea8`: `a − b` wrapped the same way.
pub fn sub_rot(a: f32, b: f32) -> f32 { add_rot(a, -b) }

/// `0x2eff48`: the unsigned angle between two yaws (≤ π).
pub fn rot_dist(a: f32, b: f32) -> f32 {
    let d = (a - b).abs();
    if PI <= d { 2.0 * PI - d } else { d }
}

/// `0x31eb18(t, step, &x)`: x moves toward t by at most `step`; returns |t − x|.
pub fn approach(t: f32, step: f32, x: &mut f32) -> f32 {
    let d = (t - *x).clamp(-step, step);
    *x += d;
    (t - *x).abs()
}

/// `0x31e9c8(e, k, d, max, &v)`: v clamped to ±|e|, `v += k·e − d·v`, clamped to ±max (max > 0) and to ±|e| again.
pub fn spring_vel(e: f32, k: f32, d: f32, max: f32, v: &mut f32) {
    let ae = e.abs();
    *v = v.clamp(-ae, ae);
    *v += k * e - d * *v;
    if 0.0 < max { *v = v.clamp(-max, max); }
    *v = v.clamp(-ae, ae);
}

/// `0x31ebd8(t, k, d, max, &x, &v)`: [`spring_vel`] on `t − x`, `x += v`; snaps to t when the error is below
/// `0.01·max`. Returns the error.
pub fn spring(t: f32, k: f32, d: f32, max: f32, x: &mut f32, v: &mut f32) -> f32 {
    spring_vel(t - *x, k, d, max, v);
    *x += *v;
    let e = t - *x;
    if e.abs() < max * 0.01 {
        *x = t;
        *v = 0.0;
    }
    e
}

/// `0x31ef68(t, x, mode)`: `sub_rot(t, x)`; with `mode` ±1 a difference of the other sign above 0.1° goes the long
/// way round.
pub fn angle_diff(t: f32, x: f32, mode: i32) -> f32 {
    let d = sub_rot(t, x);
    if mode != 0 && d * mode as f32 <= 0.0 && 0.001_745_33 < d.abs() {
        return if d <= 0.0 { d + 2.0 * PI } else { d - 2.0 * PI };
    }
    d
}

/// `0x31f0c8(t, k, d, max, &a, &v, mode)`: the angle spring. Mode 2 picks ±1 from the signs of t and a. Returns
/// the remaining difference.
pub fn turn_spring(t: f32, k: f32, d: f32, max: f32, a: &mut f32, v: &mut f32, mut mode: i32) -> f32 {
    if mode == 2 {
        mode = if (t <= 0.0 || 0.0 <= *a) && (0.0 <= t || *a <= 0.0) { 0 } else if t < 0.0 { -1 } else { 1 };
    }
    spring_vel(angle_diff(t, *a, mode), k, d, max, v);
    *a = add_rot(*a, *v);
    let e = angle_diff(t, *a, mode);
    if e.abs() < max * 0.01 {
        *a = t;
        *v = 0.0;
        return 0.0;
    }
    e
}

pub fn add(a: V3, b: V3) -> V3 { [a[0] + b[0], a[1] + b[1], a[2] + b[2]] }
pub fn sub(a: V3, b: V3) -> V3 { [a[0] - b[0], a[1] - b[1], a[2] - b[2]] }
pub fn scale(a: V3, s: f32) -> V3 { [a[0] * s, a[1] * s, a[2] * s] }
pub fn dot(a: V3, b: V3) -> f32 { a[0] * b[0] + a[1] * b[1] + a[2] * b[2] }
pub fn len(a: V3) -> f32 { dot(a, a).sqrt() }
/// `0x2eefd0`: the xy length.
pub fn len2(a: V3) -> f32 { (a[0] * a[0] + a[1] * a[1]).sqrt() }
/// `0x2ef138(l, out, v)`: v at length l (zero stays zero).
pub fn set_len(v: V3, l: f32) -> V3 {
    let n = len(v);
    if n <= 0.0 { [0.0; 3] } else { scale(v, l / n) }
}
/// `0x2ef188`: v at xy length l, z kept.
pub fn set_len2(v: V3, l: f32) -> V3 {
    let n = len2(v);
    if n <= 0.0 { [0.0, 0.0, v[2]] } else { [v[0] * l / n, v[1] * l / n, v[2]] }
}
pub fn unit(v: V3) -> V3 { set_len(v, 1.0) }
pub fn dist(a: V3, b: V3) -> f32 { len(sub(a, b)) }
/// `0x2ef050`: xy distance.
pub fn dist2(a: V3, b: V3) -> f32 { len2(sub(a, b)) }
/// `0x2ef510(x, y)`: `atan2(y, x)` (the VU0 polynomial in the game; libm here).
pub fn atan2(x: f32, y: f32) -> f32 { y.atan2(x) }
/// `0x2d4248(n)`: the slope of a raw normal, `atan2(|n.xy|, n.z)` (0 = flat).
pub fn slope_of(n: V3) -> f32 { len2(n).atan2(n[2]) }
/// The rotation rows of Euler angles (R = Rz·Ry·Rx), row i = image of axis i (`0x2ef6d8` + `0x2ef960`).
pub fn euler_rows(rot: V3) -> [V3; 3] {
    let (sx, cx) = rot[0].sin_cos();
    let (sy, cy) = rot[1].sin_cos();
    let (sz, cz) = rot[2].sin_cos();
    [
        [cz * cy, sz * cy, -sy],
        [cz * sy * sx - sz * cx, sz * sy * sx + cz * cx, cy * sx],
        [cz * sy * cx + sz * sx, sz * sy * cx - cz * sx, cy * cx],
    ]
}

// ------------------------------------------------------------------------------------------------
// The helpers the states call.

/// The slope above which the probe does not count the floor as ground (`0x2b4a08`: 50°).
pub const SLOPE_MAX: f32 = 0.872_664_7;

impl Hero {
    /// `0x2b3af8(scale, mode)`: the stick (`0x18bdd0`) as the target yaw `0x189fc0` (camera yaw + stick angle) and the
    /// target speed factor `0x189fd0 = scale·|stick|`; no stick: the yaw itself, factor 0. Mode 1 drops an axis under
    /// 0.12 when the other is over 0.7 (the quick-select / strafe readers). The stick sector and the strafe flag
    /// (`0x18b276/7`, strafing `0x18b274`) are not ported.
    pub fn stick_target(&mut self, env: &Env, scale: f32, mode: i32) {
        let [mut x, mut y] = self.stick;
        if mode == 1 {
            if 0.7 < y.abs() && x.abs() < 0.12 { x = 0.0; }
            if 0.7 < x.abs() && y.abs() < 0.12 { y = 0.0; }
        }
        let l = (x * x + y * y).sqrt();
        if 1.0 < l { (x, y) = (x / l, y / l); }
        if x == 0.0 && y == 0.0 {
            self.target_speed = 0.0;
            self.target_yaw = self.rot[2];
            self.stick_dir = self.rows[0];
            return;
        }
        self.target_yaw = add_rot(atan2(-y, -x), env.cam_yaw);
        let (sx, sy) = (if env.mirror { x } else { -x }, -y);
        // `0x2ef318(out, (−y, −x), camera)`: the stick turned into the world through the camera's rows.
        let r = &env.cam_rows;
        self.stick_dir = [sy * r[0][0] + sx * r[1][0], sy * r[0][1] + sx * r[1][1], sy * r[0][2] + sx * r[1][2]];
        self.target_speed = scale * (x * x + y * y).sqrt();
    }

    /// `0x2b3e18`: [`Hero::stick_target`] (1, 0), then target speed = run speed `0x1a7dc0` (5.95) · stick magnitude
    /// `0x18c2d8` / 60 (the wading walk 0x70: × 0.8, at least 2.5 / 60).
    pub fn run_target(&mut self, env: &Env) {
        self.stick_target(env, 1.0, 0);
        if 0.0 < self.target_speed { self.target_speed = RUN_SPEED * self.stick_mag * DT; }
        if self.state == 0x70 {
            self.target_speed *= 0.8;
            if self.target_speed < 2.5 * DT { self.target_speed = 2.5 * DT; }
        }
    }

    /// `0x2b4000(k, d, max, mode)`: the yaw `0x189eb8` springs toward the target yaw (velocity `0x189fc4`); the
    /// remaining difference goes to `0x189fc8`. (Gravity frames ≠ 0: `0x2b3f48`, not ported.)
    pub fn turn_to(&mut self, k: f32, d: f32, max: f32, mode: i32) {
        let (mut a, mut v) = (self.rot[2], self.yaw_vel);
        self.yaw_residual = turn_spring(self.target_yaw, k, d, max, &mut a, &mut v, mode);
        self.rot[2] = a;
        self.yaw_vel = v;
    }

    /// `0x2b4070`: the run's turn rates from the stick magnitude (k 0.01..0.022, max 0.032..0.0931 rad/tick, d 0.18;
    /// the quick turn `0x18a22f`: 0.035 / 0.1949), then [`Hero::turn_to`]. The weapon-stance rates (held items 0x10,
    /// 0x1c..0x1e, 0x27, 0x2e) are not ported.
    pub fn run_turn(&mut self) {
        let m = self.stick_mag;
        let (mut k, mut max) = if m == 1.0 { (0.022, 0.093_084_25) } else { ((m * 0.02).clamp(0.01, 0.022), (m * 0.063_995_41).clamp(0.031_997_707, 0.093_084_25)) };
        if self.quick_turn != 0 { (k, max) = (0.035, 0.194_895_13); }
        self.turn_to(k, 0.18, max, 0);
    }

    /// `0x2b42c0(acc, dec)`: the speed `0x189fd4` approaches the target `0x189fd0`, by `acc` when below it, else by
    /// `dec`.
    pub fn speed_step(&mut self, acc: f32, dec: f32) {
        let step = if self.speed < self.target_speed { acc } else { dec };
        approach(self.target_speed, step, &mut self.speed);
    }

    /// `0x2b4310(yaw)`: velocity xy = speed along `yaw` (a yaw > π: the hero's own), z = 0 (`0x325100(speed, yaw, 0)`).
    pub fn set_planar_vel(&mut self, yaw: f32) {
        let yaw = if PI < yaw { self.rot[2] } else { yaw };
        let (s, c) = yaw.sin_cos();
        self.vel = [self.speed * c, self.speed * s, 0.0];
    }

    /// `0x2b4560(always_forward)`: two lines (flags 2) from 0.37 and 0.111 up the body, 0.5 / 0.7 ahead along the
    /// facing (or the velocity's xy when it is longer than 0.01 and `always_forward` is 0); a hit steeper than 50°
    /// (or surface 8 / 0xc) takes the velocity's component into the wall away. Group 0x11/0x12: from the feet, 0.7
    /// ahead. Only when the velocity's xy is at least 0.1 / 60.
    pub fn wall_check(&mut self, env: &Env, always_forward: bool) {
        if len2(self.vel) < 0.1 * DT { return; }
        let (h, ahead) = if matches!(self.group, 0x11 | 0x12) { (0.0, 0.7) } else { (0.37, 0.5) };
        let a = add(self.pos, [0.0, 0.0, h]);
        let a2 = add(self.pos, [0.0, 0.0, h * 0.3]);
        let fwd = if always_forward || len2(self.vel) < 0.01 {
            scale(self.rows[0], ahead)
        } else {
            let v = [self.vel[0], self.vel[1], 0.0];
            scale(set_len(v, 1.0), ahead)
        };
        let fwd2 = if always_forward || len2(self.vel) < 0.01 { scale(self.rows[0], ahead * 1.4) } else { scale(fwd, 1.4) };
        let low = env.line(a2, add(a2, fwd2), 2);
        let low_surface = low.map(|o| o.surface_id());
        let high = env.line(a, add(a, fwd), 2);
        let slope = high.map_or(0.0, |o| slope_of(o.normal));
        let hit = if SLOPE_MAX <= slope { high } else if matches!(low_surface, Some(8) | Some(0xc)) { low } else { None };
        if let Some(o) = hit {
            let mut n = o.normal;
            n[2] = 0.0;
            let n = unit(n);
            let f = dot(n, self.vel);
            if f < 0.0 { self.vel = add(self.vel, scale(n, -f)); }
        }
    }

    /// `0x2b5600(k, min)`: the drag displacement `0x18a880` = `0x325100`(drag speed, drag yaw, drag pitch) · k, cut
    /// by `min` (zero below it). The drag source (`0x18a890..98`) is set by sliding floors, not ported: zero.
    pub fn drag(&mut self, k: f32, min: f32) {
        let v = scale(self.drag_src, k);
        let l = len(v);
        self.drag = if l <= min { [0.0; 3] } else { set_len(v, l - min) };
    }

    /// `0x2b95e0(dec)`: the carried momentum `0x189f80` shrinks by `dec` (xy) and is added to the velocity; below
    /// 0.001 it is cleared.
    pub fn momentum_decay(&mut self, dec: f32) {
        let l = len2(self.momentum);
        if 0.001 < l {
            self.momentum = set_len2([self.momentum[0], self.momentum[1], 0.0], (l - dec).max(0.0));
            self.vel = add(self.vel, self.momentum);
        } else {
            self.momentum = [0.0; 3];
        }
    }

    /// `0x2ba008(k, min)`: on the ground with a small stick (≤ 0.5), the velocity·k (at least `min` long) projected
    /// ahead; a line (flags 4) down from 0.3 above that point to 0.35 below it (Clank: 0.7) that finds no floor
    /// flatter than 50° brakes the move (`0x2b6220`: the velocity's xy taken away).
    pub fn edge_brake(&mut self, env: &Env, k: f32, min: f32) {
        let l = len2(self.vel);
        if l == 0.0 || !(self.stick_mag <= 0.5 || self.group == 6) || self.air_ticks != 0 { return; }
        let k = if self.state == 0x1f { 2.0 } else { k };
        let mut d = scale(self.vel, k);
        // `0x2d3f78(0)` then `0x2d3b38(min)`: shorter than `min`, the facing · min instead.
        if 0.0 < min && len2(d) < min { d = self.along_facing([0.0, 0.0, d[2]], min); }
        d[2] = 0.0;
        let p = add(d, self.pos);
        let a = add(p, [0.0, 0.0, 0.3]);
        let b = add(p, [0.0, 0.0, -0.35]);
        let floor = env.line(a, b, 4).is_some_and(|o| slope_of(o.normal) <= SLOPE_MAX);
        if !floor { self.vel = [0.0, 0.0, self.vel[2]]; }
    }

    /// `0x2d3b38(k, out, v)` in gravity mode 0: v + the facing (Ratchet's moby yaw) · k.
    pub fn along_facing(&self, v: V3, k: f32) -> V3 {
        let (s, c) = self.rot[2].sin_cos();
        [v[0] + c * k, v[1] + s * k, v[2]]
    }

    /// `0x2d3860(g, out, in)` in gravity mode 0: `out.z = in.z − g`.
    pub fn gravity(&mut self, g: f32) { self.vel[2] -= g; }

    /// `0x2d3860(g, vel, src)`: `vel.z = src.z − g`.
    pub fn gravity_from(&mut self, src_z: f32, g: f32) { self.vel[2] = src_z - g; }

    /// `0x2b3400`: the capsule targets (top 0.8, bottom 0.7, radius 0.45; fall 6: bottom 0.5; crouch 4: top 0.35;
    /// groups 0x11/0x12: 0) and their ease (top / bottom by 0.02, the radius spring 0.02 / 0.3 / 1/15).
    pub fn size_capsule(&mut self) {
        (self.cap_top_target, self.cap_bottom_target, self.cap_radius_target) = (0.8, 0.7, 0.45);
        if self.group == 4 {
            if self.jump.takeoff < self.timer && self.jump.descending == 0 { self.cap_bottom_target = self.jump.bottom; }
        } else if self.state == 6 {
            self.cap_bottom_target = 0.5;
        } else if self.state == 4 {
            self.cap_top_target = 0.35;
        } else if matches!(self.group, 0x11 | 0x12) {
            self.cap_bottom_target = 0.0;
            self.cap_top_target = 0.0;
        } else if self.state == 0x7c {
            self.cap_radius_target = 0.8;
        }
        let ease = self.cap_hit == 0 || self.prev_group == 0x12 || self.group == 0x11 || self.cap_radius * 0.5 < dist2(self.contact_point, self.pos);
        if ease {
            approach(self.cap_top_target, 0.02, &mut self.cap_top);
            approach(self.cap_bottom_target, 0.02, &mut self.cap_bottom);
            let (mut r, mut v) = (self.cap_radius, self.cap_radius_vel);
            spring(self.cap_radius_target, 0.02, 0.3, 1.0 / 15.0, &mut r, &mut v);
            (self.cap_radius, self.cap_radius_vel) = (r, v);
        }
    }

    /// `0x2b56f8(try)`: up to 8 capsule passes from the feet + bottom offset (flags 0x24: surface 0 passed through),
    /// each moving the position to the kernel's pushed centre; then the hero-only groups. `false` when the total
    /// push exceeds 1.5·r.
    fn capsule_try(&mut self, env: &Env) -> bool {
        let old = self.pos;
        let off = if self.group == 0x11 { 0.0 } else { self.cap_bottom };
        self.pos[2] += off;
        let r = self.cap_radius;
        for _ in 0..8 {
            let h = (self.cap_top - self.cap_bottom).max(0.05);
            let mut o = coll_capsule(env.coll, self.pos, h, r, QueryFlags::exclude_surface(0) | QueryFlags::MOBY_SUBMASK);
            if let Some(g) = coll_sphere_hero_groups(env.coll, self.pos, r) { o = Some(g); }
            let Some(o) = o else { break };
            if let Some(pc) = o.pushed_centre { self.pos = pc; }
            self.contact_normal = o.normal;
            self.contact_point = o.point;
            self.cap_hit = 1;
        }
        self.pos[2] -= off;
        len(sub(self.pos, old)) <= r * 1.5
    }

    /// `0x2b5b90`: [`Hero::capsule_try`], rolling back through the position history (16 tries) when it is rejected.
    pub fn capsule_resolve(&mut self, env: &Env) {
        for _ in 0..16 {
            if self.capsule_try(env) { return; }
            if 2 <= self.pos_hist_count {
                self.pos_hist_count -= 1;
                self.pos_hist_idx = (self.pos_hist_idx + 31) & 31;
            }
            self.pos = self.pos_hist[self.pos_hist_idx];
        }
    }

    /// `0x2b4a08`: the ground probe. A line (flags 2) from 2.2 (walk group 2: 2.9) above the feet to 48 below finds
    /// the ground point `0x18a0f0`, its z `0x18a138`, the height above it `0x18a13c` (32 without; negative when the
    /// ground is above), the normal `0x18a0c0` and slope `0x18a140`; under 0.02 with a slope ≤ 50° the air ticks
    /// `0x18a16e` reset. Water and liquid surfaces (0, 0xd, 3, 0xb), the moby under the feet and the side probes for
    /// the model's tilt are not ported.
    pub fn ground_probe(&mut self, env: &Env) {
        self.air_ticks = self.air_ticks.saturating_add(1);
        self.f16c = self.f16c.saturating_add(1);
        self.slope = 0.0;
        self.ground_z = 0.0;
        self.ground_point = [0.0; 3];
        self.height = 32.0;
        self.ground_normal = [0.0, 0.0, 1.0];
        let up = if self.group == 2 { 2.9 } else { 2.2 };
        let a = add(self.pos, [0.0, 0.0, up]);
        let b = add(self.pos, [0.0, 0.0, -48.0]);
        if let Some(o) = env.line(a, b, 2) {
            self.surface = o.surface_id();
            if 0 <= o.kind {
                self.ground_point = o.point;
                self.ground_z = o.point[2];
                self.height = dist(self.pos, self.ground_point);
                self.ground_normal = unit(o.normal);
                self.slope = slope_of(o.normal);
                if 0.0 < dot(sub(self.ground_point, self.pos), self.ground_normal) { self.height = -self.height; }
                if self.height < 0.02 {
                    self.f16c = 0;
                    if self.slope <= SLOPE_MAX { self.air_ticks = 0; }
                }
            }
        }
        if self.air_ticks == 0 {
            self.grounded_ticks += 1;
            self.last_ground_point = self.ground_point;
        } else {
            self.grounded_ticks = 0;
        }
    }

    /// `0x2b5358` (gravity mode 0): grounded and below the ground z: snap to it when within |vel.z| + 0.01 (and the
    /// capsule is clear of the contact), else undo a downward step and spring up (0.05, 0.17, 1/12).
    pub fn step_snap(&mut self) {
        if self.air_ticks != 0 || self.ground_z <= self.pos[2] {
            self.snap_vel = 0.0;
            return;
        }
        let depth = (self.pos[2] - self.ground_z).abs();
        if depth < self.vel[2].abs() + 0.01 {
            if self.cap_hit == 0 || self.cap_radius * 0.5 < dist2(self.contact_point, self.pos) {
                self.pos[2] = self.ground_z;
                return;
            }
        } else if self.eff_v[2] < 0.0 {
            self.pos[2] -= self.eff_v[2];
            if self.ground_z < self.pos[2] { self.pos[2] = self.ground_z; }
        }
        let (mut z, mut v) = (self.pos[2], self.snap_vel);
        spring(self.ground_z, 0.05, 0.17, 1.0 / 12.0, &mut z, &mut v);
        self.pos[2] = z;
        self.snap_vel = v;
    }

    /// `eff = vel·max(0, unit(disp)·vel)/|vel|`: the velocity cut to what the displacement allowed.
    fn project(disp: V3, vel: V3) -> V3 {
        let f = dot(unit(disp), vel).max(0.0);
        set_len(vel, f)
    }

    /// `0x2b5c68`: clamp the velocity to r − 0.02 (3-D; the jump-attack / slide states 0x14, 0x5b, 0x58, 0x81: xy and
    /// not below the ground; group 0xf: xy), integrate velocity and drag, the capsule, the ground probe, the snap; the
    /// effective velocity `0x189f40` (vertical `0x189f50`, horizontal `0x189f60`), its lengths `0x189f90/94`, the
    /// forward speed `0x189f98`, the slope ratio `0x189f9c` (±0.5) and the displacement `0x189f30`, capped at 52 / 60.
    /// The platform step (`0x189f20`) is not ported.
    pub fn move_collide(&mut self, env: &Env) {
        let old = self.pos;
        let lim = self.cap_radius - 0.02;
        if matches!(self.state, 0x14 | 0x5b | 0x58 | 0x81) {
            if lim < len2(self.vel) { self.vel = set_len2(self.vel, lim); }
            if self.vel[2] < -self.height { self.vel[2] = (-self.height - 0.02).min(0.0); }
        } else if self.group == 0xf {
            if lim < len2(self.vel) { self.vel = set_len2(self.vel, lim); }
        } else if !matches!(self.group, 0xd | 0xe) && self.state != 0x44 && lim < len(self.vel) {
            self.vel = set_len(self.vel, lim);
        }
        self.pos = add(add(self.pos, self.vel), self.drag);
        self.drag = [0.0; 3];
        self.cap_hit = 0;
        self.capsule_resolve(env);
        self.ground_probe(env);
        self.disp = sub(self.pos, old);
        self.step_snap();
        let d = sub(self.pos, old);
        self.eff = Self::project(d, self.vel);
        self.eff_v = Self::project([0.0, 0.0, d[2]], [0.0, 0.0, self.vel[2]]);
        self.eff_h = Self::project([d[0], d[1], 0.0], [self.vel[0], self.vel[1], 0.0]);
        self.eff_len = len(self.eff);
        self.eff_len_xy = len2(self.eff);
        let (s, c) = self.rot[2].sin_cos();
        self.fwd_speed = (self.eff[0] * c + self.eff[1] * s).max(0.0);
        self.disp = sub(self.pos, old);
        if 0.004 < self.eff_len_xy { self.slope_ratio = (self.disp[2] / self.eff_len_xy).clamp(-0.5, 0.5); }
        let cap = 52.0 * DT;
        if cap < self.eff_len {
            self.disp = scale(self.disp, cap / self.eff_len);
            self.eff_len = cap;
        }
    }
}

/// `0x1a7dc0`: the run speed, units per second (L00 .lit).
pub const RUN_SPEED: f32 = 5.95;

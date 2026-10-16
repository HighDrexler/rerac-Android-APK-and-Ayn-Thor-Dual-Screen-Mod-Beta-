//! Going Commando's follow camera, type 0 (L00): the camera update `0x2d9f38` → the current camera's type update
//! (table `0x2a3a0c`, 0x14 bytes a type: update, …, init) → type 0's update `0x4278c8`, init `0x428518` /
//! `0x424908`. Native `f32` (docs/plan/decisions.md "native-first"). R&C2's type-0 camera is R&C1's evolved — the
//! same steps in the same order with R&C2's constants and some additions (docs/plan/rac2_bringup.md §6.5) — so this
//! follows the R&C1 port's organisation (`rc_game::follow_camera`), each step read from R&C2's code.
//!
//! | Step | L00 | Here |
//! | --- | --- | --- |
//! | collision flags | `0x2d9ba8` | [`FollowCamera::pre_flags`] |
//! | up vectors, hero motion, smoothed hero z, yaw history | `0x2d9760` | [`FollowCamera::pre_motion`] |
//! | update: platform carry, state tweaks, targets … rows, spring back | `0x4278c8` | [`FollowCamera::update`] |
//! | the point the camera looks at (feet + up · 1.6) | `0x427990` | [`target_point`] |
//! | platform carry | `0x421dc8` | [`FollowCamera::platform_carry`] |
//! | hero-state tweaks | `0x4239d0` | [`FollowCamera::state_tweaks`] |
//! | targets, look, pivot, ceiling lines | `0x423150` | [`FollowCamera::targets`] |
//! | target modes | `0x422138` | [`FollowCamera::target_mode`] |
//! | vertical target | `0x4227b8` (`0x4219d0` ground target, `0x421c88` view angle) | [`FollowCamera::vertical_target`] |
//! | look smoothing | `0x423050` | in [`FollowCamera::targets`] |
//! | leash | `0x4269a0` | [`FollowCamera::leash`] |
//! | stick read / apply | `0x4264a8` / `0x426628` | [`FollowCamera::stick`] |
//! | avoidance: sphere chain `0x4255e8` (re-aim `0x4251b8`), end sphere `0x424eb8`, the reduction, the line | `0x4258d0` | [`FollowCamera::avoidance`] |
//! | placement (distance rate `0x426fa8`) | `0x4271e0` | [`FollowCamera::placement`] |
//! | rows | `0x426a70` | [`FollowCamera::rows`] |
//! | spring back to the defaults | `0x423768` | [`FollowCamera::spring_back`] |
//! | init: defaults, target reset `0x421b38`, snap `0x424548` | `0x424908` | [`FollowCamera::new`] |
//!
//! **Not ported** (no use on Aranos's opening yet, or no data source): the level's camera-tweak requests `0x2d7568`
//! (32 slots set by regions and mobys; none active), the camera-collision grid (`0x427758`, `0x4219a0`; empty), the
//! moby collision of the camera's queries (world only), the moby nudges of the avoidance (classes 0x8ba, 0xd07,
//! 0x10d0, 0x477, 0xb), the focus moby `0x1b8e84` / `0x18b270`, the stick auto-centre `$gp+0x2540`, the body cameras
//! (Clank, Giant Clank), the water settle (D+0x14e), the knock-down view (state 0x74, mode 11), the other camera types
//! (first person `0x427bf8`, …), the blends between cameras and the shakes.

use crate::hero::physics::{add, dot, len, scale, sub, V3};
use crate::hero::Hero;
use rc_formats::collision::Collision;
use rc_game::collision_query::{coll_line, coll_sphere, QueryFlags};

const HALF_PI: f32 = std::f32::consts::FRAC_PI_2;
/// `$gp+0x2474` (40°) and `+0x2470` (1.75°): the stick pitch's range and step.
const DEG40: f32 = 0.698_131_7;
const DEG1_75: f32 = 0.030_543_262;
/// The pitch limit of the rows (70°).
const DEG70: f32 = 1.221_730_6;
/// `$gp+0x256c` + 4·(option `0x1a7bb4` − 1): the yaw rate (rad/tick) the stick turns at; option 1 (1°) is the default.
const YAW_RATES: [f32; 3] = [0.017_453_294, 0.022_689_28, 0.038_397_24];

fn lerp(a: V3, b: V3, t: f32) -> V3 { add(a, scale(sub(b, a), t)) }
fn norm(v: V3, l: f32) -> V3 {
    let n = len(v);
    if n == 0.0 { [0.0; 3] } else { scale(v, l / n) }
}
/// `0x2eef88(out, a, b)` = **b × a** (R&C1's `FastVecCross` order).
fn cross(a: V3, b: V3) -> V3 { [b[1] * a[2] - a[1] * b[2], b[2] * a[0] - a[2] * b[0], b[0] * a[1] - a[0] * b[1]] }
fn asin(x: f32) -> f32 { x.clamp(-1.0, 1.0).asin() }
fn add_rot(a: f32, b: f32) -> f32 { crate::hero::physics::add_rot(a, b) }
fn sub_rot(a: f32, b: f32) -> f32 { crate::hero::physics::sub_rot(a, b) }

/// `0x2d7e98(cur, tgt, k, d, max, &v)`: `v += k·(tgt − cur) − d·v`, clamped to ±max (max ≠ 0) and ±|tgt − cur|;
/// returns `cur + v`.
fn interp(cur: f32, tgt: f32, k: f32, d: f32, max: f32, v: &mut f32) -> f32 {
    let e = tgt - cur;
    *v += k * e - d * *v;
    if max != 0.0 { *v = v.clamp(-max, max); }
    *v = v.clamp(-e.abs(), e.abs());
    cur + *v
}

/// [`interp`] per component.
fn interp3(cur: V3, tgt: V3, k: f32, d: f32, v: &mut V3) -> V3 {
    std::array::from_fn(|c| interp(cur[c], tgt[c], k, d, 0.0, &mut v[c]))
}

/// `0x2d7f80`: [`interp`] on wrapped angles.
fn ang_interp(cur: f32, tgt: f32, k: f32, d: f32, max: f32, v: &mut f32) -> f32 {
    let e = sub_rot(tgt, cur);
    *v += k * e - d * *v;
    if max != 0.0 { *v = v.clamp(-max, max); }
    *v = v.clamp(-e.abs(), e.abs());
    add_rot(cur, *v)
}

/// `0x321c00(θ, out, v, axis)`: v rotated by θ about `axis` (counter-clockwise; below 1e-5 unchanged).
fn rot(v: V3, theta: f32, axis: V3) -> V3 {
    if theta.abs() < 1e-5 { return v; }
    let k = norm(axis, 1.0);
    let (s, c) = theta.sin_cos();
    let kxv = [k[1] * v[2] - k[2] * v[1], k[2] * v[0] - k[0] * v[2], k[0] * v[1] - k[1] * v[0]];
    add(add(scale(v, c), scale(kxv, s)), scale(k, dot(k, v) * (1.0 - c)))
}

/// `0x31bdc0(a, b, t)`: the cosine blend.
fn cos_interp(a: f32, b: f32, t: f32) -> f32 {
    if t == 0.0 { return a; }
    if t == 1.0 { return b; }
    a + (b - a) * (1.0 - (t * std::f32::consts::PI).cos()) * 0.5
}

/// `0x2eeae0` (s16): 1 when already 0, 2 when it just reached 0, else 0.
fn dec_timer(t: &mut i16) -> i32 {
    if *t == 0 { return 1; }
    *t = (*t).max(1) - 1;
    if *t > 0 { 0 } else { 2 }
}

/// `0x427990` / `0x427a70`: the point the camera looks at, the feet + Ratchet's up axis · 1.6 (Clank 0.75, Giant Clank
/// 9.5).
pub fn target_point(hero: &Hero) -> V3 { add(hero.pos, scale(hero.rows[2], 1.6)) }

/// What the camera reads each tick besides the hero.
pub struct CamInput<'a> {
    pub hero: &'a Hero,
    pub coll: Option<&'a Collision>,
    /// The right stick (`0x138280` / `0x138284`), up / left negative.
    pub rstick: [f32; 2],
    /// Options `0x1a7bb0` (yaw normal) and `0x1a7bac` (pitch normal).
    pub yaw_normal: bool,
    pub pitch_normal: bool,
}

impl CamInput<'_> {
    /// `0x2de878(a, b, flags, Ratchet, 0)`: the hit point.
    fn line(&self, a: V3, b: V3, flags: u32) -> Option<V3> { self.coll.and_then(|c| coll_line(c, a, b, QueryFlags(flags))).map(|o| o.point) }
    /// `0x2df8d8(r, c, flags, …)`: the closest point and the pushed-out centre.
    fn sphere(&self, r: f32, c: V3, flags: u32) -> Option<(V3, V3)> {
        self.coll.and_then(|m| coll_sphere(m, c, r, QueryFlags(flags))).map(|o| (o.point, o.pushed_centre.unwrap_or(c)))
    }
}

/// The camera record (`0x1b9150 + 0xa0·i`: rows +0x00, position +0x30, saved forward +0x40) and its follow record
/// (+0x70 → `0x1bb130`), with the camera globals (`0x1b8dc0..0x1b8e60`). Field names are R&C1's port's; each carries
/// the R&C2 offset in the follow record (D+) or the global's address.
#[derive(Clone, Debug, Default)]
pub struct FollowCamera {
    /// Record +0x00..+0x20 (forward, left, up), +0x30 position; D+0x40 the saved forward of the row blend.
    pub rows: [V3; 3],
    pub pos: V3,
    saved_fwd: V3,
    /// D+0x00: the desired position (pivot + offset).
    desired: V3,
    /// D+0x10 (u16): the leash on; D+0x20 / +0x44: the row blend's timer / 1 over its length.
    leash: i16,
    row_blend: i16,
    row_blend_inv: f32,
    /// D+0x22: the pitch went negative this tick.
    pitch_neg: u8,
    /// D+0x24 / +0x2c, D+0x28 / +0x30: the look and pivot heights as the rows smooth them.
    look_h_s: f32,
    look_h_v: f32,
    pivot_h_s: f32,
    pivot_h_v: f32,
    /// D+0x34 / +0x38 / +0x3c / +0x40: the pitch bias, its velocity and spring.
    bias: f32,
    bias_v: f32,
    bias_k: f32,
    bias_d: f32,
    /// D+0x60 / +0x70: the target and the vertical target; +0x80 / +0x90 the raise's ends; +0xa0 the look point;
    /// +0xb0 the pivot; +0xc0 the smoothed target; +0xd0 / +0xe0 the springs' velocities; +0xf0 / +0x100 the smoothed
    /// look and its velocities.
    target: V3,
    vtarget: V3,
    raise_a: V3,
    raise_b: V3,
    look: V3,
    pivot: V3,
    smooth: V3,
    vel_h: V3,
    vel_v: V3,
    look_s: V3,
    look_s_vel: V3,
    /// D+0x110 look height (+0x114 target, +0x118 rate, +0x11c velocity, +0x13a override).
    look_h: f32,
    look_h_tgt: f32,
    look_h_rate: f32,
    look_h_vel: f32,
    look_h_ovr: u8,
    /// D+0x124 target mode, +0x125 previous, +0x126 / +0x128 its blend timer and 1 / length; the raise: +0x134 timer,
    /// +0x12c 1 / length, +0x137 frozen.
    mode: u8,
    mode_prev: u8,
    mode_t: i16,
    mode_inv: f32,
    raise_t: i16,
    raise_inv: f32,
    raise_frozen: u8,
    /// D+0x136 look from the smoothed target; +0x138 / +0x14c the blend's timer / length (90).
    look_from_s: u8,
    look_blend: i16,
    look_blend_len: i16,
    /// D+0x13c / +0x140 horizontal spring, +0x144 / +0x148 vertical spring.
    kh: f32,
    dh: f32,
    kv: f32,
    dv: f32,
    /// D+0x150: the vertical gap of the smoothed target; +0x154 the grounded countdown of the air spring.
    vgap: f32,
    vgap_t: i32,
    /// D+0x160 the offset from the pivot, +0x170 the placed offset; +0x180 / +0x184 / +0x188 the elevation, yaw and
    /// distance velocities of the placement.
    off: V3,
    placed: V3,
    el_v: f32,
    yaw_v: f32,
    dist_v: f32,
    /// D+0x18c distance, +0x190 pivot height; +0x194 / +0x198 this tick's yaw / pitch step; the overrides: distance
    /// +0x19c / +0x1a0 / +0x1a4 / +0x1a8, pivot height +0x19e / +0x1ac / +0x1b0 / +0x1b4; +0x1b8 the base distance.
    dist: f32,
    pivot_h: f32,
    yaw_step: f32,
    pitch_step: f32,
    dist_ovr: u8,
    dist_tgt: f32,
    dist_ovr_v: f32,
    dist_rate: f32,
    ph_ovr: u8,
    ph_tgt: f32,
    ph_v: f32,
    ph_rate: f32,
    base_dist: f32,
    /// D+0x1c0 / +0x1c4 the yaw input and its velocity, +0x1c8 / +0x1cc the pitch input; +0x1d0 the script bits,
    /// +0x1d4 the yaw rate, +0x1dc the scripted yaw, +0x1e0 the platform yaw, +0x1e4 the scripted pitch.
    yaw_in: f32,
    yaw_in_v: f32,
    pitch_n: f32,
    pitch_n_v: f32,
    script: u32,
    yaw_rate: f32,
    script_yaw: f32,
    plat_yaw: f32,
    script_pitch: f32,
    /// D+0x220 the distance reduction, +0x224 its restore timer (2000), +0x22a fast restore; +0x228 the end sphere's
    /// class; +0x22c / +0x230 / +0x234 the sphere chain's scale, count and step; +0x238 the end sphere's flags;
    /// +0x23c the blocked ticks; +0x23e no chain.
    red: f32,
    rec_t: i16,
    fast_rec: u8,
    end_class: i16,
    sph_scale: f32,
    sph_count: f32,
    sph_step: f32,
    end_flags: u32,
    blocked: i16,
    no_chain: u8,
    /// D+0x244 / +0x246 / +0x248 / +0x24c: running toward the camera (timer, lock, pull, pull limit 6).
    toward_t: i16,
    toward_lock: u8,
    pull: f32,
    pull_max: f32,
    /// `0x1a8900`: the leash's blend (0.85).
    leash_k: f32,
    /// The globals: `0x1b8dc0` smoothed up, `0x1b8dd0` up (−gravity), `0x1b8de0` its previous, `0x1b8df0` the
    /// spring's velocity; `0x1b8e00` the hero last tick, `0x1b8e20` his motion's horizontal direction, `0x1b8e44` its
    /// length; `0x1b8da0` the smoothed hero (z springs, `0x1b8db0`); `0x1b8e4c` the yaw history.
    up_s: V3,
    up2: V3,
    up2_prev: V3,
    up_vel: V3,
    prev_hero: V3,
    move_dir: V3,
    h_speed: f32,
    hero_s: V3,
    hero_s_v: f32,
    yaw_hist: [f32; 5],
    /// `0x1b17cc` / `0x1b17d0`: the line and sphere chain flags.
    line_flags: u32,
    sph_flags: u32,
    /// The defaults the spring back returns to (`0x1bafc0`: the record as the reset left it, `0x2d7d80`).
    d0_dist: f32,
    d0_pivot_h: f32,
    d0_look_h: f32,
}

impl FollowCamera {
    /// The camera's reset (`0x2da160`): the follow camera current, the motion pre-step, the init `0x424908`, the
    /// defaults kept (`0x2d7d80`).
    pub fn new(inp: &CamInput) -> FollowCamera {
        let h = inp.hero;
        let up = norm(h.rows[2], 1.0);
        let mut c = FollowCamera {
            up_s: up, up2: up, up2_prev: up, prev_hero: h.pos, hero_s: h.pos, yaw_hist: [h.rot[2]; 5],
            line_flags: 0xb4, sph_flags: 0x94, end_flags: 0x14, ..Default::default()
        };
        c.pre_motion(inp);
        c.init(inp);
        (c.d0_dist, c.d0_pivot_h, c.d0_look_h) = (c.dist, c.pivot_h, c.look_h);
        c
    }

    /// `0x424908` (not switched in from another camera's pose).
    fn init(&mut self, inp: &CamInput) {
        let h = inp.hero;
        self.target = h.pos;
        self.look_h = 1.5;
        (self.mode, self.look_from_s, self.look_blend_len) = (0, 0, 90);
        (self.kh, self.kv, self.dh, self.dv) = (0.015, 0.0125, 0.2, 0.2);
        self.look_h_rate = 0.005;
        (self.mode_t, self.look_blend, self.look_h_ovr, self.look_h_tgt, self.look_h_vel) = (0, 0, 0, 0.0, 0.0);
        (self.vgap, self.vgap_t) = (0.0, 0);
        (self.pivot_h, self.dist, self.base_dist) = (2.0, 4.64, 4.64);
        (self.dist_rate, self.ph_rate) = (0.003, 0.003);
        (self.yaw_step, self.pitch_step, self.dist_v, self.yaw_v, self.el_v) = (0.0, 0.0, 0.0, 0.0, 0.0);
        (self.dist_ovr, self.ph_ovr, self.dist_ovr_v, self.ph_v) = (0, 0, 0.0, 0.0);
        self.leash = 1;
        self.leash_k = 0.85;
        self.row_blend = 0;
        (self.bias_k, self.bias_d, self.bias, self.bias_v) = (0.005, 0.2, 0.0, 0.0);
        (self.pivot_h_v, self.look_h_v, self.pitch_neg) = (0.0, 0.0, 0);
        (self.yaw_in, self.yaw_in_v, self.pitch_n, self.pitch_n_v) = (0.0, 0.0, 0.0, 0.0);
        self.yaw_rate = YAW_RATES[0];
        (self.script, self.script_yaw, self.plat_yaw, self.script_pitch) = (0, 0.0, 0.0, 0.0);
        (self.toward_t, self.toward_lock, self.pull, self.pull_max) = (0, 0, self.dist, 6.0);
        (self.red, self.fast_rec, self.end_class, self.blocked, self.no_chain) = (0.0, 0, 0, 0, 0);
        self.rec_t = 2000;
        (self.sph_scale, self.sph_count, self.sph_step) = (1.0, 8.0, 0.1);
        self.target_reset(inp);
        self.snap(inp);
        self.off = sub(self.pos, self.pivot);
        self.placed = self.off;
        self.desired = self.pos;
        let hz = [self.off[0], self.off[1], 0.0];
        self.red = self.dist - len(hz);
        if self.red < -0.1 { self.fast_rec = 1; }
        self.pivot_h_s = self.pivot_h;
        self.look_h_s = self.look_h;
    }

    /// `0x421b38`: the target from the ground target, the look and the pivot above it, the springs at rest.
    fn target_reset(&mut self, inp: &CamInput) {
        let (tg, _) = self.ground_target(inp, false);
        let v = scale(self.up2, dot(tg, self.up2));
        self.vtarget = v;
        self.target = add(sub(tg, v), v);
        self.look = add(scale(self.up_s, self.look_h), self.target);
        self.look_s = self.look;
        self.look_s_vel = [0.0; 3];
        (self.mode, self.mode_inv, self.mode_prev, self.mode_t) = (0, 1.0, 0, 0);
        self.smooth = self.target;
        self.vel_h = [0.0; 3];
        self.vel_v = [0.0; 3];
        self.pivot = add(scale(self.up_s, self.pivot_h), self.target);
    }

    /// `0x424548(1)`: behind Ratchet at the distance and pivot height (his rows), pulled in 0.5 short of a wall on the
    /// line from the look point; the rows look at the look point.
    fn snap(&mut self, inp: &CamInput) {
        let r = inp.hero.rows;
        let (b, u) = (-self.dist, self.pivot_h);
        let mut pos = add(self.target, add(scale(r[0], b), scale(r[2], u)));
        let look = add(self.target, scale(r[2], self.look_h));
        if let Some(hit) = inp.line(look, pos, self.line_flags) {
            let w = sub(hit, look);
            let l = len(w);
            pos = if l == 0.0 { look } else { add(look, scale(w, (l - 0.5) / l)) };
        }
        self.pos = pos;
        let look = add(scale(self.up2, self.look_h), self.target);
        let fwd = norm(sub(look, pos), 1.0);
        let left = norm(cross(fwd, self.up2), 1.0);
        let up = cross(left, fwd);
        self.rows = [fwd, left, up];
        self.saved_fwd = fwd;
    }

    /// `0x4219d0(flag)`: the hero, or (in the air, `0x18a16c` ≠ 0) the ground under him — straight down by the height
    /// when `flag` and he is under 30 high, else a line (flags 0x12) from 0.2 up to 30 down (none: the hero, −1).
    fn ground_target(&self, inp: &CamInput, flag: bool) -> (V3, i32) {
        let h = inp.hero;
        if h.f16c == 0 { return (h.pos, 1); }
        if h.height < 30.0 && flag { return (add(h.pos, scale(self.up2, -h.height)), 1); }
        let a = add(h.pos, scale(self.up2, 0.2));
        let b = add(h.pos, scale(self.up2, -30.0));
        match inp.line(a, b, 0x12) {
            Some(p) => (p, 0),
            None => (h.pos, -1),
        }
    }

    /// One camera update (`0x2d9f38` with the follow camera current): the flags, the motion pre-step, the type-0
    /// update `0x4278c8`.
    pub fn update(&mut self, inp: &CamInput) {
        self.pre_flags(inp);
        self.pre_motion(inp);
        self.platform_carry(inp);
        self.state_tweaks(inp);
        self.targets(inp);
        self.leash();
        self.stick(inp);
        if self.avoidance(inp) { return; }
        self.placement(inp);
        self.rows(inp);
        self.spring_back();
    }

    /// The camera's yaw (`0x1b8d58`): the forward row's.
    pub fn yaw(&self) -> f32 { self.rows[0][1].atan2(self.rows[0][0]) }
    /// The pitch from the forward row.
    pub fn pitch(&self) -> f32 { (-self.rows[0][2]).atan2((self.rows[0][0] * self.rows[0][0] + self.rows[0][1] * self.rows[0][1]).sqrt()) }

    /// `0x2d9ba8`: the end sphere's flags 0x14 (0x34 in groups 0x11 / 0x12 or the wading walk 0x70), the sphere chain's
    /// with 0x80, the line's 0xb4 (the water states' surface bits are not ported).
    fn pre_flags(&mut self, inp: &CamInput) {
        let h = inp.hero;
        let f = if matches!(h.group, 0x11 | 0x12) || h.state == 0x70 { 0x34 } else { 0x14 };
        self.end_flags = f;
        self.sph_flags = f | 0x80;
        self.line_flags = 0xb4;
    }

    /// `0x2d9760`: up = −gravity, its smoothed copy (0.015, 0.2; nudged off a flip below −0.98), the hero's horizontal
    /// motion (direction and length), the smoothed hero (z springs 0.0075 / 0.175 unless in a jump, `0x18c2c0` =
    /// 0x50), the yaw history.
    fn pre_motion(&mut self, inp: &CamInput) {
        let h = inp.hero;
        let mut u = [0.0, 0.0, 1.0];
        self.up2_prev = self.up2;
        self.up2 = u;
        if dot(self.up_s, u) < -0.98 { u = add(u, [0.2; 3]); }
        self.up_s = interp3(self.up_s, u, 0.015, 0.2, &mut self.up_vel);
        self.up_s = norm(self.up_s, 1.0);
        let delta = sub(h.pos, self.prev_hero);
        let hm = sub(delta, scale(self.up2, dot(delta, self.up2)));
        self.h_speed = len(hm);
        self.move_dir = if self.h_speed == 0.0 { [0.0; 3] } else { scale(hm, 1.0 / self.h_speed) };
        self.prev_hero = h.pos;
        self.hero_s[0] = h.pos[0];
        self.hero_s[1] = h.pos[1];
        if !(h.cam_mode == 0x50 && h.state != 0x11) {
            self.hero_s[2] = interp(self.hero_s[2], h.pos[2], 0.0075, 0.175, 0.0, &mut self.hero_s_v);
        }
        self.yaw_hist.rotate_left(1);
        self.yaw_hist[4] = h.rot[2];
    }

    /// `0x421dc8`: the platform displacement under the hero moves the camera's points with him (none: the hero has no
    /// platforms yet); the rigid follow of the slide states 0xb / 0xc.
    fn platform_carry(&mut self, inp: &CamInput) {
        let h = inp.hero;
        if h.state == 0xb || h.state == 0xc {
            self.leash = 0;
            let s = scale(self.move_dir, self.h_speed);
            for p in [&mut self.pos, &mut self.smooth, &mut self.target, &mut self.look_s, &mut self.desired] { *p = add(*p, s); }
        }
    }

    /// `0x4239d0`, the branches for Ratchet on foot: the vertical spring follows the vertical gap (D+0x150, at most 2:
    /// `kv = (0.0125 − 0.002)·gap/2 + 0.002`) in the air and for 15 ticks after landing; the knock-down states 0x78 /
    /// 0x7c drop the sphere chain. The other branches (groups 0x1e, 0x11, 5, 0x10, the bodies, the focus mobys, the
    /// auto-centre, the camera-tweak requests) are not ported.
    fn state_tweaks(&mut self, inp: &CamInput) {
        let h = inp.hero;
        if h.state == 0x7c || h.state == 0x78 { self.no_chain = 1; }
        if h.grounded_ticks == 0 || self.vgap_t != 0 {
            let g = self.vgap.min(2.0);
            self.kv = (0.0125 - 0.002) * g * 0.5 + 0.002;
            if h.grounded_ticks == 0 { self.vgap_t = 0xf; } else { self.vgap_t -= 1; }
        }
    }

    /// `0x422138`: the target mode (D+0x124). From mode 0: group 2 / 0x1e → 1 (10 ticks); state 0xf → 3 (the raise 3.0
    /// over 85, look 30); 0xc → 8 (30); 0xb → 6 (the raise 1.0 over 35, 30); 0xd / 0xe → 5 (the raise 3.5 over 45, 30);
    /// group 4 → 2 (30). From the others as R&C1's `0x30fb08`, with group 0x1e beside 2 and 4.
    fn target_mode(&mut self, inp: &CamInput) {
        let h = inp.hero;
        let set = |c: &mut FollowCamera, prev: u8, m: u8, k: i16| {
            c.mode_prev = prev;
            c.mode = m;
            c.mode_t = k;
            c.mode_inv = 1.0 / k as f32;
        };
        let raise = |c: &mut FollowCamera, ticks: i16, ht: f32| {
            c.raise_t = ticks;
            c.raise_inv = 1.0 / ticks as f32;
            c.raise_frozen = 0;
            let base = c.ground_target(inp, true).0;
            c.raise_b = scale(c.up2, dot(c.up2, add(scale(c.up_s, ht), base)));
            c.raise_a = c.vtarget;
        };
        let m = self.mode;
        if m == 0 {
            if h.group == 2 || h.group == 0x1e {
                set(self, 0, 1, 10);
            } else if h.state == 0xf {
                set(self, 0, 3, 30);
                raise(self, 85, 3.0);
            } else if h.state == 0xc {
                set(self, 0, 8, 30);
            } else if h.state == 0xb {
                set(self, 0, 6, 30);
                raise(self, 35, 1.0);
            } else if h.state == 0xd || h.state == 0xe {
                set(self, 0, 5, 30);
                raise(self, 45, 3.5);
            } else if h.group == 4 {
                set(self, 0, 2, 30);
            }
            return;
        }
        if m == 2 && (h.state == 0x81 || h.state == 0x5b) { return set(self, 2, 1, 10); }
        if m != 9 && m != 1 && h.state == 0x14 {
            self.mode_prev = m;
            self.mode = 9;
            self.mode_t = 0;
            self.mode_inv = 1.0;
            raise(self, 30, 1.5);
            return;
        }
        if (m == 1 || m == 2) && !matches!(h.group, 4 | 2 | 0x1e) && h.state != 0x81 {
            self.mode = 0;
            self.mode_inv = 1.0;
            self.mode_t = 0;
            return;
        }
        let to_six = |c: &mut FollowCamera, prev: u8| {
            set(c, prev, 6, 30);
            raise(c, 35, 1.0);
        };
        if m == 7 {
            if h.jump.descending == 0 { return to_six(self, 7); }
            if h.group == 2 { return set(self, 7, 1, 20); }
        }
        if m == 4 && h.group != 4 { return set(self, 4, if h.group == 2 { 1 } else { 0 }, 20); }
        if m == 3 && h.state == 0xb { return to_six(self, 3); }
        if m == 5 && h.state == 0xc { return set(self, 5, 8, 30); }
        if m == 8 && h.state != 0xc { return set(self, 8, 0, 20); }
        if m == 6 && h.state != 0xb {
            self.mode_prev = 6;
            self.mode = 0;
            return;
        }
        if (m == 3 || m == 5) && h.group != 4 {
            self.mode_prev = m;
            self.mode = 0;
            return;
        }
        if (m == 9 || m == 10) && (h.group == 2 || 0x36 < h.f16c) {
            set(self, m, 1, 20);
            self.row_blend = 90;
            self.row_blend_inv = 1.0 / 90.0;
            self.saved_fwd = self.rows[0];
            return;
        }
        if m == 9 && h.state != 0x14 { return set(self, 9, 10, 30); }
        if m == 10 && dec_timer(&mut self.raise_t) != 0 { set(self, 10, 0, 20); }
    }

    /// `0x421c88(look height)`: the signed angle between the view and the hero's look point (the left component
    /// removed).
    fn view_angle(&self, h: &Hero) -> f32 {
        let p = add(h.pos, scale(self.up_s, self.look_h));
        let v = sub(p, self.pos);
        let v = sub(v, scale(self.rows[1], dot(v, self.rows[1])));
        let l = len(v);
        if l == 0.0 { return 0.0; }
        let a = HALF_PI - asin(dot(v, self.rows[0]) / l);
        if dot(v, self.rows[2]) < 0.0 { -a } else { a }
    }

    /// `0x4227b8`: the vertical target (D+0x70) and the target (D+0x60, horizontal). In mode 2 (a jump) the vertical
    /// target holds until the hero is back on the ground or comes down toward it (landing ETA `0x18a2ce` < 16, here
    /// computed as the jump does), and the mode drops
    /// to 1 when the hero leaves the view (−0.01 rad .. π/2); the raise modes 3 / 5 / 6 / 7 blend from the raise's
    /// start to its end (frozen when the camera touches something); 9 / 10 halfway to the raise's start.
    fn vertical_target(&mut self, inp: &CamInput) -> V3 {
        let h = inp.hero;
        let up2 = self.up2;
        let hero = h.pos;
        let hz = |p: V3| sub(p, scale(up2, dot(p, up2)));
        match self.mode {
            2 => {
                let leave = h.state == 0x11 || (h.jump.descending != 0 && 0.2 < h.height && {
                    let a = self.view_angle(h);
                    a < -0.01 || HALF_PI < a
                });
                if leave {
                    self.mode = 1;
                    self.mode_t = 10;
                    self.mode_inv = 0.1;
                }
                let mut hold = true;
                if h.f16c == 0 || (h.jump.descending != 0 && h.land_eta(h.jump.g) < 16.0) {
                    let (g, r) = self.ground_target(inp, true);
                    if r == 1 && len(self.vtarget) <= dot(g, up2) { hold = false; }
                }
                if !hold {
                    let (g, _) = self.ground_target(inp, true);
                    let v = scale(up2, dot(g, up2));
                    self.target = sub(g, v);
                    return v;
                }
                self.target = hz(hero);
                self.vtarget
            }
            3 | 5 | 6 | 7 => {
                self.vtarget = lerp(self.raise_b, self.raise_a, self.raise_t as f32 * self.raise_inv);
                if self.raise_frozen == 0 && inp.sphere(0.2, self.pos, 0x12).is_some() {
                    self.raise_b = self.vtarget;
                    self.raise_frozen = 1;
                }
                let v = self.vtarget;
                self.target = hz(hero);
                if dec_timer(&mut self.raise_t) != 0 {
                    if self.mode == 6 {
                        self.mode = 7;
                        self.raise_t = 35;
                        self.raise_b = self.raise_a;
                        self.raise_a = self.vtarget;
                    } else {
                        self.mode = 4;
                        self.mode_t = 20;
                        self.mode_inv = 1.0 / 20.0;
                    }
                }
                v
            }
            8 => {
                self.target = hz(hero);
                self.vtarget
            }
            9 | 10 => {
                let v = scale(up2, dot(hero, up2));
                self.target = hz(hero);
                lerp(v, self.raise_a, 0.5)
            }
            _ => {
                let v = scale(up2, dot(hero, up2));
                self.target = hz(hero);
                v
            }
        }
    }

    /// `0x423150`: the target mode and the vertical target, the springs of the smoothed target (horizontal kh / dh,
    /// vertical kv / dv), the look point (blended from the target to the smoothed one over 90 ticks while
    /// `look_from_s`), the smoothed look (`0x423050`), the pivot, and the ceiling / floor lines 20 along up from 0.5
    /// above the feet that keep the pivot inside.
    fn targets(&mut self, inp: &CamInput) {
        let h = inp.hero;
        self.target_mode(inp);
        let v = self.vertical_target(inp);
        self.vtarget = v;
        let (up2, up_s) = (self.up2, self.up_s);
        let mut sv = scale(up2, dot(up2, self.smooth));
        let mut sh = sub(self.smooth, sv);
        self.vgap = len(sub(sv, self.vtarget));
        sv = interp3(sv, self.vtarget, self.kv, self.dv, &mut self.vel_v);
        sh = interp3(sh, self.target, self.kh, self.dh, &mut self.vel_h);
        self.target = add(self.target, self.vtarget);
        self.smooth = add(sh, sv);
        let lo = scale(up_s, self.look_h);
        if self.look_from_s == 0 {
            dec_timer(&mut self.look_blend);
            if self.look_blend < 0 { self.look_blend = 0; }
        } else {
            self.look_blend = (self.look_blend + 1).min(self.look_blend_len);
        }
        let tt = self.look_blend as f32 / self.look_blend_len.max(1) as f32;
        self.look = add(lo, lerp(self.target, self.smooth, tt));
        // `0x423050`: the smoothed look, faster in modes 1 / 3 / 5 / 6 / 7 at the mode's start.
        if dec_timer(&mut self.mode_t) != 0 { self.mode_prev = self.mode; }
        let tm = self.mode_t as f32 * self.mode_inv;
        let tgt = add(lo, if self.mode == 1 { h.pos } else { self.target });
        let (k, dd) = if matches!(self.mode, 1 | 3 | 5 | 6 | 7) { (0.06 + (0.01 - 0.06) * tm, 0.4 + (0.2 - 0.4) * tm) } else { (0.01, 0.2) };
        self.look_s = interp3(self.look_s, tgt, k, dd, &mut self.look_s_vel);
        // The pivot and the lines.
        let base = add(h.pos, scale(up_s, 0.5));
        self.pivot = add(scale(up_s, self.pivot_h), self.target);
        let hh = 0.25 + (self.sph_step * self.sph_scale) * (self.sph_count - 1.0);
        let top = add(base, scale(up_s, 20.0));
        let bottom = sub(base, scale(up_s, 20.0));
        let cc = inp.line(base, top, self.line_flags).unwrap_or(top);
        let mut ff = inp.line(base, bottom, self.line_flags).unwrap_or(bottom);
        let p = if len(sub(ff, cc)) < hh + hh {
            scale(add(cc, ff), 0.5)
        } else if len(sub(cc, self.target)) <= self.pivot_h + hh {
            sub(cc, scale(up_s, hh))
        } else {
            if dot(ff, up2) < dot(self.target, up2) { ff = self.target; }
            self.pivot = add(scale(up_s, self.pivot_h), ff);
            return;
        };
        self.pivot = p;
        self.ph_tgt = dot(sub(p, self.target), up_s);
        self.ph_rate = 0.006;
        self.ph_ovr = 1;
    }

    /// `0x4269a0`: the offset blends toward desired − pivot (`0x1a8900`), kept between distance − reduction and the
    /// distance.
    fn leash(&mut self) {
        if self.leash <= 0 { return; }
        let w = sub(self.desired, self.pivot);
        self.off = lerp(self.off, w, self.leash_k);
        let mut l = len(self.off);
        if self.dist < l {
            l = self.dist;
            self.off = norm(self.off, l);
        } else {
            let e = self.dist - self.red;
            if l < e {
                l = e;
                self.off = norm(self.off, l);
            }
        }
        self.red = self.dist - l;
    }

    /// `0x4264a8` + `0x426628`: the right stick turns the offset about up (the yaw input springs 1 / 1, times the yaw
    /// rate; no input: the platform's yaw) and tilts it (the pitch input past a 0.3 dead zone, ×1/0.7, springs at 0.02,
    /// or 0.01 while easing back; the elevation aimed at = input · 40°, stepped by at most 1.75° a tick).
    fn stick(&mut self, inp: &CamInput) {
        let [rx, ry] = inp.rstick;
        let mut yi = if self.script & 1 != 0 { 0.0 } else if inp.yaw_normal { -rx } else { rx };
        if yi == 0.0 {
            if self.script_yaw != 0.0 { yi = self.script_yaw; }
        } else {
            self.yaw_rate = YAW_RATES[0];
        }
        let pi = if self.script & 2 != 0 {
            self.script_pitch / DEG40
        } else {
            let p = if inp.pitch_normal { -ry } else { ry };
            if p == 0.0 { self.script_pitch / DEG40 } else { p }
        };
        let up_s = self.up_s;
        self.yaw_in = interp(self.yaw_in, yi, 1.0, 1.0, 0.0, &mut self.yaw_in_v);
        self.yaw_step = self.yaw_rate * self.yaw_in;
        let p = if pi < -0.3 { (pi + 0.3) * 1.428_571_5 } else if 0.3 < pi { (pi - 0.3) * 1.428_571_5 } else { 0.0 };
        let cur = self.pitch_n;
        let easing = p.abs() < cur.abs() && ((p < 0.0 && cur < 0.0) || (0.0 < p && 0.0 < cur));
        let rate = if p == 0.0 || easing { 0.01 } else { 0.02 };
        self.pitch_n = interp(cur, p, 1.0, 1.0, rate, &mut self.pitch_n_v);
        let tgt = self.pitch_n * DEG40;
        self.off = rot(self.off, if yi == 0.0 { self.plat_yaw } else { self.yaw_step }, up_s);
        let e = dot(self.off, up_s);
        let l = len(self.off);
        let ax = norm(cross(up_s, self.off), 1.0);
        let el = e.atan2(l);
        if el.abs() < tgt.abs() && rate == 0.01 { self.pitch_n = el / DEG40; }
        self.pitch_step = if el < tgt {
            if tgt < add_rot(el, DEG1_75) { sub_rot(tgt, el) } else { DEG1_75 }
        } else if tgt < el {
            if self.end_class < 0 { 0.0 } else if add_rot(el, -DEG1_75) < tgt { sub_rot(tgt, el) } else { -DEG1_75 }
        } else {
            0.0
        };
        if tgt < 0.0 { self.pitch_neg = 1; }
        self.off = rot(self.off, self.pitch_step, ax);
    }

    /// `0x4258d0`: the sphere chain and the end sphere pull the offset in (the reduction D+0x220, at most down to 1.5
    /// from the distance); the reduction restores over 2000 ticks once nothing blocks and the right stick is still; a
    /// line pivot → camera blocked for 30 ticks resets the camera (`0x2da160`; returns true then).
    fn avoidance(&mut self, inp: &CamInput) -> bool {
        let mut blocked = false;
        if self.no_chain == 0 && self.sphere_chain(inp) { blocked = true; }
        if self.end_sphere(inp) { blocked = true; }
        if self.dist - self.red < 1.5 {
            self.rec_t = 2000;
            self.red = self.dist - 1.5;
        }
        let [rx, ry] = inp.rstick;
        let rec = if self.fast_rec != 0 {
            true
        } else if blocked {
            false
        } else if 0 < self.rec_t && self.rec_t < 2000 {
            true
        } else {
            rx < 0.3 && ry < 0.3
        };
        if rec {
            if dec_timer(&mut self.rec_t) != 0 { self.fast_rec = 0; }
            if self.fast_rec != 0 && dec_timer(&mut self.rec_t) != 0 { self.fast_rec = 0; }
            self.red = cos_interp(0.0, self.red, self.rec_t as f32 / 2000.0);
        } else if 2000 < self.rec_t {
            dec_timer(&mut self.rec_t);
        } else {
            self.rec_t = 2000;
        }
        self.off = norm(self.off, self.dist - self.red);
        let e = add(self.off, self.pivot);
        if inp.hero.state == 0x31 || self.no_chain != 0 {
            self.blocked = 0;
        } else if inp.line(self.pivot, e, self.line_flags).is_some() {
            self.blocked += 1;
            if 0x1d < self.blocked {
                *self = FollowCamera::new(inp);
                return true;
            }
        } else {
            self.blocked = 0;
        }
        false
    }

    /// `0x4255e8`: spheres along pivot → offset (radius growing by step · scale from 0.25); a hit that is not behind the
    /// pivot re-aims (`0x4251b8`), and with the right stick moving sideways the reduction grows by 0.075.
    fn sphere_chain(&mut self, inp: &CamInput) -> bool {
        let rx = inp.rstick[0];
        let mut p = self.pivot;
        let mut dir = norm(self.off, 1.0);
        let mut r = 0.25;
        let mut acc = 0.0;
        let mut blocked = false;
        let n = (self.sph_count - 1.0) as i32;
        for i in 0..n {
            if 0 < i {
                if let Some((hit, _)) = inp.sphere(r, p, self.sph_flags) {
                    let behind = i == 1 && dot(sub(hit, p), self.off) < 0.0;
                    if !behind {
                        let q = p;
                        if self.reaim(inp, acc, r, &mut p) {
                            blocked = true;
                            if 0.05 < rx.abs() {
                                let u = norm(sub(hit, q), 1.0);
                                if dot(self.up2, u).abs() < 0.75 {
                                    self.rec_t = 0x884;
                                    self.red += 0.075;
                                    if self.dist - self.red < 0.2 { self.red = self.dist - 0.2; }
                                }
                            }
                            self.off = norm(sub(p, self.pivot), self.dist - self.red);
                            dir = norm(self.off, 1.0);
                        }
                    }
                }
            }
            let s = (self.dist - self.red) / self.dist;
            r += self.sph_step * self.sph_scale;
            acc += r;
            p = add(p, scale(dir, r * s));
        }
        blocked
    }

    /// `0x4251b8`: push the sphere's centre so the pivot → camera segment clears the hit point (three passes).
    fn reaim(&mut self, inp: &CamInput, acc: f32, r: f32, p: &mut V3) -> bool {
        let hh = 0.25 + (self.sph_step * self.sph_scale) * (self.sph_count - 1.0);
        let p0 = *p;
        let a = self.pivot;
        let bp = add(a, self.off);
        let eff = self.dist - self.red;
        let Some((mut hit, mut pc)) = inp.sphere(r, p0, self.sph_flags) else { return false };
        let mut disp = [0.0; 3];
        let mut ret = false;
        for pass in 0..3 {
            if 0 < pass {
                match inp.sphere(r, pc, self.sph_flags) {
                    Some((h2, pc2)) => {
                        hit = add(h2, sub(p0, pc));
                        pc = pc2;
                    }
                    None => break,
                }
            }
            let (dd, c) = seg_dist(hit, a, bp);
            let tt = len(sub(a, c));
            let allow = (0.25 + (hh - 0.25) * (tt / eff)) * 0.75;
            if dd >= allow { break; }
            ret = true;
            let c2 = add(c, norm(sub(c, hit), allow - dd));
            let x = add(a, norm(sub(c2, a), acc));
            let push = sub(x, p0);
            disp = if pass == 0 { push } else { add(disp, push) };
        }
        *p = add(p0, disp);
        ret
    }

    /// `0x424eb8`: the camera's own sphere (radius hh) pushed out up to three times; a floor / ceiling contact keeps
    /// the length, a wall shortens the offset (not below 0.2).
    fn end_sphere(&mut self, inp: &CamInput) -> bool {
        let group = inp.hero.group;
        let rx = inp.rstick[0];
        let mut e = add(self.pivot, self.off);
        self.end_class = 2;
        let hh = 0.25 + (self.sph_step * self.sph_scale) * (self.sph_count - 1.0);
        let mut hit = false;
        for _ in 0..3 {
            let Some((pt, pc)) = inp.sphere(hh, e, self.end_flags) else { break };
            hit = true;
            if self.end_class != 0 {
                let c = dot(self.up2, norm(sub(pt, e), 1.0));
                self.end_class = if 0.75 < c { 1 } else if c < -0.75 { -1 } else { 0 };
            }
            e = pc;
        }
        let l0 = len(self.off);
        self.off = sub(e, self.pivot);
        let ret;
        if self.end_class != 0 {
            self.off = norm(self.off, self.dist - self.red);
            ret = hit;
        } else {
            let mut l = len(self.off);
            if 0.05 < rx.abs() && hit {
                l -= 0.075;
                self.rec_t = 0x884;
            }
            if group != 2 && l0 < l {
                self.off = scale(self.off, l0 / l);
                self.red = self.dist - l0;
                ret = hit;
            } else {
                if l < 0.2 {
                    self.off = if l <= 1e-4 { norm(inp.hero.rows[0], -0.2) } else { scale(self.off, 0.2 / l) };
                    l = 0.2;
                }
                self.red = self.dist - l;
                ret = true;
            }
        }
        if self.end_class == 2 { self.end_class = 0; }
        ret
    }

    /// `0x426fa8`: the distance spring (0.02 / 0.2; running toward the camera: up to 0.04 / 0.3 and pulled out toward
    /// the pull limit).
    fn dist_rate(&mut self, inp: &CamInput) -> (f32, f32) {
        let h = inp.hero;
        let toward = self.toward_lock == 0
            && ((self.toward_t != 0 && self.h_speed == 0.0 && dot(h.rows[0], self.rows[0]) <= -0.3)
                || (1e-4 < self.h_speed.abs() && dot(self.move_dir, self.rows[0]) <= -0.3));
        if toward {
            self.toward_t = 0x78;
            let f = (self.h_speed * 14.2857).min(1.0);
            let x = self.base_dist + (self.pull_max - self.base_dist) * f;
            if self.pull < x { self.pull = x; }
            self.dist_tgt = self.pull;
            self.dist_rate = 0.003;
            self.dist_ovr = 1;
            (0.02 + (0.04 - 0.02) * f, 0.2 + (0.3 - 0.2) * f)
        } else {
            if self.toward_t != 0 {
                self.toward_t = 0;
                self.pull = self.base_dist;
            }
            (0.02, 0.2)
        }
    }

    /// `0x4271e0`: the placed offset springs toward the offset in length (the distance rate), elevation (0.015 / 0.2)
    /// and yaw (0.015 / 0.2); the position = smoothed target + pivot height + placed; then a 0.25 sphere (flags 0x36)
    /// pushes it out of what it touches up to three times, moving the targets with it.
    fn placement(&mut self, inp: &CamInput) {
        let (up_s, up2) = (self.up_s, self.up2);
        let (k, dd) = self.dist_rate(inp);
        self.desired = add(self.pivot, self.off);
        let ddist = self.dist - self.red;
        let ln = interp(len(self.placed), ddist, k, dd, 0.0, &mut self.dist_v);
        let e1 = dot(self.off, up_s);
        let el_d = if 1e-4 <= ddist.abs() { asin(e1 / ddist) } else { 0.0 };
        let e2 = dot(self.placed, up_s);
        let el_p = if 1e-4 <= ln.abs() { asin(e2 / ln) } else { 0.0 };
        let el = ang_interp(el_p, el_d, 0.015, 0.2, 0.0, &mut self.el_v);
        let oh = sub(self.off, scale(up_s, e1));
        let ph = sub(self.placed, scale(up_s, e2));
        let m = len(oh) * len(ph);
        let a = if 1e-4 <= m { HALF_PI - asin(dot(oh, ph) / m) } else { 0.0 };
        let mut ys = ang_interp(0.0, a, 0.015, 0.2, 0.0, &mut self.yaw_v);
        if dot(cross(ph, up_s), oh) < 0.0 { ys = -ys; }
        let v = rot(ph, ys, up_s);
        let v = rot(v, el, cross(up_s, v));
        self.placed = norm(v, ln);
        let base = add(scale(up2, self.pivot_h), self.smooth);
        self.pos = add(base, self.placed);
        let before = self.pos;
        let mut moved = false;
        for _ in 0..3 {
            let Some((pt, pc)) = inp.sphere(0.25, self.pos, 0x36) else { break };
            if dot(self.rows[2], norm(sub(pt, self.pos), 1.0)).abs() <= 0.5 { break; }
            self.pos = pc;
            moved = true;
        }
        if moved {
            let d = sub(self.pos, before);
            for p in [&mut self.target, &mut self.smooth, &mut self.look_s, &mut self.raise_b] { *p = add(*p, d); }
        }
    }

    /// `0x426a70`: forward toward the look point (horizontal), left, then the pitch toward the look height (the pivot
    /// and look heights smoothed at 0.004 / 0.2), less the bias (on the ground, from the offset's elevation below
    /// −0.1·40°), at most 70°; the row blend toward the saved forward.
    fn rows(&mut self, inp: &CamInput) {
        let up_s = self.up_s;
        let mut fwd = self.rows[0];
        let lh = sub(self.look, scale(up_s, dot(self.look, up_s)));
        let ph = sub(self.pos, scale(up_s, dot(self.pos, up_s)));
        let hv = sub(lh, ph);
        let l = len(hv);
        if 0.05 <= l { fwd = scale(hv, 1.0 / l); }
        let left = norm(cross(fwd, up_s), 1.0);
        self.pivot_h_s = interp(self.pivot_h_s, self.pivot_h, 0.004, 0.2, 0.0, &mut self.pivot_h_v);
        self.look_h_s = interp(self.look_h_s, self.look_h, 0.004, 0.2, 0.0, &mut self.look_h_v);
        let dh = self.pivot_h_s - self.look_h_s;
        let e = dot(self.placed, up_s);
        let a = sub(scale(up_s, -e), hv);
        let w = sub(sub(self.pos, a), scale(up_s, -dh));
        let v = sub(w, self.pos);
        let lv = len(v);
        if lv != 0.0 {
            let mut pit = HALF_PI - asin(dot(fwd, v) / lv);
            let up = cross(left, fwd);
            if dot(up, v) < 0.0 { pit = -pit; }
            let mut tb = 0.0;
            if inp.hero.grounded_ticks != 0 {
                let mut q = dot(self.off, up_s).atan2(len(self.off)) / DEG40;
                if q < -0.1 {
                    q = -q;
                    if 0.5 < q { q = 1.0 - q; }
                    tb = (q + q) * 0.261_799_42;
                }
            }
            self.bias = ang_interp(self.bias, tb, self.bias_k, self.bias_d, 0.0, &mut self.bias_v);
            let p = sub_rot(pit, self.bias).clamp(-DEG70, DEG70);
            fwd = norm(rot(fwd, p, left), 1.0);
        }
        if self.row_blend != 0 {
            dec_timer(&mut self.row_blend);
            fwd = norm(lerp(fwd, self.saved_fwd, self.row_blend as f32 * self.row_blend_inv), 1.0);
        }
        let left = norm(cross(fwd, up_s), 1.0);
        let up = cross(left, fwd);
        self.rows = [fwd, left, up];
    }

    /// `0x423768`: the tick's settings back — the yaw rate, the scripted inputs off; the distance, pivot height and look
    /// height spring to their overrides or the defaults (rates +0x1a8 / +0x1b4 / +0x118, 0.2); the springs, the
    /// sphere chain (scale 1, count 8, step 0.1), the pull limit 6, the look blend 90 and the leash restored.
    fn spring_back(&mut self) {
        self.yaw_rate = YAW_RATES[0];
        (self.script_yaw, self.plat_yaw, self.script, self.script_pitch) = (0.0, 0.0, 0, 0.0);
        let t = if self.dist_ovr != 0 { self.dist_tgt } else { self.d0_dist };
        self.dist = interp(self.dist, t, self.dist_rate, 0.2, 0.0, &mut self.dist_ovr_v);
        let t = if self.ph_ovr != 0 { self.ph_tgt } else { self.d0_pivot_h };
        self.pivot_h = interp(self.pivot_h, t, self.ph_rate, 0.2, 0.0, &mut self.ph_v);
        let t = if self.look_h_ovr != 0 { self.look_h_tgt } else { self.d0_look_h };
        self.look_h = interp(self.look_h, t, self.look_h_rate, 0.2, 0.0, &mut self.look_h_vel);
        (self.dist_ovr, self.ph_ovr, self.look_h_ovr) = (0, 0, 0);
        (self.dist_tgt, self.ph_tgt, self.look_h_tgt) = (0.0, 0.0, 0.0);
        self.base_dist = self.d0_dist;
        self.leash = 1;
        self.pitch_neg = 0;
        self.look_from_s = 0;
        (self.kh, self.dh, self.kv, self.dv) = (0.015, 0.2, 0.0125, 0.2);
        (self.bias_k, self.bias_d) = (0.005, 0.2);
        self.look_blend_len = 90;
        (self.sph_scale, self.sph_count, self.sph_step) = (1.0, 8.0, 0.1);
        self.pull_max = 6.0;
        (self.toward_lock, self.no_chain) = (0, 0);
    }
}

/// The distance from `p` to the segment a → b and the closest point.
fn seg_dist(p: V3, a: V3, b: V3) -> (f32, V3) {
    let ab = sub(b, a);
    let l2 = dot(ab, ab);
    let t = if l2 == 0.0 { 0.0 } else { (dot(sub(p, a), ab) / l2).clamp(0.0, 1.0) };
    let c = add(a, scale(ab, t));
    (len(sub(p, c)), c)
}

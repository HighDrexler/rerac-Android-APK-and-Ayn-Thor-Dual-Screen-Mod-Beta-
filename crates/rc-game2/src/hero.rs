//! Ratchet in Going Commando: the hero block (L00 `0x189e20..0x18c330`), the per-tick update `0x2a6dd0` and its
//! sub-steps. Organised like R&C1's port (`rc_game::hero`), because R&C2's hero code is R&C1's evolved: the same
//! driver, dispatchers and helpers, with R&C2's constants and additions (docs/plan/rac2_bringup.md §6.4).
//! Native `f32` (docs/plan/decisions.md "native-first"); the shared pad (`rc_game::pad`: R&C2's `0x3306b0` is R&C1's
//! `ProcessPadInput`), collision queries (`rc_game::collision_query`) and Ratchet's animation player
//! (`rc_game::hero::anim`: R&C2's SetAnim `0x2d2508` is R&C1's `0x247a90`).
//!
//! * [`registry`]: every state id of R&C2's machine (SetState `0x2c4560`'s cases), its movement group, owning
//!   module and whether the port has it. [`states`]: SetState and the two dispatchers, per-state physics
//!   `0x2ba8e0` and transitions `0x2cb650`.
//! * Ported groups: [`ground`] (idle 0, stop 3, crouch 4), [`walk`] (2), [`air`] (fall 6), [`jump`] (7, 9).
//! * [`physics`]: kernels, the helpers the states call and the move pipeline.
//!
//! The update (`0x2a6dd0`, mode 0, the parts ported): Ratchet's advance (`0x2d2888`), the input + physics + move
//! step `0x2b37e8` ([`Hero::input_physics_move`]), the transitions `0x2cb650`, the write-back.
//! Not ported yet: the water / surface check `0x2aca80`, the ledge grab `0x2ad000`, the swim, gadget, weapon,
//! damage and HUD sub-steps of the update (`0x2ae140` … `0x2a97d8`).
//!
//! Field names are ours; each carries its L00 address.

pub mod air;
pub mod ground;
pub mod jump;
pub mod physics;
pub mod registry;
pub mod states;
pub mod walk;

use physics::{euler_rows, Env, V3};
use rc_game::hero::anim::{AnimCtl, AnimView};
use rc_game::ps2v::Pf;

/// Ring size of the position / yaw histories (`0x18bbb0`, `0x18c15c`).
pub const HISTORY: usize = 32;

/// The jump block (`0x18a250..0x18a350`, the fields the on-foot jumps use).
#[derive(Clone, Copy, Debug, Default)]
pub struct JumpBlock {
    /// `0x18a2cc` (s16): takeoff tick (the windup before leaving the ground).
    pub takeoff: i32,
    /// `0x18a2be`: descending.
    pub descending: u8,
    /// `0x18a2bc`: landed (counts ticks since landing; 0 in the air).
    pub landed: u8,
    /// `0x18a2d8`: capsule bottom target after takeoff.
    pub bottom: f32,
    /// `0x18a2b0`: reference height (rises with the ground, never above the feet).
    pub ref_z: f32,
    /// `0x18a280`: position at takeoff.
    pub takeoff_pos: V3,
    /// `0x18a328` / `0x18a32c`: minimum / maximum height; `0x18a2c8`: the height aimed at.
    pub hmin: f32,
    pub hmax: f32,
    pub h: f32,
    /// `0x18a338` (s16): ticks over which the height ramps from min to max while ✕ is held.
    pub ramp: i32,
    /// `0x18a340`: gravity (u/tick²); `0x18a278`: gravity once descending (0: keep).
    pub g: f32,
    pub g_down: f32,
    /// `0x18a2c4`: air stick factor.
    pub air_speed: f32,
    /// `0x18a320` / `0x18a324`: windup accel / decel.
    pub acc: f32,
    pub dec: f32,
    /// `0x18a2ec` / `0x18a2f0`: anim frames (apex, landing); `0x18a2e8`: frame of the hold before touchdown.
    pub f_apex: f32,
    pub f_land: f32,
    pub f_hold: f32,
    /// `0x18a33a` (s16): ticks before the jump gives way to the fall 6; `0x18a34a` (s16).
    pub max_air: i32,
    pub f34a: i32,
    /// `0x18a330`: fall-over height.
    pub fallover_h: f32,
    /// `0x18a344` (s16): blocked ahead in the first 10 ticks.
    pub blocked: u8,
    /// `0x18a348` (s16): keep the ground speed as the air target.
    pub keep_speed: u8,
    /// `0x18a34c` / `0x18a34e` / `0x18a34f`: double jump allowed, …
    pub f34c: u8,
    pub f34e: u8,
    pub can_double: u8,
    /// `0x18a334`.
    pub f334: f32,
    /// `0x18a2b4`: the air turn rate cap (rad/tick; it keeps its value after the jump, the fall turns with it);
    /// `0x18a2c0`: the fall speed cap (u/tick).
    pub air_turn: f32,
    pub terminal: f32,
    /// The vertical speed the jump applied (`0x18a2d0` / `0x18a2d4`).
    pub applied: f32,
    pub pending: f32,
    /// `0x18a2ce` (s16): the landing ETA in ticks, as the descending animation timing last computed it.
    pub land_eta: i16,
    /// `0x18a2bf`: no double jump from this jump (set by the flip 0xb and the scripted jumps).
    pub no_double: u8,
    /// `0x18a346` (s16): the ticks before the glide may start.
    pub f346: i32,
    /// The flip 0xb: `0x18a2f4` its direction (`0x2c3cf8`: 0 left, 1 right, 2 forward, 3 back), `0x18a2e4` the yaw 10
    /// ticks before, `0x18a2a0` the forward row, `0x18a2e0` / `0x18a290` the stick's target yaw and direction,
    /// `0x18a2f8` the ground speed carried (above 0.025), `0x18a2fc` the push along the stick.
    pub flip_dir: i32,
    pub flip_yaw: f32,
    pub flip_fwd: V3,
    pub flip_target_yaw: f32,
    pub flip_stick_dir: V3,
    pub flip_speed: f32,
    pub flip_push: f32,
}

/// The hero block. Addresses are L00 (the boot ELF's .bss: the same in every level).
#[derive(Clone, Debug)]
pub struct Hero {
    /// `0x189e20`: rotation rows (forward, left, up) from the Euler angles; copied to moby +0xc0.
    pub rows: [V3; 3],
    /// **`0x189ea0`: position (feet).**
    pub pos: V3,
    /// `0x189eb0`: Euler rotation; `[2]` (`0x189eb8`) = **yaw**.
    pub rot: V3,
    /// **`0x189f10`: velocity (u/tick).**
    pub vel: V3,
    /// `0x189f30`: displacement this tick (capped at 52 / 60).
    pub disp: V3,
    /// `0x189f40` / `0x189f50` / `0x189f60`: effective velocity, its vertical and horizontal parts.
    pub eff: V3,
    pub eff_v: V3,
    pub eff_h: V3,
    /// `0x189f80`: carried momentum.
    pub momentum: V3,
    /// `0x189f90` / `0x189f94` / `0x189f98` / `0x189f9c`: |eff|, |eff.xy|, forward speed, slope ratio.
    pub eff_len: f32,
    pub eff_len_xy: f32,
    pub fwd_speed: f32,
    pub slope_ratio: f32,
    /// `0x189fb0`: the stick's direction in the world.
    pub stick_dir: V3,
    /// `0x189fc0` / `0x189fc4` / `0x189fc8`: target yaw, yaw velocity, yaw residual.
    pub target_yaw: f32,
    pub yaw_vel: f32,
    pub yaw_residual: f32,
    /// `0x189fd0` / `0x189fd4`: target speed / speed (u/tick).
    pub target_speed: f32,
    pub speed: f32,
    /// **`0x189fd8`: state timer** (++ at the start of the post-move; 0 on every SetState); `0x189fdc` group timer.
    pub timer: i32,
    pub group_timer: i32,
    /// `0x189fe0` / `0x189fe4`: ticks since the substate / Ratchet's sequence changed (reset by the physics' end and
    /// SetState).
    pub substate_timer: i32,
    pub seq_timer: i32,
    /// **`0x18c2ec`: health** (0: only the death states can be set).
    pub hp: i32,
    /// `0x18a000` (s16): the landing lag (ticks; counted down by the post-move).
    pub land_lag: i16,
    /// `0x18c2cc`: the stop's deceleration.
    pub stop_decel: f32,
    /// `0x18a8a0` / `0x18a8a4`: the fall's gravity and its horizontal deceleration.
    pub fall_g: f32,
    pub fall_dec: f32,
    /// `0x18a230`: the run keeps its kept direction; `0x18a22e`: the walk's sequence change waits for the blend.
    pub run_keep: u8,
    pub walk_gate: u8,
    /// `0x18a15c` ≠ 0: the ground under the feet is a moby (never here: the probe is world-only).
    pub on_moby: bool,
    /// `0x18a070` / `0x18a074` / `0x18a084` (+ velocity `0x18a088`): capsule top, bottom offset, radius, and their
    /// targets `0x18a078` / `0x18a07c` / `0x18a080`.
    pub cap_top: f32,
    pub cap_bottom: f32,
    pub cap_radius: f32,
    pub cap_radius_vel: f32,
    pub cap_top_target: f32,
    pub cap_bottom_target: f32,
    pub cap_radius_target: f32,
    /// `0x18a0a7`: the capsule hit something this tick; `0x18a060` / `0x18a050`: contact point / normal (raw).
    pub cap_hit: u8,
    pub contact_point: V3,
    pub contact_normal: V3,
    /// `0x18a0f0` / `0x18a138` / `0x18a13c` / `0x18a0c0` / `0x18a140`: ground point, ground z, height above it,
    /// normal, slope; `0x18a100`: the last ground point.
    pub ground_point: V3,
    pub ground_z: f32,
    pub height: f32,
    pub ground_normal: V3,
    pub slope: f32,
    pub last_ground_point: V3,
    /// `0x18b2b0`: surface id under the feet.
    pub surface: i32,
    /// `0x18a160`: ticks on the ground; **`0x18a16e`: ticks in the air** (0 = grounded); `0x18a16c`.
    pub grounded_ticks: i32,
    pub air_ticks: i16,
    pub f16c: i16,
    /// `0x18a164`: the snap spring's velocity.
    pub snap_vel: f32,
    /// `0x18a880` / `0x18a890..`: the drag displacement and its source (sliding floors).
    pub drag: V3,
    pub drag_src: V3,
    /// `0x18bdd0`: the stick (d-pad below 0.25); `0x18c2d8` / `0x18c2dc`: its magnitude this / last tick.
    pub stick: [f32; 2],
    pub stick_mag: f32,
    pub stick_mag_prev: f32,
    /// `0x18a22c` / `0x18a22f`: the run keeps its last direction; quick turn.
    pub run_hold: u8,
    pub quick_turn: u8,
    /// `0x18a210` / `0x18a228`: the run's kept facing row and yaw.
    pub run_dir: V3,
    pub run_yaw: f32,
    /// `0x18aa40`: Ratchet's animation playback speed.
    pub anim_speed: f32,
    /// **`0x18c0b4` state**, `0x18c0b8` substate, **`0x18c0bc` movement group**; `0x18c0c0` / `0x18c0c4` the previous
    /// state / group, `0x18c0cc` / `0x18c0d0` the ones before; `0x18c0c8` the previous state's timer.
    pub state: i32,
    pub substate: i32,
    pub group: i32,
    pub prev_state: i32,
    pub prev_group: i32,
    pub prev2_state: i32,
    pub prev2_group: i32,
    pub prev_timer: i32,
    /// `0x18c2c0`: the camera's view of the state (0x50 in the jumps, 0 otherwise; the follow camera holds its smoothed
    /// height while 0x50).
    pub cam_mode: u8,
    /// `0x18c0d4`: the body (0 = Ratchet).
    pub mode: u8,
    pub jump: JumpBlock,
    /// `0x18bbb0` (32 × vec4) and `0x18c1e4` / `0x18c1e8`: position history, index, count; `0x18c15c` / `0x18c1dc` /
    /// `0x18c1e0`: yaw history.
    pub pos_hist: [V3; HISTORY],
    pub pos_hist_idx: usize,
    pub pos_hist_count: usize,
    pub yaw_hist: [f32; HISTORY],
    pub yaw_hist_idx: usize,
    pub yaw_hist_count: usize,
    /// States the port has not got, reported once each.
    pub unported_seen: Vec<i32>,
}

impl Default for Hero {
    fn default() -> Self { Hero::new() }
}

impl Hero {
    pub fn new() -> Hero {
        Hero {
            rows: euler_rows([0.0; 3]), pos: [0.0; 3], rot: [0.0; 3], vel: [0.0; 3], disp: [0.0; 3], eff: [0.0; 3], eff_v: [0.0; 3],
            eff_h: [0.0; 3], momentum: [0.0; 3], eff_len: 0.0, eff_len_xy: 0.0, fwd_speed: 0.0, slope_ratio: 0.0,
            stick_dir: [1.0, 0.0, 0.0], target_yaw: 0.0, yaw_vel: 0.0, yaw_residual: 0.0, target_speed: 0.0, speed: 0.0,
            timer: 0, group_timer: 0, substate_timer: 0, seq_timer: 0, hp: 8, land_lag: 0, stop_decel: 0.0,
            fall_g: 0.006_111_111_5, fall_dec: 0.001_944_444_5, run_keep: 0, walk_gate: 0, on_moby: false, cap_top: 0.8, cap_bottom: 0.7, cap_radius: 0.45, cap_radius_vel: 0.0,
            cap_top_target: 0.8, cap_bottom_target: 0.7, cap_radius_target: 0.45, cap_hit: 0, contact_point: [0.0; 3],
            contact_normal: [0.0; 3], ground_point: [0.0; 3], ground_z: 0.0, height: 32.0, ground_normal: [0.0, 0.0, 1.0],
            slope: 0.0, last_ground_point: [0.0; 3], surface: -1, grounded_ticks: 0, air_ticks: 0, f16c: 0, snap_vel: 0.0,
            drag: [0.0; 3], drag_src: [0.0; 3], stick: [0.0; 2], stick_mag: 0.0, stick_mag_prev: 0.0, run_hold: 0,
            quick_turn: 0, run_dir: [1.0, 0.0, 0.0], run_yaw: 0.0, anim_speed: 1.0, state: 0, substate: 0, group: 0,
            prev_state: 0, prev_group: 0, prev2_state: 0, prev2_group: 0, prev_timer: 0, cam_mode: 0, mode: 0, jump: JumpBlock { air_turn: 0.250_011_4, terminal: 0.833_333_3, ..JumpBlock::default() },
            pos_hist: [[0.0; 3]; HISTORY], pos_hist_idx: 0, pos_hist_count: 0, yaw_hist: [0.0; HISTORY], yaw_hist_idx: 0,
            yaw_hist_count: 0, unported_seen: Vec::new(),
        }
    }

    /// Placed at a spawn point (`pos`, yaw): idle, standing.
    pub fn spawn(pos: V3, yaw: f32) -> Hero {
        let mut h = Hero::new();
        h.pos = pos;
        h.rot[2] = yaw;
        h.target_yaw = yaw;
        h.rows = euler_rows(h.rot);
        h.pos_hist = [pos; HISTORY];
        h.pos_hist_count = 1;
        h
    }

    pub fn grounded(&self) -> bool { self.air_ticks == 0 }

    /// `SetAnim(blend, seq, frame)` (`0x2d2508`) through the shared player.
    pub fn set_anim(&mut self, anim: &mut dyn AnimCtl, blend: f32, seq: u8, frame: i32) {
        anim.set_anim(Pf::f(blend), seq, frame);
        self.anim_speed = 1.0;
    }

    /// `0x2a4e48(weapon)`: the idle sequence — 0, or 0x67 / 0x51 with the items that change Ratchet's stance (not
    /// ported: 0).
    pub fn idle_seq(&self) -> u8 { 0 }

    /// `0x2b3a08`: the stick's magnitude, `(max(|x|, |y|) + |stick|) / 2`, at most 1.
    fn stick_magnitude(&self) -> f32 {
        let [x, y] = self.stick;
        ((x.abs().max(y.abs()) + (x * x + y * y).sqrt()) * 0.5).min(1.0)
    }

    /// `0x2b37e8`: the stick (the d-pad below 0.25), the per-state physics, the histories, the rows, the capsule, the
    /// move and the post-move. `false` when the state has no ported physics.
    pub fn input_physics_move(&mut self, env: &Env, anim: &mut dyn AnimCtl) -> bool {
        self.stick_mag_prev = self.stick_magnitude();
        self.stick = [env.pad.lx.to_f32(), env.pad.ly.to_f32()];
        if (self.stick[0] * self.stick[0] + self.stick[1] * self.stick[1]).sqrt() < 0.25 {
            let b = env.pad.raw;
            let x = ((b >> 13) & 1) as i32 - ((b >> 15) & 1) as i32;
            let y = ((b >> 14) & 1) as i32 - ((b >> 12) & 1) as i32;
            self.stick = [x as f32, y as f32];
        }
        self.stick_mag = self.stick_magnitude();
        if !self.state_physics(env, anim) { return false; }
        self.yaw_hist[self.yaw_hist_idx] = self.rot[2];
        self.yaw_hist_idx = (self.yaw_hist_idx + 1) & 31;
        self.yaw_hist_count = (self.yaw_hist_count + 1).min(HISTORY);
        self.pos_hist[self.pos_hist_idx] = self.pos;
        self.pos_hist_idx = (self.pos_hist_idx + 1) & 31;
        self.pos_hist_count = (self.pos_hist_count + 1).min(HISTORY);
        self.rows = euler_rows(self.rot);
        self.size_capsule();
        self.move_collide(env);
        self.post_move();
        true
    }

    /// The start of `0x2c3330`: the state, group and sequence timers. Its ~35 countdown timers belong to states not
    /// ported yet.
    fn post_move(&mut self) {
        self.timer += 1;
        self.group_timer += 1;
        self.substate_timer += 1;
        self.seq_timer += 1;
        if self.land_lag != 0 { self.land_lag -= 1; }
        if self.jump.landed != 0 { self.jump.landed = self.jump.landed.saturating_add(1); }
    }

    /// `0x2ae0e8(n)`: the yaw `n` ticks before the newest in the history (at most 31, at most what it holds).
    pub fn yaw_ago(&self, n: usize) -> f32 {
        let k = n.min(31).min(self.yaw_hist_count);
        self.yaw_hist[(self.yaw_hist_idx + 63 - k) & 31]
    }

    /// Write-back to Ratchet's moby: position, Euler angles, rows.
    pub fn moby_pose(&self) -> (V3, V3, [V3; 3]) { (self.pos, self.rot, self.rows) }
}

/// The result of one tick.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HeroTick {
    Ran,
    /// A state the port has not got: the hero stands still.
    Unported(i32),
}

/// `0x2a6dd0` for Ratchet (the parts ported): his advance, the input + physics + move step, the transitions.
pub fn hero_update(hero: &mut Hero, env: &Env, anim: &mut dyn AnimCtl) -> HeroTick {
    if !registry::implemented(hero.state) {
        if !hero.unported_seen.contains(&hero.state) {
            hero.unported_seen.push(hero.state);
            eprintln!("rc_game2::hero: state {:#x} is not ported; the hero stands still", hero.state);
        }
        return HeroTick::Unported(hero.state);
    }
    anim.advance(Pf::f(hero.anim_speed));
    hero.input_physics_move(env, anim);
    hero.transitions(env, anim);
    HeroTick::Ran
}

/// The animation fields the states read.
pub fn view(anim: &dyn AnimCtl) -> AnimView { anim.view() }

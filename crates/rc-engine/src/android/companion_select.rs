//! Android companion intent, independent of the primary display and physical wheel.
//!
//! The original 0.3.6 hand state machine owns put-away and creation. This adapter
//! submits a request once the hand can accept it, observes the actual result,
//! and never keeps a request asserted while the new item is being created.
use rc_game::game_state::{Global, SessionState};
use rc_game::hero::{items::ItemData, Hero};
use rc_game::menus::{quick_select::HeroGate, scale_ticks};

const DRONE: i32 = 24;
const HOLOGUISE: i32 = 31;

/// Only actual hand equipment belongs on the companion quick-select pages.
/// Missing definitions must not default to slot zero. The drone is a one-shot
/// action and is deliberately accepted separately from held equipment.
pub(super) fn eligible(data: Option<&ItemData>, global: &Global, item: i32) -> bool {
    let Ok(id) = usize::try_from(item) else { return false; };
    if !(1..=36).contains(&item) || global.owned.get(id).is_none_or(|&n| n == 0) {
        return false;
    }
    data.and_then(|d| d.defs.get(id)).is_some_and(|d| item == DRONE || d.slot == 0)
}

/// Script/body/interaction guards. A temporary weapon lock is handled by the
/// controller, so an accepted touch is not discarded during a wrench throw.
pub(super) fn allowed(g: &HeroGate) -> bool {
    !g.early_exit && matches!(g.f594, 0 | 5)
        && (g.b13fc == 0 || g.b13f4 == 1 || g.s3f502 != 0)
        && (g.b13f7 == 0 || g.b13f4 == 1 || g.s3f502 != 0)
        && matches!(g.b13f4, 0 | 3)
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum Stage {
    #[default]
    Idle,
    Queued,
    Waiting,
    Issued,
    Switching,
    Equipped,
    Deployed,
    Cancelled,
    Rejected,
}

impl Stage {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Queued => "queued",
            Self::Waiting => "waiting",
            Self::Issued => "issued",
            Self::Switching => "switching",
            Self::Equipped => "equipped",
            Self::Deployed => "deployed",
            Self::Cancelled => "cancelled",
            Self::Rejected => "rejected",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct Status {
    pub sequence: u64,
    pub generation: u64,
    pub item: i32,
    pub stage: Stage,
}

#[derive(Clone, Copy, Debug)]
struct Pending {
    item: i32,
    generation: u64,
    dispatched: bool,
    counted: bool,
}

#[derive(Clone, Copy, Debug)]
struct Attempt {
    item: i32,
    held: i32,
    target: i32,
    saved: i32,
    previous: i32,
}

#[derive(Default)]
pub(super) struct Controller {
    status: Status,
    pending: Option<Pending>,
    attempt: Option<Attempt>,
}

impl Controller {
    pub(super) fn status(&self) -> Status { self.status }

    /// The newest valid touch replaces queued intent. Sequence numbers prevent
    /// repeated packets from deploying drones or restarting an accepted action.
    pub(super) fn accept(&mut self, sequence: u64, generation: u64, item: i32,
        data: Option<&ItemData>, global: &Global) -> bool {
        if sequence == 0 || generation == 0
            || (generation == self.status.generation && sequence <= self.status.sequence) {
            return false;
        }
        if !eligible(data, global, item) {
            self.status = Status { sequence, generation, item, stage: Stage::Rejected };
            self.pending = None;
            self.attempt = None;
            return false;
        }
        self.status = Status { sequence, generation, item, stage: Stage::Queued };
        self.pending = Some(Pending { item, generation, dispatched: false, counted: false });
        self.attempt = None;
        true
    }

    pub(super) fn cancel(&mut self) {
        if self.pending.take().is_some() { self.status.stage = Stage::Cancelled; }
        self.attempt = None;
    }

    /// Activity pause or a skipped simulation tick may leave our issued bit
    /// asserted. Clear only the field this controller actually injected, never
    /// an unrelated request subsequently written by the physical wheel/game.
    pub(super) fn cancel_session(&mut self, session: &mut SessionState) {
        if let Some(pending) = self.pending.filter(|p| p.dispatched) {
            if pending.item == DRONE { session.drone = false; }
            else if session.temp_hand == pending.item { session.temp_hand = 0; }
        }
        self.cancel();
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn before_tick(&mut self, generation: u64, gameplay: bool,
        gate: &HeroGate, hero: &mut Hero, data: Option<&ItemData>,
        global: &mut Global, session: &mut SessionState) {
        let Some(mut pending) = self.pending else { return; };
        if pending.generation != generation {
            self.cancel();
            return;
        }
        if !gameplay || gate.early_exit || !matches!(hero.mode, 0 | 3) {
            self.cancel_session(session);
            return;
        }
        if !eligible(data, global, pending.item) {
            self.cancel_session(session);
            self.status.stage = Stage::Rejected;
            return;
        }
        // Body 3 has no hand-item update. A drone flag alone cannot leave the
        // Hologuise, and would otherwise deploy unexpectedly after a later exit.
        // Keep this unavailable action explicit instead of leaving latent input.
        if hero.mode == 3 && pending.item == DRONE {
            self.cancel_session(session);
            self.status.stage = Stage::Rejected;
            return;
        }
        if ready(hero, pending.item) {
            self.finish(Stage::Equipped);
            return;
        }
        if pending.dispatched {
            // An in-flight swap needs request=0 so create_hand can run. Never
            // repeat a touch merely because the old item has not disappeared.
            self.status.stage = Stage::Switching;
            return;
        }
        self.status.stage = Stage::Waiting;
        if !allowed(gate) || hero.items.slot.swap == 2 || hero.items.slot.swap_timer > 1
            || session.temp_hand != 0 || session.drone {
            return;
        }
        let disguise_exit = hero.mode == 3 && pending.item != HOLOGUISE;
        if !disguise_exit && (hero.items.slot.state != 2 || hero.items.slot.item.is_none()) {
            return;
        }
        if pending.item == DRONE {
            session.drone = true;
        } else {
            // A prior physical request may have recorded this target without
            // starting put-away. The old ready item is authoritative here.
            if !disguise_exit && hero.items.target == pending.item {
                hero.items.target = hero.items.slot.id;
            }
            self.attempt = Some(Attempt { item: pending.item, held: hero.items.slot.id,
                target: hero.items.target, saved: global.equipped[0],
                previous: global.last_hand_item });
            session.temp_hand = pending.item;
        }
        if !pending.counted {
            let rec = &mut global.move_help[20];
            if hero.items.slot.id != pending.item { rec.count = rec.count.saturating_add(1); }
            let time = scale_ticks(global.play_time) / 600;
            if time > i32::from(rec.time) { rec.time = time as u16; }
            rec.mask |= 1u32.wrapping_shl(global.level as u32 & 31) | 0x8000_0000;
            pending.counted = true;
        }
        pending.dispatched = true;
        self.pending = Some(pending);
        self.status.stage = Stage::Issued;
    }

    pub(super) fn after_tick(&mut self, generation: u64, gameplay: bool,
        hero: &mut Hero, global: &mut Global, session: &mut SessionState) {
        let Some(mut pending) = self.pending else { return; };
        if pending.generation != generation {
            self.cancel();
            return;
        }
        if !gameplay || !matches!(hero.mode, 0 | 3) {
            self.cancel_session(session);
            return;
        }
        if pending.item == DRONE {
            if pending.dispatched && !session.drone { self.finish(Stage::Deployed); }
            return;
        }
        if ready(hero, pending.item) {
            self.finish(Stage::Equipped);
            return;
        }
        if !pending.dispatched { return; }
        if hero.mode == 3 {
            // Hologuise's ordinary body update consumes the hand request and
            // leaves the body after its own timer; do not restart that timer.
            self.attempt = None;
            self.status.stage = Stage::Switching;
            return;
        }
        if hero.items.target == pending.item && matches!(hero.items.slot.state, 0 | 3) {
            self.attempt = None;
            self.status.stage = Stage::Switching;
            return;
        }
        if let Some(attempt) = self.attempt.take() {
            if hero.items.slot.state == 2 && hero.items.slot.id == attempt.held
                && hero.items.target == attempt.item {
                // A swing/fire action can start after before_tick's readiness
                // check. Upstream records target/saved before checking its
                // cooldown. Roll back only this uncommitted attempt, otherwise
                // upstream's next request==target check would discard it.
                hero.items.target = attempt.target;
                if global.equipped[0] == attempt.item {
                    global.equipped[0] = attempt.saved;
                    global.last_hand_item = attempt.previous;
                }
            }
            if session.temp_hand == attempt.item { session.temp_hand = 0; }
            pending.dispatched = false;
            self.pending = Some(pending);
            self.status.stage = Stage::Waiting;
        } else if hero.items.slot.state == 2 && hero.items.slot.item.is_some()
            && hero.items.target != pending.item && session.temp_hand == 0 {
            // A scripted restore or disguise exit superseded the intermediate
            // target. Retry the still-current touch when the hand is ready.
            pending.dispatched = false;
            self.pending = Some(pending);
            self.status.stage = Stage::Waiting;
        }
    }

    fn finish(&mut self, stage: Stage) {
        self.status.stage = stage;
        self.pending = None;
        self.attempt = None;
    }
}

fn ready(hero: &Hero, item: i32) -> bool {
    hero.items.slot.state == 2 && hero.items.slot.id == item && hero.items.slot.item.is_some()
}

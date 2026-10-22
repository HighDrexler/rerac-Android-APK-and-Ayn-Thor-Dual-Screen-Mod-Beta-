//! Run the production companion controller through the original 0.3.6 item
//! lifecycle, including actual put-away, deletion, creation, and a ready item.
#[path = "../../rc-engine/src/android/companion_select.rs"]
mod companion_select;
#[path = "../../rc-engine/src/android/companion_policy.rs"]
mod companion_policy;

use companion_select::{Controller, Stage};
use rc_formats::moby_anim::{MobyAnimClass, MobyFrame, MobyFrameHeader, MobySequence, MobySequenceHeader};
use rc_game::game_state::{Global, SessionState};
use rc_game::hero::{anim::RecordingAnim, items::{items_update, ItemClass, ItemData, ItemDef, ItemEnv, ItemGlobals, NoHits}, Hero};
use rc_game::menus::quick_select::HeroGate;
use rc_game::{moby_runtime::MobyTable, pad::PadState, rng::Rng};

#[test]
fn keeping_the_map_visible_does_not_relax_weapon_or_interaction_guards() {
    for gate in [HeroGate{b13fc:1,..Default::default()},HeroGate{b13f7:1,..Default::default()},HeroGate{f594:1,..Default::default()},HeroGate{b13f4:2,..Default::default()},HeroGate{early_exit:true,..Default::default()}] {
        assert!(companion_policy::world_visible(0,false));
        assert!(!companion_select::allowed(&gate));
    }
}

fn data() -> ItemData {
    let frame = MobyFrame { header: MobyFrameHeader { rate: 1.0, ..Default::default() },
        quats: vec![], scales: vec![], trans: vec![], payload: vec![] };
    let sequence = MobySequence { header: MobySequenceHeader { frame_count: 1, loop_sound: 255, ..Default::default() },
        frames: vec![frame], triggers: vec![] };
    let anim = MobyAnimClass { joint_count: 0, skeleton: vec![], rest: vec![], parent_word: vec![],
        sequences: vec![Some(sequence); 16] };
    let mut defs = vec![ItemDef { slot: -1, ..Default::default() }; 37];
    for id in [8, 10, 11, 12, 24, 31] { defs[id] = ItemDef { slot: 0, o_class: 71 + id as i32, ..Default::default() }; }
    defs[2].slot = 3;
    defs[6].slot = 2;
    defs[28].slot = 1;
    defs[34].slot = 4;
    let classes = [8, 10, 11, 12, 24, 31].into_iter().map(|id|
        ItemClass { o_class: 71 + id, anim: anim.clone(), scale: 1.0, chains: vec![] }).collect();
    ItemData { defs, classes, hero_chains: vec![] }
}

struct Harness {
    control: Controller,
    hero: Hero,
    global: Global,
    session: SessionState,
    data: ItemData,
    table: MobyTable,
    anim: RecordingAnim,
    rng: Rng,
    frame: i32,
    generation: u64,
}

impl Harness {
    fn new(held: i32) -> Self {
        let mut global = Global::zeroed();
        for id in [2, 6, 8, 10, 11, 12, 24, 28, 31, 34] { global.owned[id] = 1; }
        global.equipped[0] = if held == 8 { 11 } else { held };
        global.wrench_held = i32::from(held == 8);
        let mut h = Self { control: Controller::default(), hero: Hero::new(), global,
            session: SessionState { hp: 4, ..Default::default() }, data: data(),
            table: MobyTable::new(vec![], 0), anim: RecordingAnim::default(), rng: Rng::new(),
            frame: 1, generation: 1 };
        h.hero.health = 4;
        h.hero.items.target = held;
        h.game_tick();
        h.assert_held(held);
        h
    }

    fn tap(&mut self, sequence: u64, item: i32) -> bool {
        self.control.accept(sequence, self.generation, item, Some(&self.data), &self.global)
    }

    fn before(&mut self, gameplay: bool) {
        let gate = HeroGate { early_exit: self.session.hp < 1, swap_state: self.hero.items.slot.swap,
            b13fc: self.hero.items.f13fc, b13f7: self.hero.items.f13f7, b13f4: self.hero.mode,
            held_item: self.hero.items.slot.id, ..Default::default() };
        self.control.before_tick(self.generation, gameplay, &gate, &mut self.hero,
            Some(&self.data), &mut self.global, &mut self.session);
    }

    fn game_tick(&mut self) {
        let pad = PadState::default();
        let env = ItemEnv { data: &self.data, pad: &pad, frame: self.frame, hero_moby: 0,
            coll: None, mobys: None, camera: None, camera_up: None, targets: &[] };
        let mut live = ItemGlobals { request: self.session.temp_hand, drone: self.session.drone,
            saved: self.global.equipped[0], previous: self.global.last_hand_item,
            wrench_flag: self.global.wrench_held };
        items_update(&mut self.hero, &mut live, &mut self.table, &self.anim,
            &mut self.rng, &env, &mut NoHits);
        self.session.temp_hand = live.request;
        self.session.drone = live.drone;
        self.global.equipped[0] = live.saved;
        self.global.last_hand_item = live.previous;
        self.global.wrench_held = live.wrench_flag;
        self.frame += 1;
    }

    fn after(&mut self, gameplay: bool) {
        self.control.after_tick(self.generation, gameplay, &mut self.hero, &mut self.global, &mut self.session);
    }

    fn tick(&mut self) { self.before(true); self.game_tick(); self.after(true); }

    fn finish(&mut self, item: i32) {
        for _ in 0..40 {
            self.tick();
            if self.control.status().stage == Stage::Equipped { break; }
        }
        assert_eq!(self.control.status().stage, Stage::Equipped, "{:?}", self.control.status());
        self.assert_held(item);
        assert_eq!(self.session.temp_hand, 0);
        assert_eq!(self.global.equipped[0], item);
        assert_eq!(self.global.wrench_held, i32::from(item == 8));
    }

    fn assert_held(&self, item: i32) {
        assert_eq!(self.hero.items.slot.id, item);
        assert_eq!(self.hero.items.slot.state, 2);
        assert_eq!(self.hero.items.slot.item.as_ref().unwrap().o_class, 71 + item as i16);
    }
}

#[test]
fn wrench_and_weapon_selections_complete_the_real_item_lifecycle() {
    for held in [8, 10, 11] { for wanted in [10, 11, 12] {
        let mut h = Harness::new(held);
        assert!(h.tap(1, wanted));
        h.finish(wanted);
    }}
}

#[test]
fn latest_touch_wins_before_dispatch_and_during_putaway() {
    let mut h = Harness::new(8);
    assert!(h.tap(1, 10));
    assert!(h.tap(2, 11));
    h.tick();
    assert_eq!(h.hero.items.slot.state, 3);
    assert_eq!(h.hero.items.target, 11);
    assert!(h.tap(3, 12));
    h.finish(12);
    assert_eq!(h.control.status().sequence, 3);
}

#[test]
fn accepted_touch_waits_for_cooldowns_and_thrown_wrench_locks() {
    for timer in [2, 10] {
        let mut h = Harness::new(8);
        h.hero.items.slot.swap_timer = timer;
        assert!(h.tap(1, 10));
        h.tick();
        h.assert_held(8);
        assert_eq!(h.session.temp_hand, 0);
        h.finish(10);
    }
    let mut h = Harness::new(8);
    h.hero.items.slot.swap = 2;
    assert!(h.tap(1, 10));
    h.before(true);
    assert_eq!(h.session.temp_hand, 0);
    assert_eq!(h.control.status().stage, Stage::Waiting);
    h.hero.items.slot.swap = 0;
    h.finish(10);
}

#[test]
fn lock_started_after_dispatch_rolls_back_only_uncommitted_attempt() {
    let mut h = Harness::new(8);
    assert!(h.tap(1, 10));
    h.before(true);
    assert_eq!(h.session.temp_hand, 10);
    // The hero can begin a swing between Android's input phase and items_update.
    h.hero.items.slot.swap_timer = 2;
    h.game_tick();
    assert_eq!(h.hero.items.target, 10);
    h.assert_held(8);
    h.after(true);
    assert_eq!(h.hero.items.target, 8);
    assert_eq!(h.global.equipped[0], 11);
    assert_eq!(h.session.temp_hand, 0);
    h.finish(10);
    assert_eq!(h.global.move_help[20].count, 1, "retrying one touch counts once");
}

#[test]
fn pending_request_does_not_block_empty_slot_creation() {
    let mut h = Harness::new(8);
    assert!(h.tap(1, 10));
    h.tick();
    assert_eq!(h.hero.items.slot.state, 3);
    h.tick();
    assert_eq!(h.hero.items.slot.state, 0);
    for _ in 0..2 {
        h.before(true);
        assert_eq!(h.session.temp_hand, 0, "creation must not be blocked by a repeated request");
        h.game_tick();
        h.after(true);
    }
    h.finish(10);
}

#[test]
fn target_alone_never_counts_as_equipped() {
    let mut h = Harness::new(8);
    h.hero.items.target = 10;
    assert!(h.tap(1, 10));
    assert_ne!(h.control.status().stage, Stage::Equipped);
    h.finish(10);
}

#[test]
fn level_or_gameplay_change_cancels_waiting_touch() {
    for change_level in [false, true] {
        let mut h = Harness::new(8);
        assert!(h.tap(1, 10));
        h.hero.items.slot.swap_timer = 10;
        h.tick();
        if change_level { h.generation += 1; }
        h.before(change_level);
        assert_eq!(h.control.status().stage, Stage::Cancelled);
        assert_eq!(h.session.temp_hand, 0);
        for _ in 0..12 { h.tick(); }
        h.assert_held(8);
    }
}

#[test]
fn pausing_without_a_game_tick_clears_only_this_controllers_issued_request() {
    for item in [10, 24] {
        let mut h = Harness::new(8);
        assert!(h.tap(1, item));
        h.before(true);
        assert_eq!(h.control.status().stage, Stage::Issued);
        h.after(false); // GameTick skipped by activity pause/TickBudget.
        assert_eq!(h.control.status().stage, Stage::Cancelled);
        assert_eq!(h.session.temp_hand, 0);
        assert!(!h.session.drone);
        h.tick();
        h.assert_held(8);
    }
    let mut h = Harness::new(8);
    assert!(h.tap(1, 10));
    h.before(true);
    h.session.temp_hand = 12; // A newer physical/game request has another owner.
    h.control.cancel_session(&mut h.session);
    assert_eq!(h.session.temp_hand, 12);
}

#[test]
fn invalid_unowned_and_nonhand_items_never_enter_hand_slot() {
    let mut h = Harness::new(8);
    for (i, item) in [-1, 0, 2, 6, 28, 34, 37, 100, 13].into_iter().enumerate() {
        assert!(!h.tap(i as u64 + 1, item), "item {item}");
        assert_eq!(h.control.status().stage, Stage::Rejected);
        h.tick();
        h.assert_held(8);
    }
    assert!(!companion_select::eligible(None, &h.global, 10));
    h.data.defs.truncate(10);
    assert!(!companion_select::eligible(Some(&h.data), &h.global, 10));
}

#[test]
fn drone_is_a_single_action_and_duplicate_sequence_does_not_repeat() {
    let mut h = Harness::new(8);
    assert!(h.tap(1, 24));
    h.tick();
    assert_eq!(h.control.status().stage, Stage::Deployed);
    h.assert_held(8);
    assert_eq!(h.session.temp_hand, 0);
    assert!(!h.session.drone);
    assert!(!h.tap(1, 24));
    for _ in 0..5 { h.tick(); }
    assert_eq!(h.global.move_help[20].count, 1);
    assert_eq!(h.control.status().stage.as_str(), "deployed");
}

#[test]
fn drone_in_disguise_rejects_without_leaving_a_latent_deployment() {
    let mut h = Harness::new(8);
    h.hero.mode = 3;
    assert!(h.tap(1, 24));
    h.before(true);
    assert_eq!(h.control.status().stage, Stage::Rejected);
    assert!(!h.session.drone);
    assert_eq!(h.session.temp_hand, 0);
    h.hero.mode = 0;
    for _ in 0..5 { h.tick(); }
    h.assert_held(8);
    assert!(!h.session.drone);
    assert_eq!(h.global.move_help[20].count, 0);
}

#[test]
fn losing_eligibility_after_dispatch_clears_only_owned_pending_input() {
    for item in [10, 24] {
        let mut h = Harness::new(8);
        assert!(h.tap(1, item));
        h.before(true);
        h.global.owned[item as usize] = 0;
        h.before(true); // Previous GameTick was skipped; our input is still set.
        assert_eq!(h.control.status().stage, Stage::Rejected);
        assert!(!h.session.drone);
        assert_eq!(h.session.temp_hand, 0);
        h.tick();
        h.assert_held(8);
    }
}

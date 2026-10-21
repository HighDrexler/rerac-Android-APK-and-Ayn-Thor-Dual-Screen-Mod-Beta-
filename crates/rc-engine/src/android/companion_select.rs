//! Companion selection contract recovered from the user-verified v1.0 APK.
//!
//! Reference: ReRAC-Mobile-v1.0.zip SHA256
//! 327e92b042e135dba115f332104ee947978d6aca9b3a4fa56c8e48e29bf623cc.
//! Its x86_64 selection helper (0x2de16f0) writes the session request directly;
//! it does not open/clone the physical wheel or require a pause-menu instance.
//! The JNI adapter separately validates snapshot generation and displayed slot.
use rc_game::game_state::{Global, SessionState};
use rc_game::menus::{quick_select::{HeroGate, QsItems}, scale_ticks};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Selection { Hand(i32), Drone }

/// The v1.0 companion accepted interaction owner 5 as well as owner 0. This
/// differs intentionally from the physical Triangle-wheel opening predicate.
pub(super) fn allowed(g:&HeroGate)->bool {
    !g.early_exit && g.swap_state!=2 && matches!(g.f594,0|5)
        && (g.b13fc==0 || g.b13f4==1 || g.s3f502!=0)
        && (g.b13f7==0 || g.b13f4==1 || g.s3f502!=0)
        && matches!(g.b13f4,0|3)
}

pub(super) fn available(items:&QsItems,g:&Global,item:i32)->bool {
    (1..=36).contains(&item)
        && g.owned.get(item as usize).is_some_and(|&v|v!=0)
        && items.icon.get(item as usize).is_some_and(|&v|v!=0)
}

pub(super) fn apply(items:&QsItems,item:i32,gate:&HeroGate,g:&mut Global,s:&mut SessionState)->Option<Selection> {
    if !allowed(gate) || !available(items,g,item) {return None;}
    // Preserve the original quick-select usage statistics and ammo clamp.
    let rec=&mut g.move_help[20];
    if gate.held_item!=item {rec.count=rec.count.saturating_add(1);}
    let time=scale_ticks(g.play_time)/600;
    if time>rec.time as i32 {rec.time=time as u16;}
    rec.mask|=1u32.wrapping_shl(g.level as u32&31)|0x8000_0000;
    let selected=if item==24 {s.drone=true;Selection::Drone}
        else {s.temp_hand=item;Selection::Hand(item)};
    if items.has_ammo.get(item as usize)==Some(&true) {
        if let Some(&max)=items.max_ammo.get(item as usize) {
            g.ammo[item as usize]=g.ammo[item as usize].min(max as i32);
        }
    }
    Some(selected)
}

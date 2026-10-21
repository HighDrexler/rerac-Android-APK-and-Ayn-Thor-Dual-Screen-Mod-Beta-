//! Exercise the actual Android companion adapter against the real hand swap.
//! No APK UI clicks are counted as accepted equipment changes here.
#[path="../../rc-engine/src/android/companion_select.rs"]
mod companion_select;
#[path="../../rc-engine/src/android/companion_policy.rs"]
mod companion_policy;
use rc_game::game_state::{Global,SessionState};
use rc_game::menus::quick_select::{HeroGate,QsItems};
use rc_game::hero::{Hero,items::{ItemData,ItemEnv,ItemGlobals,update_hand_selected}};
use rc_game::{pad::PadState,rng::Rng};

fn items()->QsItems {
    QsItems {icon:(0..37).map(|i|if i==0 {0}else{60000+i}).collect(),name:vec![0;37],gold_name:vec![0;37],has_ammo:vec![true;37],max_ammo:vec![40;37]}
}

#[test]
fn reference_owner_zero_and_five_taps_reach_real_hand_swap_on_both_pages() {
    let icons=items();let selections:Vec<i32>=(9..25).collect();
    for owner in [0,5] {for (slot,&item) in selections.iter().enumerate() {
        let mut global=Global::zeroed();global.owned[item as usize]=1;
        global.quick_select.copy_from_slice(&selections[..8]);global.ammo[item as usize]=70;
        let saved=global.quick_select;let mut session=SessionState {temp_hand:8,..Default::default()};
        let gate=HeroGate {f594:owner,held_item:8,..Default::default()};
        assert_eq!(companion_policy::selected_slot(&selections,slot%8,item),Some(slot));
        let result=companion_select::apply(&icons,item,&gate,&mut global,&mut session);
        assert_eq!(global.quick_select,saved,"companion does not rewrite assignments");
        assert_eq!(global.ammo[item as usize],40);
        if item==24 {assert_eq!(result,Some(companion_select::Selection::Drone));assert!(session.drone);assert_eq!(session.temp_hand,8);continue;}
        assert_eq!(result,Some(companion_select::Selection::Hand(item)));
        let mut hero=Hero::new();hero.items.slot.id=8;
        let data=ItemData::default();let pad=PadState::default();
        let env=ItemEnv {data:&data,pad:&pad,frame:18,hero_moby:0,coll:None,mobys:None,camera:None,camera_up:None,targets:&[]};
        let mut live=ItemGlobals {request:session.temp_hand,wrench_flag:1,..Default::default()};
        update_hand_selected(&mut hero,&mut live,&mut Rng::new(),&env);
        assert_eq!(hero.items.target,item,"owner {owner}, page {}, slot {}",slot/8,slot%8);
        assert_eq!(live.wrench_flag,0);
    }}
}

#[test]
fn reference_rejects_unowned_iconless_or_blocked_selections_without_mutation() {
    let icons=items();let mut global=Global::zeroed();let mut session=SessionState {temp_hand:8,..Default::default()};
    assert_eq!(companion_select::apply(&icons,10,&HeroGate::default(),&mut global,&mut session),None);
    global.owned[10]=1;
    for gate in [HeroGate{f594:1,..Default::default()},HeroGate{early_exit:true,..Default::default()},HeroGate{swap_state:2,..Default::default()},HeroGate{b13fc:1,..Default::default()},HeroGate{b13f7:1,..Default::default()},HeroGate{b13f4:2,..Default::default()}] {
        assert_eq!(companion_select::apply(&icons,10,&gate,&mut global,&mut session),None);
    }
    let mut no_icon=icons;no_icon.icon[10]=0;
    assert_eq!(companion_select::apply(&no_icon,10,&HeroGate::default(),&mut global,&mut session),None);
    for item in [-1,0,37] {assert_eq!(companion_select::apply(&no_icon,item,&HeroGate::default(),&mut global,&mut session),None);}
    assert_eq!(session.temp_hand,8);assert_eq!(global.move_help[20].count,0);
}

#[test]
fn disguise_and_script_override_match_reference() {
    let icons=items();let mut global=Global::zeroed();global.owned[10]=1;
    for gate in [HeroGate{b13f4:3,..Default::default()},HeroGate{b13fc:1,b13f7:1,s3f502:1,..Default::default()}] {
        let mut session=SessionState::default();
        assert_eq!(companion_select::apply(&icons,10,&gate,&mut global,&mut session),Some(companion_select::Selection::Hand(10)));
    }
}

//! Pure Android second-screen policy: compiled as native unit tests in CI.
//! Keep the protocol stable while upstream ReRAC changes menu implementations.

/// Native menu protocol's visible state and Java touch state. The in-engine
/// cinematics (mode 2) carry real dialogue, not a blank "video only" screen.
/// PSS movies (mode 1) and ship sequences (mode 6) still use cinematic input.
pub fn display_mode(mode:i32,front_end:bool)->(bool,u32) {
    let dialogue=mode==2||mode==4;
    let visible_menu=front_end||dialogue||matches!(mode,3|7);
    let touch=if matches!(mode,1|6){2}else if visible_menu{1}else{0};
    (visible_menu,touch)
}

/// A tap may give either the absolute index or its location in an 8-slot page.
/// Earlier builds compared only absolute positions, rejecting page 2+ taps.
/// Match exactly by item as well so a recycled/stale page selection is rejected.
pub fn selected_slot(selection:&[i32],slot:usize,item:i32)->Option<usize> {
    if item<0{return None}
    if selection.get(slot)==Some(&item){return Some(slot)}
    if slot<8 {
        return selection.iter().enumerate().find(|(i,id)|i%8==slot&&**id==item).map(|(i,_)|i)
    }
    None
}

/// The Java title screen already provides its Start/Load text. Native glyphs
/// from the game title must not draw a second overlapping copy underneath.
pub fn native_frontend_primitive(title_only:bool,is_font:bool)->bool { !title_only || !is_font }

/// FrontEnd modes 3/4 are the real menu and confirmation dialog, not the title.
/// Unknown state preserves text rather than silently hiding instructions.
pub fn title_only(front_end:bool,mode:Option<i32>)->bool {
    front_end && mode.is_some_and(|m| !matches!(m,3|4))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn cutscene_and_dialogue_keep_companion_active(){
        assert_eq!(display_mode(2,false),(true,1));
        assert_eq!(display_mode(4,false),(true,1));
        assert_eq!(display_mode(3,false),(true,1));
        assert_eq!(display_mode(0,true),(true,1));
        assert_eq!(display_mode(0,false),(false,0));
        assert_eq!(display_mode(1,false),(false,2));
        assert_eq!(display_mode(6,false),(false,2));
    }
    #[test] fn page_two_quick_select_touches_resolve(){
        let items=[10,11,12,13,14,15,16,17,20,21,22,23,24,25,26,27];
        assert_eq!(selected_slot(&items,3,13),Some(3));
        assert_eq!(selected_slot(&items,3,23),Some(11));
        assert_eq!(selected_slot(&items,11,23),Some(11));
        assert_eq!(selected_slot(&items,3,99),None);
        assert_eq!(selected_slot(&items,11,13),None);
    }
    #[test] fn front_end_save_and_overwrite_dialogs_keep_text(){
        for mode in [3,4] {
            assert!(native_frontend_primitive(title_only(true,Some(mode)),true));
        }
        assert!(native_frontend_primitive(title_only(true,None),true));
        assert!(!native_frontend_primitive(title_only(true,Some(1)),true));
        assert!(native_frontend_primitive(title_only(false,Some(1)),true));
    }
    #[test] fn preserve_title_panels_but_avoid_duplicate_text(){
        assert!(!native_frontend_primitive(true,true));
        assert!(native_frontend_primitive(true,false));
        assert!(native_frontend_primitive(false,true));
    }
}

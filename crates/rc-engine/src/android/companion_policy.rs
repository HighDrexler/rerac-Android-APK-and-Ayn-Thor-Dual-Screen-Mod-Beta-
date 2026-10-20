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
pub fn native_frontend_primitive(front_end:bool,is_font:bool)->bool { !front_end || !is_font }

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
    #[test] fn preserve_title_panels_but_avoid_duplicate_text(){
        assert!(!native_frontend_primitive(true,true));
        assert!(native_frontend_primitive(true,false));
        assert!(native_frontend_primitive(false,true));
    }

    #[test]
    fn all_published_items_round_trip_through_local_and_absolute_slots() {
        // Include empty, partial, full and third/fourth pages. The companion
        // appends owned inventory beyond the two initial eight-slot pages.
        for len in 0..=33 {
            let items: Vec<i32> = (1..=len).collect();
            for (absolute, &item) in items.iter().enumerate() {
                assert_eq!(selected_slot(&items, absolute, item), Some(absolute));
                assert_eq!(selected_slot(&items, absolute % 8, item), Some(absolute));
            }
        }
    }

    #[test]
    fn a_matching_item_on_the_wrong_radial_slot_is_rejected() {
        let items: Vec<i32> = (1..=25).collect();
        for (absolute, &item) in items.iter().enumerate() {
            for local in 0..8 {
                if local != absolute % 8 {
                    assert_eq!(selected_slot(&items, local, item), None);
                }
            }
            // An absolute position cannot be silently remapped to another page.
            for other in 8..items.len() {
                if other != absolute {
                    assert_eq!(selected_slot(&items, other, item), None);
                }
            }
        }
    }

    #[test]
    fn stale_item_slot_pairs_are_rejected_after_inventory_changes() {
        let mut items: Vec<i32> = (1..=17).collect();
        assert_eq!(selected_slot(&items, 0, 17), Some(16));
        items.swap(15, 16);
        assert_eq!(selected_slot(&items, 0, 17), None);
        assert_eq!(selected_slot(&items, 16, 17), None);
        assert_eq!(selected_slot(&items, 7, 17), Some(15));
        items[15] = 0; // Empty padding after the item leaves the published page.
        assert_eq!(selected_slot(&items, 7, 17), None);
    }

    #[test]
    fn invalid_input_and_empty_pages_cannot_resolve_to_an_item() {
        for slot in [0, 7, 8, 24, usize::MAX] {
            assert_eq!(selected_slot(&[], slot, 1), None);
            assert_eq!(selected_slot(&[0; 24], slot, 1), None);
            assert_eq!(selected_slot(&[1, 2, 3], slot, -1), None);
        }
        assert_eq!(selected_slot(&[1, 2, 3], usize::MAX, 1), None);
        assert_eq!(selected_slot(&[1, 2, 3], 0, i32::MAX), None);
    }

    #[test]
    fn returning_from_dialogue_restores_each_modes_touch_protocol() {
        // A sequence catches accidental retention of dialogue/menu visibility.
        let sequence = [
            (0, false, (false, 0)), // Gameplay.
            (2, false, (true, 1)),  // In-engine dialogue.
            (4, false, (true, 1)),  // Freeze dialogue.
            (0, false, (false, 0)),
            (3, false, (true, 1)),  // Pause menu.
            (5, false, (false, 0)), // Vendor retains its existing protocol.
            (7, false, (true, 1)),  // Slideshow.
            (1, false, (false, 2)), // PSS movie.
            (6, false, (false, 2)), // Ship sequence.
            (0, true, (true, 1)),   // Title Start/Load menu.
            (1, true, (true, 2)),   // Title movie still needs cinematic input.
            (0, false, (false, 0)),
        ];
        for (mode, front_end, expected) in sequence {
            assert_eq!(display_mode(mode, front_end), expected);
        }
    }
}

//! Prepare companion pages for the original RAC1 quick-select renderer.
use rc_game::{game_state::Global, menus::quick_select::QuickSelect};

/// Select the equipped item (or current Clank command), never an unconfirmed tap.
/// Page-local assignments also make the native name/ammo display correct on extra pages.
pub fn page(source: &QuickSelect, items: &[i32], global: &Global, held: i32, command: Option<i32>) -> (QuickSelect, Global) {
    let mut ring = source.clone();
    let mut state = global.clone();
    prepare(&mut ring, items, &mut state, held, command);
    (ring, state)
}

/// Reuse one isolated ring/global copy for all pages of the same publication.
/// The original game state and renderer remain untouched.
pub fn prepare(ring: &mut QuickSelect, items: &[i32], state: &mut Global, held: i32, command: Option<i32>) {
    ring.open = true;
    ring.command_page = command.is_some();
    ring.fade = ring.consts.steps.clamp(0, 255) as u8;
    ring.text_fade = ring.fade;
    state.quick_select.fill(0);
    for (i, entry) in ring.entries.iter_mut().enumerate() {
        let item = items.get(i).copied().unwrap_or(0);
        state.quick_select[i] = item;
        entry.item = item;
        entry.icon = usize::try_from(item).ok().filter(|&id| id != 0)
            .and_then(|id| ring.items.icon.get(id)).copied().unwrap_or(0) as i32;
    }
    let selected = command.unwrap_or(held);
    ring.sel = if selected <= 0 { -1 } else if let Some(commands) = ring.commands.as_ref().filter(|_| command.is_some()) {
        commands.entries.iter().position(|entry| entry.item == selected && entry.icon != 0).map_or(-1, |i| i as i32)
    } else if command.is_some() { -1 } else {
        ring.entries.iter().position(|entry| entry.item == selected && entry.icon != 0).map_or(-1, |i| i as i32)
    };
}

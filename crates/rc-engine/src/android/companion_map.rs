//! Companion-only image cache. Camera, markers and objectives remain live;
//! the original map composer runs whenever any pixel input changes.
use rc_game::{map::MapFile, menus::pause::{map_page, PageMenu}, game_state::GameState};
use std::sync::Arc;

struct ImageState {
    level: i32,
    map_o_matic: bool,
    file: MapFile,
    rendered_file: MapFile,
    mask: Box<[u8]>,
    palette: Option<Vec<u8>>,
}

impl ImageState {
    fn capture(menu: &mut PageMenu) -> Option<Self> {
        let index=rc_game::map::file_index(menu.map.state.level.max(0) as usize,menu.map.state.map_o_matic);
        let rendered_file=menu.map.file(index)?;
        let state = &menu.map.state;
        if menu.dest != state.level { return None; }
        Some(Self {
            level: state.level, map_o_matic: state.map_o_matic,
            file: state.file.clone()?, rendered_file, mask: state.mask.0.clone(),
            palette: menu.map.palettes.get(state.level as usize).cloned().flatten(),
        })
    }
    fn same_source(&self, menu: &PageMenu) -> bool {
        let state = &menu.map.state;
        menu.dest == self.level && state.level == self.level
            && state.map_o_matic == self.map_o_matic
            && state.file.as_ref().is_some_and(|f| Arc::ptr_eq(&f.bytes, &self.file.bytes))
            && menu.map.palettes.get(self.level as usize).and_then(Option::as_ref) == self.palette.as_ref()
    }
    fn update(&mut self, menu: &mut PageMenu) -> bool {
        let index=rc_game::map::file_index(menu.map.state.level.max(0) as usize,menu.map.state.map_o_matic);
        let Some(rendered_file)=menu.map.file(index) else{return false;};
        let state=&menu.map.state;
        let Some(file)=state.file.as_ref().filter(|_|menu.dest==state.level) else{return false;};
        self.level=state.level;self.map_o_matic=state.map_o_matic;self.file=file.clone();self.rendered_file=rendered_file;
        self.mask.clone_from(&state.mask.0);
        let palette=menu.map.palettes.get(state.level as usize).and_then(Option::as_ref);
        match (self.palette.as_mut(),palette) {
            (Some(saved),Some(current))=>saved.clone_from(current),
            _=>self.palette=palette.cloned(),
        }
        true
    }
}

#[derive(Default)]
pub struct ImageCache { image: Option<ImageState> }

impl ImageCache {
    /// Returns true only when a new picture was composed. Exact mask comparison
    /// avoids hash collisions; no gameplay state, mask or RNG is mutated.
    pub fn refresh(&mut self, menu: &mut PageMenu, game: &GameState) -> bool {
        if menu.map.shown == menu.dest && menu.map.picture.is_some()
            && self.image.as_ref().is_some_and(|key| key.same_source(menu)) {
            let key=self.image.as_mut().unwrap();
            let changed=menu.map.state.mask.0!=key.mask;
            if changed {
                // Fog changes only affect the flat map. The original three
                // globe layers depend on the file, not fog; keep those layers.
                let picture=rc_game::map::compose(&key.rendered_file,&menu.map.state.mask);
                let palette=key.palette.as_deref().unwrap_or_else(||key.rendered_file.palette());
                let texture=rc_formats::hud::decode_indexed8_raw(&picture,rc_game::map::SIZE as u32,rc_game::map::SIZE as u32,palette).ok();
                if let Some(texture)=texture {
                    menu.map.picture=Some(Arc::new(texture));menu.map.composed+=1;
                    key.mask.clone_from(&menu.map.state.mask.0);
                } else {
                    self.image=None;
                    return self.refresh(menu,game);
                }
            }
            // compose() previously did this on every refresh. Mission and hook
            // changes must still update icons even when the fog is unchanged.
            map_page::setup_markers(menu, game, menu.dest, false);
            return changed;
        }
        let before = menu.map.composed;
        menu.map.shown = -2;
        map_page::compose(menu, game);
        if menu.map.shown == menu.dest && menu.map.picture.is_some() {
            if let Some(key)=self.image.as_mut() {
                if !key.update(menu) {self.image=None;}
            } else {self.image=ImageState::capture(menu);}
        } else { self.image = None; }
        menu.map.composed != before
    }
}

/// Keep the companion's image keys separate from the primary pause map.
pub fn image_revision(frame: u64) -> u64 { (1u64 << 39) | (frame & ((1u64 << 39) - 1)) }

/// Exact native-to-Android texture conversion, shared with the benchmark.
pub fn android_pixels(texture: &rc_formats::texture::Texture) -> Vec<i32> {
    let n=(texture.width as usize).saturating_mul(texture.height as usize);
    if n>1024*1024 || texture.rgba.len()<n*4 {return vec![0,0];}
    let mut out=Vec::with_capacity(n+2);out.push(texture.width as i32);out.push(texture.height as i32);
    out.extend(texture.rgba.chunks_exact(4).take(n).map(|p| {
        let a=(u32::from(p[3])*255/128).min(255);
        ((a<<24)|(u32::from(p[0])<<16)|(u32::from(p[1])<<8)|u32::from(p[2])) as i32
    }));out
}

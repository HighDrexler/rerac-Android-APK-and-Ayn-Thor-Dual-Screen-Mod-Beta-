//! The boot's front end (`FUN_0026ede8`, the first code `main` runs): the splash screens, the Insomniac movie, the
//! title (the couch movies with "Press START") and the main menu. Spec: docs/plan/rac2_bringup.md §5.
//!
//! One [`FrontEnd::tick`] per 60 Hz frame. The engine plays the movies it asks for ([`FrontEnd::movie`]) and tells it
//! when one has ended; the front end decides the skip itself, as the PSS player's loop does (`FUN_0034fd90`).
//!
//! **Timeline** (frames): splash 10 fades in (`FUN_0026ea00`: black at alpha 0x80 → 0 in steps of 2, 65 frames),
//! holds 45, fades out (`FUN_0027d9b0(30)`: one frame, 30 frames of black at `0x80 − i·0x80/30`, i = 29..0, one
//! black frame), 30 black frames; splash 11 the same with a 25-frame hold; one frame; then movie 0xb2 (the
//! Insomniac logo, not skippable: `0x1a7478 = 3`). The game-mode loop starts with its idle counter at 0x708, so the
//! first mode-0 frame starts the title.
//!
//! **Title** (mode 0 → 1): the attract playlist (`FUN_0026ec58`, 24 entries from three shuffles of 0xb3..0xbb),
//! played from entry 1 and wrapping after 24, "Press START" over every picture (`FUN_0027db38`). A press ends it
//! (`0x1a7478 = 2`: a button down with none down the frame before, once the pad has been seen released), back to
//! mode 0: the backdrop and the main menu.
//!
//! **Main menu** (mode 3, kind 6, page 0x259438): sub-states 1, 2, 3 (one frame each: the backdrop darkened) then 4,
//! the page: Up / Down move the selection (wrapping; sound 3), Cross takes a row. 0x708 idle frames go back to the
//! title.

use crate::gui::{self, Picture, Quad};
use rc_formats::rac2::boot::{Glyph, MenuRow};
use rc_formats::rac2::gui_sprite::Sprite;
use rc_formats::rac2::front_end::Strings;
use std::collections::HashMap;

/// PS2 pad bits (libpad, active high here).
pub mod button {
    pub const TRIANGLE: u16 = 0x10;
    pub const CROSS: u16 = 0x40;
    pub const START: u16 = 0x800;
    pub const UP: u16 = 0x1000;
    pub const DOWN: u16 = 0x4000;
}

/// The Insomniac logo movie.
pub const MOVIE_LOGO: u32 = 0xb2;
/// The menu's idle frames before the title comes back (`FUN_0026ede8`, mode 3).
pub const IDLE_FRAMES: u32 = 0x708;
/// "Press START" (`FUN_0027db38`).
pub const TEXT_PRESS_START: u32 = 0x2db6;
/// The main menu's buttons (`FUN_00342bf0`: GUI sprites 0x73 and 0x74).
pub const SPRITE_FILL: u32 = 0x73;
pub const SPRITE_FRAME: u32 = 0x74;

/// What the front end draws with: the boot ELF's tables and the front end's data.
pub struct Assets {
    /// Texture slot 1's glyphs (the menus) and slot 2's ("Press START").
    pub glyphs: [Glyph; 256],
    pub glyphs_2: [Glyph; 256],
    pub strings: Strings,
    pub main_menu: Vec<MenuRow>,
    pub sprites: HashMap<u32, Sprite>,
}

/// A request to the engine.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Request {
    /// Sound effect n of the front end's set (`FUN_002e6c90(n, 0, 0)`).
    Sound(u32),
    /// A main-menu row was taken (its kind and target); nothing past the main menu is ported yet.
    Row { kind: u32, target: u32 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    /// Splash i, frame k of its timeline.
    Splash { i: usize, k: u32 },
    /// The frame between the second splash's fade and the movie.
    Gap,
    Logo,
    Title,
    /// The main menu's sub-state (1..=4).
    Menu { sub: u8 },
}

pub struct FrontEnd {
    phase: Phase,
    /// Frames in the current movie (`0x1b87f4`: the "Press START" pulse).
    movie_frames: u32,
    /// The attract playlist (entries 1..=24; entry 0 unused) and the one playing.
    playlist: [u16; 25],
    playing: usize,
    /// The skip rule's state: the pad was seen released since the movie started.
    released: bool,
    /// Last frame's held buttons.
    held_before: u16,
    /// The menu's selected row (`list + 0x60`) and idle frames (`FUN_0026ede8`'s counter).
    selected: usize,
    idle: u32,
    /// The selected button's pulse counter (`0x1a9e94`, reset by Up / Down).
    pulse: u32,
    /// The C library's `rand` state (`FUN_001163b0`).
    rand: u32,
}

/// The splash timeline: fade in (65), hold, fade out (32), black (30, the first splash only).
const FADE_IN: u32 = 65;
const FADE_OUT: u32 = 32;
const BLACK: u32 = 30;
const HOLD: [u32; 2] = [45, 25];

impl FrontEnd {
    /// `seed`: `FUN_0026ec58`'s `srand` value from the console clock (BCD bytes): `(sec + 1)·(hour + 1)·(min + 1)·
    /// (day + 1)·(month + 1)`; see [`clock_seed`].
    pub fn new(seed: u32) -> FrontEnd {
        FrontEnd {
            phase: Phase::Splash { i: 0, k: 0 },
            movie_frames: 0,
            playlist: [0; 25],
            playing: 1,
            released: false,
            held_before: 0,
            selected: 0,
            idle: 0,
            pulse: 0,
            rand: seed,
        }
    }

    /// The movie to play now (the engine keeps playing it until [`FrontEnd::tick`] is told it ended or the front end
    /// stops asking).
    pub fn movie(&self) -> Option<u32> {
        match self.phase {
            Phase::Logo => Some(MOVIE_LOGO),
            Phase::Title => Some(self.playlist[self.playing] as u32),
            _ => None,
        }
    }

    /// `FUN_002a8648(n)`: `rand() >> 16 & 0x7fff` mod n.
    fn rand_mod(&mut self, n: u32) -> u32 {
        self.rand = self.rand.wrapping_mul(0x41c6_4e6d).wrapping_add(0x3039);
        ((self.rand & 0x7fff_ffff) >> 16 & 0x7fff) % n
    }

    /// `FUN_0026eb98`: a random order of 0, 1, 2.
    fn shuffle3(&mut self) -> [u16; 3] {
        let mut p = match self.rand_mod(3) { 0 => [0, 1, 2], 1 => [1, 0, 2], _ => [2, 1, 0] };
        if self.rand_mod(2) != 0 { p.swap(1, 2); }
        p
    }

    /// `FUN_0026ec58`: the attract playlist (the RNG seeded at the call).
    fn build_playlist(&mut self) {
        let l = &mut [0u16; 25];
        let p = self.shuffle3();
        for (k, at) in [16, 20, 24, 4, 8, 12].into_iter().enumerate() { l[at] = p[k % 3] + 0xb9; }
        let (first, second) = if self.rand_mod(2) == 0 { (0xb6, 0xb3) } else { (0xb3, 0xb6) };
        let p = self.shuffle3();
        for (k, at) in [17, 19, 22, 1, 3, 6, 9, 11, 14].into_iter().enumerate() { l[at] = p[k % 3] + first; }
        let p = self.shuffle3();
        for (k, at) in [18, 21, 23, 2, 5, 7, 10, 13, 15].into_iter().enumerate() { l[at] = p[k % 3] + second; }
        self.playlist = *l;
        self.playing = 1;
    }

    fn start_title(&mut self) {
        self.build_playlist();
        self.phase = Phase::Title;
        self.movie_frames = 0;
        self.released = false;
    }

    /// One frame. `held`: the pad's buttons down now; `movie_ended`: the movie asked for last frame has ended.
    pub fn tick(&mut self, held: u16, movie_ended: bool, assets: &Assets) -> Vec<Request> {
        let pressed = held & !self.held_before;
        let mut req = Vec::new();
        match self.phase {
            Phase::Splash { i, k } => {
                let len = FADE_IN + HOLD[i] + FADE_OUT + if i == 0 { BLACK } else { 0 };
                self.phase = if k + 1 < len { Phase::Splash { i, k: k + 1 } } else if i == 0 { Phase::Splash { i: 1, k: 0 } } else { Phase::Gap };
            }
            Phase::Gap => {
                self.phase = Phase::Logo;
                self.movie_frames = 0;
            }
            Phase::Logo => {
                self.movie_frames += 1;
                // Mode 0 with the idle counter at 0x708: the title.
                if movie_ended { self.start_title(); }
            }
            Phase::Title => {
                self.movie_frames += 1;
                if held == 0 { self.released = true; }
                if self.released && held != 0 && self.held_before == 0 {
                    // Back to mode 0, counter reset: the backdrop and the menu.
                    self.phase = Phase::Menu { sub: 1 };
                    self.idle = 0;
                } else if movie_ended {
                    self.playing = if self.playing >= 24 { 1 } else { self.playing + 1 };
                    self.movie_frames = 0;
                }
            }
            Phase::Menu { sub } if sub < 4 => self.phase = Phase::Menu { sub: sub + 1 },
            Phase::Menu { .. } => {
                self.idle += 1;
                self.pulse += 1;
                let n = assets.main_menu.len();
                // `FUN_00348cb8`: every row is enabled (the UI heap is filled with 0xcd and the flags never cleared).
                if pressed & (button::UP | button::DOWN) != 0 && n > 0 {
                    req.push(Request::Sound(3));
                    self.selected = if pressed & button::UP != 0 { (self.selected + n - 1) % n } else { (self.selected + 1) % n };
                    self.pulse = 0;
                } else if pressed & button::CROSS != 0 {
                    if let Some(r) = assets.main_menu.get(self.selected) { req.push(Request::Row { kind: r.kind, target: r.target }); }
                }
                // `FUN_0026ede8`, mode 3: any button down resets the counter.
                if held != 0 { self.idle = 0; }
                if self.idle >= IDLE_FRAMES { self.start_title(); }
            }
        }
        self.held_before = held;
        req
    }

    /// The frame's 2D draws.
    pub fn draw(&self, a: &Assets) -> Vec<Quad> {
        let mut out = Vec::new();
        match self.phase {
            Phase::Splash { i, k } => {
                gui::picture(&mut out, Picture::Splash(i));
                let hold = HOLD[i];
                let alpha = if k < FADE_IN {
                    Some(0x80 - 2 * k)
                } else if k < FADE_IN + hold + 1 {
                    None
                } else if k < FADE_IN + hold + 1 + 30 {
                    let i = 29 - (k - FADE_IN - hold - 1);
                    Some(0x80 - i * 0x80 / 30)
                } else {
                    Some(0x80)
                };
                if let Some(al) = alpha.filter(|&al| al > 0) { gui::fill(&mut out, [0, 0, 0, al as u8]); }
            }
            Phase::Gap => gui::fill(&mut out, [0, 0, 0, 0x80]),
            Phase::Logo => gui::picture(&mut out, Picture::Movie),
            Phase::Title => {
                gui::picture(&mut out, Picture::Movie);
                // `FUN_0027db38`: t = (n mod 100)·0.02 − 1, f = (cos(π·t) + 1) / 2.
                let t = (self.movie_frames % 100) as f32 * 0.02 - 1.0;
                let f = ((t.clamp(-1.0, 1.0) * std::f32::consts::PI).cos() + 1.0) * 0.5;
                let c = gui::lerp_colour(f, 0x7fe0_e0e0, 0x5ff0_c070);
                gui::text(&mut out, &a.glyphs_2, 2, a.strings.get(TEXT_PRESS_START), 60.0, 328.0, 1.0, c);
            }
            Phase::Menu { sub } => {
                gui::picture(&mut out, Picture::Backdrop);
                gui::fill(&mut out, [0, 0, 0, 0x38]);
                if sub == 4 { self.draw_menu(a, &mut out); }
            }
        }
        out
    }

    /// `FUN_00342e48` then `FUN_00348e70`: per row the two button sprites at (150, 120 + 60·i), keyframe 0.25; then
    /// the labels centred on x 250 at y 131 + 60·i.
    fn draw_menu(&self, a: &Assets, out: &mut Vec<Quad>) {
        let (fill, frame) = (a.sprites.get(&SPRITE_FILL), a.sprites.get(&SPRITE_FRAME));
        for i in 0..a.main_menu.len() {
            let pos = [250.0 - 100.0, 120.0 + 60.0 * i as f32];
            if let Some(s) = fill { gui::sprite(out, s, pos, 0.25, [1.0, 1.0], gui::rgba(0x6044_2d00)); }
            // `FUN_002aa3f0(0x20fffeed, 0x70fffeed, 0x14)`: a triangle wave over 20 frames each way.
            let c = if i == self.selected {
                let k = self.pulse % 40;
                let t = if k < 20 { k as f32 / 20.0 } else { 1.0 - (k - 20) as f32 / 20.0 };
                gui::lerp_colour(t, 0x70ff_feed, 0x20ff_feed)
            } else {
                gui::rgba(0x55f0_c070)
            };
            if let Some(s) = frame { gui::sprite(out, s, pos, 0.25, [1.0, 1.0], c); }
        }
        for (i, r) in a.main_menu.iter().enumerate() {
            let t = a.strings.get(r.text);
            let w = gui::text_width(&a.glyphs, t, r.scale);
            let c = if i == self.selected { 0x70ff_feed } else { 0x80f0_f0f0 };
            gui::text(out, &a.glyphs, 1, t, (250 - (w >> 1)) as f32, 120.0 + 11.0 + 60.0 * i as f32, r.scale, gui::rgba(c));
        }
    }
}

/// `FUN_0026ec58`'s seed from the console clock's BCD bytes `[stat, sec, min, hour, pad, day, month, year]`.
pub fn clock_seed(c: [u8; 8]) -> u32 {
    (c[1] as u32 + 1).wrapping_mul(c[3] as u32 + 1).wrapping_mul(c[2] as u32 + 1).wrapping_mul(c[5] as u32 + 1).wrapping_mul(c[6] as u32 + 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rc_formats::rac2::boot::MenuRow;

    fn assets() -> Assets {
        let row = |text| MenuRow { scale: 1.0, text, kind: 4, target: 0 };
        Assets { glyphs: [Glyph::default(); 256], glyphs_2: [Glyph::default(); 256], strings: Strings::default(), main_menu: vec![row(1), row(2), row(3)], sprites: HashMap::new() }
    }

    /// Runs until the front end asks for a movie; returns the frames it took.
    fn frames_to_movie(fe: &mut FrontEnd, a: &Assets) -> u32 {
        let mut n = 0;
        while fe.movie().is_none() { fe.tick(0, false, a); n += 1; }
        n
    }

    #[test]
    fn splashes_then_the_logo_then_the_title() {
        let a = assets();
        let mut fe = FrontEnd::new(1);
        // 65 + 45 + 32 + 30, 65 + 25 + 32, one frame.
        assert_eq!(frames_to_movie(&mut fe, &a), 172 + 122 + 1);
        assert_eq!(fe.movie(), Some(MOVIE_LOGO));
        // Buttons do not skip the logo.
        fe.tick(button::START, false, &a);
        fe.tick(0, false, &a);
        assert_eq!(fe.movie(), Some(MOVIE_LOGO));
        fe.tick(0, true, &a);
        let first = fe.movie().unwrap();
        assert!((0xb3..=0xbb).contains(&first));
    }

    #[test]
    fn the_playlist_holds_each_couch_movie() {
        let mut fe = FrontEnd::new(0x1234);
        fe.build_playlist();
        let l = &fe.playlist[1..];
        assert!(l.iter().all(|m| (0xb3..=0xbb).contains(m)));
        for m in 0xb3..=0xbbu16 { assert!(l.contains(&m), "{m:#x}"); }
        assert_eq!(fe.playlist[0], 0);
    }

    #[test]
    fn a_press_after_a_release_opens_the_menu() {
        let a = assets();
        let mut fe = FrontEnd::new(7);
        frames_to_movie(&mut fe, &a);
        fe.tick(0, true, &a);
        // Held from before the title: no skip until released.
        fe.tick(button::CROSS, false, &a);
        assert!(fe.movie().is_some());
        fe.tick(0, false, &a);
        fe.tick(button::CROSS, false, &a);
        assert_eq!(fe.movie(), None);
        // Three frames of backdrop, then the page; Down moves and wraps.
        for _ in 0..3 { fe.tick(0, false, &a); }
        assert_eq!(fe.tick(button::DOWN, false, &a), [Request::Sound(3)]);
        assert_eq!(fe.selected, 1);
        fe.tick(0, false, &a);
        fe.tick(button::UP, false, &a);
        fe.tick(0, false, &a);
        fe.tick(button::UP, false, &a);
        assert_eq!(fe.selected, 2);
        fe.tick(0, false, &a);
        assert_eq!(fe.tick(button::CROSS, false, &a), [Request::Row { kind: 4, target: 0 }]);
    }

    #[test]
    fn the_menu_goes_back_to_the_title_when_idle() {
        let a = assets();
        let mut fe = FrontEnd::new(7);
        fe.phase = Phase::Menu { sub: 4 };
        for _ in 0..IDLE_FRAMES { fe.tick(0, false, &a); }
        assert!(fe.movie().is_some());
    }

    #[test]
    fn seed_from_the_clock() { assert_eq!(clock_seed([0, 0x48, 0x09, 0x20, 0, 0x15, 0x10, 0x26]), 0x49 * 0x21 * 0x0a * 0x16 * 0x11); }
}

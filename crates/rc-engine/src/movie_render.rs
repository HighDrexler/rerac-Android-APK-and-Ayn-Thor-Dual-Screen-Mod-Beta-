//! PSS movies (FMV, game mode 1) played natively from the original files at runtime (decision U10): the request
//! (`DialogStreamUpdate(n)` → `StartPssMovie`), the frame sequence and skip rule of `rc_game::movie_player`, the
//! pictures decoded by `rc_video` on a worker thread, the audio mixed into crate::audio_out, and a full-screen
//! picture. Spec: docs/plan/cutscenes.md §5, docs/formats/pss.md.
//!
//! * **Request.** crate::scene_render hands over the classes' `EngineRequest::StartMovie` and the talkers'
//!   `Handoff::Movie` through [`request_in_level`]: in-level movie n is `global/mpegs/{2 + n}.bin` (NTSC). From
//!   that frame on the gameplay tick is suspended (mode 1; `GameTick` gets a `run_if`, the menu mode is
//!   `Mode::Movie`), as `StartPssMovie` leaves the moby loop's tick and the main loop runs `MovieModeUpdate`.
//! * **Frames** ([`movie_frame`], `FixedUpdate` after the tick, one call per 60 Hz vsync): `FadeToBlack(4)` over
//!   the last game image and its closing black vsync, the movie (the audio is the clock: picture
//!   `(800·k − latency)·fps / 48000` at vsync k), `FadeToBlack(4)` over the last picture, then mode 0:
//!   `music_start_track(track, 1, 0x400)` and the talker's dialogue refresh (`MovieExitToGameplay` 0x2ad2b8).
//!   Skip: `readMpeg`'s rule (Start + L1 L2 R1 R2, or Start alone once the game is beaten / completed / on
//!   level 0), from the first movie vsync.
//! * **Decoding.** A worker thread reads the file, demuxes it, decodes the audio channel of the game language
//!   (channel 0 when the file has no such channel) and then the pictures in display order, converted to RGBA,
//!   through a 4-deep channel. Realtime playback polls without blocking and skips late colour conversions;
//!   deterministic captures wait for each requested picture. Decoder work and app-update timing are logged
//!   separately; upload counts and app-update rates are not physical display FPS.
//! * **Audio.** `StartPssMovie` / `MovieModeUpdate` stop all sounds and the music (`AudioSystem::movie_stop`);
//!   each movie vsync outputs the next 800 samples of the movie's 48 kHz audio through `AudioSystem::mix_movie`
//!   (the two SPU movie voices, group 5 at the sfx option), the fades output silence; the exit restarts the level
//!   track (`AudioSystem::movie_exit`). The latency used by the clock is the audio ring's fill (0 in frame-exact
//!   runs, so their PNGs and WAVs do not depend on the device).
//! * **Picture.** The movie is 512×416, the size of the GS draw buffer, and `setImageTag` 0x31b0b8 uploads it at
//!   (0, 0) of the display buffer: it fills exactly the picture the game's frames fill. The port shows it as a UI
//!   image in the game frame's 4:3 box, nearest-sampled, above the HUD. The main view draws the entry fade;
//!   an opaque movie uses its own 2D view with the hidden world views suspended. The black fade's alpha makes the
//!   linear-light blend give `C·(1 − coverage)` in display bytes, like the GS's black quad.
//!
//! * **Other callers** (docs/plan/progression.md `## media`): [`request`] with [`MovieRequest::movie_b`] (`PlayMovieB`:
//!   the Cinematics page, the ending `PlayMovieB(11)`), [`MovieRequest::movie_c`] (`PlayMovieC`: the extras, language 0),
//!   [`play_attract`] (the title's attract loop, boot `fun_001e9488`: back to the interrupted mode, no music restart),
//!   and the page menu's replays ([`MovieRequest::after_menu_fade`]: its `FadeToBlack(ticks(16))` before, the replay mode
//!   0x15eed8 in [`crate::menu_render::MenuMode::replay`]).
//!
//! Environment (dev checks, not game options):
//! - `RC_PLAY_MOVIE=<n>`: in-level movie n after gameplay tick 1 (as `RC_SCENE`), e.g. 3 = the Novalis holofilm.
//! - `RC_MOVIE_SKIP=<k>`: press Start with L1 L2 R1 R2 held on movie-mode frame k (the skip test).
//! - `RC_MOVIE_TRACE=1`: one line per 30 pictures and per phase change.

use crate::fly_cam::FlyCam;
use crate::gameplay::{GameTick, Persistent, Play};
use crate::input_map::PadFrame;
use bevy::asset::RenderAssetUsages;
use bevy::camera::{ClearColorConfig, visibility::RenderLayers};
use bevy::core_pipeline::tonemapping::{DebandDither, Tonemapping};
use bevy::render::view::Msaa;
use bevy::image::ImageSampler;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::ui::UiTargetCamera;
use rc_game::menus::mode::Mode;
use rc_game::moby_runtime::MobyId;
use rc_game::movie_player::{MovieContext, MovieFrame, MoviePlayer, Phase, MOVIE_ATTRACT_BASE_NTSC, MOVIE_BASE_NTSC, MOVIE_B_BASE_NTSC, MOVIE_C_BASE_NTSC, SAMPLES_PER_VSYNC};
use rc_game::pad::{button, PadState};
use std::collections::VecDeque;
use std::path::PathBuf;
use std::time::Instant;
use crate::movie_pipe::{channel as sync_channel, Receiver, Sender as SyncSender, Empty};
use std::sync::Mutex;

/// A movie the game asked for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MovieRequest {
    /// `mpegs[file]` = `global/mpegs/NNN.bin`.
    pub file: u32,
    /// The in-level movie number n (`DialogStreamUpdate(n)`), for the logs.
    pub movie: i32,
    /// The talker whose dialogue continues after the movie (0x179588).
    pub talker: Option<MobyId>,
    /// The audio language (`StartPssMovie`'s third argument: 0x15ed88, or 0 for `PlayMovieC` and the attract movies).
    pub language: Option<u8>,
    /// The caller faded the last image to black before (the page menu's `FadeToBlack(ticks(16))`), and the entry fade
    /// (4; the attract movies 12).
    pub black_entry: bool,
    pub fade_in: u32,
    /// The replay mode 0x15eed8 the caller sets for this movie and clears after it (the attract movies: 2); None: the
    /// global as it is ([`crate::menu_render::MenuMode::replay`], the page menu's post-actions set it).
    pub replay: Option<i32>,
    /// Who the movie returns to.
    pub exit: MovieExit,
}

/// What follows the movie.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MovieExit {
    /// `MovieExitToGameplay` 0x2ad2b8: mode 0, `music_start_track(level track, 1, 0x400)`, the talker refreshed.
    #[default]
    Gameplay,
    /// The title's attract movie (boot `fun_001e9488`): the mode it interrupted again, no music restart.
    #[allow(dead_code)] // the saves lane's title plays the attract movies ([`play_attract`])
    FrontEnd,
}

impl MovieRequest {
    /// `DialogStreamUpdate(n)` 0x2acf50: in-level movie n = `mpegs[2 + n]`, the game language.
    pub fn in_level(n: i32, talker: Option<MobyId>) -> MovieRequest {
        MovieRequest { file: (MOVIE_BASE_NTSC + n) as u32, movie: n, talker, language: None, black_entry: false, fade_in: rc_game::movie_player::FADE_FRAMES, replay: None, exit: MovieExit::Gameplay }
    }
    /// `PlayMovieB(n)` 0x2ad050: `mpegs[40 + n]`, the game language.
    pub fn movie_b(n: i32) -> MovieRequest { MovieRequest { file: (MOVIE_B_BASE_NTSC + n) as u32, ..MovieRequest::in_level(n, None) } }
    /// `PlayMovieC(n)` 0x2acfe8: `mpegs[70 + n]`, language 0.
    pub fn movie_c(n: i32) -> MovieRequest { MovieRequest { file: (MOVIE_C_BASE_NTSC + n) as u32, language: Some(0), ..MovieRequest::in_level(n, None) } }
    /// Boot `fun_001e9488(i)`: the title's attract movie `mpegs[80 + i]` (replay 2: any button skips; language 0; entry
    /// fade `ticks(12)`; back to the title).
    #[allow(dead_code)] // the saves lane's title plays the attract movies ([`play_attract`])
    pub fn attract(i: i32) -> MovieRequest {
        MovieRequest {
            file: (MOVIE_ATTRACT_BASE_NTSC + i) as u32,
            language: Some(0),
            fade_in: rc_game::movie_player::ATTRACT_FADE,
            replay: Some(2),
            exit: MovieExit::FrontEnd,
            ..MovieRequest::in_level(i, None)
        }
    }
    /// After the page menu's `FadeToBlack(ticks(16))` (drawn by crate::menu_render over the menu image).
    pub fn after_menu_fade(mut self) -> MovieRequest {
        self.black_entry = true;
        self
    }
}

static QUEUE: Mutex<VecDeque<MovieRequest>> = Mutex::new(VecDeque::new());

fn queue() -> std::sync::MutexGuard<'static, VecDeque<MovieRequest>> { QUEUE.lock().unwrap_or_else(|e| e.into_inner()) }

/// `DialogStreamUpdate(n)` 0x2acf50 → `StartPssMovie` 0x2ad0c0: in-level movie n (`mpegs[2 + n]`, NTSC); a
/// negative n is ignored as in the game. The movie starts this frame (the rest of the tick that asked has run).
pub fn request_in_level(n: i32, talker: Option<MobyId>) {
    if n < 0 { return; }
    request(MovieRequest::in_level(n, talker));
}

/// Any movie (`PlayMovieB` / `PlayMovieC` / the attract movies / the page menu's replays: [`MovieRequest`]'s
/// constructors). A negative movie number is ignored as in the game (`if (-1 < n)`). It starts on the next movie frame.
pub fn request(req: MovieRequest) {
    if req.movie < 0 { return; }
    queue().push_back(req);
}

/// The title's attract movie i (boot `fun_001e9488`, the `saves` lane's title calls it after its idle time): see
/// [`MovieRequest::attract`]. [`MovieState::busy`] tells when it is over.
#[allow(dead_code)] // its caller is the saves lane's title (docs/plan/progression.md `## media`)
pub fn play_attract(i: i32) { request(MovieRequest::attract(i)); }

/// A movie is queued or playing: the gameplay tick is suspended.
fn blocking(state: &MovieState) -> bool { state.running.is_some() || !queue().is_empty() }

/// Messages of the decoder thread.
enum Msg {
    Opened { fps: (u32, u32), audio: Option<Vec<[i16; 2]>>, info: String },
    Picture(u32, Vec<u8>),
    End(u32),
    Failed(String),
}

struct Running {
    req: MovieRequest,
    player: MoviePlayer,
    /// In a mutex only to make the resource `Sync` (only this system touches it: `get_mut`, no locking).
    rx: Mutex<Receiver<Msg>>,
    opened: bool,
    audio: Option<Vec<[i16; 2]>>,
    /// Pictures in the stream, once the decoder reached the end.
    total: Option<u32>,
    /// Pictures received so far; the last one is [`Running::rgba`].
    received: u32,
    rgba: Vec<u8>,
    /// The picture on screen (uploaded).
    shown: Option<u32>,
    /// The last fixed tick's desired UI state. Only Update uploads/presents it;
    /// a slow render may run up to twelve movie ticks but presents one image.
    wants_picture: bool,
    black: f32,
    pad: PadState,
    /// Movie-mode frames so far.
    frames: u32,
    last_phase: Option<Phase>,
    owner: Option<crate::audio_queue::MovieOwner>,
    media_cancelled: bool,
    /// Diagnostic counters only: never gate or slow movie playback.
    started: Instant,
    presented: u32,
    initial_underruns: u64,
    present_updates: u32,
    first_present_update: Option<Instant>,
    last_present_update: Option<Instant>,
    max_present_gap: std::time::Duration,
}

/// The movie state, published for the tick's run condition and the reports.
#[derive(Resource, Default)]
pub struct MovieState {
    running: Option<Running>,
    image: Handle<Image>,
    root: Option<Entity>,
    picture: Option<Entity>,
    fade: Option<Entity>,
    /// `RC_PLAY_MOVIE`: (movie, requested).
    debug: Option<(i32, bool)>,
    skip_at: Option<u32>,
    trace: bool,
    /// Movies played to the end or skipped (reports).
    pub finished: Vec<(u32, bool)>,
    /// The mode the running movie interrupted (restored by a [`MovieExit::FrontEnd`] exit).
    interrupted: Option<Mode>,
    /// Original camera activation, restored on every movie exit (including errors).
    suspended_cameras: Vec<(Entity, bool)>,
}

impl MovieState {
    /// A movie is queued or playing (the title waits for its attract movie with this).
    pub fn busy(&self) -> bool { blocking(self) }

    /// Keep the world for the entry fade; it is redundant once the movie
    /// owns the whole image. In-engine scenes never enter this path.
    fn covers_world(&self) -> bool {
        self.running.as_ref().is_some_and(|r| opaque_movie_frame(r.last_phase, r.black))
    }
}

fn opaque_movie_frame(phase: Option<Phase>, black: f32) -> bool {
    match phase {
        Some(Phase::Playing(_) | Phase::FadeOut(_)) => true,
        Some(Phase::FadeIn(_)) => black >= 1.0,
        _ => false,
    }
}

/// Expensive world-only work can stop while an opaque prerecorded movie is up.
/// Optional state also lets standalone diagnostics use these plugins.
pub(crate) fn world_visible(state: Option<Res<MovieState>>) -> bool {
    !state.is_some_and(|s| s.covers_world())
}

#[derive(Component)]
struct MovieUi;

#[derive(Component)]
struct MovieCamera;

pub struct MovieRenderPlugin;

impl Plugin for MovieRenderPlugin {
    fn build(&self, app: &mut App) {
        let env = |k: &str| std::env::var(k).ok().filter(|v| !v.trim().is_empty());
        let debug = env("RC_PLAY_MOVIE").and_then(|v| v.trim().parse::<i32>().ok()).map(|n| (n, false));
        let skip_at = env("RC_MOVIE_SKIP").and_then(|v| v.trim().parse().ok());
        if let Some((n, _)) = debug { println!("movie: RC_PLAY_MOVIE={n}: in-level movie {n} (mpegs[{}]) after gameplay tick 1", MOVIE_BASE_NTSC + n); }
        app.insert_resource(MovieState { debug, skip_at, trace: env("RC_MOVIE_TRACE").as_deref() == Some("1"), ..default() })
            // Mode 1 runs MovieModeUpdate instead of the game tick.
            .configure_sets(FixedUpdate, GameTick.run_if(|s: Res<MovieState>| !blocking(&s)))
            .add_systems(Startup, setup)
            .add_systems(Update, (movie_present, movie_view).chain())
            .add_systems(FixedUpdate, (movie_exit.before(GameTick), movie_frame.after(GameTick)));
    }
}

fn setup(mut commands: Commands, mut state: ResMut<MovieState>, mut images: ResMut<Assets<Image>>) {
    // This 2D view replaces both world views only while the movie is opaque.
    // DisplayPlugin redirects it to GameFrame; PresentCamera and the Android
    // companion continue to consume that same frame. No 3D/postprocess pass
    // is needed to display a decoded movie picture.
    commands.spawn((
        Camera2d,
        Camera { order: 900, is_active: false, clear_color: ClearColorConfig::Custom(Color::BLACK), ..default() },
        Msaa::Off, Tonemapping::None, DebandDither::Disabled,
        RenderLayers::none(), MovieCamera, crate::level_switch::KeepAcrossLevels,
        Name::new("movie camera (no hidden world)"),
    ));
    let size = Extent3d { width: 512, height: 416, depth_or_array_layers: 1 };
    let mut img = Image::new_fill(size, TextureDimension::D2, &[0, 0, 0, 255], TextureFormat::Rgba8UnormSrgb, RenderAssetUsages::default());
    img.sampler = ImageSampler::nearest();
    state.image = images.add(img);
    let full = || Node { position_type: PositionType::Absolute, left: Val::Px(0.0), top: Val::Px(0.0), width: Val::Percent(100.0), height: Val::Percent(100.0), ..default() };
    // On the frame's 4:3 box (crate::display); the root is black around it.
    let picture = commands.spawn((full(), ImageNode::new(state.image.clone()), Visibility::Hidden, crate::display::UiBoxed, Name::new("movie picture"))).id();
    let fade = commands.spawn((full(), BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0)), Name::new("movie fade"))).id();
    // Above the HUD composite (same global z, higher local z).
    let root = commands.spawn((full(), BackgroundColor(Color::BLACK), GlobalZIndex(i32::MAX), ZIndex(1), Visibility::Hidden, MovieUi, Name::new("movie"), crate::level_switch::KeepAcrossLevels)).add_children(&[picture, fade]).id();
    (state.root, state.picture, state.fade) = (Some(root), Some(picture), Some(fade));
}

type MovieViews<'w, 's> = Query<'w, 's, (Entity, &'static mut Camera, Option<&'static crate::render_settings::WorldCamera>, Option<&'static MovieCamera>)>;

fn movie_view(
    mut commands: Commands,
    mut state: ResMut<MovieState>,
    main: Query<Entity, With<FlyCam>>,
    mut cams: MovieViews,
    nodes: Query<(Entity, Option<&UiTargetCamera>), With<MovieUi>>,
) {
    let covered = state.covers_world();
    route_movie_view(covered, &mut state.suspended_cameras, &mut cams);
    let target = if covered {
        cams.iter().find_map(|(e, _, _, movie)| movie.map(|_| e))
    } else {
        main.iter().next()
    };
    if let Some(target) = target {
        for (entity, current) in &nodes {
            if current.is_none_or(|c| c.0 != target) {
                commands.entity(entity).insert(UiTargetCamera(target));
            }
        }
    }
}

/// Save activation once, including initially inactive cameras. A camera
/// created during playback is suspended too; deleted cameras are harmless.
fn route_movie_view(covered: bool, saved: &mut Vec<(Entity, bool)>, cams: &mut MovieViews) {
    if !covered {
        for (entity, active) in saved.drain(..) {
            if let Ok((_, mut camera, _, _)) = cams.get_mut(entity) { camera.is_active = active; }
        }
    }
    for (entity, mut camera, world, movie) in cams.iter_mut() {
        if movie.is_some() {
            if camera.is_active != covered { camera.is_active = covered; }
        } else if covered && world.is_some() {
            if !saved.iter().any(|(e, _)| *e == entity) { saved.push((entity, camera.is_active)); }
            if camera.is_active { camera.is_active = false; }
        }
    }
}

fn movie_path(file: u32) -> PathBuf { crate::level_load::extracted_root().join(format!("global/mpegs/{file:03}.bin")) }

/// Decoder/conversion elapsed work and app updates are separate diagnostics:
/// decoder mailbox waits are excluded. App updates are not GPU presentations. Drop reports cancellation/error as well as normal EOF.
struct DecodePerf {
    file: String,
    started: Instant,
    open: std::time::Duration,
    decode: std::time::Duration,
    convert: std::time::Duration,
    decoded: u32,
    converted: u32,
    skipped_conversion: u32,
}
impl Default for DecodePerf {
    fn default() -> Self {
        Self { file: String::new(), started: Instant::now(), open: Default::default(),
            decode: Default::default(), convert: Default::default(),
            decoded: 0, converted: 0, skipped_conversion: 0 }
    }
}
impl Drop for DecodePerf {
    fn drop(&mut self) {
        println!(
            "movie-decode-perf: {} wall={:.3}s open_ms={:.1} decoded={} decode_ms_per_picture={:.3} converted={} convert_ms_per_picture={:.3} skipped_conversion={}",
            self.file, self.started.elapsed().as_secs_f64(), self.open.as_secs_f64() * 1000.0,
            self.decoded, self.decode.as_secs_f64() * 1000.0 / self.decoded.max(1) as f64,
            self.converted, self.convert.as_secs_f64() * 1000.0 / self.converted.max(1) as f64,
            self.skipped_conversion,
        );
    }
}

/// The worker: open, send the audio, then every picture as RGBA in display order.
fn decode_thread(path: PathBuf, language: u8, tx: SyncSender<Msg>) {
    let mut perf = DecodePerf { file: path.display().to_string(), ..default() };
    let file = match std::fs::read(&path) {
        Ok(f) => f,
        Err(e) => {
            let _ = tx.send(Msg::Failed(format!("{}: {e}", path.display())));
            return;
        }
    };
    let mut m = match rc_video::Movie::open(&file, language) {
        Ok(m) => m,
        Err(e) => {
            let _ = tx.send(Msg::Failed(format!("{}: {e}", path.display())));
            return;
        }
    };
    drop(file);
    perf.open = perf.started.elapsed();
    let s = m.sequence;
    let info = format!(
        "{}×{} at {:.3} fps, audio channel {:?} of {:?} at {} Hz ({:.2} s), {} packs",
        s.width, s.height, m.fps(), m.audio_channel, m.channels, m.audio_rate, m.audio.as_ref().map_or(0.0, |a| a.len() as f64 / 48_000.0), m.packs
    );
    if tx.send(Msg::Opened { fps: (s.fps_num, s.fps_den), audio: m.audio.take(), info }).is_err() { return; }
    let mut n = 0u32;
    loop {
        if tx.closed() { return; }
        let start = Instant::now();
        let frame = m.next_frame();
        perf.decode += start.elapsed();
        match frame {
            Ok(Some(f)) => {
                perf.decoded += 1;
                if !tx.should_convert(n) { perf.skipped_conversion += 1; n += 1; continue; }
                let start = Instant::now();
                let mut rgba = Vec::new();
                rc_video::movie::frame_to_rgba(&f, &mut rgba);
                perf.convert += start.elapsed();
                perf.converted += 1;
                if tx.send(Msg::Picture(n, rgba)).is_err() { return; }
                n += 1;
            }
            Ok(None) => {
                let _ = tx.send(Msg::End(n));
                return;
            }
            Err(e) => {
                let _ = tx.send(Msg::Failed(format!("{} after {n} pictures: {e}", path.display())));
                return;
            }
        }
    }
}

impl Running {
    /// Handles one worker message (blocking); false when the worker is gone.
    fn receive(&mut self, blocking: bool) -> bool {
        let rx = self.rx.get_mut().unwrap_or_else(|e| e.into_inner());
        match rx.receive(blocking) {
            Ok(Msg::Opened { fps, audio, info }) => {
                println!("movie: mpegs[{}] opened: {info}", self.req.file);
                self.player.set_rate(fps.0, fps.1);
                self.audio = audio;
                self.opened = true;
            }
            Ok(Msg::Picture(i, rgba)) => {
                self.received = i + 1;
                self.rgba = rgba;
            }
            Ok(Msg::End(n)) => self.total = Some(n),
            Ok(Msg::Failed(e)) => {
                eprintln!("movie: {e}; the movie ends here");
                self.opened = true;
                self.total = Some(self.received);
            }
            Err(Empty::Pending) => return false,
            Err(Empty::Closed) => {
                self.opened = true;
                self.total = Some(self.received);
                return false;
            }
        }
        true
    }

    /// Waits until picture `p` has arrived (or the stream ended before it).
    fn fetch(&mut self, p: u32, blocking: bool) {
        self.rx.get_mut().unwrap_or_else(|e| e.into_inner()).wanted(p, !blocking);
        while !self.opened { if !self.receive(blocking) { break; } }
        while self.received <= p && self.total.is_none() { if !self.receive(blocking) { break; } }
    }
}

/// The fade quad's alpha so that the linear-light blend `C·(1 − α)` shows `C·(1 − coverage)` in display bytes.
fn fade_alpha(coverage: f32) -> f32 { if coverage >= 1.0 { 1.0 } else { 1.0 - (1.0 - coverage.clamp(0.0, 1.0)).powf(2.2) } }

#[allow(clippy::too_many_arguments)]
fn movie_frame(
    mut state: ResMut<MovieState>,
    play: Option<Res<Play>>,
    persistent: Option<Res<Persistent>>,
    mut menu: Option<ResMut<crate::menu_render::MenuMode>>,
    mut audio: Option<ResMut<crate::audio_out::AudioOut>>,
    pad: Option<Res<PadFrame>>,
    det: Res<crate::determinism::Deterministic>,
    frame_no: Res<crate::determinism::FrameNumber>,
    ticks: Res<crate::determinism::GameTicks>,
) {
    let st = &mut *state;
    // RC_PLAY_MOVIE: after gameplay tick 1.
    if let (Some((n, false)), Some(p)) = (st.debug, play.as_ref()) {
        if p.game.counter >= 2 {
            st.debug = Some((n, true));
            request_in_level(n, None);
        }
    }
    if st.running.is_none() {
        let Some(req) = queue().pop_front() else { return };
        let path = movie_path(req.file);
        let language = req.language.unwrap_or(crate::hud_render::language() as u8);
        let (tx, rx) = sync_channel(4);
        let spawn = std::thread::Builder::new().name(format!("movie {:03}", req.file)).spawn({
            let path = path.clone();
            move || decode_thread(path, language, tx)
        });
        if let Err(e) = spawn { eprintln!("movie: cannot start the decoder thread: {e}"); }
        let gs = persistent.as_ref().map(|s| &s.0.global);
        // 0x15eed8: the caller's replay mode (fun_001e9488 sets 2), else the global as the page menu left it.
        if let (Some(r), Some(m)) = (req.replay, menu.as_mut()) { m.replay = r; }
        let replay = req.replay.unwrap_or_else(|| menu.as_ref().map_or(0, |m| m.replay));
        let ctx = MovieContext { replay, game_beaten: gs.is_some_and(|g| g.game_beaten != 0), completes: gs.map_or(0, |g| g.completes), level: crate::level_load::level_index() as i32 };
        println!(
            "movie: app frame {}: DialogStreamUpdate({}) → StartPssMovie(mpegs[{}], language {language}) {}: mode 1, sounds and music stopped",
            frame_no.0, req.movie, req.file, path.display()
        );
        if let Some(a) = audio.as_mut() { a.system().movie_stop(); }
        st.interrupted = menu.as_ref().map(|m| m.state.mode);
        if let Some(m) = menu.as_mut() { m.state.set(Mode::Movie); }
        let player = MoviePlayer::new(ctx).with_entry(req.black_entry, req.fade_in);
        st.running = Some(Running {
            req,
            player,
            rx: Mutex::new(rx),
            opened: false,
            audio: None,
            total: None,
            received: 0,
            rgba: Vec::new(),
            shown: None,
            wants_picture: false,
            black: 0.0,
            pad: PadState::default(),
            frames: 0,
            last_phase: None,
            owner: audio.as_ref().map(|a| a.begin_movie()),
            media_cancelled: false,
            started: Instant::now(),
            presented: 0,
            initial_underruns: audio.as_ref().map_or(0, |a| a.underruns()),
            present_updates: 0,
            first_present_update: None,
            last_present_update: None,
            max_present_gap: std::time::Duration::ZERO,
        });
    }
    let Some(rt) = st.running.as_mut() else { return };
    // The pad (readMpeg polls it each loop); RC_MOVIE_SKIP presses the skip combination on one frame.
    if let Some(p) = &pad { rt.pad.update(Some(&p.0.bytes()), false); }
    // Android's dual-screen/touch Start button is a full cutscene skip.
    // Retain the original PS2 skip gates and transition/fade handling.
    #[cfg(target_os = "android")]
    crate::android::cinematic_skip_pad(&mut rt.pad);
    if st.skip_at == Some(rt.frames) {
        let shoulders = button::L1 | button::L2 | button::R1 | button::R2;
        rt.pad.held |= shoulders | button::START;
        rt.pad.pressed |= button::START;
        println!("movie: RC_MOVIE_SKIP: Start + L1 L2 R1 R2 on movie frame {}", rt.frames);
    }
    let latency = if det.0 { 0 } else { audio.as_ref().map_or(0, |a| a.queued()) as u32 };
    // Keep the native event loop responsive while file/audio decoding opens.
    // Frame-exact tooling still waits for every original picture.
    if !rt.opened {
        rt.fetch(0, det.0);
        if !rt.opened { return; }
    }
    if let Some(p) = rt.player.wanted_picture(latency) { rt.fetch(p, det.0); }
    let out: MovieFrame = rt.player.tick(&rt.pad, rt.total, latency);
    if rt.player.skipped && !rt.media_cancelled {
        rt.rx.get_mut().unwrap_or_else(|e|e.into_inner()).close();
        if let Some(owner) = &rt.owner { owner.cancel(); }
        rt.media_cancelled = true;
    }
    rt.frames += 1;
    // Never upload every intermediate fixed tick: Android may run several
    // 60 Hz movie ticks between two GPU frames during cutscenes.
    // Keep only the latest eligible decoded picture for movie_present (Update).
    rt.wants_picture = out.picture.is_some();
    rt.black = out.black;
    if st.trace && rt.last_phase.as_ref().map(std::mem::discriminant) != Some(std::mem::discriminant(&out.phase)) {
        println!("movie: app frame {} (movie frame {}): {:?}", frame_no.0, rt.frames, out.phase);
    }
    rt.last_phase = Some(out.phase);
    // The audio: the movie's next 800 samples while it plays, silence in the fades; one frame per vsync (none on the
    // start frame, whose gameplay tick already output it).
    if let Some(a) = audio.as_mut().filter(|a| rt.frames > 1 || a.frames_out() < ticks.0) {
        let a = &mut **a;
        let mut buf = std::mem::take(&mut a.buf);
        buf.clear();
        match (out.phase, rt.audio.as_ref()) {
            (Phase::Playing(k), Some(pcm)) => {
                let at = (k as u64 * SAMPLES_PER_VSYNC) as usize;
                let mut chunk = [[0i16; 2]; SAMPLES_PER_VSYNC as usize];
                if at < pcm.len() {
                    let n = (pcm.len() - at).min(chunk.len());
                    chunk[..n].copy_from_slice(&pcm[at..at + n]);
                }
                a.system().mix_movie(&chunk, &mut buf);
            }
            _ => buf.resize(SAMPLES_PER_VSYNC as usize, [0, 0]),
        }
        a.buf = buf;
        a.push_movie_frame(rt.owner.as_ref().map_or(0, |o| o.id));
    }
}

/// Present only once per rendered frame, after all FixedUpdate audio/movie
/// ticks have been run. Moving the pixel Vec into the Bevy image avoids a
/// 512x416x4-byte clone for each 30 Hz picture and cuts GPU uploads when
/// multiple fixed ticks catch up in a single rendered frame.
fn movie_present(
    mut state: ResMut<MovieState>,
    frame_no: Res<crate::determinism::FrameNumber>,
    mut images: ResMut<Assets<Image>>,
    mut vis: Query<&mut Visibility>,
    mut bg: Query<&mut BackgroundColor>,
) {
    let st = &mut *state;
    let Some(rt) = st.running.as_mut() else { return };
    if matches!(rt.last_phase, Some(Phase::Playing(_))) {
        let now = Instant::now();
        rt.first_present_update.get_or_insert(now);
        if let Some(last) = rt.last_present_update.replace(now) {
            rt.max_present_gap = rt.max_present_gap.max(now - last);
        }
        rt.present_updates += 1;
    }
    if rt.wants_picture && rt.received > 0 {
        let index = rt.received - 1;
        if rt.shown != Some(index) && !rt.rgba.is_empty() {
            if let Some(mut img) = images.get_mut(&st.image) {
                img.data = Some(std::mem::take(&mut rt.rgba));
                rt.shown = Some(index);
                rt.presented = rt.presented.saturating_add(1);
                if st.trace && index.is_multiple_of(30) {
                    println!("movie: rendered app frame {}: picture {} of {:?} ({} ticks)", frame_no.0, index, rt.total, rt.frames);
                }
            }
        }
    }
    // Fade over the world on entry, then keep black bars and decoder-wait
    // frames opaque while the world cameras are suspended.
    if let Some(mut color) = st.root.and_then(|e| bg.get_mut(e).ok()) {
        let want = BackgroundColor(if opaque_movie_frame(rt.last_phase, rt.black) { Color::BLACK } else { Color::NONE });
        if *color != want { *color = want; }
    }
    set_visible(&mut vis, st.root, true);
    set_visible(&mut vis, st.picture, rt.wants_picture && rt.shown.is_some());
    if let Some(mut c) = st.fade.and_then(|e| bg.get_mut(e).ok()) {
        let want = BackgroundColor(Color::srgba(0.0, 0.0, 0.0, fade_alpha(rt.black)));
        if *c != want { *c = want; }
    }
}

fn set_visible(vis: &mut Query<&mut Visibility>, e: Option<Entity>, show: bool) {
    if let Some(mut v) = e.and_then(|e| vis.get_mut(e).ok()) {
        let want = if show { Visibility::Inherited } else { Visibility::Hidden };
        if *v != want { *v = want; }
    }
}

/// `MovieExitToGameplay` 0x2ad2b8, on the frame after the exit fade's closing vsync (before the tick, which runs again
/// this frame): mode 0, the level track restarted, the talker's dialogue refreshed.
fn movie_exit(
    mut state: ResMut<MovieState>,
    mut play: Option<ResMut<Play>>,
    mut menu: Option<ResMut<crate::menu_render::MenuMode>>,
    mut audio: Option<ResMut<crate::audio_out::AudioOut>>,
    frame_no: Res<crate::determinism::FrameNumber>,
    mut vis: Query<&mut Visibility>,
) {
    let st = &mut *state;
    if !st.running.as_ref().is_some_and(|r| r.player.done()) { return; }
    let rt = st.running.take().unwrap();
    println!(
        "movie: app frame {}: mpegs[{}] {} after {} movie frames ({} of {:?} pictures decoded); mode 0, music_start_track, gameplay resumes",
        frame_no.0, rt.req.file, if rt.player.skipped { "skipped" } else { "played through" }, rt.frames, rt.received, rt.total
    );
    #[cfg(target_os = "android")]
    {
        let elapsed = rt.started.elapsed().as_secs_f64();
        let ticks_per_second = rt.frames as f64 / elapsed.max(0.001);
        let underruns = audio.as_ref().map_or(0, |a| a.underruns());
        let presenting = rt.first_present_update.zip(rt.last_present_update)
            .map_or(0.0, |(a,b)| (b-a).as_secs_f64());
        println!(
            "movie-update-perf: mpegs[{}] updates={} wall={presenting:.3}s update_hz={:.1} max_gap_ms={:.1} opaque_world_views=disabled",
            rt.req.file, rt.present_updates,
            rt.present_updates.saturating_sub(1) as f64 / presenting.max(0.001),
            rt.max_present_gap.as_secs_f64() * 1000.0,
        );
        println!(
            "movie-perf: mpegs[{}] wall={elapsed:.2}s fixed_ticks={} tick_hz={ticks_per_second:.1} decoded_through={} uploaded={} audio_underruns={} skipped={}",
            rt.req.file, rt.frames, rt.received, rt.presented,
            underruns.saturating_sub(rt.initial_underruns), rt.player.skipped
        );
    }
    st.finished.push((rt.req.file, rt.player.skipped));
    set_visible(&mut vis, st.root, false);
    set_visible(&mut vis, st.picture, false);
    let interrupted = st.interrupted.take();
    match rt.req.exit {
        MovieExit::Gameplay => {
            // MovieExitToGameplay: replay mode 2 (the menu's in-level replays) back to 0 (0x15eed8).
            if let Some(m) = menu.as_mut() {
                if m.replay == 2 { m.replay = 0; }
            }
            if let Some(a) = audio.as_mut() { a.system().movie_exit(); }
            if let Some(m) = menu.as_mut() { m.state.set(Mode::Gameplay); }
            if let Some(p) = play.as_mut() {
                if p.svc.game_mode == 1 { p.svc.game_mode = 0; }
            }
            if let (Some(npc), Some(p)) = (rt.req.talker, play.as_mut()) {
                p.svc.interact.talker = Some(npc);
                p.svc.interact.scene_ended = true;
            }
        }
        MovieExit::FrontEnd => {
            // fun_001e9488's tail: 0x15eed8 = 0, FadeToBlack(4), the idle counter 0x15f604 = 0 (the title's); no music.
            if let Some(m) = menu.as_mut() {
                m.replay = 0;
                m.state.set(interrupted.unwrap_or(Mode::Gameplay));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn world_is_kept_until_the_entry_fade_is_opaque() {
        assert!(!opaque_movie_frame(None, 0.0)); // opening
        assert!(!opaque_movie_frame(Some(Phase::FadeIn(0)), 0.25));
        assert!(opaque_movie_frame(Some(Phase::FadeIn(3)), 1.0));
        assert!(opaque_movie_frame(Some(Phase::Playing(0)), 0.0));
        assert!(opaque_movie_frame(Some(Phase::FadeOut(0)), 0.25));
        assert!(!opaque_movie_frame(Some(Phase::Done), 0.0));
    }

    #[test]
    fn movie_view_restores_cameras_and_leaves_other_targets_alone() {
        use bevy::ecs::system::SystemState;
        use crate::render_settings::WorldCamera;
        let mut world = World::new();
        let main = world.spawn((Camera::default(), WorldCamera)).id();
        let sky = world.spawn((Camera::default(), WorldCamera)).id();
        let inactive = world.spawn((Camera { is_active: false, ..default() }, WorldCamera)).id();
        let movie = world.spawn((Camera { is_active: false, ..default() }, MovieCamera)).id();
        let present = world.spawn(Camera::default()).id();
        let companion = world.spawn(Camera::default()).id();
        let mut saved = Vec::new();
        let mut query: SystemState<MovieViews> = SystemState::new(&mut world);
        route_movie_view(true, &mut saved, &mut query.get_mut(&mut world).unwrap());
        // Repeated frames must not overwrite the original active states.
        route_movie_view(true, &mut saved, &mut query.get_mut(&mut world).unwrap());
        let active = |world: &World, e| world.get::<Camera>(e).unwrap().is_active;
        assert!(!active(&world, main));
        assert!(!active(&world, sky));
        assert!(!active(&world, inactive));
        assert!(active(&world, movie));
        assert!(active(&world, present));
        assert!(active(&world, companion));
        assert_eq!(saved.len(), 3);
        let late = world.spawn((Camera::default(), WorldCamera)).id();
        route_movie_view(true, &mut saved, &mut query.get_mut(&mut world).unwrap());
        assert!(!active(&world, late));
        world.despawn(sky);
        route_movie_view(false, &mut saved, &mut query.get_mut(&mut world).unwrap());
        assert!(saved.is_empty());
        assert!(active(&world, main));
        assert!(active(&world, late));
        assert!(!active(&world, inactive));
        assert!(!active(&world, movie));
        assert!(active(&world, companion));
    }

    #[test]
    fn end_skip_and_empty_or_failed_stream_all_release_movie_view() {
        for (pictures, skip) in [(Some(3), false), (None, true), (Some(0), false)] {
            let mut player = MoviePlayer::new(MovieContext { level: 1, ..default() });
            let mut covered = false;
            let mut saw_movie_view = false;
            for _ in 0..40 {
                if player.done() {
                    // movie_exit drops Running on this tick, before gameplay resumes.
                    covered = opaque_movie_frame(None, 0.0);
                    break;
                }
                let pad = if skip && matches!(player.phase(), Phase::Playing(_)) {
                    PadState { held: button::START | button::L1 | button::L2 | button::R1 | button::R2, pressed: button::START, ..default() }
                } else { PadState::default() };
                let out = player.tick(&pad, pictures, 0);
                covered = opaque_movie_frame(Some(out.phase), out.black);
                saw_movie_view |= covered;
            }
            assert!(player.done());
            assert!(saw_movie_view);
            assert!(!covered, "movie view leaked after end/skip/error");
        }
    }

    #[test]
    fn fade_alpha_matches_display_bytes() {
        assert_eq!(fade_alpha(0.0), 0.0);
        assert_eq!(fade_alpha(1.0), 1.0);
        // Display C·(1 − c) ≈ ((C_lin·(1 − α))^(1/2.2)).
        let (c, cov) = (0.6f32, 0.25f32);
        let shown = (c.powf(2.2) * (1.0 - fade_alpha(cov))).powf(1.0 / 2.2);
        assert!((shown - c * (1.0 - cov)).abs() < 1e-5);
    }

    #[test]
    fn requests_map_to_the_ntsc_files() {
        queue().clear();
        request_in_level(3, None);
        request_in_level(-1, None);
        assert_eq!(queue().pop_front(), Some(MovieRequest::in_level(3, None)));
        assert_eq!(MovieRequest::in_level(3, None).file, 5);
        assert!(queue().is_empty());
    }
}

#[cfg(test)]
mod rendered_tests;

//! Going Commando (R&C2) in the engine: a data folder holding `RC2.HDR` (crate::disc_source::game) starts this
//! instead of the first game. Only the boot's front end so far (docs/plan/rac2_bringup.md; logic in `rc_game2`):
//! the splash screens, the Insomniac movie, the title (the couch movies with "Press START") and the main menu.
//!
//! * **Data** ([`Data::load`]): `RC2.HDR`, the front-end blob (`G/MISC.WAD` part 4), the GUI sprites (`G/HUD.WAD`), the
//!   boot ELF's glyph tables and main-menu rows; the movies from `G/MPEG*.WAD` (back to back on the disc: a movie's
//!   offset counts across them, [`movie_bytes`]).
//! * **Ticks** (`FixedUpdate`, one per 60 Hz frame, crate::frame_pace): the pad, the movie's end, then
//!   `FrontEnd::tick`.
//! * **Picture**: `FrontEnd::draw`'s quads through one 2D layer of crate::hud_render ([`hud_render::Layer`]: the GS
//!   blend in display bytes); the splash screens, the backdrop and the movie are textures of the layer's atlas (the
//!   movie's rewritten each picture), copied as the frame buffer holds them (alpha 0x80).
//! * **Movies**: decoded on a worker thread (`rc_video`), the pictures in display order at the file's rate (picture
//!   `⌊k·fps/60⌋` at frame k). **No sound yet**: the movie audio, the front end's sounds and music are not played.
//!
//! Environment: `RC_PLAY_SCRIPT` (crate::input_map::Script) drives the pad by frame, `RC_SCREENSHOT` +
//! `RC_SCREENSHOT_FRAME` capture a frame (crate::determinism), `RC_RAC2_TRACE=1` logs the front end's movies and rows.

use crate::hud_render::{self, Hud2d, HudComposite, HudMaterial, Layer, Tex};
use anyhow::{Context, Result};
use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::prelude::*;
use bevy::sprite_render::Material2dPlugin;
use bevy::ui_render::prelude::UiMaterialPlugin;
use rc_formats::rac2::boot::{BootElf, GLYPHS, GLYPHS_2, MAIN_MENU};
use rc_formats::rac2::front_end::FrontEnd as Blob;
use rc_formats::rac2::toc::{self, Toc, MISC_FRONT_END, MOVIE_FILES};
use rc_formats::rac2::{gui_sprite, Picture};
use rc_formats::texture::Texture;
use rc_game2::front_end::{clock_seed, Assets as FrontAssets, FrontEnd, Request};
use rc_game2::gui;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::sync::Mutex;

/// The layer's textures: the front end's texture set, then these.
const SPLASH0: usize = 3;
const BACKDROP: usize = SPLASH0 + 3;
const MOVIE: usize = BACKDROP + 1;

/// Everything the front end reads, loaded once.
pub struct Data {
    root: PathBuf,
    assets: FrontAssets,
    textures: Vec<Texture>,
    /// Per movie row: its bytes' range across [`MOVIE_FILES`], and the files' sizes.
    movies: Vec<Option<toc::Movie>>,
    movie_files: Vec<u64>,
}

impl Data {
    pub fn load(root: &Path) -> Result<Data> {
        let read = |rel: &str| crate::disc_source::read(root, rel);
        let hdr = read("RC2.HDR")?;
        let t = Toc::parse(&hdr).context("RC2.HDR")?;
        let misc = read("G/MISC.WAD")?;
        let part = t.misc_part(MISC_FRONT_END)?;
        let raw = misc.get(part.bytes()).context("MISC.WAD: the front end's part is past the file")?;
        let blob = Blob::parse(rc_formats::wad::decompress(raw).context("the front end's WAD")?)?;
        let hud = read("G/HUD.WAD")?;
        let block = t.gui_block()?;
        let sprites = gui_sprite::parse(hud.get(block.bytes()).context("HUD.WAD: the GUI block is past the file")?)?;
        let elf = read("SCUS_972.68")?;
        let boot = BootElf::parse(&elf)?;
        let assets = FrontAssets { glyphs: boot.glyphs(GLYPHS)?, glyphs_2: boot.glyphs(GLYPHS_2)?, strings: blob.strings(0)?, main_menu: boot.menu(MAIN_MENU)?, sprites };
        // The frame buffer's pictures are opaque; the texture set keeps its own alpha.
        let opaque = |p: Picture| { let mut rgba = p.rgba; rgba.chunks_mut(4).for_each(|c| c[3] = 0x80); Texture { width: p.width, height: p.height, rgba } };
        let mut textures: Vec<Texture> = blob.textures()?.into_iter().map(|p| Texture { width: p.width, height: p.height, rgba: p.rgba }).collect();
        if textures.len() != SPLASH0 { anyhow::bail!("the front end has {} textures, {SPLASH0} expected", textures.len()); }
        for i in 0..3 { textures.push(opaque(blob.splash(i)?)); }
        textures.push(opaque(blob.backdrop()?));
        textures.push(Texture { width: 512, height: 416, rgba: [0, 0, 0, 0x80].repeat(512 * 416) });
        let movies = (0..64).map(|r| t.movie(r)).collect::<rc_formats::buf::Result<Vec<_>>>()?;
        let movie_files = MOVIE_FILES.iter().map(|f| std::fs::metadata(root.join(f)).map(|m| m.len()).with_context(|| format!("{f} in {}", root.display()))).collect::<Result<Vec<_>>>()?;
        println!(
            "rac2: front end loaded: {} strings, {} GUI sprites, main menu {:?}",
            assets.strings.0.len(), assets.sprites.len(),
            assets.main_menu.iter().map(|r| String::from_utf8_lossy(assets.strings.get(r.text)).into_owned()).collect::<Vec<_>>()
        );
        Ok(Data { root: root.to_path_buf(), assets, textures, movies, movie_files })
    }
}

/// Movie `id`'s bytes: its row's range across [`MOVIE_FILES`] taken as one file.
fn movie_bytes(root: &Path, sizes: &[u64], m: toc::Movie) -> Result<Vec<u8>> {
    use std::io::{Read, Seek, SeekFrom};
    let (mut at, mut left) = (m.sector as u64 * 2048, m.bytes as u64);
    let mut out = Vec::with_capacity(left as usize);
    for (f, &size) in MOVIE_FILES.iter().zip(sizes) {
        if left == 0 { break; }
        if at >= size { at -= size; continue; }
        let n = left.min(size - at);
        let mut file = std::fs::File::open(root.join(f)).with_context(|| f.to_string())?;
        file.seek(SeekFrom::Start(at))?;
        let start = out.len();
        out.resize(start + n as usize, 0);
        file.read_exact(&mut out[start..])?;
        (at, left) = (0, left - n);
    }
    if left > 0 { anyhow::bail!("the movie runs {left} bytes past {}", MOVIE_FILES[2]); }
    Ok(out)
}

enum Msg { Opened(f64), Picture(Vec<u8>), End, Failed(String) }

/// A movie being played.
struct Playing {
    id: u32,
    rx: Mutex<Receiver<Msg>>,
    fps: Option<f64>,
    /// Frames shown and pictures received.
    frame: u64,
    received: u64,
    ended: bool,
}

fn decode(bytes: Vec<u8>, tx: SyncSender<Msg>) {
    let mut m = match rc_video::Movie::open(&bytes, 0) {
        Ok(m) => m,
        Err(e) => { let _ = tx.send(Msg::Failed(e.to_string())); return; }
    };
    drop(bytes);
    if tx.send(Msg::Opened(m.fps())).is_err() { return; }
    loop {
        match m.next_frame() {
            Ok(Some(f)) => {
                let mut rgba = Vec::new();
                rc_video::movie::frame_to_rgba(&f, &mut rgba);
                rgba.chunks_mut(4).for_each(|c| c[3] = 0x80);
                if tx.send(Msg::Picture(rgba)).is_err() { return; }
            }
            Ok(None) => { let _ = tx.send(Msg::End); return; }
            Err(e) => { let _ = tx.send(Msg::Failed(e.to_string())); return; }
        }
    }
}

impl Playing {
    fn start(data: &Data, id: u32) -> Playing {
        let (tx, rx) = sync_channel(4);
        let m = toc::movie_row(id).and_then(|r| data.movies.get(r).copied().flatten());
        match m.map(|m| movie_bytes(&data.root, &data.movie_files, m)) {
            Some(Ok(bytes)) => { std::thread::spawn(move || decode(bytes, tx)); }
            Some(Err(e)) => { let _ = tx.send(Msg::Failed(e.to_string())); }
            None => { let _ = tx.send(Msg::Failed(format!("no movie {id:#x}"))); }
        }
        Playing { id, rx: Mutex::new(rx), fps: None, frame: 0, received: 0, ended: false }
    }

    /// One frame: the picture for it (waiting for the worker if it is behind), or None when it has not changed.
    fn advance(&mut self) -> Option<Vec<u8>> {
        let rx = self.rx.get_mut().unwrap_or_else(|e| e.into_inner());
        let mut picture = None;
        loop {
            let want = self.fps.map(|f| (self.frame as f64 * f / 60.0) as u64 + 1);
            if self.ended || want.is_some_and(|w| self.received >= w) { break; }
            match rx.recv() {
                Ok(Msg::Opened(f)) => self.fps = Some(f),
                Ok(Msg::Picture(p)) => { self.received += 1; picture = Some(p); }
                Ok(Msg::End) | Err(_) => self.ended = true,
                Ok(Msg::Failed(e)) => { eprintln!("rac2: movie {:#x}: {e}; it ends here", self.id); self.ended = true; }
            }
        }
        self.frame += 1;
        picture
    }
}

#[derive(Resource)]
struct Rac2 {
    data: Data,
    front: FrontEnd,
    movie: Option<Playing>,
    /// The movie's picture for the next draw.
    picture: Option<Vec<u8>>,
    movie_ended: bool,
    script: Option<crate::input_map::Script>,
    ticks: u64,
    trace: bool,
    layer: Option<Layer>,
}

/// Runs Going Commando from `root` (after crate::disc_source::startup checked it).
pub fn run(root: PathBuf) -> Result<()> {
    let data = Data::load(&root)?;
    let seed = clock_seed(if crate::determinism::deterministic() { [0; 8] } else { crate::saves::clock_now() });
    let script = std::env::var("RC_PLAY_SCRIPT").ok().map(|s| crate::input_map::Script::parse(&s).map_err(anyhow::Error::msg)).transpose()?;
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin { primary_window: Some(Window { title: "ReRAC".into(), resolution: (1024u32, 768u32).into(), ..default() }), ..default() })
            .set(AssetPlugin { file_path: crate::asset_dir(), ..default() }),
    )
    .add_plugins(crate::display::DisplayPlugin)
    .add_plugins(crate::determinism::DeterminismPlugin)
    .add_plugins(crate::frame_pace::FramePacePlugin)
    .add_plugins((Material2dPlugin::<HudMaterial>::default(), UiMaterialPlugin::<HudComposite>::default()))
    .insert_resource(ClearColor(Color::BLACK))
    .insert_resource(Rac2 {
        data,
        front: FrontEnd::new(seed),
        movie: None,
        picture: None,
        movie_ended: false,
        script,
        ticks: 0,
        trace: std::env::var("RC_RAC2_TRACE").is_ok_and(|v| v.trim() == "1"),
        layer: None,
    })
    .add_systems(Startup, setup)
    .add_systems(FixedUpdate, tick)
    .add_systems(Update, draw);
    app.run();
    Ok(())
}

fn setup(
    mut commands: Commands,
    mut rac2: ResMut<Rac2>,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<HudMaterial>>,
    mut composites: ResMut<Assets<HudComposite>>,
) {
    let cam = commands.spawn((Camera2d, Camera { clear_color: ClearColorConfig::Custom(Color::BLACK), ..default() }, Name::new("rac2 camera"))).id();
    let scaling = hud_render::scaling(crate::graphics::Hud::Original);
    let layer = Layer::spawn(&mut commands, &mut images, &mut meshes, &mut materials, &mut composites, &rac2.data.textures, scaling, cam);
    rac2.layer = Some(layer);
}

#[allow(clippy::too_many_arguments)]
fn tick(
    mut rac2: ResMut<Rac2>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    motion: Res<AccumulatedMouseMotion>,
    gamepads: Query<&Gamepad>,
) {
    let r = &mut *rac2;
    let held = match &r.script {
        Some(s) => s.at(r.ticks).buttons,
        None => crate::input_map::sample(&keys, &mouse, &motion, gamepads.iter().next()).buttons,
    };
    let ended = std::mem::take(&mut r.movie_ended);
    for req in r.front.tick(held, ended, &r.data.assets) {
        match req {
            Request::Sound(n) => if r.trace { println!("rac2: frame {}: sound {n} (not played)", r.ticks) },
            Request::Row { kind, target } => println!("rac2: main menu row (kind {kind}, target {target:#x}): not in this version yet"),
        }
    }
    // The movie the front end asks for: started, kept, or stopped.
    let want = r.front.movie();
    if r.movie.as_ref().map(|m| m.id) != want {
        r.movie = want.map(|id| {
            if r.trace { println!("rac2: frame {}: movie {id:#x}", r.ticks) }
            Playing::start(&r.data, id)
        });
    }
    if let Some(m) = r.movie.as_mut() {
        if let Some(p) = m.advance() { r.picture = Some(p); }
        if m.ended {
            r.movie_ended = true;
            r.movie = None;
        }
    }
    r.ticks += 1;
}

fn draw(mut rac2: ResMut<Rac2>, mut images: ResMut<Assets<Image>>, mut meshes: ResMut<Assets<Mesh>>) {
    let r = &mut *rac2;
    let Some(layer) = r.layer.as_ref() else { return };
    if let Some(p) = r.picture.take() { layer.write(&mut images, MOVIE, &p); }
    let mut d = Hud2d::default();
    for q in r.front.draw(&r.data.assets) {
        let (tex, nearest) = match q.tex {
            gui::Tex::None => (Tex::None, false),
            gui::Tex::Slot(i) => (Tex::Frame(i), false),
            gui::Tex::Picture(gui::Picture::Splash(i)) => (Tex::Frame(SPLASH0 + i), true),
            gui::Tex::Picture(gui::Picture::Backdrop) => (Tex::Frame(BACKDROP), true),
            gui::Tex::Picture(gui::Picture::Movie) => (Tex::Frame(MOVIE), true),
        };
        d.screen_prim(tex, q.pos, q.uv, u32::from_le_bytes(q.rgba), false);
        if nearest { if let Some(p) = d.prims.last_mut() { p.nearest = true; } }
    }
    if let Some(mut m) = meshes.get_mut(&layer.mesh) { *m = layer.build(&d); }
}

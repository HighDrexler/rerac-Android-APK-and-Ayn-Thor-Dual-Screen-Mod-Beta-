//! Opt-in software-rendered regression. Synthetic pixels only: no game files,
//! hardware device, timing/FPS claims or user data.
use super::*;
use bevy::app::PluginsState;
use bevy::camera::RenderTarget;
use bevy::ecs::system::RunSystemOnce;
use bevy::render::view::screenshot::{Screenshot, ScreenshotCaptured};
use std::sync::Arc;
use std::time::Duration;

fn pump(app: &mut App) {
    app.update();
    std::thread::sleep(Duration::from_millis(10));
}

/// Screenshot readback exercises the GPU render path, not CPU Image data.
fn check_image(app: &mut App, label: &str, accept: impl Fn([u8; 4]) -> bool) -> [u8; 4] {
    let deadline = Instant::now() + Duration::from_secs(30);
    let output = std::env::var_os("RC_MOVIE_TEST_OUTPUT").map(PathBuf::from);
    loop {
        for _ in 0..4 { pump(app); }
        let result = Arc::new(Mutex::new(None));
        let receive = result.clone();
        let target = app.world().resource::<crate::display::GameFrame>().image.clone();
        app.world_mut().spawn(Screenshot::image(target)).observe(move |shot: On<ScreenshotCaptured>| {
            *receive.lock().unwrap() = Some(shot.image.clone());
        });
        let image = loop {
            pump(app);
            if let Some(image) = result.lock().unwrap().take() { break image; }
            assert!(Instant::now() < deadline, "{label}: screenshot timed out");
        };
        let rgba = image.try_into_dynamic().unwrap().to_rgba8();
        let pixel = rgba.get_pixel(rgba.width() / 2, rgba.height() / 2).0;
        if let Some(dir) = &output {
            std::fs::create_dir_all(dir).unwrap();
            rgba.save(dir.join(format!("{label}.png"))).unwrap();
        }
        if accept(pixel) {
            println!("movie-render-smoke: {label} centre={pixel:?} {}x{}", rgba.width(), rgba.height());
            return pixel;
        }
        assert!(Instant::now() < deadline, "{label}: unexpected rendered pixel {pixel:?}");
    }
}

fn fixture(failed: bool) -> Running {
    let (tx, rx) = sync_channel(8);
    if failed {
        tx.send(Msg::Failed("synthetic decoder failure".into())).ok().unwrap();
    } else {
        tx.send(Msg::Opened { fps: (30, 1), audio: None, info: "synthetic 512x416 solid red".into() }).ok().unwrap();
        tx.send(Msg::Picture(0, [240u8, 32, 16, 255].repeat(512 * 416))).ok().unwrap();
        tx.send(Msg::End(1)).ok().unwrap();
    }
    drop(tx);
    Running {
        req: MovieRequest::in_level(0, None),
        player: MoviePlayer::new(MovieContext { level: 1, ..default() }),
        rx: Mutex::new(rx), opened: false, audio: None, total: None,
        received: 0, rgba: Vec::new(), shown: None, wants_picture: false,
        black: 0.0, pad: PadState::default(), frames: 0, last_phase: None,
        owner: None, media_cancelled: false, started: Instant::now(),
        presented: 0, initial_underruns: 0, present_updates: 0,
        first_present_update: None, last_present_update: None, max_present_gap: Duration::ZERO,
    }
}

fn tick(app: &mut App) {
    app.world_mut().run_system_once(movie_frame).unwrap();
    pump(app);
}

fn finish_movie(app: &mut App) {
    for _ in 0..30 {
        if app.world().resource::<MovieState>().running.as_ref().unwrap().player.done() { break; }
        tick(app);
    }
    assert!(app.world().resource::<MovieState>().running.as_ref().unwrap().player.done());
    app.world_mut().run_system_once(movie_exit).unwrap();
    pump(app);
    assert!(app.world().resource::<MovieState>().running.is_none());
}

#[test]
#[ignore = "requires Vulkan software renderer; explicitly run in beta8 CI"]
fn rendered_movie_entry_picture_fade_exit_skip_and_failure() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins
        .set(WindowPlugin { primary_window: None, exit_condition: bevy::window::ExitCondition::DontExit, ..default() })
        .disable::<bevy::winit::WinitPlugin>()
        .disable::<bevy::audio::AudioPlugin>())
        .add_plugins(crate::display::DisplayPlugin)
        .insert_resource(crate::determinism::Deterministic(true))
        .init_resource::<crate::determinism::FrameNumber>()
        .init_resource::<crate::determinism::GameTicks>()
        .insert_resource(bevy::time::TimeUpdateStrategy::FixedTimesteps(0))
        .add_plugins(MovieRenderPlugin);
    let deadline = Instant::now() + Duration::from_secs(30);
    while app.plugins_state() != PluginsState::Ready {
        bevy::tasks::tick_global_task_pools_on_main_thread();
        assert!(Instant::now() < deadline, "renderer initialization timed out");
        std::thread::sleep(Duration::from_millis(10));
    }
    app.finish();
    app.cleanup();
    let transform = Transform::from_xyz(0.0, 0.0, 5.0);
    let main = app.world_mut().spawn((
        Camera3d::default(),
        Camera { clear_color: ClearColorConfig::Custom(Color::srgb_u8(12, 50, 180)), ..default() },
        Msaa::Off, Tonemapping::None, DebandDither::Disabled,
        crate::render_settings::WorldCamera,
        bevy::ui::IsDefaultUiCamera,
        FlyCam::from_transform(&transform, 1.0), transform,
    )).id();
    pump(&mut app);
    // There is no host window in this test; capture the actual offscreen game frame.
    let present: Vec<Entity> = app.world_mut().query_filtered::<Entity, With<crate::display::PresentCamera>>().iter(app.world()).collect();
    for e in present { app.world_mut().get_mut::<Camera>(e).unwrap().is_active = false; }
    let movie_camera = app.world_mut().query_filtered::<Entity, With<MovieCamera>>().single(app.world()).unwrap();
    let frame = app.world().resource::<crate::display::GameFrame>().image.id();
    for e in [main, movie_camera] {
        let RenderTarget::Image(image) = app.world().get::<RenderTarget>(e).unwrap() else { panic!("DisplayPlugin did not redirect the view") };
        assert_eq!(image.handle.id(), frame);
    }
    let blue = |p: [u8; 4]| p[2] > 160 && p[0] < 25;
    check_image(&mut app, "01-world", blue);
    app.world_mut().resource_mut::<MovieState>().running = Some(fixture(false));
    tick(&mut app); // partially transparent entry fade on the original world view
    check_image(&mut app, "02-entry-fade", |p| p[2] > 60 && p[2] < 160 && p[0] < 25);
    assert!(app.world().get::<Camera>(main).unwrap().is_active);
    for _ in 0..3 { tick(&mut app); } // closing opaque entry fade
    check_image(&mut app, "03-entry-black", |p| p[0] < 3 && p[1] < 3 && p[2] < 3);
    assert!(!app.world().get::<Camera>(main).unwrap().is_active);
    assert!(app.world().get::<Camera>(movie_camera).unwrap().is_active);
    let root = app.world().resource::<MovieState>().root.unwrap();
    assert_eq!(app.world().get::<UiTargetCamera>(root).unwrap().0, movie_camera);
    tick(&mut app);
    tick(&mut app); // Playing(0): actual image upload and UI draw
    check_image(&mut app, "04-movie-red", |p| p[0] > 225 && p[1] > 20 && p[1] < 45 && p[2] < 25);
    tick(&mut app); // second 60 Hz tick of the 30 FPS picture
    tick(&mut app); // first exit fade
    check_image(&mut app, "05-exit-fade", |p| p[0] > 80 && p[0] < 225 && p[1] < 40);
    finish_movie(&mut app);
    check_image(&mut app, "06-restored-world", blue);
    assert!(app.world().get::<Camera>(main).unwrap().is_active);
    assert!(!app.world().get::<Camera>(movie_camera).unwrap().is_active);
    assert_eq!(app.world().get::<UiTargetCamera>(root).unwrap().0, main);
    for (failed, label) in [(false, "07-skip-restores-world"), (true, "08-failure-restores-world")] {
        app.world_mut().resource_mut::<MovieState>().running = Some(fixture(failed));
        for _ in 0..5 { tick(&mut app); }
        if !failed {
            let mut state = app.world_mut().resource_mut::<MovieState>();
            let pad = &mut state.running.as_mut().unwrap().pad;
            pad.held = button::START | button::L1 | button::L2 | button::R1 | button::R2;
            pad.pressed = button::START;
            drop(state);
            tick(&mut app);
            let state = app.world().resource::<MovieState>();
            let running = state.running.as_ref().unwrap();
            assert!(running.player.skipped, "skip input must end playback before natural EOF");
            assert!(running.media_cancelled, "skip must cancel queued movie media");
        }
        finish_movie(&mut app);
        assert_eq!(app.world().resource::<MovieState>().finished.last().map(|(_, skipped)| *skipped), Some(!failed));
        check_image(&mut app, label, blue);
        assert!(app.world().get::<Camera>(main).unwrap().is_active);
        assert!(!app.world().get::<Camera>(movie_camera).unwrap().is_active);
        assert_eq!(app.world().get::<UiTargetCamera>(root).unwrap().0, main);
    }
    println!("movie-render-smoke: passed; generated frames, software renderer, no physical FPS measurement");
}

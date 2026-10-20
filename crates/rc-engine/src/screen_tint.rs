//! **Full-screen tints from class draw callbacks** (G-REN-030): `rc_game::moby_update::classes::draw_callbacks::
//! screen_tints` (Hoven's turret 1267's red screen, Quartu / the Fleet's alarm 408, Giant Clank's beam flash) as one
//! full-screen GS-blend pass on the main camera ([`crate::gs_post`]): each tint in order, `Cv = ((A − B)·As >> 7) + D` on
//! the display bytes with A / B / D from the low byte of its ALPHA_1 (0x44: Cs, Cd, Cd; 0x42: 0, Cs, Cd),
//! `assets/shaders/screen_tint.wgsl`.
//!
//! The game draws them inside the world pass (the list-2 callbacks after the ties, the beam's sprite after the shrubs),
//! so the mobys, particles and effects drawn later are not tinted. The pass runs after the 3-D passes, so the world
//! draws leave the frame's alpha at 0 and every later draw raises it by its coverage (crate::gs_state
//! `specialize_world`): each tint is mixed in by 1 − alpha. [L] A translucent effect over the world takes the tint
//! in proportion (the game tints the world under it and blends the effect over that). The pass runs every frame on the
//! main camera (no tint: a copy) and writes alpha 1, so the frame's alpha is whole again for the present sprite, the
//! menu snapshot and the screenshots.

use bevy::core_pipeline::fullscreen_material::FullscreenMaterial;
use bevy::ecs::schedule::ScheduleConfigs;
use bevy::ecs::system::BoxedSystem;
use bevy::prelude::*;
use bevy::render::extract_component::ExtractComponent;
use bevy::render::render_resource::ShaderType;
use bevy::shader::ShaderRef;

use crate::fly_cam::FlyCam;

/// The frame's tints (at most four; GS RGBA bytes, A = 0x80 → 1.0) and their ALPHA_1 low bytes.
#[derive(Component, ExtractComponent, Clone, Copy, Default, PartialEq, ShaderType)]
pub struct ScreenTint {
    pub rgba: [UVec4; 4],
    pub alpha: UVec4,
    pub count: UVec4,
}

impl FullscreenMaterial for ScreenTint {
    fn fragment_shader() -> ShaderRef { "shaders/screen_tint.wgsl".into() }

    /// After the 3-D passes, before the HUD.
    fn schedule_configs(system: ScheduleConfigs<BoxedSystem>) -> ScheduleConfigs<BoxedSystem> {
        system.after(bevy::core_pipeline::Core3dSystems::PostProcess).before(bevy::ui_render::ui_pass)
    }
}

pub struct ScreenTintPlugin;

impl Plugin for ScreenTintPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(crate::gs_post::GsPostPlugin::<ScreenTint>::default()).add_systems(PostUpdate, update);
    }
}

fn update(mut commands: Commands, play: Option<Res<crate::gameplay::Play>>, cams: Query<(Entity, Option<&ScreenTint>), With<FlyCam>>) {
    let Some((entity, have)) = cams.iter().next() else { return };
    let tints = play.as_deref().map(|p| rc_game::moby_update::classes::draw_callbacks::screen_tints(&p.game.mobys, &p.svc, p.game.counter)).unwrap_or_default();
    // `RC_SCREEN_TINT=<rgba hex>:<ALPHA_1 low byte hex>` (dev only): one more tint every frame, to check the pass on
    // any level (e.g. `00ffff00:42` with alpha 0xff is Giant Clank's beam flash).
    let forced = std::env::var("RC_SCREEN_TINT").ok().and_then(|v| {
        let (c, a) = v.split_once(':')?;
        Some(rc_game::moby_update::classes::draw_callbacks::ScreenTint { rgba: u32::from_str_radix(c, 16).ok()?, alpha: u8::from_str_radix(a, 16).ok()? })
    });
    let tints: Vec<_> = tints.into_iter().chain(forced).collect();
    // Always present: with no tint the pass only writes the frame's alpha back to 1 (module doc).
    let mut want = ScreenTint::default();
    for (k, s) in tints.iter().take(4).enumerate() {
        want.rgba[k] = UVec4::new(s.rgba & 0xff, (s.rgba >> 8) & 0xff, (s.rgba >> 16) & 0xff, s.rgba >> 24);
        want.alpha[k] = s.alpha as u32;
    }
    want.count.x = tints.len().min(4) as u32;
    if have != Some(&want) { commands.entity(entity).insert(want); }
}

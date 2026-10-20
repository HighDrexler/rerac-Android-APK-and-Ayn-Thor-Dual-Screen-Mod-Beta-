// Full-screen tints from class draw callbacks (Rust side: screen_tint.rs). Each tint is an untextured full-screen
// sprite with RGBAQ = its bytes, blended by the GS as Cv = ((A − B)·As >> 7) + D on the frame buffer's 8-bit,
// display-encoded values (COLCLAMP clamps to 0..255), A / B / D picked by the low byte of its ALPHA_1 (bits 0–1 A,
// 2–3 B, 6–7 D: 0 Cs, 1 Cd, 2 zero; C = As). As in underwater_tint.wgsl the pixel is re-encoded to the byte the GS
// would hold, blended in integers and written back. The game draws the tints inside the world pass, so only the part
// of the pixel drawn before the mobys takes them: the frame's alpha is the part drawn after the shrubs
// (gs_state.rs `specialize_world`), the tinted bytes are mixed back by 1 − alpha, and alpha 1 is written (this pass
// runs every frame on the main camera, with no tint too).

#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

@group(0) @binding(0) var screen_texture: texture_2d<f32>;
@group(0) @binding(1) var screen_sampler: sampler;

struct ScreenTint {
    rgba: array<vec4<u32>, 4>,
    alpha: vec4<u32>,
    count: vec4<u32>,
}
@group(0) @binding(2) var<uniform> tint: ScreenTint;

fn to_display(c: vec3<f32>) -> vec3<f32> {
    let lo = c * 12.92;
    let hi = 1.055 * pow(max(c, vec3(0.0)), vec3(1.0 / 2.4)) - 0.055;
    return select(hi, lo, c <= vec3(0.0031308));
}

fn to_linear(c: vec3<f32>) -> vec3<f32> {
    let lo = c / 12.92;
    let hi = pow((c + 0.055) / 1.055, vec3(2.4));
    return select(hi, lo, c <= vec3(0.04045));
}

fn pick(sel: u32, cs: vec3<i32>, cd: vec3<i32>) -> vec3<i32> {
    if sel == 0u { return cs; }
    if sel == 1u { return cd; }
    return vec3(0);
}

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let d = textureLoad(screen_texture, vec2<i32>(floor(in.position.xy)), 0);
    if tint.count.x == 0u { return vec4(d.rgb, 1.0); }
    let c0 = vec3<i32>(round(clamp(to_display(d.rgb), vec3(0.0), vec3(1.0)) * 255.0));
    var cd = c0;
    for (var k = 0u; k < min(tint.count.x, 4u); k++) {
        let t = tint.rgba[k];
        let cs = vec3<i32>(t.rgb);
        let a = i32(t.a);
        let sel = tint.alpha[k];
        let av = pick(sel & 3u, cs, cd);
        let bv = pick((sel >> 2u) & 3u, cs, cd);
        let dv = pick((sel >> 6u) & 3u, cs, cd);
        cd = clamp((((av - bv) * a) >> vec3<u32>(7u)) + dv, vec3(0), vec3(255));
    }
    let world = 1.0 - clamp(d.a, 0.0, 1.0);
    let c = mix(vec3<f32>(c0), vec3<f32>(cd), world);
    return vec4(to_linear(c / 255.0), 1.0);
}

// Bloom 3: upsample the smaller level and merge it with this level.
// PipeLineType::NormalWithScreen, default vertex shader, uniforms at @group(3).
// Draw into u8:  set_target(u8)
//                draw_fullscreen(d8.texture_id, WHITE, 0, 0)     <- plain copy of this level (default pipeline)
//                draw_fullscreen(d16.texture_id, WHITE, 1, up)   <- reads the screen (= d8) + its texture (= d16)
// (same for u4 with d4 + u8, and u2 with d2 + u4)

struct VertexOutput
{
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) tex_coords: vec2<f32>,
    @location(2) mode: u32,
    @location(3) world_pos: vec3<f32>
};

struct UpUniforms
{
    radius: f32, // tent radius in texels of the smaller level, ~1.0
    spread: f32, // 0..1, how much the wider levels count
};

@group(1) @binding(0) var texture: texture_2d<f32>; // the smaller level
@group(1) @binding(1) var texture_sampler: sampler;
@group(2) @binding(0) var screen: texture_2d<f32>;  // this level (copied from the target)
@group(2) @binding(1) var screen_sampler: sampler;
@group(3) @binding(0) var<uniform> settings: UpUniforms;

fn tap(uv: vec2<f32>, offset: vec2<f32>, x: f32, y: f32) -> vec3<f32>
{
    return textureSample(texture, texture_sampler, uv + vec2<f32>(x, y) * offset).rgb;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32>
{
    let uv = in.tex_coords;
    let offset = settings.radius / vec2<f32>(textureDimensions(texture));

    // 3x3 tent filter
    let center = tap(uv, offset, 0.0, 0.0) * 4.0;
    let edges = (tap(uv, offset, 0.0, -1.0) + tap(uv, offset, -1.0, 0.0) + tap(uv, offset, 1.0, 0.0) + tap(uv, offset, 0.0, 1.0)) * 2.0;
    let corners = tap(uv, offset, -1.0, -1.0) + tap(uv, offset, 1.0, -1.0) + tap(uv, offset, -1.0, 1.0) + tap(uv, offset, 1.0, 1.0);
    let wide = (center + edges + corners) / 16.0;

    let screen_uv = in.clip_position.xy / vec2<f32>(textureDimensions(screen));
    let current = textureSample(screen, screen_sampler, screen_uv).rgb;

    return vec4<f32>(current + wide, 1.0);
}

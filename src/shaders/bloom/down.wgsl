// Bloom 2: one more downsample level. PipeLineType::Normal, default vertex shader, no uniforms.
// Draw: set_target(d4), draw_fullscreen(d2.texture_id, WHITE, 0, down)   (same for d8, d16)

struct VertexOutput
{
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) tex_coords: vec2<f32>,
    @location(2) mode: u32,
    @location(3) world_pos: vec3<f32>
};

@group(1) @binding(0) var texture: texture_2d<f32>; // the bigger level
@group(1) @binding(1) var texture_sampler: sampler;

fn tap(uv: vec2<f32>, texel: vec2<f32>, x: f32, y: f32) -> vec3<f32>
{
    return textureSample(texture, texture_sampler, uv + vec2<f32>(x, y) * texel).rgb;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32>
{
    let uv = in.tex_coords;
    let texel = 1.0 / vec2<f32>(textureDimensions(texture));

    let a = tap(uv, texel, -2.0, -2.0); let b = tap(uv, texel, 0.0, -2.0); let c = tap(uv, texel, 2.0, -2.0);
    let d = tap(uv, texel, -2.0,  0.0); let e = tap(uv, texel, 0.0,  0.0); let f = tap(uv, texel, 2.0,  0.0);
    let g = tap(uv, texel, -2.0,  2.0); let h = tap(uv, texel, 0.0,  2.0); let i = tap(uv, texel, 2.0,  2.0);
    let j = tap(uv, texel, -1.0, -1.0); let k = tap(uv, texel, 1.0, -1.0);
    let l = tap(uv, texel, -1.0,  1.0); let m = tap(uv, texel, 1.0,  1.0);

    let color = e * 0.125 + (a + c + g + i) * 0.03125 + (b + d + f + h) * 0.0625 + (j + k + l + m) * 0.125;
    return vec4<f32>(color, 1.0);
}

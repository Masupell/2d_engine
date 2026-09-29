// Bloom 1: bright pass + first downsample. PipeLineType::Normal, default vertex shader, uniforms at @group(2).
// Draw: set_target(d2), draw_fullscreen(scene.texture_id, WHITE, 0, extract)

struct VertexOutput
{
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) tex_coords: vec2<f32>,
    @location(2) mode: u32,
    @location(3) world_pos: vec3<f32>
};

struct ExtractUniforms
{
    threshold: f32,
    knee: f32,
};

@group(1) @binding(0) var texture: texture_2d<f32>; // the scene
@group(1) @binding(1) var texture_sampler: sampler;
@group(2) @binding(0) var<uniform> extract: ExtractUniforms;

const LUMA: vec3<f32> = vec3<f32>(0.2126, 0.7152, 0.0722);

fn tap(uv: vec2<f32>, texel: vec2<f32>, x: f32, y: f32) -> vec3<f32>
{
    return textureSample(texture, texture_sampler, uv + vec2<f32>(x, y) * texel).rgb;
}

// 13-tap downsample (Jorge Jimenez, Call of Duty: Advanced Warfare), weights sum to 1
fn downsample_13(uv: vec2<f32>) -> vec3<f32>
{
    let texel = 1.0 / vec2<f32>(textureDimensions(texture));

    let a = tap(uv, texel, -2.0, -2.0); let b = tap(uv, texel, 0.0, -2.0); let c = tap(uv, texel, 2.0, -2.0);
    let d = tap(uv, texel, -2.0,  0.0); let e = tap(uv, texel, 0.0,  0.0); let f = tap(uv, texel, 2.0,  0.0);
    let g = tap(uv, texel, -2.0,  2.0); let h = tap(uv, texel, 0.0,  2.0); let i = tap(uv, texel, 2.0,  2.0);
    let j = tap(uv, texel, -1.0, -1.0); let k = tap(uv, texel, 1.0, -1.0);
    let l = tap(uv, texel, -1.0,  1.0); let m = tap(uv, texel, 1.0,  1.0);

    return e * 0.125 + (a + c + g + i) * 0.03125 + (b + d + f + h) * 0.0625 + (j + k + l + m) * 0.125;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32>
{
    let color = downsample_13(in.tex_coords);
    let bright = color * smoothstep(extract.threshold, extract.threshold + extract.knee, dot(color, LUMA));
    return vec4<f32>(bright, 1.0);
}

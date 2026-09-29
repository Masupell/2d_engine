// Final Bloom step

struct VertexOutput
{
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) tex_coords: vec2<f32>,
    @location(2) mode: u32,
    @location(3) world_pos: vec3<f32>
};

struct CompositeUniforms
{
    intensity: f32,
};

@group(1) @binding(0) var texture: texture_2d<f32>; // the glow (at half resolution)
@group(1) @binding(1) var texture_sampler: sampler;
@group(2) @binding(0) var screen: texture_2d<f32>;  // final scene
@group(2) @binding(1) var screen_sampler: sampler;
@group(3) @binding(0) var<uniform> composite: CompositeUniforms;

const KNEE: f32 = 0.8;
fn soft_clip(color: vec3<f32>) -> vec3<f32>
{
    let over = max(color - KNEE, vec3<f32>(0.0));
    let squeezed = KNEE + (1.0 - KNEE) * (1.0 - exp(-over / (1.0 - KNEE)));
    return mix(color, squeezed, step(vec3<f32>(KNEE), color));
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32>
{
    let screen_uv = in.clip_position.xy / vec2<f32>(textureDimensions(screen));
    let scene = textureSample(screen, screen_sampler, screen_uv);

    let bloom = textureSample(texture, texture_sampler, in.tex_coords).rgb * composite.intensity;
    return vec4<f32>(soft_clip(scene.rgb + bloom), scene.a);
}

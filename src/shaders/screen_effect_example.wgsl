// With PipeLineType::NormalWithScreen

struct VertexOutput
{
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) tex_coords: vec2<f32>,
    @location(2) mode: u32,
    @location(3) world_pos: vec3<f32>
};

// @group(0) is the camera matrix

@group(1) @binding(0) var texture: texture_2d<f32>;
@group(1) @binding(1) var texture_sampler: sampler;

// Screen Texture
@group(2) @binding(0) var screen: texture_2d<f32>;
@group(2) @binding(1) var screen_sampler: sampler;

// @group(3) for uniforms

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32>
{
    let screen_uv = in.clip_position.xy / vec2<f32>(textureDimensions(screen));
    let behind = textureSample(screen, screen_sampler, screen_uv);

    // Example: simple vignette, darker towards the edges
    let from_center = screen_uv - 0.5;
    let vignette = 1.0 - smoothstep(0.35, 0.85, length(from_center) * 1.2);

    return vec4<f32>(behind.rgb * vignette, 1.0);
}

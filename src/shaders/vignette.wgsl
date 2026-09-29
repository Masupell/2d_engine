struct VertexOutput
{
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) tex_coords: vec2<f32>,
    @location(2) mode: u32,
    @location(3) world_pos: vec3<f32>
};

@group(1) @binding(0) var texture: texture_2d<f32>;
@group(1) @binding(1) var texture_sampler: sampler;

@group(2) @binding(0) var screen: texture_2d<f32>;
@group(2) @binding(1) var screen_sampler: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32>
{
    let uv = in.clip_position.xy / vec2<f32>(textureDimensions(screen));
    let base_color = textureSample(screen, screen_sampler, uv);

    let uv_centered = uv - vec2<f32>(0.5, 0.5);
    let dist = length(uv_centered);
    let vignette = smoothstep(0.1, 0.78, dist);

    let vignette_color = vec3<f32>(0.0, 0.0, 0.0);
    let final_rgb = mix(base_color.rgb, vignette_color, vignette);

    return vec4<f32>(final_rgb, base_color.a);
}

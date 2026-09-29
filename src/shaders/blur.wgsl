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

@group(3) @binding(0)
var<uniform> radius: f32;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32>
{
    let texel_size = 1.0 / vec2<f32>(textureDimensions(screen));
    let uv = in.clip_position.xy * texel_size;
    let spacing = max(radius, 0.0);

    var color_sum = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    var weight_sum = 0.0;

    for (var y = -4; y <= 4; y = y + 1)
    {
        for (var x = -4; x <= 4; x = x + 1)
        {
            let offset = vec2<f32>(f32(x), f32(y)) * spacing * texel_size;
            let dist_sq = f32(x * x + y * y);
            let weight = exp(-dist_sq / 8.0);

            color_sum = color_sum + textureSample(screen, screen_sampler, uv + offset) * weight;
            weight_sum = weight_sum + weight;
        }
    }

    let blurred = color_sum / weight_sum;

    return vec4<f32>(blurred.rgb * in.color.rgb, blurred.a * in.color.a);
}

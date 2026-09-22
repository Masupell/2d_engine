struct VertexInput
{
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,

    @location(2) model0: vec4<f32>,
    @location(3) model1: vec4<f32>,
    @location(4) model2: vec4<f32>,
    @location(5) model3: vec4<f32>,
    @location(6) color: vec4<f32>,
    @location(7) mode: u32,
}

struct VertexOutput
{
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) tex_coords: vec2<f32>,
    @location(2) mode: u32,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput
{
    var out: VertexOutput;

    out.clip_position = vec4<f32>(in.position*2.0, 1.0);
    out.color = in.color;
    out.tex_coords = in.tex_coords;
    out.mode = in.mode;
    return out;
}



@group(0) @binding(0)
var texture: texture_2d<f32>;
@group(0) @binding(1)
var texture_sampler: sampler;

@group(1) @binding(0)
var<uniform> radius: f32;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32>
{
    let texel_size = 1.0 / vec2<f32>(textureDimensions(texture));
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

            color_sum = color_sum + textureSample(texture, texture_sampler, in.tex_coords + offset) * weight;
            weight_sum = weight_sum + weight;
        }
    }

    let blurred = color_sum / weight_sum;

    return vec4<f32>(blurred.rgb * in.color.rgb, blurred.a * in.color.a);
}

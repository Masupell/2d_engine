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

    out.clip_position = vec4<f32>(in.position*2.0, 1.0); //*2.0, because my quad is from -0.5 to 0.5, not -1.0 to 1.0
    out.color = in.color;
    out.tex_coords = in.tex_coords;
    out.mode = in.mode;
    return out;
}





@group(0) @binding(0)
var texture: texture_2d<f32>;
@group(0) @binding(1)
var texture_sampler: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> 
{
    let base_color = textureSample(texture, texture_sampler, in.tex_coords);

    let uv = in.tex_coords;
    let uv_centered = uv - vec2<f32>(0.5, 0.5);
    let dist = length(uv_centered);
    let vignette = smoothstep(0.1, 0.8, dist);
    let final_color = base_color * (1.0 - vignette * 0.8);

    return final_color;
}
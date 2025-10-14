struct VertexInput
{
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,

    //Insstance
    @location(2) model0: vec4<f32>,
    @location(3) model1: vec4<f32>,
    @location(4) model2: vec4<f32>,
    @location(5) model3: vec4<f32>,
    @location(6) color: vec4<f32>,

    @location(7) mode: u32,
    // @location(8) texture_id: u32
}

struct VertexOutput 
{
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) tex_coords: vec2<f32>,
    @location(2) mode: u32,
    // @location(3) texture_id: u32
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput 
{
    var out: VertexOutput;

    let model = mat4x4<f32>(
        in.model0,
        in.model1,
        in.model2,
        in.model3
    );

    out.clip_position = model * vec4<f32>(in.position, 1.0);
    out.color = in.color;
    out.tex_coords = in.tex_coords;
    out.mode = in.mode;
    // out.texture_id = in.texture_id;
    return out;
}


@group(0) @binding(0)
var texture: texture_2d<f32>;
@group(0) @binding(1)
var texture_sampler: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> 
{
    // return vec4<f32>(0.3, 0.2, 0.1, 1.0);
    // return in.color;
    let tex_color = textureSample(texture, texture_sampler, in.tex_coords);
    let final_color = select(in.color, tex_color, in.mode == 1u);
    return final_color;
}
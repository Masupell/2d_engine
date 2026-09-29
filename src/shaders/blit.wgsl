struct VertexInput
{
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>
}

struct VertexOutput
{
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>
}

@group(0) @binding(0) var source: texture_2d<f32>;
@group(0) @binding(1) var source_sampler: sampler;

@vertex
fn vs_main(in: VertexInput) -> VertexOutput
{
    var out: VertexOutput;
    out.clip_position = vec4<f32>(in.position.xy * 2.0, 0.0, 1.0);
    out.uv = in.tex_coords;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32>
{
    return textureSample(source, source_sampler, in.uv);
}

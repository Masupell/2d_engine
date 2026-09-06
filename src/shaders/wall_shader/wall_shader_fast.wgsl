// Should be a lot faster, then all the other wall_shaders
struct VertexOutput
{
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) tex_coords: vec2<f32>,
    @location(2) mode: u32,
    @location(3) world_pos: vec2<f32>,
};

@group(1) @binding(0)
var texture: texture_2d<f32>;
@group(1) @binding(1)
var texture_sampler: sampler;

@group(2) @binding(0)
var<uniform> band_height: f32;

fn hash(p: vec2<f32>) -> f32
{
    var p3 = fract(vec3<f32>(p.x, p.y, p.x) * 0.1031);
    p3 = p3 + dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

fn value_noise(p: vec2<f32>) -> f32
{
    let i = floor(p);
    let f = fract(p);

    let a = hash(i);
    let b = hash(i + vec2<f32>(1.0, 0.0));
    let c = hash(i + vec2<f32>(0.0, 1.0));
    let d = hash(i + vec2<f32>(1.0, 1.0));

    let u = f * f * (3.0 - 2.0 * f);

    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

fn palette_color(index: i32) -> vec3<f32>
{
    let i = u32(((index % 6) + 6) % 6);
    let palette = array<vec3<f32>, 6>
    (
        vec3<f32>(0.55, 0.24, 0.16),
        vec3<f32>(0.70, 0.43, 0.22),
        vec3<f32>(0.80, 0.65, 0.42),
        vec3<f32>(0.87, 0.79, 0.63),
        vec3<f32>(0.42, 0.31, 0.27),
        vec3<f32>(0.36, 0.17, 0.13),
    );

    return palette[i];
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32>
{
    // Straight bands
    let band_coord = in.world_pos.y / band_height;
    let band_index = i32(floor(band_coord));
    let band_frac = fract(band_coord);

    let seed_a = hash(vec2<f32>(f32(band_index) * 1.7, 4.2));
    let seed_b = hash(vec2<f32>(f32(band_index + 1) * 1.7, 4.2));
    let color_a = palette_color(i32(seed_a * 100.0));
    let color_b = palette_color(i32(seed_b * 100.0));

    let banded = mix(color_a, color_b, smoothstep(0.98, 1.0, band_frac));

    let seam_distance = min(band_frac, 1.0 - band_frac);
    let outline = 1.0 - smoothstep(0.0, 0.02, seam_distance);
    let outlined = banded * mix(1.0, 0.3, outline);

    let grain = value_noise(in.world_pos * 0.02);
    let posterized_grain = floor(grain * 3.0) / 3.0;
    let varied = outlined * mix(0.9, 1.08, posterized_grain);

    let edge_distance = min(in.tex_coords.x, 1.0 - in.tex_coords.x);
    let edge_fade = smoothstep(0.0, 0.1, edge_distance);
    let edge_shadowed = mix(varied * 0.35, varied, edge_fade);

    let final_color = edge_shadowed * in.color.rgb;
    return vec4<f32>(final_color, in.color.a);
}

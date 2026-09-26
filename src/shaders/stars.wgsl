// Star shader, star sdf from here: "https://iquilezles.org/articles/distfunctions2d/"

struct VertexOutput
{
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) tex_coords: vec2<f32>,
    @location(2) mode: u32,
    @location(3) world_pos: vec3<f32>
};

@group(1) @binding(0)
var texture: texture_2d<f32>;
@group(1) @binding(1)
var texture_sampler: sampler;

@group(2) @binding(0)
var<uniform> time: f32;

const TAU: f32 = 6.2831853;

const QUAD_ASPECT: f32 = 2.0; // 150.0/75.0
const STAR_COUNT: f32 = 4.0;
const ORBIT_RADIUS: vec2<f32> = vec2<f32>(1.35, 0.42);
const ORBIT_SPEED: f32 = 3.2;
const SPIN_SPEED: f32 = 4.0;
const STAR_RADIUS: f32 = 0.34;
const STAR_INNER: f32 = 0.5;
const BACK_SCALE: f32 = 0.65;
const OUTLINE_WIDTH: f32 = 0.07;

const FILL_COLOR: vec3<f32>    = vec3<f32>(1.0, 0.93, 0.35);
const BACK_COLOR: vec3<f32>    = vec3<f32>(0.8, 0.66, 0.22); // for stars in the back
const OUTLINE_COLOR: vec3<f32> = vec3<f32>(0.25, 0.14, 0.04);

fn rotate(p: vec2<f32>, angle: f32) -> vec2<f32>
{
    let c = cos(angle);
    let s = sin(angle);
    return vec2<f32>(p.x * c - p.y * s, p.x * s + p.y * c);
}

// 5-pointed star
fn sd_star5(p_in: vec2<f32>, r: f32, rf: f32) -> f32
{
    let v1 = vec2<f32>(0.809016994, -0.587785252);
    let v2 = vec2<f32>(-v1.x, v1.y);

    var p = vec2<f32>(abs(p_in.x), p_in.y);
    p -= 2.0 * max(dot(v1, p), 0.0) * v1;
    p -= 2.0 * max(dot(v2, p), 0.0) * v2;
    p.x = abs(p.x);
    p.y -= r;

    let ba = rf * vec2<f32>(-v1.y, v1.x) - vec2<f32>(0.0, 1.0);
    let h = clamp(dot(p, ba) / dot(ba, ba), 0.0, r);
    return length(p - ba * h) * sign(p.y * ba.x - p.x * ba.y);
}

fn star_layer(p: vec2<f32>, index: f32, t: f32) -> vec3<f32>
{
    let angle = t * ORBIT_SPEED + index * TAU / STAR_COUNT;
    let center = vec2<f32>(cos(angle), sin(angle)) * ORBIT_RADIUS;

    let front = sin(angle) * 0.5 + 0.5;
    let scale = mix(BACK_SCALE, 1.0, front);

    let local = rotate(p - center, t * SPIN_SPEED + index) / scale;
    let d = sd_star5(vec2<f32>(local.x, -local.y), STAR_RADIUS, STAR_INNER) * scale;

    return vec3<f32>(step(d, 0.0), step(d, OUTLINE_WIDTH), front);
}

fn composite(color_alpha: vec4<f32>, star: vec3<f32>) -> vec4<f32>
{
    let star_color = mix(OUTLINE_COLOR, mix(BACK_COLOR, FILL_COLOR, star.z), star.x);
    return vec4<f32>(mix(color_alpha.rgb, star_color, star.y), max(color_alpha.a, star.y));
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32>
{
    let uv = in.tex_coords * 2.0 - 1.0;
    let p = vec2<f32>(uv.x * QUAD_ASPECT, uv.y);

    var result = vec4<f32>(0.0);
    result = composite(result, star_layer(p, 0.0, time));
    result = composite(result, star_layer(p, 1.0, time));
    result = composite(result, star_layer(p, 2.0, time));
    result = composite(result, star_layer(p, 3.0, time));

    return vec4<f32>(result.rgb * in.color.rgb, result.a * in.color.a);
}

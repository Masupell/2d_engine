// Based on "https://godotshaders.com/shader/2d-fire/", a bit reworked

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

const OUTLINE_COLOR: vec3<f32> = vec3<f32>(0.06, 0.04, 0.28);
const OUTER_COLOR: vec3<f32>   = vec3<f32>(0.32, 0.14, 0.88);
const MIDDLE_COLOR: vec3<f32>  = vec3<f32>(0.16, 0.56, 1.0);
const CORE_COLOR: vec3<f32>    = vec3<f32>(0.82, 0.97, 1.0);
const SPARK_COLOR: vec3<f32>   = vec3<f32>(0.75, 0.92, 1.0);

// Flames
const FLAMES: i32 = 7;
const RADIAL_FREQUENCY: f32 = 3.2;
const FLAME_SPEED: f32 = 1.5;
const SWIRL_SPEED: f32 = 0.08;
const SWIRL_TWIST: f32 = 0.25;
const BALL_RADIUS: f32 = 0.78;
const OUTLINE_WIDTH: f32 = 0.07;
const BAND_WIDTH: f32 = 0.18;
const PULSE_SPEED: f32 = 2.0;
const PULSE_AMOUNT: f32 = 0.08;

// Sparks
const SPARK_CELLS: i32 = 19;
const SPARK_RADIAL_FREQUENCY: f32 = 6.0;
const SPARK_SPEED: f32 = 1.6;
const SPARK_THRESHOLD: f32 = 0.86;
const SPARK_INNER: f32 = 0.4;
const SPARK_OUTER: f32 = 0.92;

// pcg hash function for seemingly random values, from here: "https://www.reedbeta.com/blog/hash-functions-for-gpu-rendering/"
fn pcg(v: u32) -> u32
{
    let state = v * 747796405u + 2891336453u;
    let word = ((state >> ((state >> 28u) + 4u)) ^ state) * 277803737u;
    return (word >> 22u) ^ word;
}

fn hash(cell: vec2<i32>) -> f32
{
    let h = pcg(bitcast<u32>(cell.x) + pcg(bitcast<u32>(cell.y)));
    return f32(h) * (1.0 / 4294967295.0);
}

fn periodic_noise(p: vec2<f32>, period: i32) -> f32
{
    let cell = vec2<i32>(floor(p));
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);

    let x0 = ((cell.x % period) + period) % period;
    let x1 = (x0 + 1) % period;

    let a = hash(vec2<i32>(x0, cell.y));
    let b = hash(vec2<i32>(x1, cell.y));
    let c = hash(vec2<i32>(x0, cell.y + 1));
    let d = hash(vec2<i32>(x1, cell.y + 1));

    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

fn flame_noise(p: vec2<f32>) -> f32
{
    var v = 0.5 * periodic_noise(p, FLAMES);
    v += 0.25 * periodic_noise(p * 2.0 + vec2<f32>(0.0, 13.7), FLAMES * 2);
    v += 0.125 * periodic_noise(p * 4.0 + vec2<f32>(0.0, 31.1), FLAMES * 4);
    return v / 0.875; // 0.5+0.25+0.125
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32>
{
    let uv = in.tex_coords * 2.0 - 1.0;
    let r = length(uv);

    let angle = atan2(uv.y, uv.x) / TAU + 0.5;
    let around = angle + time * SWIRL_SPEED + (1.0 - r) * SWIRL_TWIST;

    let flame_p = vec2<f32>(around * f32(FLAMES), r * RADIAL_FREQUENCY - time * FLAME_SPEED);
    let n = flame_noise(flame_p);

    let pulse = sin(time * PULSE_SPEED) * PULSE_AMOUNT;
    let g = 1.2 * (1.0 - r / BALL_RADIUS) + pulse;

    let outline = step(n, g + OUTLINE_WIDTH);
    let outer = step(n, g);
    let middle = step(n, g - BAND_WIDTH);
    let core = step(n, g - 2.0 * BAND_WIDTH);

    let flame_color = OUTLINE_COLOR * (outline - outer) + OUTER_COLOR * (outer - middle) + MIDDLE_COLOR * (middle - core) + CORE_COLOR * core;

    let spark_p = vec2<f32>(around * f32(SPARK_CELLS), r * SPARK_RADIAL_FREQUENCY - time * SPARK_SPEED);
    let spark_ring = step(SPARK_INNER, r) * step(r, SPARK_OUTER);
    let spark = step(SPARK_THRESHOLD, periodic_noise(spark_p, SPARK_CELLS)) * spark_ring * (1.0 - outline);

    let color = mix(flame_color, SPARK_COLOR, spark);
    let alpha = max(outline, spark);

    return vec4<f32>(color * in.color.rgb, alpha * in.color.a);
}

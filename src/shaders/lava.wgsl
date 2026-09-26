// Inspired by nimitz "https://www.shadertoy.com/view/lslXRS", but cartoony lava, not realistic (no smooth shading)

struct VertexOutput
{
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) tex_coords: vec2<f32>,
    @location(2) mode: u32,
    @location(3) world_pos: vec3<f32>
};

struct LavaUniforms
{
    time: f32,
    surface: f32,
};

@group(1) @binding(0)
var texture: texture_2d<f32>;
@group(1) @binding(1)
var texture_sampler: sampler;

@group(2) @binding(0)
var<uniform> lava: LavaUniforms;

const CRUST_COLOR: vec3<f32>   = vec3<f32>(0.2, 0.07, 0.06);
const OUTLINE_COLOR: vec3<f32> = vec3<f32>(0.12, 0.03, 0.02);
const DEEP_COLOR: vec3<f32>    = vec3<f32>(0.55, 0.07, 0.03);
const MID_COLOR: vec3<f32>     = vec3<f32>(0.92, 0.28, 0.04);
const HOT_COLOR: vec3<f32>     = vec3<f32>(1.0, 0.6, 0.1);
const GLOW_COLOR: vec3<f32>    = vec3<f32>(1.0, 0.88, 0.42);

const NOISE_SCALE: f32 = 1.0 / 140.0;
const FLOW_SPEED: f32 = 0.12;
const VEIN_FREQUENCY: f32 = 9.0;

// 7.5+3.5=10.5 < 30.0
// vec3<f32>(amplitude, frequency, speed)
const WAVE_1: vec3<f32> = vec3<f32>(7.0, 0.011, 1.3);
const WAVE_2: vec3<f32> = vec3<f32>(3.5, 0.027, -2.1);

const OUTLINE_WIDTH: f32 = 5.0;
const GLOW_WIDTH: f32 = 14.0;
const HEAT_DEPTH: f32 = 180.0;

// pcg hash function like in dash_orb
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

fn value_noise(p: vec2<f32>) -> f32
{
    let cell = vec2<i32>(floor(p));
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);

    let a = hash(cell);
    let b = hash(cell + vec2<i32>(1, 0));
    let c = hash(cell + vec2<i32>(0, 1));
    let d = hash(cell + vec2<i32>(1, 1));

    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

fn fbm(p: vec2<f32>) -> f32
{
    let m = mat2x2<f32>(0.8, 0.6, -0.6, 0.8);
    let p2 = m * p * 2.02;
    let p3 = m * p2 * 2.03;

    return (0.5 * value_noise(p) + 0.25 * value_noise(p2) + 0.125 * value_noise(p3)) / 0.875;
}

fn surface_offset(x: f32, t: f32) -> f32
{
    return sin(x * WAVE_1.y + t * WAVE_1.z) * WAVE_1.x + sin(x * WAVE_2.y + t * WAVE_2.z) * WAVE_2.x;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32>
{
    let t = lava.time;
    let world = in.world_pos.xy;

    // depth>0->in lava
    let depth = world.y - (lava.surface + surface_offset(world.x, t));

    let p = world * NOISE_SCALE;
    let warp = vec2<f32>
    (
        fbm(p + vec2<f32>(0.0, t * FLOW_SPEED)),
        fbm(p + vec2<f32>(5.2, 1.3) - vec2<f32>(t * FLOW_SPEED, 0.0))
    );
    let n = fbm(p * 1.3 + warp * 1.8 + vec2<f32>(t * 0.05, -t * 0.08));

    let veins = sin(n * VEIN_FREQUENCY) * 0.5 + 0.5;
    let heat = veins * 0.75 + (1.0 - smoothstep(0.0, HEAT_DEPTH, depth)) * 0.35;

    let bands = mix(mix(mix(DEEP_COLOR, MID_COLOR, step(0.35, heat)), HOT_COLOR, step(0.62, heat)), GLOW_COLOR, step(0.85, heat));

    let with_glow = mix(bands, GLOW_COLOR, step(depth, OUTLINE_WIDTH + GLOW_WIDTH));
    let color = mix(with_glow, OUTLINE_COLOR, step(depth, OUTLINE_WIDTH));

    let alpha = step(0.0, depth);

    return vec4<f32>(color * in.color.rgb, alpha * in.color.a);
}

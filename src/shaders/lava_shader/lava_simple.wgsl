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
    splashes: mat4x4<f32>, // 4 splashes, each column = (x, start_time, strength, unused)
};

@group(1) @binding(0)
var texture: texture_2d<f32>;
@group(1) @binding(1)
var texture_sampler: sampler;

@group(2) @binding(0)
var<uniform> lava: LavaUniforms;

const OUTLINE_COLOR: vec3<f32> = vec3<f32>(0.12, 0.03, 0.02);
const DEEP_COLOR: vec3<f32>    = vec3<f32>(0.55, 0.07, 0.03);
const MID_COLOR: vec3<f32>     = vec3<f32>(0.92, 0.28, 0.04);
const HOT_COLOR: vec3<f32>     = vec3<f32>(1.0, 0.6, 0.1);
const GLOW_COLOR: vec3<f32>    = vec3<f32>(1.0, 0.88, 0.42);

const PATTERN_SCALE: f32 = 0.012;
const OUTLINE_WIDTH: f32 = 5.0;
const GLOW_WIDTH: f32 = 14.0;
const HEAT_DEPTH: f32 = 180.0;

const WAVE_1: vec3<f32> = vec3<f32>(7.0, 0.011, 1.3);
const WAVE_2: vec3<f32> = vec3<f32>(3.5, 0.027, -2.1);

const SPLASH_WIDTH: f32 = 50.0;
const RIPPLE_SOFTNESS: f32 = 40.0;
const RIPPLE_SPEED: f32 = 180.0;
const RIPPLE_K: f32 = 0.05;
const RIPPLE_OMEGA: f32 = 9.0;
const RIPPLE_FALLOFF: f32 = 250.0;
const SPLASH_DECAY: f32 = 2.0;

const DROP_GRAVITY: f32 = 1100.0;
const DROP_OUTLINE: f32 = 2.5;
const DROP_CELL: f32 = 110.0;
const DROP_CHANCE: f32 = 0.35;
const DROP_PERIOD: vec2<f32> = vec2<f32>(1.5, 4.0);
const DROP_SPEED: vec2<f32> = vec2<f32>(250.0, 480.0);
const DROP_RADIUS: vec2<f32> = vec2<f32>(4.0, 7.0);

fn pcg(v: u32) -> u32
{
    let state = v * 747796405u + 2891336453u;
    let word = ((state >> ((state >> 28u) + 4u)) ^ state) * 277803737u;
    return (word >> 22u) ^ word;
}

fn hash_variant(cell: vec2<i32>, variant: u32) -> f32
{
    let h = pcg(bitcast<u32>(cell.x) + pcg(bitcast<u32>(cell.y) + pcg(variant)));
    return f32(h) * (1.0 / 4294967295.0);
}

fn splash_wave(s: vec4<f32>, x: f32, t: f32) -> f32
{
    let age = t - s.y;
    let d = abs(x - s.x);

    let envelope = exp(-age * SPLASH_DECAY) * step(0.0, age);
    let front_radius = SPLASH_WIDTH + age * RIPPLE_SPEED;
    let front = 1.0 - smoothstep(front_radius, front_radius + RIPPLE_SOFTNESS, d);
    let falloff = exp(-d / RIPPLE_FALLOFF);

    return s.z * cos(d * RIPPLE_K - age * RIPPLE_OMEGA) * front * falloff * envelope;
}

fn surface_line(x: f32, t: f32) -> f32
{
    let waves = sin(x * WAVE_1.y + t * WAVE_1.z) * WAVE_1.x + sin(x * WAVE_2.y + t * WAVE_2.z) * WAVE_2.x;
    let splashes = splash_wave(lava.splashes[0], x, t) + splash_wave(lava.splashes[1], x, t) + splash_wave(lava.splashes[2], x, t) + splash_wave(lava.splashes[3], x, t);
    return lava.surface + waves + splashes;
}

fn ambient_droplet(world: vec2<f32>, t: f32) -> vec2<f32>
{
    let cell = i32(floor(world.x / DROP_CELL));

    let period = mix(DROP_PERIOD.x, DROP_PERIOD.y, hash_variant(vec2<i32>(cell, 0), 1u));
    let phase = hash_variant(vec2<i32>(cell, 0), 2u) * period;

    let cycle_t = (t + phase) / period;
    let seed = vec2<i32>(cell, i32(floor(cycle_t)));
    let age = fract(cycle_t) * period;

    let is_active = step(hash_variant(seed, 3u), DROP_CHANCE);
    let x0 = (f32(cell) + 0.5) * DROP_CELL;
    let speed = mix(DROP_SPEED.x, DROP_SPEED.y, hash_variant(seed, 6u));
    let radius = mix(DROP_RADIUS.x, DROP_RADIUS.y, hash_variant(seed, 7u));

    let start_y = lava.surface + sin(x0 * WAVE_1.y + t * WAVE_1.z) * WAVE_1.x;
    let pos = vec2<f32>(x0, start_y - speed * age + 0.5 * DROP_GRAVITY * age * age);
    let alive = step(pos.y, start_y) * is_active;

    let d = length(world - pos) - radius;
    return vec2<f32>(step(d, 0.0), step(d, DROP_OUTLINE)) * alive;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32>
{
    let t = lava.time;
    let world = in.world_pos.xy;

    let depth = world.y - surface_line(world.x, t);

    let p = world * PATTERN_SCALE;
    let a = sin(p.x * 1.7 + sin(p.y * 1.3 + t * 0.7) * 1.8 + t * 0.4);
    let b = sin(p.y * 2.1 + sin(p.x * 1.1 - t * 0.5) * 1.6 - t * 0.3);
    let veins = a * b * 0.5 + 0.5;
    let heat = veins * 0.75 + (1.0 - smoothstep(0.0, HEAT_DEPTH, depth)) * 0.35;

    let banded = mix(mix(mix(DEEP_COLOR, MID_COLOR, step(0.35, heat)), HOT_COLOR, step(0.62, heat)), GLOW_COLOR, step(0.85, heat));
    let with_glow = mix(banded, GLOW_COLOR, step(depth, OUTLINE_WIDTH + GLOW_WIDTH));
    let body_color = mix(with_glow, OUTLINE_COLOR, step(depth, OUTLINE_WIDTH));
    let body_alpha = step(0.0, depth);

    let drops = ambient_droplet(world, t);
    let drop_color = mix(OUTLINE_COLOR, GLOW_COLOR, drops.x);

    let color = mix(body_color, drop_color, drops.y);
    let alpha = max(body_alpha, drops.y);

    return vec4<f32>(color * in.color.rgb, alpha * in.color.a);
}

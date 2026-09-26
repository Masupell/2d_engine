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
    splashes: mat4x4<f32> // 4 splashes, each collumn = (x, start_time, strength, unused)
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

const NOISE_SCALE: f32 = 1.0 / 140.0;
const FLOW_SPEED: f32 = 0.12;
const VEIN_FREQUENCY: f32 = 9.0;

// 7.5+3.5=10.5
// vec3<f32>(amplitude, frequency, speed)
const WAVE_1: vec3<f32> = vec3<f32>(7.0, 0.011, 1.3);
const WAVE_2: vec3<f32> = vec3<f32>(3.5, 0.027, -2.1);

const OUTLINE_WIDTH: f32 = 5.0;
const GLOW_WIDTH: f32 = 14.0;
const HEAT_DEPTH: f32 = 180.0;

const SPLASH_WIDTH: f32 = 50.0;
const RIPPLE_SOFTNESS: f32 = 40.0;
const RIPPLE_SPEED: f32 = 180.0;
const RIPPLE_K: f32 = 0.05;
const RIPPLE_OMEGA: f32 = 9.0;
const RIPPLE_FALLOFF: f32 = 250.0;
const SPLASH_DECAY: f32 = 2.0;

const DROP_GRAVITY: f32 = 1100.0;
const DROP_OUTLINE: f32 = 2.5;
const DROP_STRETCH: f32 = 0.0015;

const DROP_CELL: f32 = 110.0;
const DROP_CHANCE: f32 = 0.35;
const DROP_PERIOD: vec2<f32> = vec2<f32>(1.5, 4.0);
const DROP_SPEED: vec2<f32> = vec2<f32>(250.0, 480.0);
const DROP_DRIFT: f32 = 60.0;
const DROP_RADIUS: vec2<f32> = vec2<f32>(4.0, 7.0);

const SPLASH_DROP_SPREAD: f32 = 220.0;
const SPLASH_DROP_SPEED: vec2<f32> = vec2<f32>(300.0, 540.0);
const SPLASH_DROP_RADIUS: vec2<f32> = vec2<f32>(4.0, 8.0);
const SPLASH_REFERENCE: f32 = 30.0;


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

fn hash_variant(cell: vec2<i32>, variant: u32) -> f32
{
    let h = pcg(bitcast<u32>(cell.x) + pcg(bitcast<u32>(cell.y) + pcg(variant)));
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

fn droplet(world: vec2<f32>, start: vec2<f32>, velocity: vec2<f32>, age: f32, radius: f32) -> vec2<f32>
{
    let pos = vec2<f32>(start.x + velocity.x * age, start.y - velocity.y * age + 0.5 * DROP_GRAVITY * age * age);
    let alive = step(pos.y, start.y) * step(0.0, age);

    let speed_y = velocity.y - DROP_GRAVITY * age;
    let stretch = 1.0 + abs(speed_y) * DROP_STRETCH;
    let q = (world - pos) * vec2<f32>(1.0, 1.0 / stretch);
    let d = length(q) - radius;

    return vec2<f32>(step(d, 0.0), step(d, DROP_OUTLINE)) * alive;
}

fn ambient_droplet(world: vec2<f32>, cell: i32, t: f32) -> vec2<f32>
{
    let period = mix(DROP_PERIOD.x, DROP_PERIOD.y, hash_variant(vec2<i32>(cell, 0), 1u));
    let phase = hash_variant(vec2<i32>(cell, 0), 2u) * period;

    let cycle_t = (t + phase) / period;
    let seed = vec2<i32>(cell, i32(floor(cycle_t)));
    let age = fract(cycle_t) * period;

    let is_active = step(hash_variant(seed, 3u), DROP_CHANCE);
    let x0 = (f32(cell) + 0.25 + 0.5 * hash_variant(seed, 4u)) * DROP_CELL;
    let velocity = vec2<f32>((hash_variant(seed, 5u) - 0.5) * 2.0 * DROP_DRIFT, mix(DROP_SPEED.x, DROP_SPEED.y, hash_variant(seed, 6u)));
    let radius = mix(DROP_RADIUS.x, DROP_RADIUS.y, hash_variant(seed, 7u));

    return droplet(world, vec2<f32>(x0, surface_line(x0, t)), velocity, age, radius) * is_active;
}

// 4 droplets thrown at splash, bigger splash->faster droplets
fn splash_droplet(world: vec2<f32>, s: vec4<f32>, index: u32, t: f32) -> vec2<f32>
{
    let seed = vec2<i32>(bitcast<i32>(s.y), i32(index)); // unique start time
    let power = clamp(s.z / SPLASH_REFERENCE, 0.4, 1.3);

    let velocity = vec2<f32>((hash_variant(seed, 1u) - 0.5) * 2.0 * SPLASH_DROP_SPREAD, mix(SPLASH_DROP_SPEED.x, SPLASH_DROP_SPEED.y, hash_variant(seed, 2u))) * power;
    let radius = mix(SPLASH_DROP_RADIUS.x, SPLASH_DROP_RADIUS.y, hash_variant(seed, 3u));

    return droplet(world, vec2<f32>(s.x, lava.surface), velocity, t - s.y, radius);
}

fn splash_droplets(world: vec2<f32>, s: vec4<f32>, t: f32) -> vec2<f32>
{
    let a = max(splash_droplet(world, s, 0u, t), splash_droplet(world, s, 1u, t));
    let b = max(splash_droplet(world, s, 2u, t), splash_droplet(world, s, 3u, t));
    return max(a, b);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32>
{
    let t = lava.time;
    let world = in.world_pos.xy;

    // depth>0->in lava
    let depth = world.y - surface_line(world.x, t);//(lava.surface + surface_offset(world.x, t));

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
    let body_color = mix(with_glow, OUTLINE_COLOR, step(depth, OUTLINE_WIDTH));
    let body_alpha = step(0.0, depth);

    let cell = i32(floor(world.x / DROP_CELL));
    var drops = max(ambient_droplet(world, cell - 1, t), ambient_droplet(world, cell, t));
    drops = max(drops, ambient_droplet(world, cell + 1, t));

    drops = max(drops, splash_droplets(world, lava.splashes[0], t));
    drops = max(drops, splash_droplets(world, lava.splashes[1], t));
    drops = max(drops, splash_droplets(world, lava.splashes[2], t));
    drops = max(drops, splash_droplets(world, lava.splashes[3], t));

    let drop_color = mix(OUTLINE_COLOR, GLOW_COLOR, drops.x);

    let color = mix(body_color, drop_color, drops.y);
    let alpha = max(body_alpha, drops.y);

    return vec4<f32>(color * in.color.rgb, alpha * in.color.a);
}

// Wall break hazard: cracks spreading during the warning, then a jagged hole opens.
// PipeLineType::Normal, default vertex shader, no uniforms.
// Parameters come through the tint (see Hazard::draw_area):
//   r = cracks  (0..1, grows during the warning)
//   g = open    (0..1, the hole opening after the warning)
//   b = seed    (random per hazard, so every hole looks different)
//   a = seconds since the hole opened (0 before), for the falling stones
// Branchless: step / mix / clamp only.

struct VertexOutput
{
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) tex_coords: vec2<f32>,
    @location(2) mode: u32,
    @location(3) world_pos: vec3<f32>
};

@group(1) @binding(0) var texture: texture_2d<f32>;
@group(1) @binding(1) var texture_sampler: sampler;

const TAU: f32 = 6.2831853;
const PI: f32 = 3.1415926;

const OUTLINE: vec3<f32>   = vec3<f32>(0.12, 0.08, 0.06);
const HOLE_CORE: vec3<f32> = vec3<f32>(0.02, 0.015, 0.015);
const HOLE_DEEP: vec3<f32> = vec3<f32>(0.07, 0.05, 0.05);
const HOLE_WALL: vec3<f32> = vec3<f32>(0.22, 0.15, 0.11);
const RIM: vec3<f32>       = vec3<f32>(0.64, 0.57, 0.49);
const ROCK: vec3<f32>      = vec3<f32>(0.55, 0.48, 0.41);
const ROCK_DARK: vec3<f32> = vec3<f32>(0.42, 0.36, 0.30);

const QUAD_ASPECT: f32 = 1.29;    // width/height (360 / 280)

const EDGE_RADIUS: f32 = 0.68;
const EDGE_JAG_BIG: f32 = 0.16;
const EDGE_CELLS_BIG: i32 = 11;
const EDGE_JAG_SMALL: f32 = 0.07;
const EDGE_CELLS_SMALL: i32 = 27;
const RIM_WIDTH: f32 = 0.07;
const OUTLINE_WIDTH: f32 = 0.035;

const CRACK_COUNT: f32 = 7.0;
const CRACK_WIDTH: f32 = 0.04;
const CRACK_STEPS: f32 = 8.0;
const CRACK_JAG: f32 = 0.3;
const BRANCH_COUNT: f32 = 13.0;
const BRANCH_WIDTH: f32 = 0.025;
const SHAKE: f32 = 0.02;

const STONE_GRAVITY: f32 = 5.0;
const STONE_LIFE: f32 = 1.0;
const STONE_BURST: i32 = 6;
const STONE_TRICKLE: i32 = 2;

const CHUNK_COUNT: i32 = 4;
const CHUNK_TIME: f32 = 0.45;

fn pcg(v: u32) -> u32
{
    let state = v * 747796405u + 2891336453u;
    let word = ((state >> ((state >> 28u) + 4u)) ^ state) * 277803737u;
    return (word >> 22u) ^ word;
}

fn hash(i: i32, seed: f32) -> f32
{
    let h = pcg(bitcast<u32>(i) + pcg(u32(seed * 65535.0)));
    return f32(h) * (1.0 / 4294967295.0);
}

fn angular_linear(around: f32, cells: i32, seed: f32) -> f32
{
    let x = around * f32(cells);
    let i = i32(floor(x));
    let i0 = ((i % cells) + cells) % cells;
    let i1 = (i0 + 1) % cells;
    return mix(hash(i0, seed), hash(i1, seed), fract(x));
}

fn around_of(p: vec2<f32>, seed: f32) -> f32
{
    return fract(atan2(p.y, p.x) / TAU + 0.5 + seed);
}

fn hole_edge(around: f32, seed: f32) -> f32
{
    return EDGE_RADIUS + angular_linear(around, EDGE_CELLS_BIG, seed) * EDGE_JAG_BIG + angular_linear(around, EDGE_CELLS_SMALL, seed + 0.3) * EDGE_JAG_SMALL;
}

fn cracks(r: f32, around: f32, count: f32, seed: f32, width: f32, start: f32, max_length: f32) -> f32
{
    let slice_pos = around * count;
    let cell = i32(floor(slice_pos));

    let base = 0.3 + 0.4 * hash(cell, seed);
    let step_pos = r * CRACK_STEPS;
    let k = i32(floor(step_pos));
    let kink = mix(hash(cell * 97 + k, seed + 0.1), hash(cell * 97 + k + 1, seed + 0.1), fract(step_pos)) - 0.5;

    let across = abs(fract(slice_pos) - base - kink * CRACK_JAG);
    let distance = across / count * TAU * r;

    let tip = start + (max_length - start) * (0.55 + 0.45 * hash(cell, seed + 0.2));
    let taper = clamp(1.0 - (r - start) / max(tip - start, 0.0001), 0.0, 1.0);

    return step(distance, width * taper) * step(start, r) * step(r, tip);
}

fn stone(p: vec2<f32>, pos: vec2<f32>, size: f32) -> vec2<f32>
{
    let d = abs(p.x - pos.x) * QUAD_ASPECT + abs(p.y - pos.y);
    return vec2<f32>(step(d, size), step(d, size + 0.03));
}

fn falling_stone(p: vec2<f32>, t: f32, key: i32, seed: f32) -> vec2<f32>
{
    let angle = -(0.2 + 0.6 * hash(key, seed + 0.4)) * PI; // upper half (y points down)
    let direction = vec2<f32>(cos(angle), sin(angle));
    let origin = direction * hole_edge(around_of(direction, seed), seed);

    let drift = (hash(key, seed + 0.5) - 0.5) * 0.25;
    let pos = origin + vec2<f32>(drift * t, 0.5 * STONE_GRAVITY * t * t);
    let size = mix(0.035, 0.07, hash(key, seed + 0.6));

    return stone(p, pos, size) * step(0.0, t) * step(t, STONE_LIFE);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32>
{
    let crack_progress = in.color.r;
    let open = in.color.g;
    let seed = in.color.b;
    let since_open = in.color.a;

    let base_p = in.tex_coords * 2.0 - 1.0;

    let shake = smoothstep(0.6, 1.0, crack_progress) * (1.0 - step(0.001, open)) * SHAKE;
    let wobble_time = crack_progress * 40.0;
    let p = base_p + vec2<f32>(sin(wobble_time * 7.1), cos(wobble_time * 5.3)) * shake;

    let r = length(p);
    let around = around_of(p, seed);

    let edge = hole_edge(around, seed) * open;

    let inside = step(r, edge);
    let depth = r / max(edge, 0.0001); // 0 center, 1 at the edge
    let hole_color = mix(mix(HOLE_CORE, HOLE_DEEP, step(0.45, depth)), HOLE_WALL, step(0.78, depth));

    let rim = step(r, edge + RIM_WIDTH * open) - inside;
    let edge_line = step(abs(r - edge), OUTLINE_WIDTH) * step(0.001, open);
    let rim_line = step(abs(r - (edge + RIM_WIDTH * open)), OUTLINE_WIDTH * 0.7) * step(0.001, open);

    let main_cracks = cracks(r, around, CRACK_COUNT, seed, CRACK_WIDTH, 0.0, crack_progress);

    let branch_growth = clamp((crack_progress - 0.4) / 0.6, 0.0, 1.0);
    let branch_start = 0.3 + 0.3 * hash(i32(floor(around * BRANCH_COUNT)), seed + 0.7);
    let branch_cracks = cracks(r, around, BRANCH_COUNT, seed + 0.9, BRANCH_WIDTH, branch_start, branch_start + 0.35 * branch_growth) * step(0.001, branch_growth);

    let impact = step(r, 0.1 * crack_progress);
    let crack_alpha = max(max(main_cracks, branch_cracks), impact);

    var color = OUTLINE;
    var alpha = crack_alpha;

    color = mix(color, RIM, rim);
    alpha = max(alpha, rim);

    color = mix(color, hole_color, inside);
    alpha = max(alpha, inside);

    let lines = max(edge_line, rim_line);
    color = mix(color, OUTLINE, lines);
    alpha = max(alpha, lines);

    let chunk_t = since_open;
    let chunk_shrink = clamp(1.0 - chunk_t / CHUNK_TIME, 0.0, 1.0) * step(0.001, open);

    for (var c = 0; c < CHUNK_COUNT; c++)
    {
        let angle = hash(c, seed + 0.15) * TAU;
        let start = vec2<f32>(cos(angle), sin(angle)) * (0.15 + 0.25 * hash(c, seed + 0.25));
        let pos = start + start * 0.8 * chunk_t + vec2<f32>(0.0, 0.5 * STONE_GRAVITY * chunk_t * chunk_t);
        let size = mix(0.14, 0.22, hash(c, seed + 0.35)) * chunk_shrink;

        let chunk = stone(p, pos, size) * step(0.0005, size);
        let chunk_color = mix(ROCK, ROCK_DARK, step(0.5, hash(c, seed + 0.45)));
        color = mix(color, mix(OUTLINE, chunk_color, chunk.x), chunk.y);
        alpha = max(alpha, chunk.y);
    }

    var stones = vec2<f32>(0.0);

    for (var i = 0; i < STONE_BURST; i++)
    {
        let start = hash(i, seed + 0.3) * 0.6;
        stones = max(stones, falling_stone(p, since_open - start, i, seed));
    }

    for (var j = 0; j < STONE_TRICKLE; j++)
    {
        let period = 1.8 + hash(j, seed + 0.55) * 1.4;
        let later = max(since_open - 1.0, 0.0);
        let cycle = floor(later / period);
        let local_t = later - cycle * period;
        let key = 100 + j * 1000 + i32(cycle);
        let happens = step(hash(key, seed + 0.65), 0.6) * step(1.0, since_open);
        stones = max(stones, falling_stone(p, local_t, key, seed) * happens);
    }

    color = mix(color, mix(OUTLINE, ROCK, stones.x), stones.y);
    alpha = max(alpha, stones.y);

    return vec4<f32>(color, alpha);
}

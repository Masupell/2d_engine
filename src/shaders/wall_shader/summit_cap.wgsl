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
@group(2) @binding(0) var<uniform> time: f32;

const CAP_HEIGHT: f32 = 90.0;
const CAP_SURFACE: f32 = 50.0;

const END_RADIUS: f32 = 40.0;
const END_DROP: f32 = -25.0;

const OUTLINE: vec3<f32>     = vec3<f32>(0.13, 0.09, 0.06);
const GRASS_LIGHT: vec3<f32> = vec3<f32>(0.55, 0.80, 0.33);
const GRASS: vec3<f32>       = vec3<f32>(0.38, 0.66, 0.24);
const GRASS_DARK: vec3<f32>  = vec3<f32>(0.25, 0.48, 0.17);
const GRASS_BACK: vec3<f32>  = vec3<f32>(0.20, 0.40, 0.15);
const DIRT: vec3<f32>        = vec3<f32>(0.47, 0.33, 0.20);
const DIRT_DARK: vec3<f32>   = vec3<f32>(0.37, 0.25, 0.15);

const OUTLINE_WIDTH: f32 = 1.0;
const GROUND_WOBBLE: f32 = 8.0;
const GRASS_DEPTH: f32 = 10.0;
const LIP_DEPTH: f32 = 26.0;

const BLADE_FILL: f32 = 1.1; // percentage of cell for the base of the grass
const BLADE_CURVE: f32 = 0.6;
const ROOT_DEPTH: f32 = 3.0;

// Wind
const WIND_BEND: f32 = 12.0;
const WIND_SCALE: f32 = 0.004; // smaller makes bigger wind patches
const WIND_SPEED: f32 = 0.6;
const WIND_FLUTTER_SPEED: f32 = 4.0;
const WIND_FLUTTER_WAVE: f32 = 0.08;

fn pcg(v: u32) -> u32
{
    let state = v * 747796405u + 2891336453u;
    let word = ((state >> ((state >> 28u) + 4u)) ^ state) * 277803737u;
    return (word >> 22u) ^ word;
}

fn hash(i: f32) -> f32
{
    return f32(pcg(bitcast<u32>(i32(i)))) * (1.0 / 4294967295.0);
}

// 1d value noise
fn noise(x: f32) -> f32
{
    let i = floor(x);
    let f = fract(x);
    let u = f * f * (3.0 - 2.0 * f);
    return mix(hash(i), hash(i + 1.0), u);
}

fn wind_lean(x: f32, t: f32) -> f32
{
    let gust = noise(x * WIND_SCALE - t * WIND_SPEED);
    let flutter = 0.75 + 0.25 * sin(t * WIND_FLUTTER_SPEED - x * WIND_FLUTTER_WAVE);
    return WIND_BEND * (0.35 + 0.65 * gust) * flutter;
}

fn grass_layer(x: f32, above: f32, lean: f32, cell_width: f32, heights_in: vec2<f32>, height_scale: f32, grid_shift: f32, hash_offset: f32, base: vec3<f32>, tip: vec3<f32>) -> vec4<f32>
{
    let heights = heights_in * height_scale;
    let bend = lean * pow(clamp(above / heights.y, 0.0, 1.0), 2.0);
    let blade_x = (x - bend) / cell_width + grid_shift;

    let cell = floor(blade_x);
    let across = fract(blade_x);
    let height = mix(heights.x, heights.y, hash(cell + hash_offset));

    let d = abs(across - 0.5) * 2.0 / BLADE_FILL;

    let rooted = step(-ROOT_DEPTH, above);
    let inner = step(above, height * (1.0 - pow(min(d, 1.0), BLADE_CURVE))) * step(d, 1.0) * rooted;

    let outline_d = OUTLINE_WIDTH / (cell_width * BLADE_FILL * 0.5);
    let outer_d = max(d - outline_d, 0.0);
    let outer = step(above, (height + OUTLINE_WIDTH) * (1.0 - pow(min(outer_d, 1.0), BLADE_CURVE))) * step(outer_d, 1.0) * rooted;

    let shade = mix(base, tip, step(0.55, above / height));
    let color = mix(OUTLINE, shade, inner);

    return vec4<f32>(color, outer);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32>
{
    let y = in.tex_coords.y * CAP_HEIGHT;
    let x = in.world_pos.x;
    let t = time;

    let width_px = 1.0 / max(abs(dpdx(in.tex_coords.x)), 0.000001);
    let edge = min(in.tex_coords.x, 1.0 - in.tex_coords.x) * width_px;
    let e = clamp(edge / END_RADIUS, 0.0, 1.0);
    let round = sqrt(1.0 - (1.0 - e) * (1.0 - e));

    // uneven ground
    let ground = CAP_SURFACE + (noise(x * 0.02) - 0.5) * GROUND_WOBBLE + (1.0 - round) * END_DROP;
    let above = ground - y;

    // grass strip + dirt ledge
    let lip = ground + LIP_DEPTH + noise(x * 0.035 + 17.0) * round;//10.0;
    let depth = y - ground;

    let grass_depth = GRASS_DEPTH + noise(x * 0.08 + 5.0) * 5.0;
    let is_grass = step(depth, grass_depth);
    let grass_strip = mix(GRASS, GRASS_DARK, step(grass_depth - 3.0, depth));
    let dirt_color = mix(DIRT, DIRT_DARK, step(0.6, noise(x * 0.05 + depth * 0.15)));
    let soil_layered = mix(dirt_color, grass_strip, is_grass);

    let soil = step(ground, y) * step(y, lip);
    let soil_inside = step(ground + OUTLINE_WIDTH*2.5, y) * step(y, lip - OUTLINE_WIDTH*2.5) * step(OUTLINE_WIDTH*2.5, edge);
    var result = vec4<f32>(mix(OUTLINE, soil_layered, soil_inside), soil);

    // grass layers
    let lean = wind_lean(x, t);

    let back = grass_layer(x, above, lean, 16.0, vec2<f32>(14.0, 28.0), round, 0.0, 11.0, GRASS_BACK, GRASS);
    result = vec4<f32>(mix(result.rgb, back.rgb, back.a), max(result.a, back.a));

    let front = grass_layer(x, above, lean, 8.0, vec2<f32>(10.0, 23.0), round, 0.2, 47.0, GRASS, GRASS_LIGHT);
    result = vec4<f32>(mix(result.rgb, front.rgb, front.a), max(result.a, front.a));

    return vec4<f32>(result.rgb * in.color.rgb, result.a * in.color.a);
}

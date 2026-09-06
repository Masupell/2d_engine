struct VertexOutput
{
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) tex_coords: vec2<f32>,
    @location(2) mode: u32,
    @location(3) world_pos: vec2<f32>,
};

// scale: scale of the cracks
struct MountainUniforms
{
    scale: f32,
    band_height: f32,
    tilt_strength: f32,
    crack_density: f32,
};

@group(1) @binding(0)
var texture: texture_2d<f32>;
@group(1) @binding(1)
var texture_sampler: sampler;

@group(2) @binding(0)
var<uniform> mountain: MountainUniforms;

fn hash(p: vec2<f32>) -> f32
{
    var p3 = fract(vec3<f32>(p.x, p.y, p.x) * 0.1031);
    p3 = p3 + dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

fn hash2(p: vec2<f32>) -> vec2<f32>
{
    let x = hash(p);
    let y = hash(p + vec2<f32>(19.19, 7.29));
    return vec2<f32>(x, y);
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

fn fbm(p: vec2<f32>) -> f32
{
    var value = 0.0;
    var amplitude = 0.5;
    var freq = 1.0;
    var pp = p;

    for (var i = 0; i < 5; i = i + 1)
    {
        value = value + amplitude * value_noise(pp * freq);
        freq = freq * 2.0;
        amplitude = amplitude * 0.5;
    }

    return value;
}

// only 2 octaves
fn fbm2(p: vec2<f32>) -> f32
{
    var value = 0.0;
    var amplitude = 0.5;
    var freq = 1.0;
    var pp = p;

    for (var i = 0; i < 2; i = i + 1)
    {
        value = value + amplitude * value_noise(pp * freq);
        freq = freq * 2.0;
        amplitude = amplitude * 0.5;
    }

    return value;
}

fn segment_distance(p: vec2<f32>, a: vec2<f32>, b: vec2<f32>) -> f32
{
    let pa = p - a;
    let ba = b - a;
    let h = clamp(dot(pa, ba) / max(dot(ba, ba), 0.0001), 0.0, 1.0);
    return length(pa - ba * h);
}

fn jagged_crack_distance(c: vec2<f32>, seed: f32, p: vec2<f32>) -> f32
{
    var current = c + hash2(c + vec2<f32>(seed + 1.0, 0.0));
    var angle = hash(c + vec2<f32>(seed + 2.0, 0.0)) * 6.28318;

    var min_dist = 999.0;

    for (var i = 0; i < 3; i = i + 1)
    {
        let turn = (hash(c + vec2<f32>(seed + 10.0 + f32(i), 0.0)) - 0.5) * 2.4;
        angle = angle + turn;
        let len = 0.3 + hash(c + vec2<f32>(seed + 20.0 + f32(i), 0.0)) * 0.4;
        let next = current + vec2<f32>(cos(angle), sin(angle)) * len;

        min_dist = min(min_dist, segment_distance(p, current, next));
        current = next;
    }

    return min_dist;
}

// Creates cracks only in some places, with threee edges
fn crack_segments(p: vec2<f32>, density: f32, seed: f32) -> f32
{
    let cell = floor(p);
    var min_dist = 999.0;

    for (var y = -1; y <= 1; y = y + 1)
    {
        for (var x = -1; x <= 1; x = x + 1)
        {
            let c = cell + vec2<f32>(f32(x), f32(y));
            let presence = hash(c + vec2<f32>(seed, seed * 1.7));

            // If statement (not allowed)
            // But without it, it would calculate 'jagged_crack_distance' for each cell
            // which would increase cost of this function by 4-5 times (depending on crack_density)
            if (presence < density)
            {
                min_dist = min(min_dist, jagged_crack_distance(c, seed, p));
            }
        }
    }

    return min_dist;
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
    let band_height = max(mountain.band_height, 1.0);
    let crack_scale = max(mountain.scale, 0.0001);

    // tile_seed, so tile does not just tilt the same way everywhere
    let tilt_seed = fbm2(vec2<f32>(in.world_pos.y * 0.0012, 37.0));
    let tilt = (tilt_seed - 0.5) * 2.0 * mountain.tilt_strength;
    let tilted_y = in.world_pos.y + in.world_pos.x * tilt;

    let band_warp = (fbm(in.world_pos * 0.004) - 0.5) * band_height * 1.5;
    let band_coord = (tilted_y + band_warp) / band_height;
    let band_index = i32(floor(band_coord));
    let band_frac = fract(band_coord);

    let seed_a = hash(vec2<f32>(f32(band_index) * 1.7, 4.2));
    let seed_b = hash(vec2<f32>(f32(band_index + 1) * 1.7, 4.2));
    let color_a = palette_color(i32(seed_a * 100.0));
    let color_b = palette_color(i32(seed_b * 100.0));

    let banded = mix(color_a, color_b, smoothstep(0.99, 1.0, band_frac));

    // Thin dark line between bands
    let seam_distance = min(band_frac, 1.0 - band_frac);
    let outline = 1.0 - smoothstep(0.0, 0.015, seam_distance);
    let outlined = banded * mix(1.0, 0.3, outline);

    // Not sure about the look here
    let grain = value_noise(in.world_pos / crack_scale * 3.0);
    let posterized_grain = floor(grain * 3.0) / 3.0;
    let varied = outlined * mix(0.96, 1.16, posterized_grain);

    let p_big = in.world_pos / crack_scale * 0.9;
    let p_small = in.world_pos / crack_scale * 2.4 + vec2<f32>(5.2, 9.7);

    let dist_big = crack_segments(p_big, mountain.crack_density, 11.0);
    let dist_small = crack_segments(p_small, mountain.crack_density * 0.8, 47.0);

    let crack_big = 1.0 - smoothstep(0.0, 0.025, dist_big);
    let crack_small = 1.0 - smoothstep(0.0, 0.02, dist_small);
    let cracks = clamp(crack_big + crack_small * 0.6, 0.0, 1.0);

    let cracked = mix(varied, varied * 0.15, cracks);

    let edge_distance = min(in.tex_coords.x, 1.0 - in.tex_coords.x);
    let edge_fade = smoothstep(0.02, 0.1, edge_distance);
    let edge_shadowed = mix(cracked * 0.35, cracked, edge_fade);

    let final_color = edge_shadowed * in.color.rgb;

    return vec4<f32>(final_color, in.color.a);
}

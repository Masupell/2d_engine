// Shader from SimonFestugato: 'https://fragcoord.xyz/s/lqsnjc0h'
// Just for testing shader reloading
struct VertexOutput
{
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) tex_coords: vec2<f32>,
    @location(2) mode: u32,
    @location(3) world_pos: vec3<f32>,
};

@group(1) @binding(0) var texture: texture_2d<f32>;
@group(1) @binding(1) var texture_sampler: sampler;

@group(2) @binding(0) var<uniform> time: f32;

fn hash11(point: f32) -> f32
{
    var p = fract(point * 0.1031);
    p *= p + 33.33;
    p *= p + p;
    return fract(p);
}

fn hash12(point: vec2<f32>) -> f32
{
    var p3 = fract(vec3<f32>(point.x, point.y, point.x) * 0.1031);
    p3 += vec3<f32>(dot(p3, p3.yzx + vec3<f32>(33.33)));
    return fract((p3.x + p3.y) * p3.z);
}

fn rotate2d(theta: f32) -> mat2x2<f32>
{
    let c = cos(theta);
    let s = sin(theta);

    return mat2x2<f32>(
        vec2<f32>(c, -s),
        vec2<f32>(s, c)
    );
}

fn noise(p: vec2<f32>) -> f32
{
    let ip = floor(p);
    let fp = fract(p);

    let a = hash12(ip);
    let b = hash12(ip + vec2<f32>(1.0, 0.0));
    let c = hash12(ip + vec2<f32>(0.0, 1.0));
    let d = hash12(ip + vec2<f32>(1.0, 1.0));

    let t = smoothstep(vec2<f32>(0.0), vec2<f32>(1.0), fp);

    return mix(mix(a, b, t.x), mix(c, d, t.x), t.y);
}

fn fbm(point: vec2<f32>, octave_count: i32) -> f32
{
    var p = point;
    var value = 0.0;
    var amplitude = 0.5;

    for (var i = 0; i < octave_count; i += 1)
    {
        value += amplitude * noise(p);
        p = rotate2d(0.45) * p;
        p *= 2.0;
        amplitude *= 0.5;
    }

    return value;
}

const LIGHTNING_COLOR = vec3<f32>(0.515, 0.257, 1.0);

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32>
{
    var uv = in.clip_position.xy / vec2<f32>(1280.0, 720.0);
    uv = 2.0 * uv - vec2<f32>(1.0);

    let noise_value = fbm(uv + vec2<f32>(0.8 * time), 10);

    uv += vec2<f32>(2.0 * noise_value - 1.0);

    let dist = abs(uv.x);
    let strobe = hash11(time);

    let intensity = mix(0.0, 0.07, strobe) / max(dist, 0.0001);
    let col = LIGHTNING_COLOR * pow(intensity, 1.0);
    let alpha = 1.0 - exp(-intensity);

    return vec4<f32>(pow(col, vec3<f32>(1.0)), alpha);
}

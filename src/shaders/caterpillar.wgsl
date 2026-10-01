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

const OUTLINE: vec3<f32>      = vec3<f32>(0.08, 0.20, 0.06);
const GREEN: vec3<f32>        = vec3<f32>(0.3, 0.47, 0.2);
const GREEN_STRIPE: vec3<f32> = vec3<f32>(0.24, 0.44, 0.14);
const GREEN_LIGHT: vec3<f32>  = vec3<f32>(0.33, 0.6, 0.18);
const GREEN_DARK: vec3<f32>   = vec3<f32>(0.22, 0.36, 0.15);
const EYE_WHITE: vec3<f32>    = vec3<f32>(0.97, 0.97, 0.94);
const PUPIL: vec3<f32>        = vec3<f32>(0.05, 0.05, 0.07);
const CHEEK: vec3<f32>        = vec3<f32>(0.92, 0.55, 0.50);
const MOUTH: vec3<f32>        = vec3<f32>(0.18, 0.05, 0.05);

const OUTLINE_WIDTH: f32 = 0.1;

const MOUTH_Y: f32 = 0.45; // HEAD_LIFT/HEAD_RADIUS (19/42)
const EYE_Y: f32 = -0.28;
const EYE_SPACING: f32 = 0.34;
const EYE_RADIUS: f32 = 0.25;
const PUPIL_RADIUS: f32 = 0.12;
const LOOK_SHIFT: f32 = 0.01;
const PUPIL_SHIFT: f32 = 0.1;
const MOUTH_RADIUS: f32 = 0.2;

fn circle(p: vec2<f32>, center: vec2<f32>, radius: f32) -> f32
{
    return step(length(p - center), radius);
}

fn ellipse(p: vec2<f32>, center: vec2<f32>, radii: vec2<f32>) -> f32
{
    return step(length((p - center) / radii), 1.0);
}

fn body(p: vec2<f32>, stripe: f32, head: f32, look: vec2<f32>) -> vec4<f32>
{
    let r = length(p);
    let shape = step(r, 1.0);
    let inside = step(r, 1.0 - OUTLINE_WIDTH);

    let base = mix(GREEN, GREEN_STRIPE, stripe);
    let bellied = mix(base, GREEN_DARK, step(0.45, p.y + 0.15 * p.x * p.x));
    let lit = mix(bellied, GREEN_LIGHT, circle(p, vec2<f32>(-0.32, -0.42), 0.26));
    var color = mix(OUTLINE, lit, inside);

    let mouth_outline = circle(p, vec2<f32>(0.0, MOUTH_Y), MOUTH_RADIUS + 0.06);
    let mouth = circle(p, vec2<f32>(0.0, MOUTH_Y), MOUTH_RADIUS);

    let eye_offset = vec2<f32>(0.0, EYE_Y) + look * LOOK_SHIFT;
    let left_eye = vec2<f32>(-EYE_SPACING, 0.0) + eye_offset;
    let right_eye = vec2<f32>(EYE_SPACING, 0.0) + eye_offset;
    let pupil_offset = look * PUPIL_SHIFT;

    let eye_outline = max(circle(p, left_eye, EYE_RADIUS + 0.06), circle(p, right_eye, EYE_RADIUS + 0.06));
    let eye_white = max(circle(p, left_eye, EYE_RADIUS), circle(p, right_eye, EYE_RADIUS));
    let pupil = max(circle(p, left_eye + pupil_offset, PUPIL_RADIUS), circle(p, right_eye + pupil_offset, PUPIL_RADIUS));
    let shine_offset = pupil_offset + vec2<f32>(-0.04, -0.05);
    let shine = max(circle(p, left_eye + shine_offset, 0.04), circle(p, right_eye + shine_offset, 0.04));

    let cheek_y = 0.18;
    let cheeks = max(circle(p, vec2<f32>(-0.6 + look.x * 0.08, cheek_y), 0.13), circle(p, vec2<f32>(0.6 + look.x * 0.08, cheek_y), 0.13));

    color = mix(color, CHEEK, cheeks * inside * head);
    color = mix(color, OUTLINE, mouth_outline * head);
    color = mix(color, MOUTH, mouth * head);
    color = mix(color, OUTLINE, eye_outline * head);
    color = mix(color, EYE_WHITE, eye_white * head);
    color = mix(color, PUPIL, pupil * head);
    color = mix(color, EYE_WHITE, shine * head);

    return vec4<f32>(color, shape);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32>
{
    let stripe = in.color.r; // switches between 0/1
    let head = in.color.g; // body part, 1 = head, 0 = body
    let look = vec2<f32>(in.color.b, in.color.a) * 2.0 - 1.0;

    let p = in.tex_coords * 2.0 - 1.0;
    return body(p, stripe, head, look);
}

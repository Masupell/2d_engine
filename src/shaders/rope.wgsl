struct VertexOutput
{
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) tex_coords: vec2<f32>,
    @location(2) mode: u32,
};

@group(2) @binding(0) var<uniform> uniform_color: vec3<f32>; // To test uniforms

const ROPE_COLOR: vec3<f32> = vec3<f32>(0.3, 0.16, 0.04);
const TWISTS_PER_SEGMENT: f32 = 3.8;
const STRAND_CONTRAST: f32 = 0.4;

fn hash(p: vec2<f32>) -> f32
{
    return fract(sin(dot(p, vec2<f32>(12.9898,78.233))) * 43758.5453); // numbers from glsl random number thing
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32>
{
    // x goes along the rope from, y goes from left to right (0 to 1, center being 0.5)
    let u = in.tex_coords.x;
    let v = in.tex_coords.y;

    // w: shifting, so 0 is at the center and it goes from -1 to 1
    let w = clamp((v - 0.5) * 2.0, -1.0, 1.0);

    // rounded 3d highlight, using w as x-component for a cylinders surface normal
    let normal = vec3<f32>(w, 0.0, sqrt(max(0.0, 1.0 - w * w)));
    let light_dir = normalize(vec3<f32>(-0.35, 0.0, 1.0));
    let diffuse = max(dot(normal, light_dir), 0.0);
    let specular = pow(diffuse, 8.0) * 0.1;
    let shading = 0.35 + diffuse * 0.6;

    // rope twisting
    let angle = asin(w);
    let rope_noise = hash(vec2<f32>(floor(u * 8.0), floor(v * 8.0)));
    let phase = (u * TWISTS_PER_SEGMENT + angle * 0.9 + (rope_noise - 0.5) * 0.5) * 6.2831853;
    let strands = sin(phase) * 0.5 + sin(phase * 2.0 + 1.0) * 0.15;
    let strand_shading = 1.0 + strands * STRAND_CONTRAST;

    var color = ROPE_COLOR * shading * strand_shading + vec3<f32>(specular);
    color = mix(color, uniform_color, 0.3);
    let rim = smoothstep(0.55, 1.0, abs(w));
    color *= mix(1.0, 0.8, rim);

    let aa = fwidth(w) * 1.5 + 0.0001;
    let alpha = 1.0 - smoothstep(1.0 - aa, 1.0, abs(w));

    return vec4<f32>(color, alpha);
}

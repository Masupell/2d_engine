struct VertexOutput 
{
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) tex_coords: vec2<f32>,
    @location(2) mode: u32,
};

@group(0) @binding(0) var texture: texture_2d<f32>;
@group(0) @binding(1) var texture_sampler: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> 
{
    let tex_size = vec2<f32>(textureDimensions(texture));
    let uv = in.tex_coords;
    
    // Sobel-algorithm but does not have any blur
    // Sobel kernels
    let gx = array<array<i32, 3>, 3>
    (
        array(-1,  0,  1),
        array(-2,  0,  2),
        array(-1,  0,  1)
    );
    
    let gy = array<array<i32, 3>, 3>
    (
        array(-1, -2, -1),
        array( 0,  0,  0),
        array( 1,  2,  1)
    );

    var gx_sum: f32 = 0.0;
    var gy_sum: f32 = 0.0;
    
    // Surrounding pixels
    for (var y: i32 = -1; y <= 1; y = y + 1) 
    {
        for (var x: i32 = -1; x <= 1; x = x + 1) 
        {
            let offset = vec2<f32>(f32(x), f32(y)) / tex_size;
            let sample_color = textureSample(texture, texture_sampler, uv + offset);
            let brightness = dot(sample_color.rgb, vec3<f32>(0.299, 0.587, 0.114));
            
            gx_sum = gx_sum + f32(gx[y + 1][x + 1]) * brightness;
            gy_sum = gy_sum + f32(gy[y + 1][x + 1]) * brightness;
        }
    }
    
    // Gradient magnitude
    let edge_strength = gx_sum * gx_sum + gy_sum * gy_sum;
    
    // Normalize (assuming a max expected value)
    let threshold: f32 = 0.01;
    let edge = select(vec4(0.0, 0.0, 0.0, 0.0), vec4(1.0, 1.0, 1.0, 1.0), edge_strength > threshold);
    // let edge = select(vec4(0.0, 0.0, 0.0, 1.0), vec4(input_texture.rgb, 1.0), edge_strength > threshold);
    
    return edge;
}
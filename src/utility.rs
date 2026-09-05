use std::{collections::HashMap, sync::Arc};

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex
{
    pub position: [f32; 3],
    pub tex_coords: [f32; 2]
}

impl Vertex
{
    pub fn new(pos: [f32; 3], tex_pos: [f32; 2]) -> Self
    {
        Vertex
        {
            position: pos,
            tex_coords: tex_pos
        }
    }

    pub fn desc() -> wgpu::VertexBufferLayout<'static>
    {
        wgpu::VertexBufferLayout
        {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes:
            &[
                wgpu::VertexAttribute
                {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3
                },
                wgpu::VertexAttribute
                {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                }
            ]
        }
    }
}


#[derive(Copy, Clone)]
pub enum DrawType
{
    Color([f32; 4]),
    Texture(u32)
}

pub enum MaterialType
{
    Color([f32; 4]),
    Texture(std::sync::Arc<wgpu::BindGroup>, [f32; 4])
}

// #[derive(Copy, Clone)]
pub struct DrawCommand
{
    pub mesh_id: usize,
    pub transform: [[f32; 4]; 4], // 4x4 model matrix
    // pub kind: DrawType,
    pub z_index: u32,
    pub material: Arc<Material>,
    pub layer: DrawLayer,
    // default: [0, 0, 1, 1] (in uv-space, so from 0..1)
    // meshes with baked in uv, should leave this at default
    pub uv_rect: [f32; 4]
}

pub const FULL_UV_RECT: [f32; 4] = [0.0, 0.0, 1.0, 1.0];

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct InstanceData
{
    pub model: [[f32; 4]; 4],
    pub color: [f32; 4],
    pub mode: u32, //0 = color, 1 = texture
    pub uv_rect: [f32; 4]
}

impl InstanceData
{
    pub fn desc() -> wgpu::VertexBufferLayout<'static>
    {
        wgpu::VertexBufferLayout
        {
            array_stride: std::mem::size_of::<InstanceData>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes:
            &[
                wgpu::VertexAttribute
                {
                    offset: 0,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute
                {
                    offset: std::mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute
                {
                    offset: 2 * std::mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 4,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute
                {
                    offset: 3 * std::mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 5,
                    format: wgpu::VertexFormat::Float32x4,
                },
                // Color vec4
                wgpu::VertexAttribute
                {
                    offset: 4 * std::mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 6,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute //mode
                {
                    offset: 5 * std::mem::size_of::<[u32; 4]>() as wgpu::BufferAddress,
                    shader_location: 7,
                    format: wgpu::VertexFormat::Uint32
                },
                wgpu::VertexAttribute
                {
                    offset: 5 * std::mem::size_of::<[f32; 4]>() as wgpu::BufferAddress + std::mem::size_of::<u32>() as wgpu::BufferAddress,
                    shader_location: 8,
                    format: wgpu::VertexFormat::Float32x4
                }
            ],
        }
    }
}


pub struct Mesh
{
    pub vertex_buf: wgpu::Buffer,
    pub index_buf: wgpu::Buffer,
    pub vertex_capacity: usize,
    pub index_capacity: usize,
    pub index_count: u32
}

pub enum MeshID
{
    QUAD = 0
}



// CPU side
#[derive(Clone)]
pub struct MeshData
{
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u16>
}

impl MeshData
{
    pub fn new() -> Self
    {
        Self
        {
            vertices: Vec::new(),
            indices: Vec::new()
        }
    }

    pub fn add_vertex(&mut self, vertex: Vertex) -> u16
    {
        let index = self.vertices.len() as u16;
        self.vertices.push(vertex);
        index
    }

    pub fn add_triangle(&mut self, a: u16, b: u16, c: u16)
    {
        self.indices.extend_from_slice(&[a, b, c]);
    }
}


pub struct Material
{
    // pub shader: Arc<Shader>, // Will do it later
    // pub texture: Option<Arc<Texture>>
    pub pipeline_id: u8, // 0 for base, 1 for the next and so on (not sure if this is the best way, but works for now I think)
    pub kind: MaterialType
}

impl Material
{
    // pub fn new(shader: Arc<Shader>, texture: Option<Arc<Texture>>) -> Self
    // {
    //     Material
    //     {
    //         // shader,
    //         // texture

    //     }
    // }
    pub fn color(color: [f32; 4], pipeline_id: u8) -> Self
    {
        Material
        {
            kind: MaterialType::Color(color),
            pipeline_id
        }
    }

    pub fn texture(texture: Arc<wgpu::BindGroup>, tint: [f32; 4], pipeline_id: u8) -> Self
    {
        Material
        {
            kind: MaterialType::Texture(texture, tint),
            pipeline_id
        }
    }
}


#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform
{
    pub view_proj: [[f32; 4]; 4],
}


pub enum PipeLineType
{
    Normal, // takes camera layout
    PostProcess // does not
}


pub enum CoordSpace
{
    World,
    Screen, // Virtual, dont really use normal screen size for drawing, so thats fine for now
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum DrawLayer
{
    World, // Affected by post-processing step
    UI // Drawn after
}


#[derive(Clone, Copy, Debug, PartialEq)]
pub enum UniformType
{
    Float,
    Vec2,
    Vec3,
    Vec4,
    Int,
    UInt,
    Mat4
}

impl UniformType
{
    pub fn size(&self) -> usize
    {
        match self
        {
            UniformType::Float | UniformType::Int | UniformType::UInt => 4,
            UniformType::Vec2 => 8,
            UniformType::Vec3 => 12,
            UniformType::Vec4 => 16,
            UniformType::Mat4 => 64
        }
    }

    pub fn align(&self) -> usize
    {
        match self
        {
            UniformType::Float | UniformType::Int | UniformType::UInt => 4,
            UniformType::Vec2 => 8,
            UniformType::Vec3 | UniformType::Vec4 => 16,
            UniformType::Mat4 => 16
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum UniformValue
{
    Float(f32),
    Vec2([f32; 2]),
    Vec3([f32; 3]),
    Vec4([f32; 4]),
    Int(i32),
    UInt(u32),
    Mat4([[f32; 4]; 4])
}

impl UniformValue
{
    pub fn kind(&self) -> UniformType
    {
        match self
        {
            UniformValue::Float(_) => UniformType::Float,
            UniformValue::Vec2(_) => UniformType::Vec2,
            UniformValue::Vec3(_) => UniformType::Vec3,
            UniformValue::Vec4(_) => UniformType::Vec4,
            UniformValue::Int(_) => UniformType::Int,
            UniformValue::UInt(_) => UniformType::UInt,
            UniformValue::Mat4(_) => UniformType::Mat4,
        }
    }

    pub fn write_into(&self, bytes: &mut [u8])
    {
        match self
        {
            UniformValue::Float(v) => bytes.copy_from_slice(&v.to_le_bytes()),
            UniformValue::Int(v) => bytes.copy_from_slice(&v.to_le_bytes()),
            UniformValue::UInt(v) => bytes.copy_from_slice(&v.to_le_bytes()),
            UniformValue::Vec2(v) => bytes.copy_from_slice(bytemuck::cast_slice(v)),
            UniformValue::Vec3(v) => bytes.copy_from_slice(bytemuck::cast_slice(v)),
            UniformValue::Vec4(v) => bytes.copy_from_slice(bytemuck::cast_slice(v)),
            UniformValue::Mat4(v) => bytes.copy_from_slice(bytemuck::cast_slice(v)),
        }
    }
}

pub struct PipelineUniforms
{
    pub buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
    pub group_index: u32, // where the bind_group is, 2 for normal pipeline, 1 for post-process pipeline
    pub offsets: HashMap<String, (usize, UniformType)>
}

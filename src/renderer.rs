use std::{collections::HashMap, ops::Range, sync::Arc};

use wgpu::util::DeviceExt;

use crate::{MeshBuilder, MeshTopology, TargetHandle, shader::ShaderModuleHandle, target::{RenderTarget, Snapshot, create_snapshot, create_target_sampler, create_target_texture, format_is_usable, scaled_size}, text::{FontAtlas, TextCache, rasterize_font_atlas}, texture::{FilterMode, Texture, TextureEntry}, utility::{CameraUniform, CoordSpace, DrawLayer, FULL_UV_RECT, InstanceData, Mesh, MeshData, PipeLineType, PipelineUniforms, UniformType, UniformValue, Vertex}};

pub const QUAD_VERTICES: &[Vertex] =
&[
    Vertex { position: [-0.5, -0.5, 0.0], tex_coords: [0.0, 1.0] },
    Vertex { position: [ 0.5, -0.5, 0.0], tex_coords: [1.0, 1.0] },
    Vertex { position: [ 0.5,  0.5, 0.0], tex_coords: [1.0, 0.0] },
    Vertex { position: [-0.5,  0.5, 0.0], tex_coords: [0.0, 0.0] }
];

pub const QUAD_INDICES: &[u16] =
&[
    0, 1, 2,
    2, 3, 0
];

const DEFAULT_FONT_PATH: &str = "src/image/Montserrat-Bold.ttf"; // Gotta change the location later
const DEFAULT_FONT_SIZE: f32 = 128.0; // Size for now, later add multiple sizes and it chooses from it, or better a 'MSDF' (creating a signed distance field, and reconstruct it when needed)
const TEXT_CACHE_CAPACITY: usize = 64;

const IDENTITY: [[f32; 4]; 4] =
[
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0],
    [0.0, 0.0, 0.0, 1.0]
];

const WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
const WHITE_TEXTURE: usize = 0;
const MODE_COLOR: u32 = 0;
const MODE_TEXTURE: u32 = 1;

// Sort key layout (u128): [target: 16][layer: 8][z_index: 32][pipeline: 16][submission index: 32]
// Sorting by this number = sorting by layer, then z, then pipeline, then submission order.
const INDEX_BITS: u32 = 32;
const INDEX_MASK: u128 = (1 << INDEX_BITS) - 1;
const PIPELINE_SHIFT: u32 = INDEX_BITS;
const Z_SHIFT: u32 = PIPELINE_SHIFT + 16;
const LAYER_SHIFT: u32 = Z_SHIFT + 32;
const TARGET_SHIFT: u32 = LAYER_SHIFT + 8;

pub(crate) const SCREEN_TARGET: u16 = u16::MAX;

const SURFACE_SLOT: usize = 0;

const BLIT_SHADER: &str = include_str!("shaders/blit.wgsl");

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum ScreenReadMode
{
    // For pretty much everything, copy target and keep drawing into it
    Copy,
    // Currently does the same as Copy
    // Later, should do the target-swapping, for fullscreen post-process effectes
    // saves the copying step (and therefore faster)
    Swap,
}

#[derive(Copy, Clone)]
pub(crate) struct DrawCommand
{
    pub(crate) mesh_id: usize,
    pub(crate) transform: [[f32; 4]; 4], // 4x4 model matrix
    pub(crate) z_index: u32,
    pub(crate) texture_id: usize,
    pub(crate) color: [f32; 4],
    pub(crate) mode: u32, // color or texture
    pub(crate) pipeline_id: u8,
    pub(crate) layer: DrawLayer,
    // default: [0, 0, 1, 1] (in uv-space, so from 0..1)
    // meshes with baked in uv, should leave this at default
    pub(crate) uv_rect: [f32; 4],
    pub(crate) target: u16 // target id or SCREEN_TARGET
}

impl DrawCommand
{
    fn sort_key(&self, index: usize) -> u128
    {
        (self.target as u128) << TARGET_SHIFT | (self.layer as u128) << LAYER_SHIFT | (self.z_index as u128) << Z_SHIFT | (self.pipeline_id as u128) << PIPELINE_SHIFT | index as u128
    }

    // draws with the same values can be one instance
    fn batches_with(&self, other: &DrawCommand) -> bool
    {
        (self.pipeline_id == other.pipeline_id) & (self.mesh_id == other.mesh_id) & (self.texture_id == other.texture_id)
    }
}

struct PipelineSource
{
    layout: wgpu::PipelineLayout,
    vertex: ShaderModuleHandle,
    fragment: ShaderModuleHandle,
}

impl PipelineSource
{
    fn create(&self, device: &wgpu::Device, format: wgpu::TextureFormat) -> wgpu::RenderPipeline
    {
        create_render_pipeline(device, &self.layout, &self.vertex.module, &self.vertex.entry, &self.fragment.module, &self.fragment.entry, format)
    }
}

pub(crate) struct PipelineEntry
{
    source: PipelineSource,
    variants: Vec<wgpu::RenderPipeline>, // for other formats (surfaceformat, hdr, etc)
    uniforms: Option<PipelineUniforms>,
    reads_screen: bool,
    screen_read_mode: ScreenReadMode
}

impl PipelineEntry
{
    fn variant(&self, format_slot: usize) -> &wgpu::RenderPipeline
    {
        &self.variants[format_slot]
    }
}

fn create_render_pipeline(device: &wgpu::Device, layout: &wgpu::PipelineLayout, vertex_module: &wgpu::ShaderModule, vertex_entry: &str, fragment_module: &wgpu::ShaderModule, fragment_entry: &str, format: wgpu::TextureFormat) -> wgpu::RenderPipeline
{
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor
    {
        label: Some("Render Pipeline"),
        layout: Some(layout),
        vertex: wgpu::VertexState
        {
            module: vertex_module,
            entry_point: Some(vertex_entry),
            buffers: &[Vertex::desc(), InstanceData::desc()],
            compilation_options: wgpu::PipelineCompilationOptions::default()
        },
        fragment: Some(wgpu::FragmentState
        {
            module: fragment_module,
            entry_point: Some(fragment_entry),
            targets: &[Some(wgpu::ColorTargetState
            {
                format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL
            })],
            compilation_options: wgpu::PipelineCompilationOptions::default()
        }),
        primitive: wgpu::PrimitiveState
        {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None,
            polygon_mode: wgpu::PolygonMode::Fill, //::Line only work with required_features: wgpu::Features::POLYGON_MODE_LINE in request device
            unclipped_depth: false,
            conservative: false
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState
        {
            count: 1,
            mask: !0,
            alpha_to_coverage_enabled: false
        },
        multiview: None,
        cache: None
    })
}

// one render pass basically, sorted
#[derive(Copy, Clone)]
struct Segment
{
    target: u16,
    start: usize,
    end: usize,
    copy_before: bool,
    load: wgpu::LoadOp<wgpu::Color>
}

pub struct Renderer
{
    pipelines: Vec<PipelineEntry>,
    formats: Vec<wgpu::TextureFormat>,
    pub(crate) draw_commands: Vec<DrawCommand>,
    sort_keys: Vec<u128>,
    targets: Vec<RenderTarget>,
    target_sampler: wgpu::Sampler,
    current_target: u16, // where to draw to
    screen_size: (u32, u32), // basically window size, but not float and for targets
    segments: Vec<Segment>,
    screen_snapshot: Option<Snapshot>,
    screen_canvas: Option<Snapshot>, // only if the surface can't be copied from
    screen_copyable: bool,  // surface was configured with COPY_SRC
    screen_to_canvas: bool,
    blit_pipeline: wgpu::RenderPipeline,
    instance_buf: Option<wgpu::Buffer>,
    instance_capacity: usize,
    instances: Vec<InstanceData>, // reused each frame
    fullscreen_instance_buf: wgpu::Buffer, // for the screen_texture
    meshes: Vec<Mesh>, // Simple for now, later gonna change it, so it does not load all meshes ni the beginning, but only creates a mesh the first time it is requested
    pub window_size: (f32, f32),
    pub virtual_size: (f32, f32),
    textures: Vec<TextureEntry>,
    pub(crate) texture_bindgroup_layout: wgpu::BindGroupLayout,
    default_vertex: ShaderModuleHandle,
    default_fragment: ShaderModuleHandle,
    pub camera_pos: (f32, f32),
    camera_buf: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    camera_bind_group_layout: wgpu::BindGroupLayout,
    clear_color: wgpu::Color,
    pub(crate) fonts: Vec<crate::text::FontAtlas>,
    text_cache: TextCache,
    // when creating a new mesh, it can check if thee is free space here (from a previously deleted and freed mesh) and add it there, instead of allocating a new gpu buffer
    free_mesh_ids: Vec<usize>,
    uniform_values: HashMap<String, UniformValue>
}

impl Renderer
{
    pub(crate) fn new(device: &wgpu::Device, queue: &wgpu::Queue, format: wgpu::TextureFormat, screen_size: (u32, u32), screen_copyable: bool, window_size: (f32, f32)) -> Self
    {
        let texture_bindgroup_layout = Texture::bind_group_layout(&device);

        let default_vertex = ShaderModuleHandle::default_vertex(device);
        let default_fragment = ShaderModuleHandle::default_fragment(device);

        let camera_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor
        {
            label: Some("Camera Bind Group Layout"),
            entries: &[wgpu::BindGroupLayoutEntry
            {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer
                {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: Some(std::num::NonZeroU64::new(std::mem::size_of::<[[f32; 4]; 4]>() as u64).unwrap())//None,
                },
                count: None
            }],
        });

        let camera_buf = device.create_buffer(&wgpu::BufferDescriptor
        {
            label: Some("Camera Buffer"),
            size: std::mem::size_of::<[[f32; 4]; 4]>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false
        });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor
        {
            label: Some("Camera Bind Group"),
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry
            {
                binding: 0,
                resource: camera_buf.as_entire_binding()
            }]
        });

        let default_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor
        {
            label: Some("Default Pipeline Layout"),
            bind_group_layouts:
            &[
                &camera_bind_group_layout,
                &texture_bindgroup_layout
            ],
            push_constant_ranges: &[]
        });

        let default_source = PipelineSource { layout: default_layout, vertex: default_vertex.clone(), fragment: default_fragment.clone() };
        let default_pipeline = PipelineEntry
        {
            variants: vec![default_source.create(device, format)],
            source: default_source,
            uniforms: None,
            reads_screen: false,
            screen_read_mode: ScreenReadMode::Copy
        };

        let blit_module = device.create_shader_module(wgpu::ShaderModuleDescriptor
        {
            label: Some("Blit Shader"),
            source: wgpu::ShaderSource::Wgsl(BLIT_SHADER.into())
        });
        let blit_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor
        {
            label: Some("Blit Pipeline Layout"),
            bind_group_layouts: &[&texture_bindgroup_layout],
            push_constant_ranges: &[]
        });
        let blit_pipeline = create_render_pipeline(device, &blit_layout, &blit_module, "vs_main", &blit_module, "fs_main", format);

        let fullscreen_instance = InstanceData { model: IDENTITY, color: WHITE, mode: MODE_TEXTURE, uv_rect: FULL_UV_RECT };
        let fullscreen_instance_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor
        {
            label: Some("Fullscreen Instance Buffer"),
            contents: bytemuck::bytes_of(&fullscreen_instance),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let vertex_capacity = QUAD_VERTICES.len() * std::mem::size_of::<Vertex>();
        let index_capacity = QUAD_INDICES.len() * std::mem::size_of::<u16>();
        let vertex_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor
        {
            label: Some("Quad Vertex Buffer"),
            contents: bytemuck::cast_slice(QUAD_VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor
        {
            label: Some("Quad Index Buffer"),
            contents: bytemuck::cast_slice(QUAD_INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });

        let index_count = QUAD_INDICES.len() as u32;

        let quad_mesh = Mesh
        {
            vertex_buf,
            index_buf,
            vertex_capacity,
            index_capacity,
            index_count
        };

        let meshes = vec![quad_mesh];

        let default_texture = Texture::white(device, queue, FilterMode::Linear, FilterMode::Linear).unwrap();
        let default_bindgroup = Arc::new(default_texture.bind_group(device, &texture_bindgroup_layout));

        let mut textures = vec![TextureEntry { bind_group: default_bindgroup, size: (1.0, 1.0) }];

        let default_font = Self::build_font_atlas(device, queue, &texture_bindgroup_layout, &mut textures, DEFAULT_FONT_PATH, &default_charset(), DEFAULT_FONT_SIZE);

        Self
        {
            pipelines: vec![default_pipeline],
            formats: vec![format],
            draw_commands: Vec::new(),
            sort_keys: Vec::new(),
            targets: Vec::new(),
            target_sampler: create_target_sampler(device),
            current_target: SCREEN_TARGET,
            screen_size,
            segments: Vec::new(),
            screen_snapshot: None,
            screen_canvas: None,
            screen_copyable: screen_copyable,
            screen_to_canvas: false,
            blit_pipeline,
            instance_buf: None,
            instance_capacity: 0,
            instances: Vec::new(),
            fullscreen_instance_buf,
            meshes,
            window_size,
            virtual_size: window_size,
            textures,
            texture_bindgroup_layout,
            default_vertex,
            default_fragment,
            camera_pos: (0.0, 0.0),
            camera_buf,
            camera_bind_group,
            camera_bind_group_layout,
            clear_color: wgpu::Color {r: 0.0, g: 0.0, b: 0.0, a: 1.0},
            fonts: vec![default_font],
            text_cache: TextCache::new(TEXT_CACHE_CAPACITY),
            free_mesh_ids: Vec::new(),
            uniform_values: HashMap::new()
        }
    }

    fn format_slot(&mut self, device: &wgpu::Device, format: wgpu::TextureFormat) -> usize
    {
        match self.formats.iter().position(|&f| f == format)
        {
            Some(slot) => slot,
            None =>
            {
                self.formats.push(format);
                for pipeline_id in 0..self.pipelines.len()
                {
                    self.fill_variant(device, pipeline_id);
                }
                self.formats.len() - 1
            }
        }
    }

    fn fill_variant(&mut self, device: &wgpu::Device, pipeline_id: usize)
    {
        let entry = &mut self.pipelines[pipeline_id];

        for &format in &self.formats[entry.variants.len()..]
        {
            entry.variants.push(entry.source.create(&device, format));
        }
    }

    fn base_bind_group_layouts(&self, pipeline_type: &PipeLineType) -> Vec<&wgpu::BindGroupLayout>
    {
        // Normal: 0 camera, 1 texture, (2 uniforms)
        // NormalWithScreen: 0 camera, 1 texture, 2 screen copy, (3 uniforms)
        match pipeline_type
        {
            PipeLineType::Normal => vec![&self.camera_bind_group_layout, &self.texture_bindgroup_layout],
            PipeLineType::NormalWithScreen => vec![&self.camera_bind_group_layout, &self.texture_bindgroup_layout, &self.texture_bindgroup_layout]
        }
    }

    fn build_entry(&self, device: &wgpu::Device, queue: Option<&wgpu::Queue>, fragment_path: Option<&str>, vertex_path: Option<&str>, pipeline_type: PipeLineType, uniforms: Option<&[(&str, UniformType)]>) -> PipelineEntry
    {
        let vertex = match vertex_path
        {
            Some(path) => ShaderModuleHandle::from_path(device, path, "vs_main"),
            None => self.default_vertex.clone() // cheap because arc
        };

        let fragment = match fragment_path
        {
            Some(path) => ShaderModuleHandle::from_path(device, path, "fs_main"),
            None => self.default_fragment.clone()
        };

        let reads_screen = matches!(pipeline_type, PipeLineType::NormalWithScreen);

        let uniform_setup = uniforms.map(|uniforms|
        {
            let (offsets, buffer_size) = Self::compute_uniform_layout(uniforms);

            let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor
            {
                label: Some("Custom Uniform Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry
                {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer
                    {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None
                    },
                    count: None
                }],
            });

            (offsets, buffer_size, layout)
        });

        let mut bind_group_layouts = self.base_bind_group_layouts(&pipeline_type);
        let group_index = bind_group_layouts.len() as u32;

        if let Some((_, _, layout)) = &uniform_setup
        {
            bind_group_layouts.push(layout);
        }

        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor
        {
            label: Some("Pipeline Layout"),
            bind_group_layouts: &bind_group_layouts,
            push_constant_ranges: &[]
        });

        let uniforms = uniform_setup.map(|(offsets, buffer_size, layout)|
        {
            let buffer = device.create_buffer(&wgpu::BufferDescriptor
            {
                label: Some("Custom Uniform Buffer"),
                size: buffer_size as u64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false
            });

            // If any uniforms got set before this pipeline exists for some reason, otherwise buffers will be initialized with the default
            for (name, (offset, _)) in &offsets
            {
                if let (Some(value), Some(queue)) = (self.uniform_values.get(name), queue)
                {
                    Self::write_uniform(&buffer, queue, *offset, value);
                }
            }

            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor
            {
                label: Some("Custom Uniform Bind Group"),
                layout: &layout,
                entries: &[wgpu::BindGroupEntry { binding: 0, resource: buffer.as_entire_binding() }]
            });

            PipelineUniforms { buffer, bind_group, group_index, offsets }
        });

        PipelineEntry { source: PipelineSource { layout, vertex, fragment }, variants: Vec::new(), uniforms, reads_screen, screen_read_mode: ScreenReadMode::Copy }
    }

    fn push_entry(&mut self, device: &wgpu::Device, entry: PipelineEntry) -> usize
    {
        let id = self.pipelines.len();
        self.pipelines.push(entry);
        self.fill_variant(device, id);
        id
    }

    pub(crate) fn add_pipeline(&mut self, device: &wgpu::Device, fragment_path: Option<&str>, vertex_path: Option<&str>, pipeline_type: PipeLineType) -> usize
    {
        let entry = self.build_entry(device, None, fragment_path, vertex_path, pipeline_type, None);
        self.push_entry(device, entry)
    }

    // Same as 'add_pipeline', but with uniforms
    // An example usage:
    // struct CustomUniforms { time: f32, color: vec4<f32> };
    // @group(2) @binding(0) var<uniform> custom: CustomUniforms;
    // In rust:
    // renderer.add_pipeline_with_uniforms(device, queue, config, Some(path), None, PipeLineType::Normal, &[("time", UniformType::Float), ("color", UniformType::Mat4)]);
    // Has to have the same order in rust as in the shader
    // Names technically dont have to match
    // I am using a struct, instead of individual bindings, would hav to declare individual bindings otherwise (but costs more space as well, I believe, for simple things like float and int)
    pub(crate) fn add_pipeline_with_uniforms(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, fragment_path: Option<&str>, vertex_path: Option<&str>, pipeline_type: PipeLineType, uniforms: &[(&str, UniformType)]) -> usize
    {
        let entry = self.build_entry(device, Some(queue), fragment_path, vertex_path, pipeline_type, Some(uniforms));
        self.push_entry(device, entry)
    }

    pub(crate) fn replace_pipeline_with_uniforms(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, fragment_path: Option<&str>, vertex_path: Option<&str>, pipeline_type: PipeLineType, uniforms: &[(&str, UniformType)], pipeline_id: usize)
    {
        // silently skips if not exisiting for now
        if pipeline_id >= self.pipelines.len() { return; }

        self.pipelines[pipeline_id] = self.build_entry(device, Some(queue), fragment_path, vertex_path, pipeline_type, Some(uniforms));
        self.fill_variant(device, pipeline_id);
    }

    pub fn set_screen_read_mode(&mut self, pipeline_id: usize, mode: ScreenReadMode)
    {
        self.pipelines[pipeline_id].screen_read_mode = mode;
    }

    // algined by wgsls uniform rules
    fn compute_uniform_layout(uniforms: &[(&str, UniformType)]) -> (HashMap<String, (usize, UniformType)>, usize)
    {
        let mut offsets = HashMap::new();
        let mut cursor = 0_usize;

        for (name, kind) in uniforms
        {
            let align = kind.align();
            cursor = (cursor + align - 1) / align*align;
            offsets.insert(name.to_string(), (cursor, *kind));
            cursor += kind.size();
        }

        // Rounds buffer up to 16 bytes
        let total_size = (((cursor+15)/16)*16).max(16);

        (offsets, total_size)
    }

    fn write_uniform(buffer: &wgpu::Buffer, queue: &wgpu::Queue, offset: usize, value: &UniformValue)
    {
        let mut bytes = [0u8; 64]; // 64 for the biggest unifrom type (mat4)
        let size = value.kind().size();
        value.write_into(&mut bytes[..size]);
        queue.write_buffer(buffer, offset as u64, &bytes[..size]);
    }

    // sets uniform for all shaders that have this uniform
    // value has to match the one when the pipeline got created
    // Small problem, when two shaders have the same name, but different value, as that will not work right now
    pub(crate) fn set_uniform(&mut self, queue: &wgpu::Queue, name: &str, value: UniformValue)
    {
        match self.uniform_values.get_mut(name)
        {
            Some(exisiting) => *exisiting = value,
            None => { self.uniform_values.insert(name.to_string(), value); }
        }

        for pipeline_uniforms in self.pipelines.iter().filter_map(|entry| entry.uniforms.as_ref())
        {
            if let Some(&(offset, expected_kind)) = pipeline_uniforms.offsets.get(name)
            {
                debug_assert_eq!(value.kind(), expected_kind, "set_uniform(\"{name}\"): value doesn't match the value that was declared for this pipeline");
                Self::write_uniform(&pipeline_uniforms.buffer, queue, offset, &value);
            }
        }
    }

    // Marks a mesh slot available for reuse
    // DOes not free underlying gpy buffer immidiately though
    // Instead, next thing that call 'reserve_mesh_slot', gets that id back and can overwrite it
    pub fn free_mesh(&mut self, mesh_id: usize)
    {
        self.free_mesh_ids.push(mesh_id);
    }

    // If available can use MeshBuilder::with_mesh_id, otherwise just ::new
    pub fn reserve_mesh_slot(&mut self) -> Option<usize>
    {
        self.free_mesh_ids.pop()
    }

    pub(crate) fn create_mesh(&mut self, device: &wgpu::Device, data: &MeshData) -> usize
    {
        let vertex_size = data.vertices.len() * std::mem::size_of::<Vertex>();
        let index_size = data.indices.len() * std::mem::size_of::<u16>();
        let vertex_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor
        {
            label: None,
            contents: bytemuck::cast_slice(&data.vertices),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST
        });

        let index_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor
        {
            label: None,
            contents: bytemuck::cast_slice(&data.indices),
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST
        });

        let mesh = Mesh
        {
            vertex_buf,
            index_buf,
            vertex_capacity: vertex_size,
            index_capacity: index_size,
            index_count: data.indices.len() as u32
        };

        let id = self.meshes.len();
        self.meshes.push(mesh);

        id
    }

    // Not sure if there is a better way?
    pub(crate) fn update_mesh(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, mesh_id: usize, data: &MeshData)
    {
        let mesh = &mut self.meshes[mesh_id];

        let vertex_size = data.vertices.len() * std::mem::size_of::<Vertex>();
        let index_size = data.indices.len() * std::mem::size_of::<u16>();

        if vertex_size > mesh.vertex_capacity
        {
            let new_capacity = vertex_size.next_power_of_two(); // Not final
            mesh.vertex_buf = device.create_buffer(&wgpu::BufferDescriptor
            {
                label: None,
                size: new_capacity as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false
            });
            queue.write_buffer(&mesh.vertex_buf, 0, bytemuck::cast_slice(&data.vertices));
            mesh.vertex_capacity = new_capacity;
        }
        else
        {
            queue.write_buffer(&mesh.vertex_buf, 0, bytemuck::cast_slice(&data.vertices));
        }

        if index_size > mesh.index_capacity
        {
            let new_capacity = index_size.next_power_of_two();
            mesh.index_buf = device.create_buffer(&wgpu::BufferDescriptor
            {
                label: None,
                size: new_capacity as u64,
                usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false
            });
            queue.write_buffer(&mesh.index_buf, 0, bytemuck::cast_slice(&data.indices));
            mesh.index_capacity = new_capacity;
        }
        else
        {
            queue.write_buffer(&mesh.index_buf, 0, bytemuck::cast_slice(&data.indices));
        }
        mesh.index_count = data.indices.len() as u32;
    }

    // to only update position of mesh
    pub(crate) fn update_mesh_vertices(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, mesh_id: usize, vertices: &[Vertex])
    {
        let mesh = &mut self.meshes[mesh_id];

        let vertex_size = vertices.len() * std::mem::size_of::<Vertex>();

        if vertex_size > mesh.vertex_capacity
        {
            let new_capacity = vertex_size.next_power_of_two();

            mesh.vertex_buf = device.create_buffer(&wgpu::BufferDescriptor
            {
                label: None,
                size: new_capacity as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false
            });
            mesh.vertex_capacity = new_capacity;
        }
        queue.write_buffer(&mesh.vertex_buf, 0, bytemuck::cast_slice(vertices));
    }

    pub(crate) fn load_texture(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, path: &str, mag_filter: FilterMode, min_filter: FilterMode) -> usize
    {
        let error = format!("Failed to load texture with path: {}", path);
        let texture = Texture::new(device, queue, path, mag_filter, min_filter).expect(&error);
        let extent = texture.texture.size();
        let bind_group = Arc::new(texture.bind_group(device, &self.texture_bindgroup_layout));
        let id = self.textures.len();
        self.textures.push(TextureEntry { bind_group, size: (extent.width as f32, extent.height as f32) });
        id
    }


    fn build_font_atlas(device: &wgpu::Device, queue: &wgpu::Queue, texture_bindgroup_layout: &wgpu::BindGroupLayout, textures: &mut Vec<TextureEntry>, font_path: &str, charset: &str, size: f32) -> FontAtlas
    {
        let rasterized = rasterize_font_atlas(font_path, charset, size).unwrap_or_else(|e| panic!("Failed to rasterize font '{}': {:?}", font_path, e));

        // let output = image::GrayImage::from_vec(rasterized.width as u32, rasterized.height as u32, rasterized.bitmap.to_vec());
        // match output
        // {
        //     Some(image) =>
        //     {
        //         image.save(path).unwrap();
        //     }
        //     None => println!("Could not create Image")
        // }

        let texture = Texture::from_alpha_bitmap(device, queue, &rasterized.bitmap, rasterized.width, rasterized.height, FilterMode::Linear, FilterMode::Linear, Some(font_path)).expect("Failed to create font atlas texture");
        let bind_group = Arc::new(texture.bind_group(device, texture_bindgroup_layout));
        let texture_id = textures.len();
        textures.push(TextureEntry { bind_group, size: (rasterized.width as f32, rasterized.height as f32) });

        FontAtlas
        {
            texture_id,
            glyphs: rasterized.glyphs,
            line_height: rasterized.line_height,
            ascent: rasterized.ascent,
            native_size: size,
            cap_height: rasterized.cap_height
        }
    }

    // changes default font
    pub fn set_default_font(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, font_path: &str, size: f32)
    {
        self.fonts[0] = Self::build_font_atlas(device, queue, &self.texture_bindgroup_layout, &mut self.textures, font_path, &default_charset(), size);
        self.text_cache.invalidate_font(0);
    }

    pub fn add_font(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, font_path: &str, size: f32) -> usize
    {
        let atlas = Self::build_font_atlas(device, queue, &self.texture_bindgroup_layout, &mut self.textures, font_path, &default_charset(), size);
        let id = self.fonts.len();
        self.fonts.push(atlas);
        id
    }

    pub(crate) fn load_char(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, char: char) -> Option<usize>
    {
        if let Ok(text) = crate::text::rasterize_char("engine/src/image/Montserrat-Bold.ttf", char)
        {
            let texture = Texture::from_alpha_bitmap(device, queue, &text.0, text.1, text.2, FilterMode::Linear, FilterMode::Linear, Some("char")).expect("Failed to create Texture");
            let bind_group = Arc::new(texture.bind_group(device, &self.texture_bindgroup_layout));
            let id = self.textures.len();
            self.textures.push(TextureEntry { bind_group, size: (text.1 as f32, text.2 as f32) });
            Some(id)
        }
        else
        {
            None
        }
    }

    pub(crate) fn load_text(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, text: &str, size: f32) -> Option<usize>
    {
        match crate::text::rasterize_static_text("engine/src/image/Montserrat-Bold.ttf", text, size)
        {
            Ok(text) =>
            {
                // Test
                let output = image::GrayImage::from_vec(text.1 as u32, text.2 as u32, text.0.to_vec());
                match output
                {
                    Some(image) =>
                    {
                        image.save("engine/src/image/text_texture.png").unwrap();
                    }
                    None => println!("Hello")
                }
                // output.save("engine/src/image/text_texture.png").unwrap();
                //

                let texture = Texture::from_alpha_bitmap(device, queue, &text.0, text.1, text.2, FilterMode::Linear, FilterMode::Linear, Some("text")).expect("Failed to create Texture");
                let bind_group = Arc::new(texture.bind_group(device, &self.texture_bindgroup_layout));
                let id = self.textures.len();
                self.textures.push(TextureEntry { bind_group, size: (text.1 as f32, text.2 as f32) });
                Some(id)
            }
            Err(e) =>
            {
                println!("Text Rasterizing Failed: {:?}", e);
                None
            }
        }
    }

    pub fn surface_format(&self) -> wgpu::TextureFormat
    {
        self.formats[SURFACE_SLOT]
    }

    // scale relative to the window (1.0 = full, 0.5 = half resolution), format e.g. surface_format() or Rgba16Float
    pub(crate) fn create_render_target(&mut self, device: &wgpu::Device, scale: f32, format: wgpu::TextureFormat) -> TargetHandle
    {
        let id = self.targets.len();
        debug_assert!(id < SCREEN_TARGET as usize, "too many render targets");
        debug_assert!(format_is_usable(format, device.features()), "{format:?} can't be used as a render target on this device (check GraphicsContext::supports_format)");

        let size = scaled_size(self.screen_size, scale);
        let (texture, view, entry) = create_target_texture(device, &self.texture_bindgroup_layout, &self.target_sampler, format, size);

        let texture_id = self.textures.len();
        self.textures.push(entry);

        let format_slot = self.format_slot(device, format);
        self.targets.push(RenderTarget { scale, format, format_slot, texture, view, size, texture_id, clear: Some(wgpu::Color::TRANSPARENT), snapshot: None });

        TargetHandle { id, texture_id }
    }

    pub(crate) fn resize_targets(&mut self, device: &wgpu::Device, width: u32, height: u32)
    {
        self.screen_size = (width.max(1), height.max(1));

        for target in &mut self.targets
        {
            let size = scaled_size(self.screen_size, target.scale);
            let (texture, view, entry) = create_target_texture(device, &self.texture_bindgroup_layout, &self.target_sampler, target.format, size);

            target.texture = texture;
            target.view = view;
            target.size = size;
            target.snapshot = None; // wrong size, recreated once its needed
            self.textures[target.texture_id] = entry;
        }
        self.screen_snapshot = None;
        self.screen_canvas = None;
    }

    pub fn set_target(&mut self, target: TargetHandle)
    {
        self.current_target = target.id as u16;
    }

    pub fn set_screen_target(&mut self)
    {
        self.current_target = SCREEN_TARGET;
    }

    // Clear color of the target each frame, None keeps last frames contents
    pub fn set_target_clear(&mut self, target: TargetHandle, color: Option<[f64; 4]>)
    {
        self.targets[target.id].clear = color.map(|c| wgpu::Color { r: c[0], g: c[1], b: c[2], a: c[3] });
    }

    // in pixels
    pub fn target_size(&self, target: TargetHandle) -> (u32, u32)
    {
        self.targets[target.id].size
    }

    pub(crate) fn end_frame(&mut self)
    {
        self.draw_commands.clear();
        self.current_target = SCREEN_TARGET;
    }

    // Command in position in draw order
    fn command_at(&self, position: usize) -> &DrawCommand
    {
        &self.draw_commands[(self.sort_keys[position] & INDEX_MASK) as usize]
    }

    fn ensure_snapshot(&mut self, device: &wgpu::Device, target: u16)
    {
        if target == SCREEN_TARGET
        {
            let format = self.surface_format();

            if self.screen_snapshot.is_none()
            {
                self.screen_snapshot = Some(create_snapshot(device, &self.texture_bindgroup_layout, &self.target_sampler, format, self.screen_size));
            }

            // If it can't copy
            self.screen_to_canvas = !self.screen_copyable;
            if self.screen_to_canvas & self.screen_canvas.is_none()
            {
                self.screen_canvas = Some(create_snapshot(device, &self.texture_bindgroup_layout, &self.target_sampler, format, self.screen_size));
            }
        }
        else
        {
            let target = &mut self.targets[target as usize];
            if target.snapshot.is_none()
            {
                target.snapshot = Some(create_snapshot(device, &self.texture_bindgroup_layout, &self.target_sampler, target.format, target.size));
            }
        }
    }

    fn first_load(&self, target: u16) -> wgpu::LoadOp<wgpu::Color>
    {
        match target
        {
            SCREEN_TARGET => wgpu::LoadOp::Clear(self.clear_color),
            id => self.targets[id as usize].clear.map_or(wgpu::LoadOp::Load, wgpu::LoadOp::Clear)
        }
    }

    fn build_segments(&mut self, device: &wgpu::Device)
    {
        self.segments.clear();
        self.screen_to_canvas = false;

        let count = self.sort_keys.len();
        let mut position = 0;

        while position < count
        {
            let target = (self.sort_keys[position] >> TARGET_SHIFT) as u16;

            let mut load = self.first_load(target);
            let mut segment_start = position;
            let mut copy_before = false;

            let mut dirty = true;
            let mut reader_dirty = false;
            let mut copy_z = 0;

            while position < count && (self.sort_keys[position] >> TARGET_SHIFT) as u16 == target
            {
                let (reads, z) = { let cmd = self.command_at(position); (self.pipelines[cmd.pipeline_id as usize].reads_screen, cmd.z_index) };

                let needs_copy = reads & (dirty | (reader_dirty & (z != copy_z)));
                if needs_copy
                {
                    self.segments.push(Segment { target, start: segment_start, end: position, copy_before, load });

                    segment_start = position;
                    copy_before = true;
                    load = wgpu::LoadOp::Load;
                    copy_z = z;
                    dirty = false;
                    reader_dirty = false;
                }
                dirty |= !reads;
                reader_dirty |= reads;
                position += 1;
            }
            self.segments.push(Segment { target, start: segment_start, end: position, copy_before, load });
        }

        if self.segments.last().map_or(true, |segment| segment.target != SCREEN_TARGET)
        {
            self.segments.push(Segment { target: SCREEN_TARGET, start: count, end: count, copy_before: false, load: wgpu::LoadOp::Clear(self.clear_color) });
        }

        for index in 0..self.segments.len()
        {
            let segment = self.segments[index];
            if segment.copy_before
            {
                self.ensure_snapshot(device, segment.target);
            }
        }
    }


    pub(crate) fn render_frame(&self, encoder: &mut wgpu::CommandEncoder, surface_texture: &wgpu::Texture, surface_view: &wgpu::TextureView)
    {
        for segment in &self.segments
        {
            let (view, texture, size, format_slot, snapshot) = match segment.target
            {
                SCREEN_TARGET =>
                {
                    let (view, texture) = match (self.screen_to_canvas, &self.screen_canvas)
                    {
                        (true, Some(canvas)) => (&canvas.view, &canvas.texture),
                        _ => (surface_view, surface_texture)
                    };
                    (view, texture, self.screen_size, SURFACE_SLOT, self.screen_snapshot.as_ref())
                }
                id =>
                {
                    let target = &self.targets[id as usize];
                    (&target.view, &target.texture, target.size, target.format_slot, target.snapshot.as_ref())
                }
            };

            if segment.copy_before
            {
                let snapshot = snapshot.expect("shouldn't happen, snapshot in render frame");
                encoder.copy_texture_to_texture(texture.as_image_copy(), snapshot.texture.as_image_copy(), wgpu::Extent3d { width: size.0, height: size.1, depth_or_array_layers: 1 });
            }

            let snapshot_bind_group = snapshot.map(|snapshot| snapshot.entry.bind_group.as_ref());
            self.draw_range(encoder, view, format_slot, segment.load, segment.start..segment.end, snapshot_bind_group);

            if self.screen_to_canvas
            {
                if let Some(canvas) = &self.screen_canvas
                {
                    self.blit(encoder, canvas.entry.bind_group.as_ref(), surface_view);
                }
            }
        }
    }

    // copies a texture onto the view
    fn blit(&self, encoder: &mut wgpu::CommandEncoder, source: &wgpu::BindGroup, view: &wgpu::TextureView)
    {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor
        {
            label: Some("Blit Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment
            {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations
                {
                    load: wgpu::LoadOp::Clear(self.clear_color),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
        });

        let mesh = &self.meshes[0];
        render_pass.set_pipeline(&self.blit_pipeline);
        render_pass.set_vertex_buffer(0, mesh.vertex_buf.slice(..));
        render_pass.set_vertex_buffer(1, self.fullscreen_instance_buf.slice(..));
        render_pass.set_index_buffer(mesh.index_buf.slice(..), wgpu::IndexFormat::Uint16);
        render_pass.set_bind_group(0, source, &[]);
        render_pass.draw_indexed(0..mesh.index_count, 0, 0..1);
    }

    fn draw_range(&self, encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView, format_slot: usize, load: wgpu::LoadOp<wgpu::Color>, range: Range<usize>, snapshot: Option<&wgpu::BindGroup>)
    {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor
        {
            label: None,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment
            {
                view,
                resolve_target: None,
                ops: wgpu::Operations
                {
                    load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
        });

        let Some(instance_buf) = &self.instance_buf else { return; };
        if range.is_empty() { return; }

        render_pass.set_vertex_buffer(1, instance_buf.slice(..));

        render_pass.set_bind_group(0, &self.camera_bind_group, &[]);

        let mut bound_pipeline = usize::MAX;
        let mut bound_mesh = usize::MAX;
        let mut bound_texture = usize::MAX;

        let mut start = range.start;
        while start < range.end
        {
            let cmd = self.command_at(start);

            // Increase current batch, if next commands are the same
            let mut end = start + 1;
            while end < range.end && self.command_at(end).batches_with(cmd)
            {
                end += 1;
            }

            let pipeline_id = cmd.pipeline_id as usize;
            if pipeline_id != bound_pipeline
            {
                bound_pipeline = pipeline_id;
                let entry = &self.pipelines[pipeline_id];
                render_pass.set_pipeline(entry.variant(format_slot));

                if let (true, Some(snapshot)) = (entry.reads_screen, snapshot)
                {
                    render_pass.set_bind_group(2, snapshot, &[]);
                }

                if let Some(pipeline_uniform) = &entry.uniforms
                {
                    render_pass.set_bind_group(pipeline_uniform.group_index, &pipeline_uniform.bind_group, &[]);
                }
            }

            let mesh = &self.meshes[cmd.mesh_id];
            if cmd.mesh_id != bound_mesh
            {
                bound_mesh = cmd.mesh_id;
                render_pass.set_vertex_buffer(0, mesh.vertex_buf.slice(..));
                render_pass.set_index_buffer(mesh.index_buf.slice(..), wgpu::IndexFormat::Uint16);
            }

            if cmd.texture_id != bound_texture
            {
                bound_texture = cmd.texture_id;
                render_pass.set_bind_group(1, self.textures[cmd.texture_id].bind_group.as_ref(), &[]);
            }

            render_pass.draw_indexed(0..mesh.index_count, 0, start as u32..end as u32);
            start = end;
        }
    }

    fn push_command(&mut self, mesh_id: usize, transform: [[f32; 4]; 4], texture_id: usize, color: [f32; 4], mode: u32, uv_rect: [f32; 4], layer: DrawLayer, z_index: u32, pipeline_id: u8)
    {
        let target = self.current_target;
        self.draw_commands.push(DrawCommand { mesh_id, transform, z_index, texture_id, color, mode, pipeline_id, layer, uv_rect, target });
    }

    pub fn texture_size(&self, texture_id: usize) -> (f32, f32)
    {
        self.textures[texture_id].size
    }

    // no anti-aliasing right now
    pub fn draw_rect_outline(&mut self, center: (f32, f32), size: (f32, f32), thickness: f32, color: [f32; 4], rotation: f32, space: CoordSpace, layer: DrawLayer, z_index: u32, shader_id: u8)
    {
        let half = (size.0 * 0.5, size.1 * 0.5);
        let t = thickness.min(half.0).min(half.1);
        let inner_height = size.1 - 2.0 * t;

        let edges =
        [
            ((0.0, -half.1 + t * 0.5), (size.0, t)), // top
            ((0.0,  half.1 - t * 0.5), (size.0, t)), // bottom
            ((-half.0 + t * 0.5, 0.0), (t, inner_height)), // left
            (( half.0 - t * 0.5, 0.0), (t, inner_height)), // right
        ];

        let cos = rotation.cos();
        let sin = rotation.sin();

        for (offset, edge_size) in edges
        {
            let rotated = (offset.0 * cos - offset.1 * sin, offset.0 * sin + offset.1 * cos);
            let pos = (center.0 + rotated.0, center.1 + rotated.1);

            let transform = match space
            {
                CoordSpace::World => self.matrix(pos, edge_size, rotation),
                CoordSpace::Screen => self.ui_matrix(pos, edge_size, rotation),
            };

            self.push_command(0, transform, WHITE_TEXTURE, color, MODE_COLOR, FULL_UV_RECT, layer, z_index, shader_id);
        }
    }

    // basically for a post-proceses effect with a NormalWithScreen pipeline
    pub fn draw_fullscreen(&mut self, texture_id: usize, color: [f32; 4], z_index: u32, id: u8)
    {
        let transform = self.ui_matrix((self.virtual_size.0 * 0.5, self.virtual_size.1 * 0.5), self.virtual_size, 0.0);
        self.push_command(0, transform, texture_id, color, MODE_TEXTURE, FULL_UV_RECT, DrawLayer::World, z_index, id);
    }

    pub fn draw(&mut self, mesh_id: usize, transform: [[f32; 4]; 4], color: [f32; 4], z_index: u32, id: u8)
    {
        self.push_command(mesh_id, transform, WHITE_TEXTURE, color, MODE_COLOR, FULL_UV_RECT, DrawLayer::World, z_index, id);
    }

    pub fn draw_ui(&mut self, mesh_id: usize, transform: [[f32; 4]; 4], color: [f32; 4], z_index: u32, id: u8)
    {
        self.push_command(mesh_id, transform, WHITE_TEXTURE, color, MODE_COLOR, FULL_UV_RECT, DrawLayer::UI, z_index, id);
    }

    pub fn draw_texture(&mut self, mesh_id: usize, transform: [[f32; 4]; 4], texture_id: usize, z_index: u32, id: u8)
    {
        self.push_command(mesh_id, transform, texture_id, WHITE, MODE_TEXTURE, FULL_UV_RECT, DrawLayer::World, z_index, id);
    }

    pub fn draw_texture_ui(&mut self, mesh_id: usize, transform: [[f32; 4]; 4], texture_id: usize, z_index: u32, id: u8)
    {
        self.push_command(mesh_id, transform, texture_id, WHITE, MODE_TEXTURE, FULL_UV_RECT, DrawLayer::UI, z_index, id);
    }

    pub fn draw_tinted_texture(&mut self, mesh_id: usize, transform: [[f32; 4]; 4], texture_id: usize, tint: [f32; 4], z_index: u32, id: u8)
    {
        self.push_command(mesh_id, transform, texture_id, tint, MODE_TEXTURE, FULL_UV_RECT, DrawLayer::World, z_index, id);
    }

    pub fn draw_tinted_texture_ui(&mut self, mesh_id: usize, transform: [[f32; 4]; 4], texture_id: usize, tint: [f32; 4], z_index: u32, id: u8)
    {
        self.push_command(mesh_id, transform, texture_id, tint, MODE_TEXTURE, FULL_UV_RECT, DrawLayer::UI, z_index, id);
    }

    pub fn draw_texture_atlas(&mut self, mesh_id: usize, transform: [[f32; 4]; 4], texture_id: usize, rect_pos: (f32, f32), rect_size: (f32, f32), z_index: u32, shader_id: u8)
    {
        self.draw_texture_atlas_layer(mesh_id, transform, texture_id, rect_pos, rect_size, DrawLayer::World, z_index, shader_id);
    }

    pub fn draw_texture_atlas_ui(&mut self, mesh_id: usize, transform: [[f32; 4]; 4], texture_id: usize, rect_pos: (f32, f32), rect_size: (f32, f32), z_index: u32, shader_id: u8)
    {
        self.draw_texture_atlas_layer(mesh_id, transform, texture_id, rect_pos, rect_size, DrawLayer::UI, z_index, shader_id);
    }

    fn draw_texture_atlas_layer(&mut self, mesh_id: usize, transform: [[f32; 4]; 4], texture_id: usize, rect_pos: (f32, f32), rect_size: (f32, f32), layer: DrawLayer, z_index: u32, shader_id: u8)
    {
        let texture_size = self.textures[texture_id].size;

        let uv_rect =
        [
            rect_pos.0 / texture_size.0,
            rect_pos.1 / texture_size.1,
            rect_size.0 / texture_size.0,
            rect_size.1 / texture_size.1,
        ];

        self.push_command(mesh_id, transform, texture_id, WHITE, MODE_TEXTURE, uv_rect, layer, z_index, shader_id);
    }


    // draws mesh as is, so only use it for meshes created with world transform, not the quad in the beginning for example
    pub fn draw_mesh(&mut self, mesh_id: usize, texture_id: usize, z_index: u32, shader_id: u8)
    {
        self.draw_mesh_transformed(mesh_id, texture_id, IDENTITY, None, DrawLayer::World, z_index, shader_id);
    }

    // Same as normal draw_mesh, but with tint and transform, meant only for local space, not world space (text for example)
    pub fn draw_mesh_transformed(&mut self, mesh_id: usize, texture_id: usize, transform: [[f32; 4]; 4], tint: Option<[f32; 4]>, layer: DrawLayer, z_index: u32, shader_id: u8)
    {
        self.push_command(mesh_id, transform, texture_id, tint.unwrap_or(WHITE), MODE_TEXTURE, FULL_UV_RECT, layer, z_index, shader_id);
    }

    pub(crate) fn build_text_mesh(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, font_id: usize, text: &str) -> usize
    {
        if let Some(mesh_id) = self.text_cache.get(font_id, text)
        {
            return mesh_id;
        }

        let reuse_id = self.text_cache.reserve_slot();

        let mut mesh_builder = match reuse_id
        {
            Some(id) => MeshBuilder::with_mesh_id(MeshTopology::Triangles, id),
            None => MeshBuilder::new(MeshTopology::Triangles)
        };

        let line_height = self.fonts[font_id].line_height;
        let mut cursor_y = 0.0;

        for line in text.lines()
        {
            let mut cursor_x = 0.0;

            for ch in line.chars()
            {
                if let Some(glyph) = self.fonts[font_id].glyphs.get(&ch)
                {
                    if glyph.size[0] > 0.0 && glyph.size[1] > 0.0
                    {
                        let x0 = cursor_x + glyph.offset[0];
                        let x1 = x0 + glyph.size[0];
                        let y_top = cursor_y - glyph.offset[1];
                        let y_bottom = cursor_y - (glyph.offset[1] + glyph.size[1]);

                        mesh_builder.add_vertex((x0, y_top), (glyph.uv_min[0], glyph.uv_min[1]));
                        mesh_builder.add_vertex((x0, y_bottom), (glyph.uv_min[0], glyph.uv_max[1]));
                        mesh_builder.add_vertex((x1, y_bottom), (glyph.uv_max[0], glyph.uv_max[1]));

                        mesh_builder.add_vertex((x0, y_top), (glyph.uv_min[0], glyph.uv_min[1]));
                        mesh_builder.add_vertex((x1, y_bottom), (glyph.uv_max[0], glyph.uv_max[1]));
                        mesh_builder.add_vertex((x1, y_top), (glyph.uv_max[0], glyph.uv_min[1]));
                    }

                    cursor_x += glyph.advance;
                }
            }
            cursor_y -= line_height;
        }

        let mesh_id = mesh_builder.build(self, device, queue);
        self.text_cache.insert(font_id, text.to_string(), mesh_id);
        mesh_id
    }

    pub fn text_bounds(&self, text: &str, pos: (f32, f32), height_px: f32) -> ((f32, f32), f32, f32)
    {
        self.text_bounds_with_font(0, text, pos, height_px)
    }

    pub fn text_bounds_with_font(&self, font_id: usize, text: &str, pos: (f32, f32), height_px: f32) -> ((f32, f32), f32, f32)
    {
        let atlas = &self.fonts[font_id];
        let scale = height_px / atlas.native_size;

        let width = self.measure_text_width_with_font(font_id, text, height_px);

        let line_count = text.lines().count().max(1);
        let scaled_line_height = atlas.line_height * scale;
        let height = height_px + (line_count - 1) as f32 * scaled_line_height;

        (pos, width, height)
    }

    pub fn draw_text(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, text: &str, pos: (f32, f32), height_px: f32, color: [f32; 4], rotation: f32, space: CoordSpace, layer: DrawLayer, z_index: u32, shader_id: u8)
    {
        self.draw_text_with_font(device, queue, 0, text, pos, height_px, color, rotation, space, layer, z_index, shader_id);
    }

    pub fn draw_text_with_font(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, font_id: usize, text: &str, pos: (f32, f32), height_px: f32, color: [f32; 4], rotation: f32, space: CoordSpace, layer: DrawLayer, z_index: u32, shader_id: u8)
    {
        let atlas = &self.fonts[font_id];
        let scale = height_px / atlas.native_size;
        let texture_id = atlas.texture_id;
        let ascent = atlas.ascent;

        let mesh_id = self.build_text_mesh(device, queue, font_id, text);

        let baseline_pos = (pos.0, pos.1 + ascent * scale);

        // to rotate around center point
        let (_, width, height) = self.text_bounds_with_font(font_id, text, pos, height_px);
        let center = (pos.0 + width * 0.5, pos.1 + height * 0.5);
        let pivoted_pos = rotate_point_around(baseline_pos, center, rotation);

        let transform = match space
        {
            CoordSpace::World => self.matrix(pivoted_pos, (scale, scale), rotation),
            CoordSpace::Screen => self.ui_matrix(pivoted_pos, (scale, scale), rotation),
        };

        self.draw_mesh_transformed(mesh_id, texture_id, transform, Some(color), layer, z_index, shader_id);
    }

    pub fn draw_text_centered(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, text: &str, center: (f32, f32), height_px: f32, color: [f32; 4], rotation: f32, space: CoordSpace, layer: DrawLayer, z_index: u32, shader_id: u8)
    {
        self.draw_text_with_font_centered(device, queue, 0, text, center, height_px, color, rotation, space, layer, z_index, shader_id);
    }

    pub fn draw_text_with_font_centered(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, font_id: usize, text: &str, center: (f32, f32), height_px: f32, color: [f32; 4], rotation: f32, space: CoordSpace, layer: DrawLayer, z_index: u32, shader_id: u8)
    {
        let width = self.measure_text_width_with_font(font_id, text, height_px);
        let top_left = (center.0 - width * 0.5, center.1 - height_px * 0.5);

        self.draw_text_with_font(device, queue, font_id, text, top_left, height_px, color, rotation, space, layer, z_index, shader_id);
    }

    pub fn draw_text_outline(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, text: &str, pos: (f32, f32), height_px: f32, color: [f32; 4], outline_color: [f32; 4], outline_width: f32, rotation: f32, space: CoordSpace, layer: DrawLayer, z_index: u32, shader_id: u8)
    {
        self.draw_text_with_font_outline(device, queue, 0, text, pos, height_px, color, outline_color, outline_width, rotation, space, layer, z_index, shader_id);
    }

    // Only really works for a small outline width
    pub fn draw_text_with_font_outline(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, font_id: usize, text: &str, pos: (f32, f32), height_px: f32, color: [f32; 4], outline_color: [f32; 4], outline_width: f32, rotation: f32, space: CoordSpace, layer: DrawLayer, z_index: u32, shader_id: u8)
    {
        const DIRECTIONS: [(f32, f32); 8] =
        [
            (-1.0, -1.0), (0.0, -1.0), (1.0, -1.0),
            (-1.0,  0.0),              (1.0,  0.0),
            (-1.0,  1.0), (0.0,  1.0), (1.0,  1.0),
        ];

        let steps = outline_width.ceil().max(1.0) as i32;

        for step in 1..=steps
        {
            let radius = outline_width * step as f32 / steps as f32;

            for (dx, dy) in DIRECTIONS
            {
                let offset_pos = (pos.0 + dx * radius, pos.1 + dy * radius);
                self.draw_text_with_font(device, queue, font_id, text, offset_pos, height_px, outline_color, rotation, space, layer, z_index, shader_id);
            }
        }

        self.draw_text_with_font(device, queue, font_id, text, pos, height_px, color, rotation, space, layer, z_index, shader_id);
    }

    pub fn draw_text_centered_outline(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, text: &str, center: (f32, f32), height_px: f32, color: [f32; 4], outline_color: [f32; 4], outline_width: f32, rotation: f32, space: CoordSpace, layer: DrawLayer, z_index: u32, shader_id: u8)
    {
        self.draw_text_with_font_centered_outline(device, queue, 0, text, center, height_px, color, outline_color, outline_width, rotation, space, layer, z_index, shader_id);
    }

    pub fn draw_text_with_font_centered_outline(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, font_id: usize, text: &str, center: (f32, f32), height_px: f32, color: [f32; 4], outline_color: [f32; 4], outline_width: f32, rotation: f32, space: CoordSpace, layer: DrawLayer, z_index: u32, shader_id: u8)
    {
        let width = self.measure_text_width_with_font(font_id, text, height_px);
        let top_left = (center.0 - width * 0.5, center.1 - height_px * 0.5);

        self.draw_text_with_font_outline(device, queue, font_id, text, top_left, height_px, color, outline_color, outline_width, rotation, space, layer, z_index, shader_id);
    }

    pub fn measure_text_width(&self, text: &str, height_px: f32) -> f32
    {
        self.measure_text_width_with_font(0, text, height_px)
    }

    pub fn measure_text_width_with_font(&self, font_id: usize, text: &str, height_px: f32) -> f32
    {
        let atlas = &self.fonts[font_id];
        let scale = height_px / atlas.native_size;

        let width = text.lines().map(|line| line.chars().filter_map(|ch| atlas.glyphs.get(&ch)).map(|glyph| glyph.advance).sum::<f32>()).fold(0.0_f32, f32::max);
        width * scale
    }

    pub(crate) fn prepare_frame(&mut self, device: &wgpu::Device, queue: &wgpu::Queue)
    {
        self.sort_keys.clear();

        if self.draw_commands.is_empty()
        {
            self.build_segments(device);
            return;
        }
        debug_assert!(self.draw_commands.len() <= INDEX_MASK as usize, "more than {INDEX_MASK} draw commands in one frame");

        self.sort_keys.extend(self.draw_commands.iter().enumerate().map(|(index, cmd)| cmd.sort_key(index)));
        self.sort_keys.sort_unstable();

        self.instances.clear();
        self.instances.extend(self.sort_keys.iter().map(|&key|
        {
            let cmd = &self.draw_commands[(key & INDEX_MASK) as usize];
            InstanceData { model: cmd.transform, color: cmd.color, mode: cmd.mode, uv_rect: cmd.uv_rect }
        }));

        let instance_size = self.instances.len() * std::mem::size_of::<InstanceData>();

        if instance_size > self.instance_capacity
        {
            let new_capacity = instance_size.next_power_of_two(); // same as in mesh creation temporary

            self.instance_buf = Some(device.create_buffer(&wgpu::BufferDescriptor
            {
                label: Some("Instance Buffer"),
                size: new_capacity as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false
            }));
            self.instance_capacity = new_capacity;
        }

        queue.write_buffer(self.instance_buf.as_ref().unwrap(), 0, bytemuck::cast_slice(&self.instances));
        self.build_segments(device);
    }

    pub(crate) fn set_clear_color(&mut self, color: [f64; 4])
    {
        let clear = wgpu::Color { r: color[0], g: color[1], b: color[2], a: color[3] };
        self.clear_color = clear;
    }

    pub fn set_camera_pos(&mut self, position: (f32, f32), queue: &wgpu::Queue)
    {
        self.camera_pos = position;
        self.update_camera(queue);
    }

    fn update_camera(&self, queue: &wgpu::Queue)
    {
        let camera = CameraUniform
        {
            view_proj: self.camera_matrix()
        };

        queue.write_buffer(&self.camera_buf, 0, bytemuck::bytes_of(&camera));
    }

    // uses virtual size
    fn camera_matrix(&self) -> [[f32; 4]; 4]
    {
        let width = self.virtual_size.0;
        let height = self.virtual_size.1;

        let left = self.camera_pos.0 - width * 0.5;
        let right = self.camera_pos.0 + width * 0.5;

        let top = self.camera_pos.1 - height * 0.5;
        let bottom = self.camera_pos.1 + height * 0.5;

        [
            [2.0 / (right - left), 0.0, 0.0, 0.0],
            [0.0, -2.0 / (bottom - top), 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [-(right + left) / (right - left), (bottom + top) / (bottom - top), 0.0, 1.0]
        ]
    }


    pub fn pixel_matrix(&self, pos: (f32, f32), size: (f32, f32), rotation: f32) -> [[f32; 4]; 4]
    {
        let to_virtual = (self.virtual_size.0 / self.window_size.0, self.virtual_size.1 / self.window_size.1);

        let virtual_pos = (pos.0 * to_virtual.0, pos.1 * to_virtual.1);
        let virtual_size = (size.0 * to_virtual.0, size.1 * to_virtual.1);

        self.ui_matrix(virtual_pos, virtual_size, rotation)
    }

    pub fn ui_matrix(&self, pos: (f32, f32), size: (f32, f32), rotation: f32) -> [[f32; 4]; 4]
    {
        let world_pos =
        (
            self.camera_pos.0 + pos.0 - self.virtual_size.0 * 0.5,
            self.camera_pos.1 + pos.1 - self.virtual_size.1 * 0.5 // +, - => (0,0) is top-left, -,+ => (0,0) is bottom-left
        );

        self.matrix(world_pos, size, rotation)
    }

    // Still draws with pixels, but this time everything gets drawn like it looks with the original screen-size, so resized looks the same (in relation to each other)
    // If using this, when trying to use the windowsize, use virtual_size instead of window_size
    // Because everything here is in relation to the original "virtual" size, not the actual window size
    pub fn matrix(&self, pos: (f32, f32), size: (f32, f32), rotation: f32) -> [[f32; 4]; 4]
    {
        let cos = rotation.cos();
        let sin = rotation.sin();

        [
            [cos*size.0, sin*size.0, 0.0, 0.0],
            [sin*size.1, -cos*size.1, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [pos.0, pos.1, 0.0, 1.0]
        ]
    }

    // Size in relative to the original, not pixels
    pub fn texture_matrix(&self, pos: (f32, f32), scale: (f32, f32), rotation: f32, texture_size: (f32, f32)) -> [[f32; 4]; 4]
    {
        self.matrix(pos, (texture_size.0 * scale.0, texture_size.1 * scale.1), rotation)
    }
}

fn default_charset() -> String
{
    (' '..='~').collect()
}

// gives pivot point to rotate around (for matrix, which uses center, and text which used top-left)
fn rotate_point_around(point: (f32, f32), pivot: (f32, f32), rotation: f32) -> (f32, f32)
{
    let cos = rotation.cos();
    let sin = rotation.sin();
    let dx = point.0 - pivot.0;
    let dy = point.1 - pivot.1;

    (pivot.0 + dx * cos - dy * sin, pivot.1 + dx * sin + dy * cos)
}

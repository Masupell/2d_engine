use std::sync::Arc;

use wgpu::util::DeviceExt;

use crate::{MeshBuilder, MeshTopology, shader::ShaderModuleHandle, text::{FontAtlas, TextCache, rasterize_font_atlas}, texture::{FilterMode, Texture, TextureEntry}, utility::{CameraUniform, CoordSpace, DrawCommand, DrawLayer, FULL_UV_RECT, InstanceData, Material, MaterialType, Mesh, MeshData, PipeLineType, Vertex}};



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


pub struct Renderer
{
    // pub pipeline: wgpu::RenderPipeline,
    pub(crate) pipelines: Vec<wgpu::RenderPipeline>,
    pub(crate) draw_commands: Vec<DrawCommand>,
    instance_buf: Option<wgpu::Buffer>,
    instance_capacity: usize,
    meshes: Vec<Mesh>, // Simple for now, later gonna change it, so it does not load all meshes ni the beginning, but only creates a mesh the first time it is requested
    pub window_size: (f32, f32),
    pub virtual_size: (f32, f32),
    textures: Vec<TextureEntry>,
    pub(crate) texture_bindgroup_layout: wgpu::BindGroupLayout,
    default_vertex: ShaderModuleHandle,
    default_fragment: ShaderModuleHandle,
    // diffuse_bind_group: wgpu::BindGroup,
    // texture_bind_groups: Vec<wgpu::BindGroup>
    pub camera_pos: (f32, f32),
    camera_buf: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    camera_bind_group_layout: wgpu::BindGroupLayout,
    clear_color: wgpu::Color,
    pub(crate) fonts: Vec<crate::text::FontAtlas>,
    text_cache: TextCache,
    // when creating a new mesh, it can check if thee is free space here (from a previously deleted and freed mesh) and add it there, instead of allocating a new gpu buffer
    free_mesh_ids: Vec<usize>
}

impl Renderer
{
    pub(crate) fn new(device: &wgpu::Device, config: &wgpu::SurfaceConfiguration, queue: &wgpu::Queue, window_size: (f32, f32)) -> Self
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


        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor
        {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts:
            &[
                &camera_bind_group_layout,
                &texture_bindgroup_layout
            ],
            push_constant_ranges: &[]
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor
        {
            label: Some("Render Pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState
            {
                module: &default_vertex.module,
                entry_point: Some(&default_vertex.entry),
                buffers: &[Vertex::desc(), InstanceData::desc()],
                compilation_options: wgpu::PipelineCompilationOptions::default()
            },
            fragment: Some(wgpu::FragmentState
            {
                module: &default_fragment.module,
                entry_point: Some(&default_fragment.entry),
                targets: &[Some(wgpu::ColorTargetState
                {
                    format: config.format,
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
                cull_mode: None,//Some(wgpu::Face::Back),
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
            pipelines: vec![pipeline],
            draw_commands: Vec::new(),
            instance_buf: None,
            instance_capacity: 0,
            meshes,
            window_size,
            virtual_size: window_size,
            textures,
            texture_bindgroup_layout,
            default_vertex,
            default_fragment,
            // diffuse_bind_group
            // texture_bind_groups
            camera_pos: (0.0, 0.0),
            camera_buf,
            camera_bind_group,
            camera_bind_group_layout,
            clear_color: wgpu::Color {r: 0.0, g: 0.0, b: 0.0, a: 1.0},
            fonts: vec![default_font],
            text_cache: TextCache::new(TEXT_CACHE_CAPACITY),
            free_mesh_ids: Vec::new()
        }
    }

    pub(crate) fn add_pipeline(&mut self, device: &wgpu::Device, config: &wgpu::SurfaceConfiguration, fragment_path: Option<&str>, vertex_path: Option<&str>, pipeline_type: PipeLineType) -> usize
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

        let bind_group_layouts = match pipeline_type
        {
            PipeLineType::Normal => vec![&self.camera_bind_group_layout, &self.texture_bindgroup_layout],
            PipeLineType::PostProcess => vec![&self.texture_bindgroup_layout]
        };

        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor
        {
            label: Some("Pipeline Layout"),
            bind_group_layouts: &bind_group_layouts,
            push_constant_ranges: &[]
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor
        {
            label: Some("Render Pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState
            {
                module: &vertex.module,
                entry_point: Some(&vertex.entry),
                buffers: &[Vertex::desc(), InstanceData::desc()],
                compilation_options: wgpu::PipelineCompilationOptions::default()
            },
            fragment: Some(wgpu::FragmentState
            {
                module: &fragment.module,
                entry_point: Some(&fragment.entry),
                targets: &[Some(wgpu::ColorTargetState
                {
                    format: config.format,
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
                cull_mode: None,//Some(wgpu::Face::Back),
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
        });
        let id = self.pipelines.len();
        self.pipelines.push(pipeline);
        id
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

    pub(crate) fn begin_pass(&self, encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView, layer: DrawLayer)
    {
        let load_op = match layer
        {
            DrawLayer::World => wgpu::LoadOp::Clear(self.clear_color),
            DrawLayer::UI => wgpu::LoadOp::Load
        };

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor
        {
            label: Some("Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment
            {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations
                {
                    load: load_op,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
        });

        if let Some(ref instance_buf) = self.instance_buf
        {
            render_pass.set_vertex_buffer(1, instance_buf.slice(..));
            let mut current_pipeline: Option<u8> = None;

            for (instance_id, cmd) in self.draw_commands.iter().enumerate()
            {
                if cmd.layer != layer { continue; }

                if Some(cmd.material.pipeline_id) != current_pipeline
                {
                    current_pipeline = Some(cmd.material.pipeline_id);
                    render_pass.set_pipeline(&self.pipelines[cmd.material.pipeline_id as usize]);
                }

                let mesh = &self.meshes[cmd.mesh_id];

                render_pass.set_vertex_buffer(0, mesh.vertex_buf.slice(..));
                render_pass.set_index_buffer(mesh.index_buf.slice(..), wgpu::IndexFormat::Uint16);


                render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
                // render_pass.set_bind_group(0, &self.diffuse_bind_group, &[]);
                match &cmd.material.kind
                {
                    MaterialType::Color(_) =>
                    {
                        render_pass.set_bind_group(1, self.textures[0].bind_group.as_ref(), &[]);
                    }
                    MaterialType::Texture(texture, _) =>
                    {
                        render_pass.set_bind_group(1, texture.as_ref(), &[]);
                    }
                }


                render_pass.draw_indexed(0..mesh.index_count, 0, instance_id as u32..instance_id as u32 + 1);
            }
        }
    }

    pub(crate) fn screen_texture(&self, encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView, pipeline_id: usize, texture: &wgpu::BindGroup)
    {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor
        {
            label: Some("Single Texture Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment
            {
                view,
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

        render_pass.set_pipeline(&self.pipelines[pipeline_id]); // Post Processing Shader, then just draws full-screen texture with it

        let mesh = &self.meshes[0]; // Just a quad
        render_pass.set_vertex_buffer(1, self.instance_buf.as_ref().unwrap().slice(..));
        render_pass.set_vertex_buffer(0, mesh.vertex_buf.slice(..));
        render_pass.set_index_buffer(mesh.index_buf.slice(..), wgpu::IndexFormat::Uint16);

        render_pass.set_bind_group(0, texture, &[]);

        render_pass.draw_indexed(0..mesh.index_count, 0, 0..1);
    }

    pub fn draw(&mut self, mesh_id: usize, transform: [[f32; 4]; 4], color: [f32; 4], z_index: u32, id: u8)
    {
        self.draw_commands.push(DrawCommand { mesh_id, transform, z_index, material: Arc::new(Material::color(color, id)), layer: DrawLayer::World, uv_rect: FULL_UV_RECT });
    }

    pub fn draw_ui(&mut self, mesh_id: usize, transform: [[f32; 4]; 4], color: [f32; 4], z_index: u32, id: u8)
    {
        self.draw_commands.push(DrawCommand { mesh_id, transform, z_index, material: Arc::new(Material::color(color, id)), layer: DrawLayer::UI, uv_rect: FULL_UV_RECT });
    }

    pub fn draw_texture(&mut self, mesh_id: usize, transform: [[f32; 4]; 4], texture_id: usize, z_index: u32, id: u8)
    {
        let texture = Arc::clone(&self.textures[texture_id].bind_group);
        self.draw_commands.push(DrawCommand { mesh_id, transform, z_index, material: Arc::new(Material::texture(texture, [1.0, 1.0, 1.0, 1.0], id)), layer: DrawLayer::World, uv_rect: FULL_UV_RECT });
    }

    pub fn draw_texture_ui(&mut self, mesh_id: usize, transform: [[f32; 4]; 4], texture_id: usize, z_index: u32, id: u8)
    {
        let texture = Arc::clone(&self.textures[texture_id].bind_group);
        self.draw_commands.push(DrawCommand { mesh_id, transform, z_index, material: Arc::new(Material::texture(texture, [1.0, 1.0, 1.0, 1.0], id)), layer: DrawLayer::UI, uv_rect: FULL_UV_RECT });
    }

    pub fn draw_tinted_texture(&mut self, mesh_id: usize, transform: [[f32; 4]; 4], texture_id: usize, tint: [f32; 4], z_index: u32, id: u8)
    {
        let texture = Arc::clone(&self.textures[texture_id].bind_group);
        self.draw_commands.push(DrawCommand { mesh_id, transform, z_index, material: Arc::new(Material::texture(texture, tint, id)), layer: DrawLayer::World, uv_rect: FULL_UV_RECT });
    }

    pub fn draw_tinted_texture_ui(&mut self, mesh_id: usize, transform: [[f32; 4]; 4], texture_id: usize, tint: [f32; 4], z_index: u32, id: u8)
    {
        let texture = Arc::clone(&self.textures[texture_id].bind_group);
        self.draw_commands.push(DrawCommand { mesh_id, transform, z_index, material: Arc::new(Material::texture(texture, tint, id)), layer: DrawLayer::UI, uv_rect: FULL_UV_RECT });
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
        let entry = &self.textures[texture_id];
        let texture_size = entry.size;
        let texture = Arc::clone(&entry.bind_group);

        let uv_rect =
        [
            rect_pos.0 / texture_size.0,
            rect_pos.1 / texture_size.1,
            rect_size.0 / texture_size.0,
            rect_size.1 / texture_size.1,
        ];

        self.draw_commands.push(DrawCommand { mesh_id, transform, z_index, material: Arc::new(Material::texture(texture, [1.0, 1.0, 1.0, 1.0], shader_id)), layer, uv_rect });
    }


    // draws mesh as is, so only use it for meshes created with world transform, not the quad in the beginning for example
    pub fn draw_mesh(&mut self, mesh_id: usize, texture_id: usize, z_index: u32, shader_id: u8)
    {
        const IDENTITY: [[f32; 4]; 4] =
        [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0]
        ];

        self.draw_mesh_transformed(mesh_id, texture_id, IDENTITY, None, DrawLayer::World, z_index, shader_id);
    }

    // Same as normal draw_mesh, but with tint and transform, meant only for local space, not world space (text for example)
    pub fn draw_mesh_transformed(&mut self, mesh_id: usize, texture_id: usize, transform: [[f32; 4]; 4], tint: Option<[f32; 4]>, layer: DrawLayer, z_index: u32, shader_id: u8)
    {
        let texture = Arc::clone(&self.textures[texture_id].bind_group);
        let material = match tint
        {
            Some(tint) => Material::texture(texture, tint, shader_id),
            None => Material::texture(texture, [1.0, 1.0, 1.0, 1.0], shader_id),
        };

        self.draw_commands.push(DrawCommand { mesh_id, transform, z_index, material: Arc::new(material), layer, uv_rect: FULL_UV_RECT });
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

    pub fn draw_text(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, text: &str, pos: (f32, f32), height_px: f32, color: [f32; 4], space: CoordSpace, layer: DrawLayer, z_index: u32, shader_id: u8)
    {
        self.draw_text_with_font(device, queue, 0, text, pos, height_px, color, space, layer, z_index, shader_id);
    }

    pub fn draw_text_with_font(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, font_id: usize, text: &str, pos: (f32, f32), height_px: f32, color: [f32; 4], space: CoordSpace, layer: DrawLayer, z_index: u32, shader_id: u8)
    {
        let atlas = &self.fonts[font_id];
        let scale = height_px / atlas.native_size;
        let texture_id = atlas.texture_id;
        let ascent = atlas.ascent;

        let mesh_id = self.build_text_mesh(device, queue, font_id, text);

        let baseline_pos = (pos.0, pos.1 + ascent * scale);

        let transform = match space
        {
            CoordSpace::World => self.matrix(baseline_pos, (scale, scale), 0.0),
            CoordSpace::Screen => self.ui_matrix(baseline_pos, (scale, scale), 0.0),
        };

        self.draw_mesh_transformed(mesh_id, texture_id, transform, Some(color), layer, z_index, shader_id);
    }

    pub fn draw_text_centered(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, text: &str, center: (f32, f32), height_px: f32, color: [f32; 4], space: CoordSpace, layer: DrawLayer, z_index: u32, shader_id: u8)
    {
        self.draw_text_with_font_centered(device, queue, 0, text, center, height_px, color, space, layer, z_index, shader_id);
    }

    pub fn draw_text_with_font_centered(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, font_id: usize, text: &str, center: (f32, f32), height_px: f32, color: [f32; 4], space: CoordSpace, layer: DrawLayer, z_index: u32, shader_id: u8)
    {
        let width = self.measure_text_width_with_font(font_id, text, height_px);
        let top_left = (center.0 - width * 0.5, center.1 - height_px * 0.5);

        self.draw_text_with_font(device, queue, font_id, text, top_left, height_px, color, space, layer, z_index, shader_id);
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

    pub(crate) fn upload_instances(&mut self, device: &wgpu::Device, queue: &wgpu::Queue)
    {
        if self.draw_commands.is_empty()
        {
            // self.instance_buf = None;
            // self.instance_capacity = 0;
            return;
        }

        // self.draw_commands.sort_by_key(|cmd| cmd.z_index);
        self.draw_commands.sort_by_key(|cmd| (cmd.z_index, cmd.material.pipeline_id)); // Index is most important, inside same layer it still sorts pipeline though


        let instances: Vec<InstanceData> = self.draw_commands.iter().map(|cmd|
        {
            let material = &cmd.material;
            match material.kind
            {
                MaterialType::Color(color) => InstanceData
                {
                    model: cmd.transform,
                    color: color,
                    mode: 0,
                    uv_rect: cmd.uv_rect
                },
                MaterialType::Texture(_, tint) => InstanceData
                {
                    model: cmd.transform,
                    color: tint,
                    mode: 1,
                    uv_rect: cmd.uv_rect
                },

            }
        }).collect();

        let instance_size = instances.len() * std::mem::size_of::<InstanceData>();

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

        queue.write_buffer(self.instance_buf.as_ref().unwrap(), 0, bytemuck::cast_slice(&instances));
        // if let Some(ref buf) = self.instance_buf // If it already exists, dont create it again
        // {
        //     queue.write_buffer(buf, 0, bytemuck::cast_slice(&instances));
        // }
        // else
        // {
        //     self.instance_buf = Some(device.create_buffer_init(&wgpu::util::BufferInitDescriptor
        //     {
        //         label: Some("Instance Buffer"),
        //         contents: bytemuck::cast_slice(&instances),
        //         usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST
        //     }));
        // }
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

    // pos in pixels, size as in 1.0 is default scale, rotation in radians (all for 2D, would work for 3D, but this is 2D)
    // pub fn to_matrix(&self, pos: (f32, f32), size: (f32, f32), rotation: f32) -> [[f32; 4]; 4]
    // {
    //     let aspect = self.window_size.0/self.window_size.1;
    //     let scale = 1./aspect;

    //     let cos = rotation.cos();
    //     let sin = rotation.sin();

    //     [
    //         [scale*cos*size.0, sin*size.0, 0.0, 0.0],
    //         [scale*-sin*size.1, cos*size.1, 0.0, 0.0],
    //         [0.0, 0.0, 1.0, 0.0],
    //         [(pos.0/self.window_size.0)*2.0-1.0, -((pos.1/self.window_size.1)*2.0-1.0), 0.0, 1.0]
    //     ]
    // }

    // Size in pixels now too
    // Always stays the same size, even if screen gets resized (so always 100px big for example), so not relative says but static
    // pub fn pixel_matrix(&self, pos: (f32, f32), size: (f32, f32), rotation: f32) -> [[f32; 4]; 4]
    // {
    //     let aspect = self.window_size.0/self.window_size.1;
    //     let scale = 1./aspect;

    //     let cos = rotation.cos();
    //     let sin = rotation.sin();

    //     let pixel_size = ((size.0/self.window_size.1)*2.0, (size.1/self.window_size.1)*2.0);

    //     [
    //         [scale*cos*pixel_size.0, sin*pixel_size.0, 0.0, 0.0],
    //         [scale*-sin*pixel_size.1, cos*pixel_size.1, 0.0, 0.0],
    //         [0.0, 0.0, 1.0, 0.0],
    //         [(pos.0/self.window_size.0)*2.0-1.0, -((pos.1/self.window_size.1)*2.0-1.0), 0.0, 1.0]
    //     ]
    // }
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
    // pub fn matrix(&self, pos: (f32, f32), size: (f32, f32), rotation: f32) -> [[f32; 4]; 4]
    // {
    //     let aspect = self.window_size.0/self.window_size.1;
    //     let scale = 1./aspect;

    //     let cos = rotation.cos();
    //     let sin = rotation.sin();

    //     let scale_x = (size.0/self.virtual_size.1)*2.0;
    //     let scale_y = (size.1/self.virtual_size.1)*2.0;

    //     [
    //         [scale*cos*scale_x, sin*scale_x, 0.0, 0.0],
    //         [scale*-sin*scale_y, cos*scale_y, 0.0, 0.0],
    //         [0.0, 0.0, 1.0, 0.0],
    //         [(pos.0/self.virtual_size.0)*2.0-1.0, -((pos.1/self.virtual_size.1)*2.0-1.0), 0.0, 1.0]
    //     ]
    // }
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

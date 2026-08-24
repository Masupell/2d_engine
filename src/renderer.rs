use std::sync::Arc;

use wgpu::util::DeviceExt;

use crate::{shader::ShaderModuleHandle, texture::{FilterMode, Texture}, utility::{CameraUniform, DrawCommand, InstanceData, Material, MaterialType, Mesh, MeshData, PipeLineType, Vertex}};



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
    textures: Vec<Arc<wgpu::BindGroup>>,
    pub(crate) texture_bindgroup_layout: wgpu::BindGroupLayout,
    default_vertex: ShaderModuleHandle,
    default_fragment: ShaderModuleHandle,
    // diffuse_bind_group: wgpu::BindGroup,
    // texture_bind_groups: Vec<wgpu::BindGroup>
    pub camera_pos: (f32, f32),
    camera_buf: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    camera_bind_group_layout: wgpu::BindGroupLayout
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

        Self
        {
            pipelines: vec![pipeline],
            draw_commands: Vec::new(),
            instance_buf: None,
            instance_capacity: 0,
            meshes,
            window_size,
            virtual_size: window_size,
            textures: vec![default_bindgroup],
            texture_bindgroup_layout,
            default_vertex,
            default_fragment,
            // diffuse_bind_group
            // texture_bind_groups
            camera_pos: (0.0, 0.0),
            camera_buf,
            camera_bind_group,
            camera_bind_group_layout
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
        let bindgroup = Arc::new(texture.bind_group(device, &self.texture_bindgroup_layout));
        let id = self.textures.len();
        self.textures.push(bindgroup);
        id
    }

    pub(crate) fn load_char(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, char: char) -> Option<usize>
    {
        if let Ok(text) = crate::text::rasterize_char("engine/src/image/Montserrat-Bold.ttf", char)
        {
            let texture = Texture::from_alpha_bitmap(device, queue, &text.0, text.1, text.2, Some("char")).expect("Failed to create Texture");
            let bindgroup = Arc::new(texture.bind_group(device, &self.texture_bindgroup_layout));
            let id = self.textures.len();
            self.textures.push(bindgroup);
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

                let texture = Texture::from_alpha_bitmap(device, queue, &text.0, text.1, text.2, Some("text")).expect("Failed to create Texture");
                let bindgroup = Arc::new(texture.bind_group(device, &self.texture_bindgroup_layout));
                let id = self.textures.len();
                self.textures.push(bindgroup);
                Some(id)
            }
            Err(e) =>
            {
                println!("Text Rasterizing Failed: {:?}", e);
                None
            }
        }
    }

    pub(crate) fn begin_pass(&self, encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView)
    {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor
        {
            label: Some("Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment
            {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations
                {
                    load: wgpu::LoadOp::Clear(wgpu::Color
                    {
                        r: 0.0,
                        g: 0.0,
                        b: 0.0,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
        });

        // render_pass.set_pipeline(&self.pipelines[0]);

        if let Some(ref instance_buf) = self.instance_buf
        {
            render_pass.set_vertex_buffer(1, instance_buf.slice(..));
            let mut current_pipeline: Option<u8> = None;

            for (instance_id, cmd) in self.draw_commands.iter().enumerate()
            {
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
                        render_pass.set_bind_group(1, self.textures[0].as_ref(), &[]);
                    }
                    MaterialType::Texture(texture) =>
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
                    load: wgpu::LoadOp::Clear(wgpu::Color
                    {
                        r: 0.0,
                        g: 0.0,
                        b: 0.0,
                        a: 1.0,
                    }),
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
        self.draw_commands.push(DrawCommand { mesh_id, transform, /*kind: DrawType::Color(color), */z_index, material: Arc::new(Material::color(color, id)) });
    }

    pub fn draw_texture(&mut self, mesh_id: usize, transform: [[f32; 4]; 4], texture_id: usize, z_index: u32, id: u8)
    {
        let texture = Arc::clone(&self.textures[texture_id]);
        self.draw_commands.push(DrawCommand { mesh_id, transform, /*kind: DrawType::Texture(texture_id), */z_index, material: Arc::new(Material::texture(texture, id)) });
    }

    // draws mesh as is, so only use it for meshes created with world transform, not the quad in the beginning for example
    pub fn draw_mesh(&mut self, mesh_id: usize, texture_id: usize, z_index: u32, shader_id: u8)
    {
        let texture = Arc::clone(&self.textures[texture_id]);
        self.draw_commands.push(DrawCommand
        {
            mesh_id,
            transform: // identity matrix
            [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0]
            ],
            z_index,
            material: Arc::new(Material::texture(texture, shader_id))
        });
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
        self.draw_commands.sort_by_key(|cmd| (cmd.material.pipeline_id, cmd.z_index)); // Sorting by pipeline now first


        let instances: Vec<InstanceData> = self.draw_commands.iter().map(|cmd|
        {
            let material = &cmd.material;
            match material.kind
            {
                MaterialType::Color(color) => InstanceData
                {
                    model: cmd.transform,
                    color: color,
                    mode: 0
                },
                MaterialType::Texture(_) => InstanceData
                {
                    model: cmd.transform,
                    color: [0.0, 0.0, 0.0, 1.0], // Ignored here
                    mode: 1
                }
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

    // Always on screen
    // y opposite to usual (minus = down)
    pub fn ui_matrix(&self, pos: (f32, f32), size: (f32, f32), rotation: f32) -> [[f32; 4]; 4]
    {
        let world_pos =
        (
            self.camera_pos.0 + pos.0 - self.virtual_size.0 * 0.5,
            self.camera_pos.1 - pos.1 + self.virtual_size.1 * 0.5 // +, - to have at top-left 0,0
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

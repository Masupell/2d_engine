use std::{collections::HashMap, hash::{DefaultHasher, Hash, Hasher}, sync::Arc, u64};
use std::result::Result::Ok;
use anyhow::*;

use crate::{shader::Shader, texture::Texture, utility::{InstanceData, Vertex}};

pub struct TextureHandle(pub u64);

impl TextureHandle
{
    pub fn default() -> Self
    {
        Self(u64::MAX)
    }
}

pub enum TextureState
{
    Loaded(Arc<Texture>),
    Loading
}

pub struct AssetManager
{
    pub(crate) texture_bindgroup_layout:  wgpu::BindGroupLayout,
    pub textures: TextureAssets,
    
    pub shader: Shader,
    pub pipelines: PipelineAssets,

    load_queue: Vec<LoadRequest>
}

impl AssetManager
{
    pub(crate) fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Result<Self>
    {
        let texture_bindgroup_layout = Texture::bind_group_layout(device);
        
        let mut textures = TextureAssets::new();
        textures.load_default_texture(device, queue, &texture_bindgroup_layout)?;

        let shader = Shader::default(device);

        Ok(Self
        {
            texture_bindgroup_layout,
            textures,

            shader,
            pipelines: PipelineAssets::new(),

            load_queue: Vec::new()
        })
    }

    pub fn request_texture<F>(&mut self, path: &str, callback: F) -> TextureHandle
    where F: Fn(TextureHandle) + Send + 'static
    {
        // self.load_queue.push(LoadRequest
        // {
        //     path: path.to_string(),
        //     callback: Box::new(callback)
        // });
        let id = self.textures.request_texture(path, callback, &mut self.load_queue);
        TextureHandle(id)
    }


    pub fn process_loading(&mut self, device: &wgpu::Device, queue: &wgpu::Queue)
    {
        let mut completed = Vec::new();

        for (i, request) in self.load_queue.iter().enumerate()
        {
            match Texture::new(device, queue, &request.path, &self.texture_bindgroup_layout)
            {
                Ok(texture) => 
                {
                    let arc_texture = Arc::new(texture);
                    let id = self.textures.path_to_id[&hash_path(&request.path)];
                    self.textures.finalize_load(id, arc_texture);
                    (request.callback)(TextureHandle(id));
                    completed.push(i);
                },
                Err(e) => eprintln!("Failed to load {}: {:?}", request.path, e),
            }

            // match self.textures.load_texture(device, queue, &request.path, &self.texture_bindgroup_layout)
            // {
            //     Ok(id) => 
            //     {
            //         (request.callback)(id);
            //         completed.push(i);
            //     }
            //     Err(e) =>
            //     {
            //         eprintln!("Failed to load {}: {:?}", request.path, e);
            //     }
            // }
        }

        for &i in completed.iter().rev() 
        {
            self.load_queue.swap_remove(i);
        }
    }
}

pub struct LoadRequest
{
    path: String,
    callback: Box<dyn Fn(TextureHandle) + Send + 'static>
}

pub struct TextureAssets
{
    textures: HashMap<u64, TextureState>, // Maybe store path later too, for hot reloading (but right now it is completely fine)
    path_to_id: HashMap<u64, u64>,
    next_id: u64
}

impl TextureAssets
{
    pub(crate) fn new() -> Self
    {        
        Self
        {
            textures: HashMap::new(),
            path_to_id: HashMap::new(),
            next_id: 0
        }
    }

    pub(crate) fn load_default_texture(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, layout: &wgpu::BindGroupLayout) -> Result<u64>
    {
        let hash = hash_path("white_texture");
        let default_texture = Arc::new(Texture::white(device, queue, layout)?);
        let id = self.next_id;
        self.next_id += 1;
        self.textures.insert(id, TextureState::Loaded(default_texture));
        self.path_to_id.insert(hash, id);
        Ok(id)
    }

    pub(crate) fn request_texture<F>(&mut self, path: &str, callback: F, load_queue: &mut Vec<LoadRequest>) -> u64
    where F: Fn(TextureHandle) + Send + 'static
    {
        let hash = hash_path(path);

        if let Some(&id) = self.path_to_id.get(&hash)
        {
            if let TextureState::Loaded(_) = self.textures[&id]
            {
                callback(TextureHandle(id))
            }
            return id;
        }

        let id = self.next_id;
        self.next_id += 1;

        self.textures.insert(id, TextureState::Loading);
        self.path_to_id.insert(hash, id);

        load_queue.push(LoadRequest
        {
            path: path.to_string(),
            callback: Box::new(callback)
        });

        id
    }

    pub fn finalize_load(&mut self, id: u64, texture: Arc<Texture>)
    {
        self.textures.insert(id, TextureState::Loaded(texture));
    }

    pub fn get_texture(&self, handle: &TextureHandle) -> Option<Arc<Texture>>
    {
        match self.textures.get(&handle.0)?
        {
            TextureState::Loaded(texture) => Some(Arc::clone(texture)),
            TextureState::Loading => None,
        }
    }

    // pub(crate) fn load_texture(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, path: &str, layout: &wgpu::BindGroupLayout) -> Result<u64>
    // {
    //     let hash = hash_path(path);

    //     if let Some(&id) = self.path_to_id.get(&hash)
    //     {
    //         return Ok(id);
    //     }

    //     let texture = Texture::new(device, queue, path, layout)?;

    //     let id = self.next_id;
    //     self.next_id += 1;

    //     self.textures.insert(id, Arc::new(texture));
    //     self.path_to_id.insert(hash, id);

    //     Ok(id)
    // }

    // Both those functions, I need to rewrite to fit better, so right now no text for me anymore :(
    // pub fn load_char(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, char: char) -> Option<usize>
    // {
    //     if let Ok(text) = crate::text::rasterize_char("engine/src/image/Montserrat-Bold.ttf", char)
    //     {
    //         let texture = Texture::from_alpha_bitmap(device, queue, &text.0, text.1, text.2, Some("char"), &self.texture_bindgroup_layout).expect("Failed to create Texture");
    //         let id = self.textures.len();
    //         self.textures.push(bindgroup);
    //         Some(id)
    //     }
    //     else
    //     {
    //         None
    //     }
    // }

    // pub fn load_text(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, text: &str, size: f32) -> Option<usize>
    // {
    //     match crate::text::rasterize_static_text("engine/src/image/Montserrat-Bold.ttf", text, size) 
    //     {
    //         Ok(text) => 
    //         {
    //             // Test
    //             let output = image::GrayImage::from_vec(text.1 as u32, text.2 as u32, text.0.to_vec());
    //             match output 
    //             {
    //                 Some(image) =>
    //                 {
    //                     image.save("engine/src/image/text_texture.png").unwrap();
    //                 }
    //                 None => println!("Hello")
    //             }
    //             // output.save("engine/src/image/text_texture.png").unwrap();
    //             //

    //             let texture = Texture::from_alpha_bitmap(device, queue, &text.0, text.1, text.2, Some("text")).expect("Failed to create Texture");
    //             let bindgroup = Arc::new(texture.bind_group(device, &self.texture_bindgroup_layout));
    //             let id = self.textures.len();
    //             self.textures.push(bindgroup);
    //             Some(id)
    //         }
    //         Err(e) => 
    //         {
    //             println!("Text Rasterizing Failed: {:?}", e);
    //             None
    //         }
    //     }
    // }

    // pub fn get_ref(&self, id: u64) -> Option<&Texture> 
    // {
    //     self.textures.get(&id).map(|arc| arc.as_ref())
    // }

    // pub fn get_bind_group(&self, id: u64) -> Option<Arc<wgpu::BindGroup>> 
    // {
    //     self.textures.get(&id).map(|t| Arc::clone(&t.bind_group))
    // }

    pub fn get_bind_group(&self, handle: TextureHandle) -> Option<Arc<wgpu::BindGroup>> 
    {
        match &self.textures[&handle.0] 
        {
            TextureState::Loaded(texture) => Some(Arc::clone(&texture.bind_group)),
            TextureState::Loading => None
        }
    }
}


pub struct PipelineAssets
{
    pipelines: HashMap<u64, Arc<wgpu::RenderPipeline>>,
    path_to_id: HashMap<u64, u64>,
    next_id: u64
}

impl PipelineAssets
{
    pub fn new() -> Self
    {
        Self 
        {
            pipelines: HashMap::new(),
            path_to_id: HashMap::new(),
            next_id: 0,
        }    
    }

    pub(crate) fn load_pipeline(&mut self, device: &wgpu::Device, config: &wgpu::SurfaceConfiguration, fragment_path: Option<&str>, vertex_path: Option<&str>, shader: &mut Shader, layout: &wgpu::BindGroupLayout) -> u64
    {
        let mut buff = String::new();

        if let Some(path) = fragment_path
        {
            shader.new_fragment(device, path, "fs_main");
            buff += path;
        }
        if let Some(path) = vertex_path
        {
            shader.new_vertex(device, path, "vs_main");
            buff += path;
        }
        let hash = hash_path(&buff);

        if let Some(&id) = self.path_to_id.get(&hash) 
        {
            return id;
        }

        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor
        {
            label: Some("Pipeline Layout"),
            bind_group_layouts: 
            &[
                layout
            ],
            push_constant_ranges: &[]
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor
        {
            label: Some("Render Pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState 
            {
                module: &shader.vertex_module,
                entry_point: Some(&shader.vs_entry),
                buffers: &[Vertex::desc(), InstanceData::desc()],
                compilation_options: wgpu::PipelineCompilationOptions::default()
            },
            fragment: Some(wgpu::FragmentState
            {
                module: &shader.fragment_module,
                entry_point: Some(&shader.fs_entry),
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
                cull_mode: Some(wgpu::Face::Back),
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

        let id = self.next_id;
        self.next_id += 1;

        self.pipelines.insert(id, Arc::new(pipeline));
        self.path_to_id.insert(hash, id);

        id
    }

    pub fn get_pipeline(&self, id: u64) -> Option<Arc<wgpu::RenderPipeline>>
    {
        self.pipelines.get(&id).cloned()
    }
}


fn hash_path(path: &str) -> u64 
{
    let mut hasher = DefaultHasher::new();
    path.hash(&mut hasher);
    hasher.finish()
}
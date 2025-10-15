use std::{collections::HashMap, error::Error, hash::{DefaultHasher, Hash, Hasher}, sync::Arc};
use anyhow::*;

use crate::texture::Texture;

pub struct AssetManager
{
    pub(crate) texture_bindgroup_layout:  wgpu::BindGroupLayout,
    textures: HashMap<u64, Arc<Texture>>, // Maybe store path later too, for hot reloading (but right now it is completely fine)
    path_to_id: HashMap<u64, u64>,
    next_id: u64
}

impl AssetManager
{
    pub fn new(device: &wgpu::Device) -> Self
    {
        let texture_bindgroup_layout = Texture::bind_group_layout(device);
        
        Self
        {
            texture_bindgroup_layout,
            textures: HashMap::new(),
            path_to_id: HashMap::new(),
            next_id: 0
        }
    }

    pub(crate) fn load_default_texture(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) -> Result<u64>
    {
        let hash = hash_path("white_texture");
        let default_texture = Texture::white(device, queue, &self.texture_bindgroup_layout)?;
        let id = self.next_id;
        self.next_id += 1;
        self.textures.insert(id, Arc::new(default_texture));
        self.path_to_id.insert(hash, id);
        Ok(id)
    }

    pub fn load_texture(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, path: &str) -> Result<u64>
    {
        let hash = hash_path(path);

        if let Some(&id) = self.path_to_id.get(&hash)
        {
            return Ok(id);
        }

        let texture = Texture::new(device, queue, path, &self.texture_bindgroup_layout)?;

        let id = self.next_id;
        self.next_id += 1;

        self.textures.insert(id, Arc::new(texture));
        self.path_to_id.insert(hash, id);

        Ok(id)
    }

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

    pub fn get_texture(&self, id: u64) -> Option<Arc<Texture>>
    {
        self.textures.get(&id).cloned()
    }

    pub fn get_bind_group(&self, id: u64) -> Option<Arc<wgpu::BindGroup>> 
    {
        self.textures.get(&id).map(|t| Arc::clone(&t.bind_group))
    }
}

fn hash_path(path: &str) -> u64 
{
    let mut hasher = DefaultHasher::new();
    path.hash(&mut hasher);
    hasher.finish()
}
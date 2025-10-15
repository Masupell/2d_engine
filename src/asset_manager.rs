use std::{collections::HashMap, hash::{DefaultHasher, Hash, Hasher}, sync::Arc};

use crate::texture::Texture;

pub struct AssetManager
{
    pub(crate) texture_bindgroup_layout:  wgpu::BindGroupLayout,
    textures: HashMap<u64, Arc<Texture>>,
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

    pub fn load_texture(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, path: &str) -> u64
    {
        let hash = hash_path(path);

        if let Some(&id) = self.path_to_id.get(&hash)
        {
            return id;
        }

        let texture = Texture::new(device, queue, path, &self.texture_bindgroup_layout).unwrap(); //No error Handling

        let id = self.next_id;
        self.next_id += 1;

        self.textures.insert(id, Arc::new(texture));
        self.path_to_id.insert(hash, id);

        id
    }

    pub fn get_texture(&self, id: u64) -> Option<Arc<Texture>>
    {
        self.textures.get(&id).cloned()
    }

    // pub fn get_bind_group(&self, id: u64) -> Option<Arc<
}

fn hash_path(path: &str) -> u64 
{
    let mut hasher = DefaultHasher::new();
    path.hash(&mut hasher);
    hasher.finish()
}
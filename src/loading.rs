use crate::{renderer::{Renderer, WHITE_TEXTURE}, texture::{FilterMode, Texture, TextureEntry}, threads::{Task, TaskPoll, ThreadPool}};
use std::sync::Arc;

pub(crate) struct DecodedImage
{
    rgba: Vec<u8>,
    width: u32,
    height: u32
}

pub(crate) struct PendingTexture
{
    texture_id: usize,
    task: Task<Result<DecodedImage, String>>,
    mag_filter: FilterMode,
    min_filter: FilterMode
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub(crate) enum TextureSource
{
    Path(String),
    Bytes(usize, usize) // (address, length)
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub(crate) struct TextureKey
{
    source: TextureSource,
    mag_filter: FilterMode,
    min_filter: FilterMode
}

impl TextureKey
{
    fn path(path: &str, mag_filter: FilterMode, min_filter: FilterMode) -> Self
    {
        Self { source: TextureSource::Path(path.to_string()), mag_filter, min_filter }
    }

    fn bytes(bytes: &'static [u8], mag_filter: FilterMode, min_filter: FilterMode) -> Self
    {
        Self { source: TextureSource::Bytes(bytes.as_ptr() as usize, bytes.len()), mag_filter, min_filter }
    }
}

fn decode_file(path: &str) -> Result<DecodedImage, String>
{
    let bytes = std::fs::read(path).map_err(|e| format!("'{path}': {e}"))?;
    decode_bytes(&bytes).map_err(|e| format!("'{path}': {e}"))
}

fn decode_bytes(bytes: &[u8]) -> Result<DecodedImage, String>
{
    let image = image::load_from_memory(bytes).map_err(|e| e.to_string())?.to_rgba8();
    let (width, height) = image.dimensions();
    Ok(DecodedImage { rgba: image.into_raw(), width, height })
}

impl Renderer
{
    // returns id immidiately, but is white until it is loaded
    pub fn load_texture_async(&mut self, path: &str, mag_filter: FilterMode, min_filter: FilterMode) -> usize
    {
        let key = TextureKey::path(path, mag_filter, min_filter);
        if let Some(&id) = self.texture_cache.get(&key) { return id; }

        let path = path.to_string();
        let task = self.threads.spawn(move || decode_file(&path));
        let id = self.push_pending(task, mag_filter, min_filter);
        self.remember_texture(key, id);
        id
    }

    pub fn load_texture_from_bytes_async(&mut self, bytes: &'static [u8], mag_filter: FilterMode, min_filter: FilterMode) -> usize
    {
        let key = TextureKey::bytes(bytes, mag_filter, min_filter);
        if let Some(&id) = self.texture_cache.get(&key) { return id; }

        let task = self.threads.spawn(move || decode_bytes(bytes));
        let id = self.push_pending(task, mag_filter, min_filter);
        self.remember_texture(key, id);
        id
    }

    fn remember_texture(&mut self, key: TextureKey, id: usize)
    {
        self.texture_keys.insert(id, key.clone());
        self.texture_cache.insert(key, id);
    }

    fn push_pending(&mut self, task: Task<Result<DecodedImage, String>>, mag_filter: FilterMode, min_filter: FilterMode) -> usize
    {
        let texture_id = self.textures.len();

        let placeholder = TextureEntry { bind_group: Arc::clone(&self.textures[WHITE_TEXTURE].bind_group), size: (1.0, 1.0) };
        self.textures.push(placeholder);

        self.pending_textures.push(PendingTexture { texture_id, task, mag_filter, min_filter });
        texture_id
    }

    // Called once per frame before update, updates pending_texture ids
    pub(crate) fn poll_pending_textures(&mut self, device: &wgpu::Device, queue: &wgpu::Queue)
    {
        if self.pending_textures.is_empty() { return; }

        let mut pending = std::mem::take(&mut self.pending_textures);
        pending.retain_mut(|load| match load.task.poll()
        {
            TaskPoll::Pending => true, // keep waiting
            TaskPoll::Ready(Ok(image)) =>
            {
                let texture = Texture::from_rgba8(device, queue, &image.rgba, image.width, image.height, load.mag_filter, load.min_filter, None);
                let bind_group = Arc::new(texture.bind_group(device, &self.texture_bindgroup_layout));
                self.textures[load.texture_id] = TextureEntry { bind_group, size: (image.width as f32, image.height as f32) };
                false
            }
            // stays white
            TaskPoll::Ready(Err(error)) =>
            {
                log::error!("Failed to load texture {}: {error}", load.texture_id);
                false
            }
            TaskPoll::Failed =>
            {
                log::error!("Texture decode panicked for texture {}", load.texture_id);
                false
            }
        });

        self.pending_textures = pending;
    }


    pub fn unload_texture(&mut self, id: usize)
    {
        // only removes game loaded texture (so if id does not exist, or it is engine loaded, like the white texture, ...)
        let Some(key) = self.texture_keys.remove(&id) else
        {
            println!("texture({id} is not a loaded texture (engine owned or does not exist)");
            return;
        };

        self.texture_cache.remove(&key);
        self.pending_textures.retain(|load| load.texture_id != id);
        self.textures[id] = TextureEntry { bind_group: Arc::clone(&self.textures[WHITE_TEXTURE].bind_group), size: (0.0, 0.0) };
        self.unloaded_textures.insert(id);
    }


    pub fn texture_count(&self) -> usize
    {
        self.textures.len() - self.unloaded_textures.len()
    }

    // rough gpu memory of all textures, assumes 4 bytes per pixel
    pub fn texture_memory_estimate(&self) -> usize
    {
        self.textures.iter().map(|t| (t.size.0 * t.size.1) as usize * 4).sum()
    }

    // 0 -> everythings done (for loading screens for example)
    pub fn textures_loading(&self) -> usize
    {
        self.pending_textures.len()
    }

    pub fn is_texture_loaded(&self, texture_id: usize) -> bool
    {
        !self.unloaded_textures.contains(&texture_id) && !self.pending_textures.iter().any(|load| load.texture_id == texture_id)
    }




    pub(crate) fn load_texture(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, path: &str, mag_filter: FilterMode, min_filter: FilterMode) -> usize
    {
        let key = TextureKey::path(path, mag_filter, min_filter);
        if let Some(&id) = self.texture_cache.get(&key) { return id; }

        let texture = Texture::new(device, queue, path, mag_filter, min_filter).unwrap_or_else(|e| panic!("Failed to load texture '{path}': {e}"));
        let id = self.register_texture(device, texture);
        self.remember_texture(key, id);
        id
    }

    pub(crate) fn load_texture_from_bytes(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, bytes: &'static [u8], mag_filter: FilterMode, min_filter: FilterMode) -> usize
    {
        let key = TextureKey::bytes(bytes, mag_filter, min_filter);
        if let Some(&id) = self.texture_cache.get(&key) { return id; }

        let texture = Texture::from_bytes(device, queue, bytes, mag_filter, min_filter).unwrap_or_else(|e| panic!("Failed to load texture: {e}"));
        let id = self.register_texture(device, texture);
        self.remember_texture(key, id);
        id
    }

    fn register_texture(&mut self, device: &wgpu::Device, texture: Texture) -> usize
    {
        let extent = texture.texture.size();
        let bind_group = Arc::new(texture.bind_group(device, &self.texture_bindgroup_layout));
        let id = self.textures.len();
        self.textures.push(TextureEntry { bind_group, size: (extent.width as f32, extent.height as f32) });
        id
    }
}


pub enum Loading<T>
{
    Idle,
    Running(Task<T>),
    Done(T),
    Failed
}

impl<T: Send + 'static> Loading<T>
{
    // Runs each frame, returns true at the moment it's done
    pub fn update(&mut self) -> bool
    {
        let Loading::Running(task) = self else { return false; };

        match task.poll()
        {
            TaskPoll::Pending => false,
            TaskPoll::Ready(value) => { *self = Loading::Done(value); true }
            TaskPoll::Failed => { *self = Loading::Failed; false }
        }
    }

    // Borrow result (keep doing something with it constantly after it's done)
    pub fn get(&self) -> Option<&T>
    {
        match self { Loading::Done(value) => Some(value), _ => None }
    }

    // one shot use (take value once) and returns to Idle
    pub fn take(&mut self) -> Option<T>
    {
        match std::mem::replace(self, Loading::Idle)
        {
            Loading::Done(value) => Some(value),
            other => { *self = other; None }
        }
    }

    pub fn is_running(&self) -> bool
    {
        matches!(self, Loading::Running(_))
    }
}

impl<T> Default for Loading<T>
{
    fn default() -> Self { Loading::Idle }
}

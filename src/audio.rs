pub mod audio_backend;

use std::collections::HashMap;

use crate::{assets::AssetSource, audio::audio_backend::{BackEnd, BackendSound}};

pub struct Audio
{
    backend: Option<BackEnd>,
    sounds: HashMap<usize, BackendSound>,
    next_id: usize, // to not reuse ids
    cache: HashMap<AssetSource, usize>, // source -> id, to check if it's already loaded
    sources: HashMap<usize, AssetSource>, // id -> source, for unloading
    master_volume: f32
}

impl Audio
{
    pub(crate) fn new() -> Self
    {
        let backend = BackEnd::open();
        if backend.is_none()
        {
            println!("No audio device detected, running without audio");
        }

        Self
        {
            backend,
            sounds: HashMap::new(),
            next_id: 0,
            cache: HashMap::new(),
            sources: HashMap::new(),
            master_volume: 1.0
        }
    }

    pub fn load_sound(&mut self, path: &str) -> usize
    {
        let source = AssetSource::path(path);
        if let Some(&id) = self.cache.get(&source) { return id; }

        let sound = BackendSound::load(path).unwrap_or_else(|e| panic!("Failed to load sound '{path}': {e}"));
        self.register_sound(source, sound)
    }

    pub fn load_sound_from_bytes(&mut self, bytes: &'static [u8]) -> usize
    {
        let source = AssetSource::bytes(bytes);
        if let Some(&id) = self.cache.get(&source) { return id; }

        let sound = BackendSound::from_bytes(bytes).unwrap_or_else(|e| panic!("Failed to load sound: {e}"));
        self.register_sound(source, sound)
    }

    // Currently only for short sound-effects, no streaming yet
    fn register_sound(&mut self, source: AssetSource, sound: BackendSound) -> usize
    {
        let id = self.next_id;
        self.next_id += 1;

        self.sounds.insert(id, sound);
        self.cache.insert(source.clone(), id);
        self.sources.insert(id, source);
        id
    }

    pub fn unload_sound(&mut self, sound_id: usize)
    {
        let Some(source) = self.sources.remove(&sound_id) else
        {
            println!("sound({sound_id} does not exist");
            return;
        };

        self.cache.remove(&source);
        self.sounds.remove(&sound_id);
    }

    pub fn is_sound_loaded(&self, sound_id: usize) -> bool
    {
        self.sounds.contains_key(&sound_id)
    }

    pub fn sound_count(&self) -> usize
    {
        self.sounds.len()
    }

    pub fn play(&self, sound_id: usize)
    {
        self.play_with(sound_id, 1.0, 1.0);
    }

    // volume: 0.0..1.0
    pub fn play_with(&self, sound_id: usize, volume: f32, pitch: f32)
    {
        let volume = (volume * self.master_volume).max(0.0);
        if volume <= 0.0 { return; }
        let pitch = pitch.max(0.01);

        if let (Some(backend), Some(sound)) = (&self.backend, self.sounds.get(&sound_id))
        {
            backend.play(sound, volume, pitch)
        }
    }

    pub fn set_master_volume(&mut self, volume: f32)
    {
        self.master_volume = volume.max(0.0);
    }

    pub fn master_volume(&self) -> f32
    {
        self.master_volume
    }
}

pub mod audio_backend;

use std::collections::HashMap;

use crate::{SoundId, assets::AssetSource, audio::audio_backend::{BackEnd, BackendSound}, handles::Slots};

pub struct Audio
{
    backend: Option<BackEnd>,
    sounds: Slots<SoundId, BackendSound>, //HashMap<usize, BackendSound>,
    cache: HashMap<AssetSource, SoundId>, // source -> id, to check if it's already loaded
    sources: HashMap<SoundId, AssetSource>, // id -> source, for unloading
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
            sounds: Slots::new(),
            cache: HashMap::new(),
            sources: HashMap::new(),
            master_volume: 1.0
        }
    }

    pub fn load_sound(&mut self, path: &str) -> SoundId
    {
        let source = AssetSource::path(path);
        if let Some(&id) = self.cache.get(&source) { return id; }

        let sound = BackendSound::load(path).unwrap_or_else(|e| panic!("Failed to load sound '{path}': {e}"));
        self.register_sound(source, sound)
    }

    pub fn load_sound_from_bytes(&mut self, bytes: &'static [u8]) -> SoundId
    {
        let source = AssetSource::bytes(bytes);
        if let Some(&id) = self.cache.get(&source) { return id; }

        let sound = BackendSound::from_bytes(bytes).unwrap_or_else(|e| panic!("Failed to load sound: {e}"));
        self.register_sound(source, sound)
    }

    // Currently only for short sound-effects, no streaming yet
    fn register_sound(&mut self, source: AssetSource, sound: BackendSound) -> SoundId
    {
        let id = self.sounds.insert(sound);
        self.cache.insert(source.clone(), id);
        self.sources.insert(id, source);
        id
    }

    pub fn unload_sound(&mut self, id: SoundId)
    {
        let Some(source) = self.sources.remove(&id) else
        {
            println!("sound({id:?} does not exist");
            return;
        };

        self.cache.remove(&source);
        self.sounds.remove(id);
    }

    pub fn is_sound_loaded(&self, id: SoundId) -> bool
    {
        self.sounds.contains(id)
    }

    pub fn sound_count(&self) -> usize
    {
        self.sounds.len()
    }

    pub fn play(&self, id: SoundId)
    {
        self.play_with(id, 1.0, 1.0);
    }

    // volume: 0.0..1.0
    pub fn play_with(&self, id: SoundId, volume: f32, pitch: f32)
    {
        let volume = (volume * self.master_volume).max(0.0);
        if volume <= 0.0 { return; }
        let pitch = pitch.max(0.01);

        if let (Some(backend), Some(sound)) = (&self.backend, self.sounds.get(id))
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

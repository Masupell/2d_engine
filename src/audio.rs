pub mod audio_backend;

use crate::audio::audio_backend::{BackEnd, BackendSound};

pub struct Audio
{
    backend: Option<BackEnd>,
    sounds: Vec<BackendSound>,
    master_volume: f32
}

impl Audio
{
    pub(crate) fn new() -> Self
    {
        let backend = BackEnd::open();
        if backend.is_none()
        {
            println!("NO audio device detected, running without audio");
        }

        Self
        {
            backend,
            sounds: Vec::new(),
            master_volume: 1.0
        }
    }

    pub fn load_sound(&mut self, path: &str) -> usize
    {
        let sound = BackendSound::load(path).unwrap_or_else(|e| panic!("Failed to load sound '{path}': {e}"));
        let id = self.sounds.len();
        self.sounds.push(sound);
        id
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

        if let Some(backend) = &self.backend
        {
            backend.play(&self.sounds[sound_id], volume, pitch)
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

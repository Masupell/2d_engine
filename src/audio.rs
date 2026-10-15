pub mod audio_backend;

use std::collections::HashMap;

use crate::{PlaybackId, SoundId, assets::AssetSource, audio::audio_backend::{BackEnd, BackendSound, BackendVoice}, handles::Slots};

pub struct Audio
{
    backend: Option<BackEnd>,
    sounds: Slots<SoundId, BackendSound>, //HashMap<usize, BackendSound>,
    cache: HashMap<SoundKey, SoundId>, // source -> id, to check if it's already loaded
    sources: HashMap<SoundId, SoundKey>, // id -> source, for unloading
    voices: Slots<PlaybackId, Voice>, // sounds that are playing right now
    master_volume: f32
}

// (source, streamed), to avoid getting it from cache, if loading a buffered and streamed sound
type SoundKey = (AssetSource, bool);

struct Voice
{
    backend: BackendVoice,
    volume: f32 // without master volume
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
            voices: Slots::new(),
            master_volume: 1.0
        }
    }

    // decodes in the beginning and keeps in memory, for short things like sound effects
    pub fn load_sound(&mut self, path: &str) -> SoundId
    {
        let key = (AssetSource::path(path), false);
        if let Some(&id) = self.cache.get(&key) { return id; }

        let sound = BackendSound::load(path).unwrap_or_else(|e| panic!("Failed to load sound '{path}': {e}"));
        self.register_sound(key, sound)
    }

    pub fn load_sound_from_bytes(&mut self, bytes: &'static [u8]) -> SoundId
    {
        let key = (AssetSource::bytes(bytes), false);
        if let Some(&id) = self.cache.get(&key) { return id; }

        let sound = BackendSound::from_bytes(bytes).unwrap_or_else(|e| panic!("Failed to load sound: {e}"));
        self.register_sound(key, sound)
    }

    // Decodes while playing, does not use as much memory, but uses a bit of cpu, for things like music
    pub fn load_sound_streamed(&mut self, path: &str) -> SoundId
    {
        let key = (AssetSource::path(path), true);
        if let Some(&id) = self.cache.get(&key) { return id; }

        let sound = BackendSound::load_streamed(path).unwrap_or_else(|e| panic!("Failed to load sound '{path}': {e}"));
        self.register_sound(key, sound)
    }

    pub fn load_sound_streamed_from_bytes(&mut self, bytes: &'static [u8]) -> SoundId
    {
        let key = (AssetSource::bytes(bytes), true);
        if let Some(&id) = self.cache.get(&key) { return id; }

        let sound = BackendSound::from_bytes_streamed(bytes).unwrap_or_else(|e| panic!("Failed to load sound: {e}"));
        self.register_sound(key, sound)
    }

    fn register_sound(&mut self, key: SoundKey, sound: BackendSound) -> SoundId
    {
        let id = self.sounds.insert(sound);
        self.cache.insert(key.clone(), id);
        self.sources.insert(id, key);
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

    // Id not important for one time sounds
    pub fn play(&mut self, id: SoundId) -> PlaybackId
    {
        self.play_with(id, 1.0, 1.0)
    }

    // volume: 0.0..1.0
    pub fn play_with(&mut self, id: SoundId, volume: f32, pitch: f32) -> PlaybackId
    {
        self.start(id, volume, pitch, false)
    }

    pub fn play_looped(&mut self, id: SoundId, volume: f32, pitch: f32) -> PlaybackId
    {
        self.start(id, volume, pitch, true)
    }

    fn start(&mut self, id: SoundId, volume: f32, pitch: f32, looped: bool) -> PlaybackId
    {
        self.voices.retain(|voice| !voice.backend.is_finished());

        let (Some(backend), Some(sound)) = (&self.backend, self.sounds.get(id)) else { return PlaybackId::NONE; };

        let volume = volume.max(0.0);
        match backend.play(sound, volume * self.master_volume, pitch.max(0.01), looped)
        {
            Ok(backend) => self.voices.insert(Voice { backend, volume }),
            Err(e) =>
            {
                println!("Failed to play sound({id:?}): {e}");
                PlaybackId::NONE
            }
        }
    }

    pub fn stop(&mut self, playback: PlaybackId)
    {
        self.voices.remove(playback);
    }

    pub fn stop_all(&mut self)
    {
        self.voices.retain(|_| false);
    }

    pub fn pause(&self, playback: PlaybackId)
    {
        if let Some(voice) = self.voices.get(playback) { voice.backend.pause(); }
    }

    pub fn resume(&self, playback: PlaybackId)
    {
        if let Some(voice) = self.voices.get(playback) { voice.backend.resume(); }
    }

    pub fn set_volume(&mut self, playback: PlaybackId, volume: f32)
    {
        let master = self.master_volume;
        if let Some(voice) = self.voices.get_mut(playback)
        {
            voice.volume = volume.max(0.0);
            voice.backend.set_volume(voice.volume * master);
        }
    }

    pub fn set_pitch(&self, playback: PlaybackId, pitch: f32)
    {
        if let Some(voice) = self.voices.get(playback) { voice.backend.set_pitch(pitch.max(0.01)); }
    }

    pub fn is_playing(&self, playback: PlaybackId) -> bool
    {
        self.voices.get(playback).is_some_and(|voice| !voice.backend.is_finished() && !voice.backend.is_paused())
    }

    pub fn set_master_volume(&mut self, volume: f32)
    {
        self.master_volume = volume.max(0.0);
        for voice in self.voices.values()
        {
            voice.backend.set_volume(voice.volume * self.master_volume);
        }
    }

    pub fn master_volume(&self) -> f32
    {
        self.master_volume
    }
}

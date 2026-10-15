pub mod audio_backend;

use std::collections::HashMap;

use crate::{PlaybackId, SoundId, assets::AssetSource, audio::audio_backend::{BackEnd, BackendSound, BackendVoice}, handles::Slots};

pub struct Audio
{
    backend: Option<BackEnd>,
    sounds: Slots<SoundId, BackendSound>, //HashMap<usize, BackendSound>,
    cache: HashMap<AssetSource, SoundId>, // source -> id, to check if it's already loaded
    sources: HashMap<SoundId, AssetSource>, // id -> source, for unloading
    voices: Slots<PlaybackId, Voice>, // sounds that are playing right now
    master_volume: f32
}

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
        let backend = backend.play(sound, volume * self.master_volume, pitch.max(0.01), looped);
        self.voices.insert(Voice { backend, volume })
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

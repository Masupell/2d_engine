pub mod audio_backend;
pub mod synth;

use std::collections::HashMap;

use crate::{PlaybackId, SoundId, assets::AssetSource, audio::{audio_backend::{BackEnd, BackendSound, BackendVoice}, synth::{SAMPLE_RATE, SoundBuilder}}, handles::Slots};

pub struct Audio
{
    backend: Option<BackEnd>,
    sounds: Slots<SoundId, BackendSound>, //HashMap<usize, BackendSound>,
    cache: HashMap<SoundKey, SoundId>, // source -> id, to check if it's already loaded
    sources: HashMap<SoundId, SoundKey>, // id -> source, for unloading
    voices: Slots<PlaybackId, Voice>, // sounds that are playing right now
    max_voices: usize,
    started: u64, // to find oldest voice
    master_volume: f32
}

const DEFAULT_MAX_VOICES: usize = 32;

// (source, streamed), to avoid getting it from cache, if loading a buffered and streamed sound
type SoundKey = (AssetSource, bool);

struct Voice
{
    backend: BackendVoice,
    volume: f32, // without master volume
    started: u64,
    looped: bool
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
            max_voices: DEFAULT_MAX_VOICES,
            started: 0,
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

    // Not cached, every call makes a new sound
    pub fn create_sound(&mut self, sound: &SoundBuilder) -> SoundId
    {
        let peak = sound.samples().iter().fold(0.0_f32, |peak, sample| peak.max(sample.abs())); // anything above 1.0 would clip, sound distorted
        let scale = if peak > 1.0 { 1.0 / peak } else { 1.0 };

        self.sounds.insert(BackendSound::from_samples(sound.samples().iter().map(|sample| sample * scale), SAMPLE_RATE))
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
        if self.sounds.remove(id).is_none()
        {
            println!("sound({id:?}) does not exist");
            return;
        }

        if let Some(source) = self.sources.remove(&id)
        {
            self.cache.remove(&source);
        }
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
        if !self.sounds.contains(id) || !self.make_room() { return PlaybackId::NONE; }
        let (Some(backend), Some(sound)) = (&self.backend, self.sounds.get(id)) else { return PlaybackId::NONE; };

        let volume = volume.max(0.0);
        match backend.play(sound, volume * self.master_volume, pitch.max(0.01), looped)
        {
            Ok(voice) => self.insert_voice(voice, volume, looped),
            Err(e) =>
            {
                println!("Failed to play sound({id:?}): {e}");
                PlaybackId::NONE
            }
        }
    }

    // Runs on audio thread, so has to keep up (44100 times per second, so nothing too crazy complicated)
    // values above 1.0 distort
    pub fn play_generator(&mut self, samples: impl Iterator<Item = f32> + Send + 'static, volume: f32) -> PlaybackId
    {
        if !self.make_room() { return PlaybackId::NONE; }
        let Some(backend) = &self.backend else { return PlaybackId::NONE; };

        let volume = volume.max(0.0);
        let voice = backend.play_generator(samples, SAMPLE_RATE, volume * self.master_volume, 1.0);
        self.insert_voice(voice, volume, false)
    }

    fn make_room(&mut self) -> bool
    {
        self.voices.retain(|voice| !voice.backend.is_finished());
        self.backend.is_some() && (self.voices.len() < self.max_voices || self.steal_voice())
    }

    fn insert_voice(&mut self, backend: BackendVoice, volume: f32, looped: bool) -> PlaybackId
    {
        self.started += 1;
        self.voices.insert(Voice { backend, volume, started: self.started, looped })
    }

    // Stops the oldest non-looping sound (so soundeffects, not music for example)
    // if nothing, then sound is not played (returns false)
    fn steal_voice(&mut self) -> bool
    {
        let oldest = self.voices.iter().filter(|(_, voice)| !voice.looped).min_by_key(|(_, voice)| voice.started).map(|(id, _)| id);
        oldest.is_some_and(|id| self.voices.remove(id).is_some())
    }

    pub fn set_max_voices(&mut self, max: usize)
    {
        self.max_voices = max.max(1);
    }

    pub fn voice_count(&self) -> usize
    {
        self.voices.len()
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

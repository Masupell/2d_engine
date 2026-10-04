// If I ever want to swap out rodio for more low-level, like cpal

use std::fs::File;
use std::io::BufReader;
use rodio::{Decoder, MixerDeviceSink};
use rodio::source::{Buffered, Source};

pub(crate) struct BackendSound
{
    source: Buffered<Decoder<BufReader<File>>>
}

impl BackendSound
{
    pub(crate) fn load(path: &str) -> Result<Self, String>
    {
        let file = File::open(path).map_err(|e| e.to_string())?;
        let decoder = Decoder::new(BufReader::new(file)).map_err(|e| e.to_string())?;

        Ok(Self {source: decoder.buffered() })
    }
}

pub(crate) struct BackEnd
{
    sink: MixerDeviceSink
}

impl BackEnd
{
    pub(crate) fn open() -> Option<Self>
    {
        rodio::DeviceSinkBuilder::open_default_sink().ok().map(|mut sink|
        {
            sink.log_on_drop(false);
            Self { sink }
        })
    }

    pub(crate) fn play(&self, sound: &BackendSound, volume: f32, pitch: f32)
    {
        let instance = sound.source.clone().amplify(volume).speed(pitch);
        self.sink.mixer().add(instance);
    }
}

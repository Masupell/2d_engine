// If I ever want to swap out rodio for more low-level, like cpal

use std::borrow::Cow;
use std::io::Cursor;
use rodio::{Decoder, MixerDeviceSink};
use rodio::source::{Buffered, Source};

type SoundSource = Buffered<Decoder<Cursor<Cow<'static, [u8]>>>>;

pub(crate) struct BackendSound
{
    source: SoundSource
}

impl BackendSound
{
    pub(crate) fn load(path: &str) -> Result<Self, String>
    {
        let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
        Self::from_data(Cow::Owned(bytes))
    }

    pub(crate) fn from_bytes(bytes: &'static [u8]) -> Result<Self, String>
    {
        Self::from_data(Cow::Borrowed(bytes))
    }

    fn from_data(data: Cow<'static, [u8]>) -> Result<Self, String>
    {
        let decoder = Decoder::new(Cursor::new(data)).map_err(|e| e.to_string())?;
        Ok(Self { source: decoder.buffered() })
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

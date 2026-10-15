// If I ever want to swap out rodio for more low-level, like cpal

use std::borrow::Cow;
use std::fs::File;
use std::io::{BufReader, Cursor, Read, Seek};
use std::num::NonZero;
use std::time::Duration;
use rodio::{Decoder, MixerDeviceSink, Player, Sample};
use rodio::buffer::SamplesBuffer;
use rodio::decoder::DecoderBuilder;
use rodio::source::{Buffered, Source};

type SoundSource = Buffered<Decoder<Cursor<Cow<'static, [u8]>>>>;

pub(crate) enum BackendSound
{
    Buffered(SoundSource), // decoded once
    StreamedFile(String), // decoded while playing
    StreamedBytes(&'static [u8]),
    Samples(SamplesBuffer) // generated, clone is cheap because of arc inside
}

impl BackendSound
{
    pub(crate) fn load_streamed(path: &str) -> Result<Self, String>
    {
        file_decoder(path)?.build().map_err(|e| e.to_string())?;
        Ok(BackendSound::StreamedFile(path.to_string()))
    }

    pub(crate) fn from_samples(samples: impl Iterator<Item = f32>, sample_rate: u32) -> Self
    {
        let samples: Vec<Sample> = samples.map(|sample| sample as Sample).collect();
        let rate = NonZero::new(sample_rate).expect("sample rate can't be 0");
        BackendSound::Samples(SamplesBuffer::new(NonZero::<u16>::MIN, rate, samples)) // 1 channel (mono)
    }

    pub(crate) fn from_bytes_streamed(bytes: &'static [u8]) -> Result<Self, String>
    {
        bytes_decoder(bytes).build().map_err(|e| e.to_string())?;
        Ok(BackendSound::StreamedBytes(bytes))
    }

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
        Ok(BackendSound::Buffered(decoder.buffered()))
    }
}

// byte_len and seekable like rodio's own TryFrom<File>, helps it find the length/seek
fn file_decoder(path: &str) -> Result<DecoderBuilder<BufReader<File>>, String>
{
    let file = File::open(path).map_err(|e| e.to_string())?;
    let len = file.metadata().map_err(|e| e.to_string())?.len();
    Ok(Decoder::builder().with_data(BufReader::new(file)).with_byte_len(len).with_seekable(true))
}

fn bytes_decoder(bytes: &'static [u8]) -> DecoderBuilder<Cursor<&'static [u8]>>
{
    Decoder::builder().with_data(Cursor::new(bytes)).with_byte_len(bytes.len() as u64).with_seekable(true)
}

// looped decoder starts again from the beginning, instead of repeat_infinite, which would keep all samples in memory again
fn append_stream<R: Read + Seek + Send + Sync + 'static>(player: &Player, decoder: DecoderBuilder<R>, looped: bool) -> Result<(), String>
{
    match looped
    {
        true => player.append(decoder.build_looped().map_err(|e| e.to_string())?),
        false => player.append(decoder.build().map_err(|e| e.to_string())?)
    }
    Ok(())
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

    // Can only fail for streamed sounds (e.g. file got deleted after loading)
    pub(crate) fn play(&self, sound: &BackendSound, volume: f32, pitch: f32, looped: bool) -> Result<BackendVoice, String>
    {
        let player = self.new_player(volume, pitch);

        match (sound, looped)
        {
            (BackendSound::Buffered(source), true) => player.append(source.clone().repeat_infinite()),
            (BackendSound::Buffered(source), false) => player.append(source.clone()),
            (BackendSound::Samples(buffer), true) => player.append(buffer.clone().repeat_infinite()),
            (BackendSound::Samples(buffer), false) => player.append(buffer.clone()),
            (BackendSound::StreamedFile(path), _) => append_stream(&player, file_decoder(path)?, looped)?,
            (BackendSound::StreamedBytes(bytes), _) => append_stream(&player, bytes_decoder(bytes), looped)?
        }

        Ok(BackendVoice { player })
    }

    pub(crate) fn play_generator<I: Iterator<Item = f32> + Send + 'static>(&self, samples: I, sample_rate: u32, volume: f32, pitch: f32) -> BackendVoice
    {
        let player = self.new_player(volume, pitch);
        let rate = NonZero::new(sample_rate).expect("sample rate can't be 0");
        player.append(Generator { samples, rate });
        BackendVoice { player }
    }

    fn new_player(&self, volume: f32, pitch: f32) -> Player
    {
        let player = Player::connect_new(self.sink.mixer());
        player.set_volume(volume);
        player.set_speed(pitch);
        player
    }
}

struct Generator<I>
{
    samples: I,
    rate: NonZero<u32>
}

impl<I: Iterator<Item = f32>> Iterator for Generator<I>
{
    type Item = Sample;

    fn next(&mut self) -> Option<Sample>
    {
        self.samples.next().map(|sample| sample as Sample)
    }
}

impl<I: Iterator<Item = f32>> Source for Generator<I>
{
    fn current_span_len(&self) -> Option<usize> { None } // doesn't change
    fn channels(&self) -> NonZero<u16> { NonZero::<u16>::MIN } // mono
    fn sample_rate(&self) -> NonZero<u32> { self.rate }
    fn total_duration(&self) -> Option<Duration> { None } // unknown
}

pub(crate) struct BackendVoice
{
    player: Player
}

impl BackendVoice
{
    pub(crate) fn set_volume(&self, volume: f32) { self.player.set_volume(volume); }
    pub(crate) fn set_pitch(&self, pitch: f32) { self.player.set_speed(pitch); }
    pub(crate) fn pause(&self) { self.player.pause(); }
    pub(crate) fn resume(&self) { self.player.play(); }
    pub(crate) fn is_paused(&self) -> bool { self.player.is_paused() }
    pub(crate) fn is_finished(&self) -> bool { self.player.empty() }
}

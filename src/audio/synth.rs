// Mono Sound, One Sample is a value between -1.0 to 1.0, SAMPLE_RATE of them per second

pub const SAMPLE_RATE: u32 = 44_100;
const RATE: f32 = SAMPLE_RATE as f32;

#[derive(Clone, Default)]
pub struct SoundBuilder
{
    samples: Vec<f32>
}

impl SoundBuilder
{
    pub fn new() -> Self
    {
        Self::default()
    }

    pub fn from_samples(samples: Vec<f32>) -> Self
    {
        Self { samples }
    }

    // On top of whats already there, at start (in seconds), overlapping parts play at the same time
    pub fn add(&mut self, start: f32, samples: &[f32]) -> &mut Self
    {
        let first = seconds_to_samples(start);
        let end = first + samples.len();
        if end > self.samples.len() { self.samples.resize(end, 0.0); }

        for (sample, other) in self.samples[first..end].iter_mut().zip(samples)
        {
            *sample += other;
        }
        self
    }

    // After the end
    pub fn append(&mut self, samples: &[f32]) -> &mut Self
    {
        self.samples.extend_from_slice(samples);
        self
    }

    pub fn duration(&self) -> f32
    {
        self.samples.len() as f32 / RATE
    }

    pub fn samples(&self) -> &[f32]
    {
        &self.samples
    }

    pub fn samples_mut(&mut self) -> &mut Vec<f32>
    {
        &mut self.samples
    }

    pub fn into_samples(self) -> Vec<f32>
    {
        self.samples
    }
}

pub fn seconds_to_samples(seconds: f32) -> usize
{
    (seconds.max(0.0) * RATE).round() as usize
}

// 60 = C4, 261.63Hz
// +12 is one octave up, +1 one halftone
pub fn note(number: f32) -> f32
{
    440.0 * 2.0_f32.powf((number - 69.0) / 12.0)
}

// saves as .wav (16 bit, mono, SAMPLE_RATE)
// Header is simpler, no need for a crate (ai generated header)
pub fn save_wav(path: &str, samples: &[f32]) -> std::io::Result<()>
{
    let peak = samples.iter().fold(0.0_f32, |peak, sample| peak.max(sample.abs()));
    let scale = if peak > 1.0 { 1.0 / peak } else { 1.0 };
    let data_len = (samples.len() * 2) as u32; // 2 bytes per sample

    // header: what kind of file, then the format, then the samples
    let mut bytes = Vec::with_capacity(44 + data_len as usize);
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + data_len).to_le_bytes()); // size of everything after this
    bytes.extend_from_slice(b"WAVE");

    bytes.extend_from_slice(b"fmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes()); // size of this part
    bytes.extend_from_slice(&1u16.to_le_bytes()); // 1 = plain samples (PCM), no compression
    bytes.extend_from_slice(&1u16.to_le_bytes()); // channels
    bytes.extend_from_slice(&SAMPLE_RATE.to_le_bytes());
    bytes.extend_from_slice(&(SAMPLE_RATE * 2).to_le_bytes()); // bytes per second
    bytes.extend_from_slice(&2u16.to_le_bytes()); // bytes per sample (all channels)
    bytes.extend_from_slice(&16u16.to_le_bytes()); // bits per sample

    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data_len.to_le_bytes());
    for sample in samples
    {
        let value = (sample * scale).clamp(-1.0, 1.0) * i16::MAX as f32;
        bytes.extend_from_slice(&(value as i16).to_le_bytes());
    }

    std::fs::write(path, bytes)
}

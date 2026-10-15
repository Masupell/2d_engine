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

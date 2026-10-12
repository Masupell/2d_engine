use crate::{handles::TextureId, tools::Repeat, Draw};

#[derive(Clone, Debug)]
pub struct Animation
{
    pub texture: TextureId,
    frames: Vec<[f32; 4]>,
    durations: Vec<f32>,
    pub repeat: Repeat
}

impl Animation
{
    // Grid Spritesheet
    // Animation::grid(texture, 8, 4, 8..16, 12.0, Repeat::Loop) is the second row of a 8x4 spritesheet
    // running at 12fps
    // (columns is vertical, but counted left to right, same for rows, just horizontal and counted down,
    // so, columns is x and rows y)
    pub fn grid(texture: TextureId, columns: u32, rows: u32, frames: std::ops::Range<u32>, fps: f32, repeat: Repeat) -> Self
    {
        let (w, h) = (1.0 / columns as f32, 1.0 / rows as f32);

        let frames: Vec<[f32; 4]> = frames.map(|index|
        {
            let (column, row) = (index % columns, index / columns);
            [column as f32 * w, row as f32 * h, w, h]
        }).collect();

        Self::from_uv_rects(texture, frames, fps, repeat)
    }

    // For arbitrary uv-rects
    pub fn from_uv_rects(texture: TextureId, frames: Vec<[f32; 4]>, fps: f32, repeat: Repeat) -> Self
    {
        debug_assert!(!frames.is_empty(), "animation without frames");
        let frame_time = 1.0 / fps.max(0.0001);
        let durations = vec![frame_time; frames.len()];
        Self { texture, frames, durations, repeat }
    }

    pub fn with_durations(mut self, durations: Vec<f32>) -> Self
    {
        debug_assert_eq!(durations.len(), self.frames.len(), "vector needs the same length as the frames");
        self.durations = durations;
        self
    }

    pub fn frame_count(&self) -> usize
    {
        self.frames.len()
    }

    pub fn total_duration(&self) -> f32
    {
        self.durations.iter().sum()
    }

    pub fn uv_rect(&self, frame: usize) -> [f32; 4]
    {
        self.frames[frame.min(self.frames.len() - 1)]
    }

    pub fn draw(&self, player: &AnimationPlayer, pos: (f32, f32), size: (f32, f32), rotation: f32) -> Draw
    {
        Draw { uv_rect: self.uv_rect(player.frame), ..Draw::sprite(self.texture, pos, size, rotation) }
    }
}

// Struct that advances any animation
#[derive(Copy, Clone, Debug, Default)]
pub struct AnimationPlayer
{
    frame: usize,
    elapsed: f32,
    backwards: bool,
    finished: bool,
    pub speed: f32
}

impl AnimationPlayer
{
    pub fn new() -> Self
    {
        Self { speed: 1.0, ..Default::default() }
    }

    // Repeat::Once Animation, returns true when finished
    // When changing animation, set frame to 0, so it starts at 0 again (with restart best if backwards was ever true)
    pub fn update(&mut self, animation: &Animation, dt: f64) -> bool
    {
        if self.finished || animation.frames.is_empty() { return false; }

        self.elapsed += dt as f32 * self.speed;
        let last = animation.frames.len() - 1;

        while self.elapsed >= animation.durations[self.frame]
        {
            self.elapsed -= animation.durations[self.frame];

            match animation.repeat
            {
                Repeat::Once if self.frame == last =>
                {
                    self.elapsed = 0.0;
                    self.finished = true;
                    return true;
                }
                Repeat::Once => self.frame += 1,
                Repeat::Loop => self.frame = if self.frame == last { 0 } else { self.frame + 1 },
                Repeat::PingPong =>
                {
                    if last == 0 { continue; }
                    if self.backwards && self.frame == 0 { self.backwards = false; }
                    else if !self.backwards && self.frame == last { self.backwards = true; }
                    self.frame = if self.backwards { self.frame - 1 } else { self.frame + 1 };
                }
            }
        }
        false
    }

    pub fn frame(&self) -> usize
    {
        self.frame
    }

    pub fn is_finished(&self) -> bool
    {
        self.finished
    }

    pub fn restart(&mut self)
    {
        let speed = self.speed;
        *self = Self { speed, ..Default::default() };
    }

    pub fn set_frame(&mut self, frame: usize)
    {
        self.frame = frame;
        self.elapsed = 0.0;
        self.finished = false;
    }
}

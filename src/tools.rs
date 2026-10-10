use std::f32::consts::{PI, TAU};

use crate::Context;


pub trait Lerp: Copy { fn lerp(self, other: Self, t: f32) -> Self; }
impl Lerp for f32 { fn lerp(self, other: Self, t: f32) -> Self { self + (other - self) * t } }
impl Lerp for (f32, f32) { fn lerp(self, other: Self, t: f32) -> Self { (self.0.lerp(other.0, t), self.1.lerp(other.1, t)) } }

/// like pos: Interpolated<(f32, f32)>, only things that are drawn, so velocity: f32 would stay the same
///
/// For interpolating things automatically, to make it look smoother for differnt fps then physcis_update ticks (60)
pub struct Interpolated<T: Lerp> { prev: T, curr: T, step: u64 }

impl<T: Lerp> Interpolated<T>
{
    pub fn new(v: T) -> Self { Self { prev: v, curr: v, step: 0 } }

    pub fn set(&mut self, v: T, ctx: &Context)
    {
        if self.step != ctx.physics_step() { self.prev = self.curr; self.step = ctx.physics_step(); }
        self.curr = v;
    }

    pub fn teleport(&mut self, v: T, ctx: &Context) { self.prev = v; self.curr = v; self.step = ctx.physics_step(); }

    /// For update
    pub fn get(&self) -> T { self.curr }

    /// For rendering
    pub fn visual(&self, ctx: &Context) -> T
    {
        if self.step != ctx.physics_step() { return self.curr; }
        self.prev.lerp(self.curr, ctx.alpha())
    }
}

/// returns 0..1 between a and b
pub fn inverse_lerp(a: f32, b: f32, value: f32) -> f32
{
    if a == b { 0.0 } else { (value - a) / (b - a) }
}

/// Maps from in_a..in_b to out_a..out_b
pub fn remap(value: f32, in_a: f32, in_b: f32, out_a: f32, out_b: f32) -> f32
{
    out_a + (out_b - out_a) * inverse_lerp(in_a, in_b, value)
}

/// regular smoothstep function like in shader
pub fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32
{
    let t = inverse_lerp(edge0, edge1, x).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Moves towards target by max_delta, doesn't overshoot
pub fn move_toward(current: f32, target: f32, max_delta: f32) -> f32
{
    let diff = target - current;
    if diff.abs() <= max_delta { target } else { current + max_delta.copysign(diff) }
}

/// Important for lerping outside of physics_update (so each frame, for graphics)
/// Lerp is fine if frame-rate stays the same, if not:
///
/// (Called Each Frame) Lerp does `f32.lerp(target, 0.1)` -> move 10% of the remaining distance per frame
///
/// 60fps: after one second remaining distance = 0.9^60 = 0.18%
///
/// 240fps: after one second remaining distance = 0.9^240 = 0.00000000001%
///
/// Damping makes it frame rate independent, after one second:
/// speed 0: 1 - e^0 = 0 (no movement);
/// speed 1: e^(-1): 0.37, 37% of distance remains;
/// speed 2: e^(-2): 0.14, 14% of distance remains
pub fn damp<T: Lerp>(current: T, target: T, speed: f32, dt: f64) -> T
{
    current.lerp(target, 1.0 - (-speed * dt as f32).exp())
}

/// Wraps an angle into -PI..PI
pub fn wrap_angle(angle: f32) -> f32
{
    (angle + PI).rem_euclid(TAU) - PI
}

/// Lerps between angles, goes the shorter way
pub fn lerp_angle(a: f32, b: f32, t: f32) -> f32
{
    a + wrap_angle(b - a) * t
}




/// Returns a 0..1 progress value
///
/// In starts slow and speeds up; Out starts fast and slows down
///
/// Sine, Quad, Cubic, Expo is the strength of the curve
///
/// Back: Pulls slightly back before moving for In, overshots and returns for Out
///
/// Elastic overshoots and wobbles; Bounce hits target and bounces
#[derive(Copy, Clone, PartialEq, Debug, Default)]
pub enum Ease
{
    #[default]
    Linear,
    InQuad, OutQuad, InOutQuad,
    InCubic, OutCubic, InOutCubic,
    InSine, OutSine, InOutSine,
    InExpo, OutExpo, InOutExpo,
    InBack, OutBack, InOutBack,
    OutElastic,
    OutBounce
}

impl Ease
{
    pub fn apply(self, t: f32) -> f32
    {
        let t = t.clamp(0.0, 1.0);
        const BACK: f32 = 1.70158;
        const BACK_INOUT: f32 = BACK * 1.525;

        match self
        {
            Ease::Linear => t,

            Ease::InQuad => t * t,
            Ease::OutQuad => 1.0 - (1.0 - t) * (1.0 - t),
            Ease::InOutQuad => if t < 0.5 { 2.0 * t * t } else { 1.0 - (-2.0 * t + 2.0).powi(2) / 2.0 },

            Ease::InCubic => t * t * t,
            Ease::OutCubic => 1.0 - (1.0 - t).powi(3),
            Ease::InOutCubic => if t < 0.5 { 4.0 * t * t * t } else { 1.0 - (-2.0 * t + 2.0).powi(3) / 2.0 },

            Ease::InSine => 1.0 - (t * PI / 2.0).cos(),
            Ease::OutSine => (t * PI / 2.0).sin(),
            Ease::InOutSine => -((PI * t).cos() - 1.0) / 2.0,

            Ease::InExpo => if t == 0.0 { 0.0 } else { 2f32.powf(10.0 * t - 10.0) },
            Ease::OutExpo => if t == 1.0 { 1.0 } else { 1.0 - 2f32.powf(-10.0 * t) },
            Ease::InOutExpo => match t
            {
                0.0 => 0.0,
                1.0 => 1.0,
                _ if t < 0.5 => 2f32.powf(20.0 * t - 10.0) / 2.0,
                _ => (2.0 - 2f32.powf(-20.0 * t + 10.0)) / 2.0
            },

            Ease::InBack => (BACK + 1.0) * t * t * t - BACK * t * t,
            Ease::OutBack => 1.0 + (BACK + 1.0) * (t - 1.0).powi(3) + BACK * (t - 1.0).powi(2),
            Ease::InOutBack => if t < 0.5
            {
                ((2.0 * t).powi(2) * ((BACK_INOUT + 1.0) * 2.0 * t - BACK_INOUT)) / 2.0
            }
            else
            {
                ((2.0 * t - 2.0).powi(2) * ((BACK_INOUT + 1.0) * (t * 2.0 - 2.0) + BACK_INOUT) + 2.0) / 2.0
            },

            Ease::OutElastic => match t
            {
                0.0 => 0.0,
                1.0 => 1.0,
                _ => 2f32.powf(-10.0 * t) * ((t * 10.0 - 0.75) * (TAU / 3.0)).sin() + 1.0
            },

            Ease::OutBounce =>
            {
                const N: f32 = 7.5625;
                const D: f32 = 2.75;
                if t < 1.0 / D { N * t * t }
                else if t < 2.0 / D { let t = t - 1.5 / D; N * t * t + 0.75 }
                else if t < 2.5 / D { let t = t - 2.25 / D; N * t * t + 0.9375 }
                else { let t = t - 2.625 / D; N * t * t + 0.984375 }
            }
        }
    }
}



#[derive(Copy, Clone, PartialEq, Debug, Default)]
pub enum Repeat
{
    #[default]
    Once,
    Loop,
    PingPong
}

/// Animates a value over a duration
/// ```
/// let mut tween = Tween::new(0.0, 1.0, 0.5, Ease::InOutBack).repeating(Repeat::PingPong);
/// // Each Frame:
/// tween_value = tween.update(dt);
/// ```
#[derive(Copy, Clone, Debug)]
pub struct Tween<T: Lerp>
{
    pub from: T,
    pub to: T,
    pub duration: f32,
    pub ease: Ease,
    pub repeat: Repeat,
    elapsed: f32
}

impl<T: Lerp> Tween<T>
{
    pub fn new(from: T, to: T, duration: f32, ease: Ease) -> Self
    {
        Self { from, to, duration: duration.max(0.0001), ease, repeat: Repeat::Once, elapsed: 0.0 }
    }

    pub fn repeating(self, repeat: Repeat) -> Self
    {
        Self { repeat, ..self }
    }

    pub fn update(&mut self, dt: f64) -> T
    {
        self.elapsed += dt as f32;

        // Once: stop at the end. Loop/PingPong: keep elapsed small so f32 precision stays fine forever
        let cycle = match self.repeat
        {
            Repeat::Once => { self.elapsed = self.elapsed.min(self.duration); 0.0 }
            Repeat::Loop => self.duration,
            Repeat::PingPong => self.duration * 2.0
        };
        if cycle > 0.0 { self.elapsed = self.elapsed.rem_euclid(cycle); }

        self.value()
    }

    pub fn value(&self) -> T
    {
        self.from.lerp(self.to, self.ease.apply(self.raw_progress()))
    }

    // 0..1 progress before easing
    pub fn raw_progress(&self) -> f32
    {
        let t = self.elapsed / self.duration;
        match self.repeat
        {
            Repeat::PingPong if t > 1.0 => 2.0 - t,
            _ => t.min(1.0)
        }
    }

    pub fn is_finished(&self) -> bool
    {
        self.repeat == Repeat::Once && self.elapsed >= self.duration
    }

    pub fn restart(&mut self)
    {
        self.elapsed = 0.0;
    }

    /// New Target, starts from wherever it is now
    pub fn retarget(&mut self, to: T)
    {
        self.from = self.value();
        self.to = to;
        self.elapsed = 0.0;
    }
}





/// Counts Time, needs to be updated each frame with tick(dt);
/// tick returns true when duration is up (repeating counts from 0 and repeats)
///
/// Timer::once(10.0) or Timer::repeating(10.0)
#[derive(Copy, Clone, Debug)]
pub struct Timer
{
    pub duration: f32,
    pub repeating: bool,
    elapsed: f32,
    finished: bool
}

impl Timer
{
    pub fn once(duration: f32) -> Self
    {
        Self { duration: duration.max(0.0), repeating: false, elapsed: 0.0, finished: false }
    }

    pub fn repeating(duration: f32) -> Self
    {
        Self { duration: duration.max(0.0001), repeating: true, elapsed: 0.0, finished: false }
    }

    pub fn tick(&mut self, dt: f64) -> bool
    {
        if self.finished && !self.repeating { return false; }

        self.elapsed += dt as f32;
        if self.elapsed < self.duration { return false; }

        if self.repeating
        {
            self.elapsed -= self.duration;
            self.elapsed = self.elapsed.min(self.duration);
        }
        else
        {
            self.elapsed = self.duration;
            self.finished = true;
        }
        true
    }

    pub fn reset(&mut self)
    {
        self.elapsed = 0.0;
        self.finished = false;
    }

    pub fn is_finished(&self) -> bool { self.finished }
    pub fn progress(&self) -> f32 { (self.elapsed / self.duration).clamp(0.0, 1.0) }
    pub fn remaining(&self) -> f32 { (self.duration - self.elapsed).max(0.0) }
}







/// PCG32 based small random number generator, same one as Godots
///
/// Own Rng, not using rand crate, of course not as sophisticated, but for a game this is fine, and needs less dependencies, possible renaiming problems
/// or update seed changes
#[derive(Clone, Debug)]
pub struct Rng
{
    state: u64,
    increment: u64
}

impl Rng
{
    const MULTIPLIER: u64 = 6364136223846793005;

    pub fn with_seed(seed: u64) -> Self
    {
        let mut mix = seed;
        let mut next = ||
        {
            mix = mix.wrapping_add(0x9E3779B97F4A7C15);
            let mut z = mix;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
            z ^ (z >> 31)
        };

        let mut rng = Self { state: next(), increment: next() | 1 }; // increment has to be odd
        rng.next_u32();
        rng
    }

    pub fn new() -> Self
    {
        let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(0x1234_5678);
        Self::with_seed(nanos)
    }

    pub fn next_u32(&mut self) -> u32
    {
        let old = self.state;
        self.state = old.wrapping_mul(Self::MULTIPLIER).wrapping_add(self.increment);
        let xorshifted = (((old >> 18) ^ old) >> 27) as u32;
        let rotation = (old >> 59) as u32;
        xorshifted.rotate_right(rotation)
    }

    pub fn next_u64(&mut self) -> u64
    {
        (self.next_u32() as u64) << 32 | self.next_u32() as u64
    }

    /// 0.0 <= x < 1.0
    pub fn f32(&mut self) -> f32
    {
        (self.next_u32() >> 8) as f32 * (1.0 / (1u32 << 24) as f32)
    }

    /// includes 1, but not 0.0 (0.0 < x <= 1.0)
    pub fn f32_nonzero(&mut self) -> f32
    {
        1.0 - self.f32()
    }

    /// 0.0 <= x <= 1.0
    pub fn f32_inclusive(&mut self) -> f32
    {
        (self.next_u32() >> 8) as f32 / ((1u32 << 24) - 1) as f32
    }

    /// min <= x < max
    pub fn range_f32(&mut self, min: f32, max: f32) -> f32
    {
        min + (max - min) * self.f32()
    }

    /// min <= x < max
    pub fn range_i32(&mut self, min: i32, max: i32) -> i32
    {
        if max <= min { return min; }
        let span = (max as i64 - min as i64) as u64;
        min + ((self.next_u32() as u64 * span) >> 32) as i32
    }

    /// 0 <= x < len, for indexing
    pub fn index(&mut self, len: usize) -> usize
    {
        if len == 0 { return 0; }
        ((self.next_u32() as u64 * len as u64) >> 32) as usize
    }

    pub fn bool(&mut self) -> bool
    {
        self.next_u32() & 1 == 1
    }

    /// 0.5 is 50% chance of it happening
    pub fn chance(&mut self, probability: f32) -> bool
    {
        self.f32() < probability
    }

    /// -1.0 or 1.0
    pub fn sign(&mut self) -> f32
    {
        if self.bool() { 1.0 } else { -1.0 }
    }

    /// random angle in radians, 0..TAU
    pub fn angle(&mut self) -> f32
    {
        self.f32() * TAU
    }

    /// random direction with length 1
    pub fn unit_vector(&mut self) -> (f32, f32)
    {
        let (sin, cos) = self.angle().sin_cos();
        (cos, sin)
    }

    /// random point in circle
    pub fn in_circle(&mut self, radius: f32) -> (f32, f32)
    {
        let r = radius * self.f32().sqrt();
        let (x, y) = self.unit_vector();
        (x * r, y * r)
    }

    pub fn pick<'a, T>(&mut self, items: &'a [T]) -> Option<&'a T>
    {
        if items.is_empty() { None } else { Some(&items[self.index(items.len())]) }
    }

    /// Picks an index based on its weight
    /// compared to the total (does not have to add up to 100, ratio is important).
    /// Example with loot table:
    /// ```
    /// let rarity = ["common", "rare", "legendary"];
    /// let index = rng.weighted_index(&[70.0, 25.0, 5.0]).unwrap(); // 70% / 25% / 5%
    /// ```
    pub fn weighted_index(&mut self, weights: &[f32]) -> Option<usize>
    {
        let total: f32 = weights.iter().sum();
        if total <= 0.0 { return None; }

        let mut roll = self.f32() * total;
        for (index, &weight) in weights.iter().enumerate()
        {
            if roll < weight { return Some(index); }
            roll -= weight;
        }
        Some(weights.len() - 1)
    }

    /// Fisher-Yates   Knuth-Shuffle
    ///
    /// Shuffles array
    pub fn shuffle<T>(&mut self, items: &mut [T])
    {
        for i in (1..items.len()).rev()
        {
            let j = self.index(i + 1);
            items.swap(i, j);
        }
    }
}

impl Default for Rng
{
    fn default() -> Self { Self::new() }
}

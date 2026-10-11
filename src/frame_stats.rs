use std::{cell::Cell, fmt::Write, time::{Duration, Instant}};

use crate::{Draw, RenderContext, Renderer, Text, threads::ThreadPool};

const REFRESH: Duration = Duration::from_millis(500);

#[derive(Copy, Clone)]
pub(crate) enum Section
{
    Update, // game.update
    Physics, // all physics_updates
    Render, // game.render + sorting/culling/upload + encoding + submit on CPU
    Wait // mostly waiting for vsync (or gpu)
}
impl Section { const COUNT: usize = 4; }


#[derive(Default)]
pub(crate) struct RenderStats
{
    pub(crate) commands: u32, // all draw_calls includes overlay itself
    pub(crate) culled: u32,
    pub(crate) instances: u32, // all actually drawn things
    pub(crate) passes: u32,
    pub(crate) draw_calls: Cell<u32>,
    pub(crate) pipeline_switches: Cell<u32>,
    pub(crate) texture_binds: Cell<u32>
}

impl RenderStats
{
    pub(crate) fn reset(&mut self)
    {
        *self = Self::default();
    }

    pub(crate) fn bump(counter: &Cell<u32>)
    {
        if cfg!(debug_assertions) { counter.set(counter.get() + 1); }
    }
}

pub(crate) struct FrameStats
{
    enabled: bool,
    last_frame: Instant,

    window_start: Instant,
    frames: u32,
    frame_sum: f64,
    frame_min: f64,
    frame_max: f64,
    sections: [f64; Section::COUNT], // seconds
    physics_steps: u32,

    text: String // text overlay
}

impl FrameStats
{
    pub(crate) fn new() -> Self
    {
        let now = Instant::now();
        Self
        {
            enabled: false,
            last_frame: now,
            window_start: now,
            frames: 0,
            frame_sum: 0.0,
            frame_min: f64::MAX,
            frame_max: 0.0,
            sections: [0.0; Section::COUNT],
            physics_steps: 0,
            text: String::new()
        }
    }

    pub(crate) fn enabled(&self) -> bool
    {
        cfg!(debug_assertions) && self.enabled
    }

    pub(crate) fn set_enabled(&mut self, enabled: bool)
    {
        self.enabled = enabled;
        let now = Instant::now();
        self.last_frame = now;
        self.reset_window(now);
        self.text.clear();
    }

    pub(crate) fn start(&self) -> Option<Instant>
    {
        self.enabled().then(Instant::now)
    }

    pub(crate) fn record(&mut self, section: Section, start: Option<Instant>)
    {
        if let Some(start) = start
        {
            self.sections[section as usize] += start.elapsed().as_secs_f64();
        }
    }

    pub(crate) fn count_physics_step(&mut self)
    {
        if self.enabled()
        {
            self.physics_steps += 1;
        }
    }

    pub(crate) fn end_frame(&mut self, renderer: &Renderer, threads: &ThreadPool)
    {
        if !self.enabled() { return; }

        let now = Instant::now();
        let frame = (now - self.last_frame).as_secs_f64();
        self.last_frame = now;

        self.frames += 1;
        self.frame_sum += frame;
        self.frame_min = self.frame_min.min(frame);
        self.frame_max = self.frame_max.max(frame);

        if now - self.window_start >= REFRESH
        {
            self.build_text(renderer, threads);
            self.reset_window(now);
        }
    }

    pub(crate) fn text(&self) -> &str
    {
        &self.text
    }

    fn reset_window(&mut self, now: Instant)
    {
        self.window_start = now;
        self.frames = 0;
        self.frame_sum = 0.0;
        self.frame_min = f64::MAX;
        self.frame_max = 0.0;
        self.sections = [0.0; Section::COUNT];
        self.physics_steps = 0;
    }

    fn build_text(&mut self, renderer: &Renderer, threads: &ThreadPool)
    {
        if self.frames == 0 { return; }

        let frames = self.frames as f64;
        let ms = |seconds: f64| seconds * 1000.0;
        let avg = |seconds: f64| seconds * 1000.0 / frames;

        let frame_avg = self.frame_sum / frames;
        let fps = if frame_avg > 0.0 { 1.0 / frame_avg } else { 0.0 };
        let steps = self.physics_steps as f64 / frames;

        let r = &renderer.stats;
        let s = self.sections;

        self.text.clear();

        let pool = threads.status();
        let texture_mb = renderer.texture_memory_estimate() as f64 / (1024.0 * 1024.0);
        let _ = write!(self.text,
            "{fps:.0} FPS   frame {:.2} ms  (min {:.2} / max {:.2})\n\
             update {:.2}   physics {:.2} ({steps:.1} steps)   render {:.2}   wait {:.2} ms\n\
             commands {}  culled {}  instances {}\n\
             draw calls {}  passes {}  pipelines {}  texture binds {}\n\
             textures: {} loaded (~{texture_mb:.1} MB)  {} loading\n\
             thread: {} busy  {} alive / {} max  {} queued",
            ms(frame_avg), ms(self.frame_min), ms(self.frame_max),
            avg(s[Section::Update as usize]), avg(s[Section::Physics as usize]), avg(s[Section::Render as usize]), avg(s[Section::Wait as usize]),
            r.commands, r.culled, r.instances,
            r.draw_calls.get(), r.passes, r.pipeline_switches.get(), r.texture_binds.get(),
            renderer.texture_count(), renderer.textures_loading(),
            pool.busy, pool.alive, pool.max, pool.queued
        );
    }
}

pub(crate) fn draw_overlay(ctx: &mut RenderContext)
{
    if !ctx.context.stats.enabled() { return; }

    let text = ctx.context.stats.text();
    if text.is_empty() { return; }

    let renderer = &mut *ctx.graphics.renderer;

    const HEIGHT: f32 = 16.0;
    const MARGIN: f32 = 8.0;
    const PADDING: f32 = 6.0;

    let (left, top, _, _) = renderer.ui_bounds();
    let (width, height) = renderer.text_size(text, HEIGHT);

    let box_size = (width + PADDING * 2.0, height + PADDING * 2.0);
    let box_center = (left + MARGIN + box_size.0 * 0.5, top + MARGIN + box_size.1 * 0.5);

    renderer.draw(Draw { z: u32::MAX - 1, ..Draw::rect(box_center, box_size, 0.0, [0.0, 0.0, 0.0, 0.65]).ui() });
    renderer.draw_text(Text { z: u32::MAX, ..Text::new(text, (left + MARGIN + PADDING, top + MARGIN + PADDING), HEIGHT, [1.0, 1.0, 1.0, 1.0]).ui() });
}

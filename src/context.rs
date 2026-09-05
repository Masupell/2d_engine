use crate::{Input, Renderer, UniformValue, texture::FilterMode, utility::{PipeLineType, UniformType}};


pub struct Context // General Settings, will hold AssetManager in the future and things like that I think
{
    screen_size: (u32, u32),
    vsync: bool, // Maybe put those two, with other things into a seperate struct later
    fullscreen: bool,
    fixed_dt: f64, // fixed dt
    fps: u32,
    pub(crate) pending_actions: Vec<ContextAction>,
    pub(crate) post_process_pipelines: Vec<usize>
}

impl Context
{
    pub(crate) fn new(screen_size: (u32, u32), vsync: bool, fullscreen: bool) -> Self
    {
        Self
        {
            screen_size,
            vsync,
            fullscreen,
            fixed_dt: 1.0 / 60.0,
            fps: 0,
            pending_actions: Vec::new(),
            post_process_pipelines: Vec::new()
        }
    }

    // pub fn set_screen_size(&mut self, screen_size: (u32, u32)) // Should not be accessed by game, but also needs to be public because it needs to be accessed by renderloop
    // {
    //     self.screen_size = screen_size;
    // }

    pub fn screen_size(&self) -> (u32, u32)
    {
        self.screen_size
    }

    pub fn toggle_fullscreen(&mut self)
    {
        self.fullscreen = !self.fullscreen;
        self.pending_actions.push(ContextAction::ToggleFullscreen(self.fullscreen));
    }

    pub fn set_fullscreen(&mut self, fullscreen: bool)
    {
        self.fullscreen = fullscreen;
        self.pending_actions.push(ContextAction::ToggleFullscreen(fullscreen));
    }

    pub fn toggle_vsync(&mut self)
    {
        self.vsync = !self.vsync;
        self.pending_actions.push(ContextAction::SetVSync(self.vsync));
    }

    pub fn set_vsync(&mut self, vsync: bool)
    {
        self.vsync = vsync;
        self.pending_actions.push(ContextAction::SetVSync(vsync));
    }

    pub fn fixed_dt(&self) -> f64
    {
        self.fixed_dt
    }

    pub fn set_fixed_dt(&mut self, fixed_dt: f64)
    {
        self.fixed_dt = fixed_dt;
    }

    pub fn fps(&self) -> u32
    {
        self.fps
    }

    pub(crate) fn set_fps(&mut self, fps: u32)
    {
        self.fps = fps
    }

    pub fn set_title(&mut self, title: impl Into<String>)
    {
        self.pending_actions.push(ContextAction::SetTitle(title.into()));
    }

    // For only one
    pub fn set_post_process_pipeline(&mut self, id: usize)
    {
        self.post_process_pipelines = vec![id];
    }

    pub fn add_post_process_pipeline(&mut self, id: usize)
    {
        self.post_process_pipelines.push(id);
    }

    pub fn clear_post_process_pipelines(&mut self)
    {
        self.post_process_pipelines.clear();
    }
}

pub enum ContextAction
{
    ToggleFullscreen(bool),
    SetVSync(bool),
    SetTitle(String)
}

pub struct UpdateContext<'a>
{
    pub input: &'a mut Input,
    pub dt: f64,
    pub context: &'a mut Context, // For the general stuff
    pub graphics: GraphicsContext<'a>
}

impl<'a> UpdateContext<'a>
{
    pub(crate) fn new(input: &'a mut Input, context: &'a mut Context, dt: f64, renderer: &'a mut Renderer, device: &'a wgpu::Device, queue: &'a wgpu::Queue, config: &'a wgpu::SurfaceConfiguration) -> Self
    {
        Self
        {
            input,
            dt,
            context,
            graphics: GraphicsContext::new(renderer, device, queue, config)
        }
    }
}

pub struct RenderContext<'a> // Seperate from UpdateContext because of borrowing and stuff
{
    pub context: &'a mut Context,
    pub graphics: GraphicsContext<'a>
}

impl<'a> RenderContext<'a>
{
    pub(crate) fn new(context: &'a mut Context, renderer: &'a mut Renderer, device: &'a wgpu::Device, queue: &'a wgpu::Queue, config: &'a wgpu::SurfaceConfiguration) -> Self
    {
        Self
        {
            context,
            graphics: GraphicsContext::new(renderer, device, queue, config)
        }
    }
}

pub struct GraphicsContext<'a> // Can load now, at any time, but still not perfect
{
    pub renderer: &'a mut Renderer, //public for now, later should add methods
    pub device: &'a wgpu::Device,
    pub queue: &'a wgpu::Queue,
    pub(crate) config: &'a wgpu::SurfaceConfiguration,
}

impl<'a> GraphicsContext<'a>
{
    pub(crate) fn new(renderer: &'a mut Renderer, device: &'a wgpu::Device, queue: &'a wgpu::Queue, config: &'a wgpu::SurfaceConfiguration) -> Self
    {
        Self
        {
            renderer,
            device,
            queue,
            config
        }
    }

    pub fn load_texture(&mut self, path: &str, mag_filter: FilterMode, min_filter: FilterMode) -> usize
    {
        self.renderer.load_texture(self.device, self.queue, path, mag_filter, min_filter)
    }

    pub fn load_char(&mut self, char: char) -> Option<usize>
    {
        self.renderer.load_char(self.device, self.queue, char)
    }

    pub fn load_text(&mut self, text: &str, size: f32) -> Option<usize>
    {
        self.renderer.load_text(self.device, self.queue, text, size)
    }

    pub fn load_shader(&mut self, fragment_path: Option<&str>, vertex_path: Option<&str>, pipeline_type: PipeLineType) -> usize
    {
        self.renderer.add_pipeline(self.device, self.config, fragment_path, vertex_path, pipeline_type)
    }

    pub fn load_shader_with_uniform(&mut self, fragment_path: Option<&str>, vertex_path: Option<&str>, pipeline_type: PipeLineType, uniforms: &[(&str, UniformType)]) -> usize
    {
        self.renderer.add_pipeline_with_uniforms(self.device, self.queue, self.config, fragment_path, vertex_path, pipeline_type, uniforms)
    }

    pub fn set_uniform(&mut self, name: &str, value: UniformValue)
    {
        self.renderer.set_uniform(self.queue, name, value);
    }

    pub fn set_camera_pos(&mut self, position: (f32, f32))
    {
        self.renderer.set_camera_pos(position, self.queue);
    }

    pub fn set_clear_color(&mut self, color: [f64; 4])
    {
        self.renderer.set_clear_color(color);
    }

    pub fn set_default_font(&mut self, font_path: &str, size: f32)
    {
        self.renderer.set_default_font(self.device, self.queue, font_path, size);
    }

    pub fn add_font(&mut self, font_path: &str, size: f32) -> usize
    {
        self.renderer.add_font(self.device, self.queue, font_path, size)
    }
}

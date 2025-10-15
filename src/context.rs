use crate::{Input, Renderer};


pub struct Context // General Settings, will hold AssetManager in the future and things like that I think
{
    screen_size: (u32, u32),
    vsync: bool, // Maybe put those two, with other things into a seperate struct later
    fullscreen: bool,
    pub(crate) pending_actions: Vec<ContextAction>
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
            pending_actions: Vec::new()
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
}

pub enum ContextAction
{
    ToggleFullscreen(bool),
    SetVSync(bool)
}

pub struct UpdateContext<'a>
{
    pub input: &'a Input,
    pub dt: f64,
    pub context: &'a mut Context // For the general stuff
}

impl<'a> UpdateContext<'a>
{
    pub(crate) fn new(input: &'a Input, context: &'a mut Context, dt: f64) -> Self
    {
        Self
        {
            input,
            dt,
            context
        }
    }
}

pub struct RenderContext<'a> // Seperate from UpdateContext because of borrowing and stuff
{
    pub renderer: &'a mut Renderer,
    pub context: &'a mut Context
}

impl<'a> RenderContext<'a>
{
    pub(crate) fn new(renderer: &'a mut Renderer, context: &'a mut Context) -> Self
    {
        Self
        {
            renderer,
            context
        }
    }
}


pub trait Loader // Will be replaced by Asset Manager in the Future, or rather, maybe this loader will stay, but will be implemented for it
{
    fn load_texture(&mut self, path: &str) -> usize;
    fn load_char(&mut self, char: char) -> Option<usize>;
    fn load_text(&mut self, text: &str, size: f32) -> Option<usize>;
    fn load_shader(&mut self, fragment_path: Option<&str>, vertex_path: Option<&str>) -> usize; // Returns pipeline number
}

pub struct LoadingContext<'a> 
{
    renderer: &'a mut Renderer,
    device: &'a wgpu::Device,
    queue: &'a wgpu::Queue,
    config: &'a wgpu::SurfaceConfiguration
}

impl<'a> LoadingContext<'a>
{
    pub(crate) fn new(renderer: &'a mut Renderer, device: &'a wgpu::Device, queue: &'a wgpu::Queue, config: &'a wgpu::SurfaceConfiguration) -> Self
    {
        Self { renderer, device, queue, config }
    }
}

impl<'a> Loader for LoadingContext<'a> // Will be replaced by Asset Manager
{
    fn load_texture(&mut self, path: &str) -> usize 
    {
        self.renderer.load_texture(self.device, self.queue, path)
    }

    fn load_char(&mut self, char: char) -> Option<usize>
    {
        self.renderer.load_char(self.device, self.queue, char)
    }

    fn load_text(&mut self, text: &str, size: f32) -> Option<usize>
    {
        self.renderer.load_text(self.device, self.queue, text, size)
    }
    
    fn load_shader(&mut self, fragment_path: Option<&str>, vertex_path: Option<&str>) -> usize 
    {
        self.renderer.add_pipeline(self.device, self.config, fragment_path, vertex_path)
    }
}
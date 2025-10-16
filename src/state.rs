use std::iter;
use winit::{event::*,window::Window};

use crate::{asset_manager::{AssetManager, TextureHandle}, renderer::Renderer, texture::Texture};

pub struct State<'a> 
{
    pub(crate) surface: wgpu::Surface<'a>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    pub size: winit::dpi::PhysicalSize<u32>,
    window: &'a Window,
    pub renderer: Renderer,
    screen_texture: Texture
}

impl<'a> State<'a> 
{
    pub async fn new(window: &'a Window) -> (State<'a>, AssetManager) 
    {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor 
        {
            #[cfg(not(target_arch = "wasm32"))]
            backends: wgpu::Backends::PRIMARY,
            #[cfg(target_arch = "wasm32")]
            backends: wgpu::Backends::GL,
            ..Default::default()
        });

        let surface = instance.create_surface(window).unwrap();

        let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions 
        {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }).await.unwrap();

        let (device, queue) = adapter.request_device(&wgpu::DeviceDescriptor 
        {
            label: None,
            required_features: wgpu::Features::POLYGON_MODE_LINE,  // empty()
            required_limits: if cfg!(target_arch = "wasm32") 
            {
                wgpu::Limits::downlevel_webgl2_defaults()
            } 
            else 
            {
                wgpu::Limits::default()
            },
            memory_hints: Default::default(),
        },None,).await.unwrap();

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps.formats.iter().copied().find(|f| f.is_srgb()).unwrap_or(surface_caps.formats[0]);
        let config = wgpu::SurfaceConfiguration 
        {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            desired_maximum_frame_latency: 2,
            view_formats: vec![],
        };

        surface.configure(&device, &config);

        let size = window.inner_size();

        let assets = AssetManager::new(&device, &queue).unwrap(); //Proper error handling gonna come soon, moved it in here because the renderer needs the default texture

        let renderer = Renderer::new(&device, &config, (size.width as f32, size.height as f32), &assets.shader, assets.textures.get_bind_group(TextureHandle(0)).unwrap()); // Gotta check if this works, might not, especially because I probably did some stupid error here

        let screen_texture = Texture::screen_texture(&device, size.width as u32, size.height as u32, &assets.texture_bindgroup_layout);

        let state = Self 
        {
            surface,
            device,
            queue,
            config,
            size,
            window,
            renderer,
            screen_texture
        };

        (state, assets)
    }

    pub fn window(&self) -> &Window 
    {
        &self.window
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) 
    {
        if new_size.width > 0 && new_size.height > 0 
        {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
            self.renderer.window_size = (new_size.width as f32, new_size.height as f32);
        }
    }

    #[allow(unused)]
    pub fn input(&mut self, event: &WindowEvent) -> bool { false }

    #[allow(unused)]
    pub fn update(&mut self) {}

    pub fn render<T>(&mut self, draw: T) -> Result<(), wgpu::SurfaceError> where T: FnOnce(&mut Renderer)
    {
        let output = self.surface.get_current_texture()?;
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor 
        {
            label: Some("Render Encoder"),
        });

        draw(&mut self.renderer);
        self.renderer.upload_instances(&self.device, &self.queue);
        self.renderer.begin_pass(&mut encoder, &view/*self.screen_texture.view/&view*/); // Normal Render Pass -> outputs to Texture, not View
        // self.renderer.begin_pass(&mut encoder, &view);
        // self.renderer.screen_texture(&mut encoder, &view, 2, &self.screen_texture.bind_group); // Manual here for now. remember to remove from here later

        self.queue.submit(iter::once(encoder.finish()));
        output.present();

        self.renderer.draw_commands.clear();

        Ok(())
    }

    pub fn set_fullscreen(&mut self, fullscreen: bool)
    {
        if fullscreen
        {
            let monitor = self.window().current_monitor();
            self.window.set_fullscreen(Some(winit::window::Fullscreen::Borderless(monitor)));
        }
        else 
        {
            self.window.set_fullscreen(None);
        }
    }
}
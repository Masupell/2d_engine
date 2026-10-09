use std::iter;
use winit::{event::*,window::Window};

use crate::{Context, RenderContext, renderer::Renderer, threads::ThreadPool};

pub struct State<'a>
{
    pub(crate) surface: wgpu::Surface<'a>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    pub size: winit::dpi::PhysicalSize<u32>,
    window: &'a Window,
    pub renderer: Renderer,
    view_format: wgpu::TextureFormat
}

impl<'a> State<'a>
{
    pub async fn new(window: &'a Window, screen_resolution: (f32, f32), threads: ThreadPool) -> State<'a>
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
        let view_format = surface_format.add_srgb_suffix();
        let copy_src = if view_format == surface_format { surface_caps.usages & wgpu::TextureUsages::COPY_SRC } else { wgpu::TextureUsages::empty() };
        let config = wgpu::SurfaceConfiguration
        {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | copy_src,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,//surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            desired_maximum_frame_latency: 2,
            view_formats: if view_format == surface_format { vec![] } else { vec![view_format] },
        };

        surface.configure(&device, &config);

        let size = window.inner_size();
        let renderer = Renderer::new(&device, &queue, view_format, (size.width.max(1), size.height.max(1)), config.usage.contains(wgpu::TextureUsages::COPY_SRC), screen_resolution, threads);

        Self
        {
            surface,
            device,
            queue,
            config,
            size,
            window,
            renderer,
            view_format
        }
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
            self.renderer.resize_targets(&self.device, new_size.width, new_size.height);
        }
    }

    #[allow(unused)]
    pub fn input(&mut self, event: &WindowEvent) -> bool { false }

    #[allow(unused)]
    pub fn update(&mut self) {}

    pub fn render<T>(&mut self, context: &mut Context, draw: T) -> Result<(), wgpu::SurfaceError> where T: FnOnce(&mut RenderContext)
    {
        let output = self.surface.get_current_texture()?;
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor { format: Some(self.view_format), ..Default::default() });

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor
        {
            label: Some("Render Encoder"),
        });

        {
            let mut render_ctx = RenderContext::new(context, &mut self.renderer, &self.device, &self.queue, &self.config);
            draw(&mut render_ctx);
        }

        self.renderer.prepare_frame(&self.device, &self.queue);

        // All targets in order of creation, then the screen (world, ui)
        self.renderer.render_frame(&mut encoder, &output.texture, &view);

        self.queue.submit(iter::once(encoder.finish()));
        output.present();

        self.renderer.end_frame();

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

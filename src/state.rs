use std::iter;
use winit::{event::*,window::Window};

use crate::{Context, RenderContext, renderer::Renderer, texture::Texture, utility::DrawLayer};

pub struct State<'a>
{
    pub(crate) surface: wgpu::Surface<'a>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    pub size: winit::dpi::PhysicalSize<u32>,
    window: &'a Window,
    pub renderer: Renderer,
    // Ping-pong pair (gets passed from the one to the other and so forth, allowing for infinite post-process effects)
    screen_textures: [Texture; 2],
    bind_groups: [wgpu::BindGroup; 2]
}

impl<'a> State<'a>
{
    pub async fn new(window: &'a Window) -> State<'a>
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
            present_mode: wgpu::PresentMode::Fifo,//surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            desired_maximum_frame_latency: 2,
            view_formats: vec![],
        };

        surface.configure(&device, &config);

        let size = window.inner_size();
        let renderer = Renderer::new(&device, &config, &queue, (size.width as f32, size.height as f32));

        let screen_texture_a = Texture::screen_texture(&device, surface_format, size.width as u32, size.height as u32);
        let screen_texture_b = Texture::screen_texture(&device, surface_format, size.width as u32, size.height as u32);
        let bind_group_a = screen_texture_a.bind_group(&device, &renderer.texture_bindgroup_layout);
        let bind_group_b = screen_texture_b.bind_group(&device, &renderer.texture_bindgroup_layout);

        Self
        {
            surface,
            device,
            queue,
            config,
            size,
            window,
            renderer,
            screen_textures: [screen_texture_a, screen_texture_b],
            bind_groups: [bind_group_a, bind_group_b]
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
            for i in 0..2
            {
                self.screen_textures[i] = Texture::screen_texture(&self.device, self.config.format, new_size.width, new_size.height);
                self.bind_groups[i] = self.screen_textures[i].bind_group(&self.device, &self.renderer.texture_bindgroup_layout);
            }
        }
    }

    #[allow(unused)]
    pub fn input(&mut self, event: &WindowEvent) -> bool { false }

    #[allow(unused)]
    pub fn update(&mut self) {}

    pub fn render<T>(&mut self, context: &mut Context, draw: T) -> Result<(), wgpu::SurfaceError> where T: FnOnce(&mut RenderContext)
    {
        let output = self.surface.get_current_texture()?;
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor
        {
            label: Some("Render Encoder"),
        });

        {
            let mut render_ctx = RenderContext::new(context, &mut self.renderer, &self.device, &self.queue, &self.config);
            draw(&mut render_ctx);
        }

        self.renderer.upload_instances(&self.device, &self.queue);
        // self.renderer.begin_pass(&mut encoder, &self.screen_texture.view/*&view*/); // Normal Render Pass -> outputs to Texture, not View
        // // self.renderer.begin_pass(&mut encoder, &view);
        // if let Some(pp_id) = context.post_process_pipeline
        // {
        //     self.renderer.screen_texture(&mut encoder, &view, pp_id, &self.bind_group); // Manual here for now. remember to remove from here later
        // }
        if context.post_process_pipelines.is_empty()
        {
            self.renderer.begin_pass(&mut encoder, &view, DrawLayer::World);
            println!("???");
        }
        else
        {
            self.renderer.begin_pass(&mut encoder, &self.screen_textures[0].view, DrawLayer::World);

            let mut current: usize = 0;
            let last_index = context.post_process_pipelines.len()-1;
            // println!("Good so far, {}", context.post_process_pipelines.len());

            for (i, &pipeline_id) in context.post_process_pipelines.iter().enumerate()
            {
                if i == last_index // Final pass
                {
                    // println!("Should work?");
                    self.renderer.screen_texture(&mut encoder, &view, pipeline_id, &self.bind_groups[current]);
                }
                else
                {
                    let next = 1 - current;
                    self.renderer.screen_texture(&mut encoder, &self.screen_textures[next].view, pipeline_id, &self.bind_groups[current]);
                    current = next;
                }
            }
        }

        // Draws UI on top of everything else
        self.renderer.begin_pass(&mut encoder, &view, DrawLayer::UI);

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

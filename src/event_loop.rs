use winit::{dpi::LogicalSize, event::*, event_loop::EventLoop, window::WindowBuilder};

use crate::{asset_manager::AssetManager, context::{Context, ContextAction, RenderContext, UpdateContext}, input::Input, state::State};

pub trait EngineEvent 
{
    // fn setup(&mut self, loader: &mut dyn Loader);
    // fn update(&mut self, input: &Input, dt: f64);
    // fn render(&self, renderer: &mut Renderer);
    fn setup(&mut self, ctx: &mut Context);
    fn update(&mut self, update_ctx: &mut UpdateContext);
    fn render(&self, render_ctx: &mut RenderContext);
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
pub async fn game_loop<T: EngineEvent + 'static>(mut game: Box<T>, title: &str, size: (i32, i32))
{
    cfg_if::cfg_if! 
    {
        if #[cfg(target_arch = "wasm32")] 
        {
            std::panic::set_hook(Box::new(console_error_panic_hook::hook));
            console_log::init_with_level(log::Level::Info).expect("Couldn't initialize logger");
        } else 
        {
            env_logger::init();
        }
    }

    let event_loop = EventLoop::new().unwrap();
    let window = WindowBuilder::new().with_title(title).with_inner_size(LogicalSize::new(size.0, size.1)).build(&event_loop).unwrap();

    #[cfg(target_arch = "wasm32")]
    {
        use winit::dpi::PhysicalSize;

        use winit::platform::web::WindowExtWebSys;
        web_sys::window().and_then(|win| win.document()).and_then(|doc| 
        {
            let dst = doc.get_element_by_id("wasm-example")?;
            let canvas = web_sys::Element::from(window.canvas()?);
            dst.append_child(&canvas).ok()?;
            Some(())
        }).expect("Couldn't append canvas to document body.");

        let _ = window.request_inner_size(PhysicalSize::new(450, 400));
    }

    let (mut state, assets) = State::new(&window).await;
    let mut surface_configured = false;
    let size = window.inner_size();
    let mut input = Input::new((size.width as f64, size.height as f64));

    let mut ctx = Context::new((size.width, size.height), false, false, assets);
    game.setup(&mut ctx);

    let mut last_frame_time = std::time::Instant::now();
    let mut fps_accumulator = 0.0;
    let mut fps_counter = 0;
    event_loop.run(move |event, control_flow| 
    {
        match event 
        {
            Event::WindowEvent 
            {
                ref event,
                window_id,
            } 
            if window_id == state.window().id() => 
            {
                input.update_inputs(&event);
                if !state.input(event) 
                {
                    match event 
                    {
                        WindowEvent::CloseRequested => control_flow.exit(),
                        WindowEvent::Resized(physical_size) => 
                        {
                            log::info!("physical_size: {physical_size:?}");
                            surface_configured = true;
                            state.resize(*physical_size);
                            input.update_screen((physical_size.width as f64, physical_size.height as f64));
                        }
                        WindowEvent::RedrawRequested => 
                        {
                            if !surface_configured 
                            {
                                return;
                            }
                            let now = std::time::Instant::now();
                            let dt = (now - last_frame_time).as_secs_f64();
                            last_frame_time = now;

                            ctx.assets.process_loading(&state.device, &state.queue);

                            let mut update_ctx = UpdateContext::new(&input, &mut ctx, dt);

                            game.update(&mut update_ctx);

                            // Process things like fullscreen toggle, etc
                            while let Some(action) = ctx.pending_actions.pop() 
                            {
                                match action 
                                {
                                    ContextAction::ToggleFullscreen(fullscreen) => 
                                    {
                                        state.set_fullscreen(fullscreen);
                                    }
                                    ContextAction::SetVSync(vsync) => 
                                    {
                                        let present_mode = if vsync { wgpu::PresentMode::AutoVsync } else { wgpu::PresentMode::AutoNoVsync };
                                        state.config.present_mode = present_mode;
                                        state.surface.configure(&state.device, &state.config);
                                    }
                                }
                            }

                            match state.render(|renderer| 
                            {
                                let mut render_ctx = RenderContext::new(renderer, &mut ctx);
                                // game.render(renderer);
                                game.render(&mut render_ctx);
                            })
                            {
                                Ok(_) => {}
                                Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => state.resize(state.size),
                                Err(wgpu::SurfaceError::OutOfMemory | wgpu::SurfaceError::Other) => 
                                {
                                    log::error!("OutOfMemory");
                                    control_flow.exit();
                                }   
                                Err(wgpu::SurfaceError::Timeout) => 
                                {
                                    log::warn!("Surface timeout");
                                }
                            }

                            // FPS Counter
                            fps_accumulator += dt;
                            fps_counter += 1;
                            if fps_accumulator >= 0.25 {
                                let fps = (fps_counter as f64 / fps_accumulator) as u32;
                                state.window().set_title(&format!("FPS: {}", fps));
                                fps_accumulator = 0.0;
                                fps_counter = 0;
                            }

                            input.prev_update();
                            state.window().request_redraw();
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }).unwrap();
}
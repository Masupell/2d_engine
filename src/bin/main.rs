use engine::*;

struct App
{
    x: f32,
    y: f32,
    async_texture_test: TextureId,
    async_task_test: Loading<Vec<f32>>,
    shader_test: ShaderId,
    time: f32,
    scroll: f32
}

const CLIP_PANEL: u32 = 1;

impl EngineEvent for App
{
    fn setup(&mut self, ctx: &mut Context, graphics: &mut GraphicsContext, _input: &mut Input)
    {
        // graphics.set_clear_color([0.05, 0.03, 0.22, 1.0]);
        graphics.renderer.set_pixels_per_unit(1280.0/12.0);

        self.shader_test = graphics.load_shader_with_uniform(Some(ShaderInput::File("src/bin/reload_test.wgsl")), None, PipeLineType::Normal, &[("time", UniformType::Float)]);

        ctx.toggle_stats();
    }

    fn physics_update(&mut self, update_ctx: &mut UpdateContext)
    {
        if update_ctx.input.is_key_pressed(Key::F11)
        {
            update_ctx.context.toggle_fullscreen();
        }
        if update_ctx.input.is_key_pressed(Key::KeyQ)
        {
            update_ctx.context.toggle_vsync();
        }
        let position_change = ((self.x-640.0)/(update_ctx.graphics.renderer.pixels_per_unit*update_ctx.graphics.renderer.camera_zoom), (self.y-360.0)/(update_ctx.graphics.renderer.pixels_per_unit*update_ctx.graphics.renderer.camera_zoom));
        update_ctx.graphics.set_camera_pos(position_change);

        if update_ctx.input.is_key_hold(Key::ArrowLeft)
        {
            update_ctx.graphics.renderer.change_camera_rotation(-0.01);
        }
        if update_ctx.input.is_key_hold(Key::ArrowRight)
        {
            update_ctx.graphics.renderer.change_camera_rotation(0.01);
        }

        if update_ctx.input.is_key_released(Key::Space)
        {
            self.async_texture_test = update_ctx.graphics.load_texture_async("src/image/font_atlas_debug.png", FilterMode::Linear, FilterMode::Linear);
            self.async_task_test = Loading::Running(update_ctx.context.threads.spawn(move || heavy_calculation(12, 2048)));
            let old = vec![0, 1, 2, 3, 4, 5, 6];
            let mut new = vec![0; old.len()];
            update_ctx.context.threads.parallel_for_mut(&mut new, 2, |i, slice|
            {
                let start = i*2;
                for (j, value) in slice.iter_mut().enumerate()
                {
                    *value = old[start+j];
                }
            });
            println!("Old: {old:?}, New: {new:?}");
        }

        if update_ctx.input.is_key_pressed(Key::Enter)
        {
            let mut rng = Rng::new();
            println!("{}", rng.range_f32(5.0, 100.0));
        }

        if update_ctx.input.is_key_pressed(Key::F3) { update_ctx.context.toggle_stats(); }

        if update_ctx.input.is_key_released(Key::KeyU)
        {
            update_ctx.graphics.renderer.unload_texture(self.async_texture_test);
        }

        if update_ctx.input.is_key_released(Key::KeyS)
        {
            update_ctx.graphics.renderer.unload_shader(self.shader_test);
        }
    }

    fn update(&mut self, update_ctx: &mut UpdateContext)
    {
        (self.x, self.y) = update_ctx.input.mouse_position_f32();

        self.async_task_test.update();
        if let Some(result) = self.async_task_test.take()
        {
            println!("Finished {}", result.len());
        }

        if update_ctx.input.hover(CLIP_PANEL, (900.0, 50.0), (200.0, 100.0), 0)
        {
            self.scroll += update_ctx.input.scroll_y();
        }

        if !update_ctx.input.pointer_over_ui()
        {
            update_ctx.graphics.renderer.camera_zoom *= 1.1_f32.powf(update_ctx.input.scroll_y());
        }

        self.time += update_ctx.dt as f32;
        update_ctx.graphics.set_uniform("time", UniformValue::Float(self.time));
    }

    fn render(&self, render_ctx: &mut RenderContext)
    {
        render_ctx.graphics.renderer.draw(Draw::rect((0.0, 0.0), (1.0, 1.0), 0.0, [1.0, 1.0, 1.0, 1.0])); // meters
        render_ctx.graphics.renderer.draw(Draw::rect((0.0, 0.8), (5.0, 0.25), 0.0, [1.0, 0.0, 0.0, 1.0]));
        render_ctx.graphics.renderer.draw_text(Text::new("Hello", (640.0, 360.0), 45.0, [0.0, 1.0, 0.0, 1.0]).center().ui());

        render_ctx.graphics.renderer.draw(Draw::sprite(self.async_texture_test, (131.0, 125.0), (111.0, 105.0), 0.0).ui());

        render_ctx.graphics.renderer.with_clip(ClipRect::ui((900.0, 50.0), (200.0, 100.0)), |r|
        {
            r.draw_text(Text::new("ClipText", (1000.0, 100.0+self.scroll*10.0), 45.0, [1.0, 1.0, 1.0, 1.0]).center().ui());
        });

        render_ctx.graphics.renderer.draw_fullscreen(TextureId::WHITE, [1.0, 1.0, 1.0, 1.0], 1, self.shader_test);
    }
}

impl App
{
    fn new() -> Self
    {
        Self
        {
            x: 0.0,
            y: 0.0,
            async_texture_test: TextureId::WHITE,
            async_task_test: Loading::Idle,
            shader_test: ShaderId::DEFAULT,
            time: 0.0,
            scroll: 0.0
        }
    }
}

fn main()
{
    run(App::new(), "Example", (1280, 720));
}



// Generates a 3d terrain, jst a random function, what it does is not important, just that it does something complicated,
// which would block the main thread
fn heavy_calculation(seed: u32, size: usize) -> Vec<f32>
{
    let mut terrain = vec![0.0; size * size];

    for y in 0..size
    {
        for x in 0..size
        {
            let mut height = 0.0;

            for octave in 0..12
            {
                let frequency = 2.0_f32.powi(octave);
                let amplitude = 0.5_f32.powi(octave);

                let nx = x as f32 * frequency / size as f32;
                let ny = y as f32 * frequency / size as f32;

                let phase_x = (seed as f32 * 0.001).sin();
                let phase_y = (seed as f32 * 0.002).cos();

                let value = (nx + phase_x).sin() * (ny + phase_y).cos();

                height += value * amplitude;
            }
            terrain[y * size + x] = height;
        }
    }

    terrain
}

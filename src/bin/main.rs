use engine::*;
// use rand::Rng;


struct App { x: f32, y: f32, async_texture_test: usize, async_task_test: Option<Task<Vec<f32>>> }

impl EngineEvent for App
{
    fn setup(&mut self, _ctx: &mut Context, graphics: &mut GraphicsContext, _input: &mut Input)
    {
        graphics.set_clear_color([0.05, 0.03, 0.22, 1.0]);
        graphics.renderer.set_pixels_per_unit(1280.0/12.0);
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

        if update_ctx.input.is_key_hold(Key::ArrowDown)
        {
            update_ctx.graphics.renderer.change_camera_zoom(-0.01);
        }
        if update_ctx.input.is_key_hold(Key::ArrowUp)
        {
            update_ctx.graphics.renderer.change_camera_zoom(0.01);
        }

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
            self.async_task_test = Some(update_ctx.context.threads.spawn(move || heavy_calculation(12, 2048)));
        }
    }

    fn update(&mut self, update_ctx: &mut UpdateContext)
    {
        (self.x, self.y) = update_ctx.input.mouse_position_f32();

        if let Some(task) = &mut self.async_task_test
        {
            if let Some(result) = task.try_take()
            {
                println!("Finished {}", result.len());
            }
        }
    }

    fn render(&self, render_ctx: &mut RenderContext)
    {
        render_ctx.graphics.renderer.draw(Draw::rect((0.0, 0.0), (1.0, 1.0), 0.0, [1.0, 1.0, 1.0, 1.0])); // meters
        render_ctx.graphics.renderer.draw(Draw::rect((0.0, 0.8), (5.0, 0.25), 0.0, [1.0, 0.0, 0.0, 1.0]));
        render_ctx.graphics.renderer.draw_text(Text::new("Hello", (640.0, 360.0), 45.0, [0.0, 1.0, 0.0, 1.0]).center().ui());

        render_ctx.graphics.renderer.draw(Draw::sprite(self.async_texture_test, (131.0, 125.0), (111.0, 105.0), 0.0).ui());
    }
}

impl App
{
    fn new() -> Self
    {
        Self { x: 0.0, y: 0.0, async_texture_test: 0, async_task_test: None }
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

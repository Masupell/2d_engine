use engine::*;
// use rand::Rng;


struct App { x: f32, y: f32}

impl EngineEvent for App
{
    fn setup(&mut self, _ctx: &mut Context, graphics: &mut GraphicsContext, _input: &mut Input)
    {
        graphics.set_clear_color([0.05, 0.03, 0.22, 1.0]);
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
    }

    fn update(&mut self, update_ctx: &mut UpdateContext)
    {
        self.x = update_ctx.input.mouse_position().0 as f32;
        self.y = update_ctx.input.mouse_position().1 as f32;
    }

    fn render(&self, render_ctx: &mut RenderContext)
    {
        render_ctx.graphics.renderer.draw_text_centered("Hello", (640.0, 360.0), 45.0, [0.9, 0.8, 0.8, 1.0], 0.0, CoordSpace::Screen, DrawLayer::World, 0, 0);
    }
}

impl App
{
    fn new() -> Self
    {
        Self {x: 0.0, y: 0.0}
    }
}

fn main()
{
    run(App::new(), "Example", (1280, 720));
}

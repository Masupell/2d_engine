use engine::*;
// use rand::Rng;


struct App { x: f32, y: f32}

impl EngineEvent for App
{
    fn setup(&mut self, ctx: &mut Context, graphics: &mut GraphicsContext, input: &mut Input)
    {
        graphics.set_clear_color([1.0, 0.0, 0.0, 1.0]);
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
        // render_ctx.graphics.renderer.draw(0, render_ctx.graphics.renderer.ui_matrix((640.0, 360.0), (100.0, 100.0), 0.0), [1.0, 1.0, 1.0, 1.0], 0, 0);
        render_ctx.graphics.renderer.draw_texture(0, render_ctx.graphics.renderer.ui_matrix((640.0, 360.0), (500.0, 500.0), 0.0), 1, 0, 0);
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

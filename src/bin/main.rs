use engine::*;
// use rand::Rng;

type ActionFn = fn(&mut App, &mut UpdateContext);

const ACTION_TABLE: [ActionFn; 5] =
[
    App::toggle_fullscreen,
    App::escape,
    App::mouse_left_pressed,
    App::mouse_left_released,
    App::mouse_left_hold,
];

struct App { x: f32, y: f32}

impl App
{
    fn toggle_fullscreen(&mut self, ctx: &mut UpdateContext)
    {
        ctx.context.toggle_fullscreen();
    }

    fn escape(&mut self, ctx: &mut UpdateContext)
    {

    }

    fn mouse_left_pressed(&mut self, ctx: &mut UpdateContext)
    {

    }

    fn mouse_left_released(&mut self, ctx: &mut UpdateContext)
    {

    }

    fn mouse_left_hold(&mut self, ctx: &mut UpdateContext)
    {

    }
}

impl EngineEvent for App
{
    fn setup(&mut self, ctx: &mut Context, loader: &mut dyn Loader)
    {
        ctx.toggle_vsync();
        ctx.toggle_fullscreen();
        loader.load_texture("src/image/owl.jpg");
        loader.load_shader(Some("src/shaders/test.wgsl"), None);
        loader.load_shader(Some("src/shaders/post_process.wgsl"), Some("src/shaders/post_process.wgsl"));
    }

    fn update(&mut self, update_ctx: &mut UpdateContext)
    {
        self.x = update_ctx.input.mouse_position().0 as f32;
        self.y = update_ctx.input.mouse_position().1 as f32;

        for action in update_ctx.input.actions()
        {
            ACTION_TABLE[*action as usize](self, update_ctx);
        }
    }

    fn render(&self, render_ctx: &mut RenderContext)
    {
        render_ctx.renderer.draw_texture(0, render_ctx.renderer.texture_matrix((render_ctx.renderer.virtual_size.0/2.0, render_ctx.renderer.virtual_size.1/2.0), (1.0, 1.0), 0.0, (1920.0, 1014.0)), 1, 0, 0);
        render_ctx.renderer.draw_texture(0, render_ctx.renderer.texture_matrix((self.x, self.y), (0.5, 0.5), 0.0, (1920.0, 1014.0)), 1, 0, 1);
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
    pollster::block_on(game_loop(Box::new(App::new()), "Performance", (1280, 720)));
}

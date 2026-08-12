use engine::*;
// use rand::Rng;

type ActionFn = fn(&mut App, &mut UpdateContext);

const ACTION_TABLE: [ActionFn; 8] =
[
    App::toggle_fullscreen,
    App::escape,
    App::mouse_left_pressed,
    App::mouse_left_released,
    App::mouse_left_hold,
    App::print,
    App::hover,
    App::unhover
];

struct App
{
    x: f32,
    y: f32,
    button: no_if::button::Button
}

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

    fn print(&mut self, _ctx: &mut UpdateContext)
    {
        println!("Button Click detected")
    }

    fn hover(&mut self, _ctx: &mut UpdateContext)
    {
        println!("Button Hover")
    }

    fn unhover(&mut self, _ctx: &mut UpdateContext)
    {
        println!("Button Leaves Hover")
    }
}

impl EngineEvent for App
{
    fn setup(&mut self, ctx: &mut Context, loader: &mut dyn Loader)
    {
        ctx.toggle_vsync();
        // loader.load_texture("src/image/owl.jpg");
        loader.load_texture("src/image/Player.png");
        loader.load_shader(Some("src/shaders/test.wgsl"), None);
        loader.load_shader(Some("src/shaders/post_process.wgsl"), Some("src/shaders/post_process.wgsl"));
    }

    fn update(&mut self, update_ctx: &mut UpdateContext)
    {
        self.x = update_ctx.input.mouse_position().0 as f32;
        self.y = update_ctx.input.mouse_position().1 as f32;

        self.button.update(update_ctx.input);

        let actions = update_ctx.input.actions().to_vec();
        for action in actions
        {
            ACTION_TABLE[action as usize](self, update_ctx);
        }
    }

    fn render(&self, render_ctx: &mut RenderContext)
    {
        // render_ctx.renderer.draw_texture(0, render_ctx.renderer.texture_matrix((render_ctx.renderer.virtual_size.0/2.0, render_ctx.renderer.virtual_size.1/2.0), (1.0, 1.0), 0.0, (1920.0, 1014.0)), 1, 0, 0);
        // render_ctx.renderer.draw_texture(0, render_ctx.renderer.texture_matrix((self.x, self.y), (0.5, 0.5), 0.0, (1920.0, 1014.0)), 1, 0, 1);

        render_ctx.renderer.draw_texture(0, render_ctx.renderer.matrix((100.0, 100.0), (200.0, 200.0), 0.0), 1, 0, 0);
    }
}

impl App
{
    fn new() -> Self
    {
        let mut button = Button::new(Rect::new(0.0, 0.0, 200.0, 200.0));
        button.set_action(ButtonEvent::Click, Action::Print);
        button.set_action(ButtonEvent::Hover, Action::Hover);
        button.set_action(ButtonEvent::Unhover, Action::UnHover);

        Self
        {
            x: 0.0,
            y: 0.0,
            button
        }
    }
}

fn main()
{
    pollster::block_on(game_loop(Box::new(App::new()), "Performance", (1280, 720)));
}

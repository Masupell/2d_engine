pub mod player;

use engine::*;

use crate::player::Player;
// use rand::Rng;

type ActionFn = fn(&mut App, &mut UpdateContext);

const ACTION_TABLE: [ActionFn; 9] =
[
    App::toggle_fullscreen,
    App::escape,
    App::mouse_left_pressed,
    App::mouse_left_released,
    App::mouse_left_hold,
    App::print,
    App::hover,
    App::unhover,
    App::released
];

struct App
{
    x: f32,
    y: f32,
    player: Player
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
        println!("Button Hover");
        // self.button.set_size_centered((220.0, 110.0));
    }

    fn unhover(&mut self, _ctx: &mut UpdateContext)
    {
        println!("Button Leaves Hover");
        // self.button.set_size_centered((200.0, 100.0));
    }

    fn released(&mut self, _ctx: &mut UpdateContext)
    {
        println!("Button Released")
    }
}

impl EngineEvent for App
{
    fn setup(&mut self, ctx: &mut Context, loader: &mut dyn Loader)
    {
        // ctx.toggle_vsync();
        // loader.load_texture("src/image/owl.jpg");
        // loader.load_texture("src/image/Player.png", FilterMode::Nearest, FilterMode::Nearest);
        // let button_texture = loader.load_texture("src/image/button.png", FilterMode::Linear, FilterMode::Linear);
        // self.button.set_texture(button_texture);
        let player_texture = loader.load_texture("src/image/player.png", FilterMode::Linear, FilterMode::Linear);
        self.player.set_texture(player_texture);
        loader.load_shader(Some("src/shaders/test.wgsl"), None);
        loader.load_shader(Some("src/shaders/post_process.wgsl"), Some("src/shaders/post_process.wgsl"));
    }

    fn update(&mut self, update_ctx: &mut UpdateContext)
    {
        self.x = update_ctx.input.mouse_position().0 as f32;
        self.y = update_ctx.input.mouse_position().1 as f32;

        // self.button.update(update_ctx.input);
        self.player.rotate(std::f32::consts::PI*update_ctx.dt as f32);
        self.player.update(update_ctx.input);

        // println!(
        //     "dt: {:.4}, rotation: {:.4}",
        //     update_ctx.dt,
        //     self.player.collision.rotation
        // );

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

        // render_ctx.renderer.draw_texture(0, render_ctx.renderer.matrix((100.0, 100.0), (200.0, 200.0), 0.0), 1, 0, 0);
        self.player.draw(render_ctx, 0, 0);
    }
}

impl App
{
    fn new() -> Self
    {
        let player = Player::new((640.0, 360.0), 128.0, 128.0);

        Self
        {
            x: 0.0,
            y: 0.0,
            player
        }
    }
}

fn main()
{
    pollster::block_on(game_loop(Box::new(App::new()), "Performance", (1280, 720)));
}

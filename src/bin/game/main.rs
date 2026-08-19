pub mod player;
pub mod rope;

use engine::*;

use crate::player::Player;
// use rand::Rng;

type ActionFn = fn(&mut App, &mut UpdateContext);

const ACTION_TABLE: [ActionFn; Action::COUNT] =
[
    App::toggle_fullscreen,
    App::escape,
    App::mouse_left_pressed,
    App::mouse_left_released,
    App::mouse_left_hold,
    App::player_rotate_left,
    App::player_rotate_right,
    App::player_place_checkpoint,
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

    fn player_rotate_left(&mut self, ctx: &mut UpdateContext) { self.player.rotate_left(ctx.dt as f32); }
    fn player_rotate_right(&mut self, ctx: &mut UpdateContext) { self.player.rotate_right(ctx.dt as f32); }

    fn player_place_checkpoint(&mut self, _ctx: &mut UpdateContext)
    {
        println!("Checkpoint Placed");
    }
}

impl EngineEvent for App
{
    fn setup(&mut self, ctx: &mut Context, loader: &mut dyn Loader, input: &mut Input)
    {
        // ctx.toggle_vsync();
        // loader.load_texture("src/image/owl.jpg");
        // loader.load_texture("src/image/Player.png", FilterMode::Nearest, FilterMode::Nearest);
        // let button_texture = loader.load_texture("src/image/button.png", FilterMode::Linear, FilterMode::Linear);
        // self.button.set_texture(button_texture);
        let player_texture = loader.load_texture("src/image/player.png", FilterMode::Linear, FilterMode::Linear);
        self.player.set_texture(player_texture);
        register_keys(input);
        loader.load_shader(Some("src/shaders/test.wgsl"), None);
        loader.load_shader(Some("src/shaders/post_process.wgsl"), Some("src/shaders/post_process.wgsl"));
        loader.load_texture("src/image/cheetah.jpg", FilterMode::Linear, FilterMode::Linear);
    }

    fn physics_update(&mut self, update_ctx: &mut UpdateContext)
    {
        // let direction = (self.x-self.player.collision.x, self.y-self.player.collision.y);
        // let target_angle = -direction.1.atan2(direction.0)-std::f32::consts::PI/2.0;

        // let current_angle = self.player.collision.rotation;
        // let max_rotation = std::f32::consts::PI*2.0 * update_ctx.dt as f32;

        // let angle_diff = (target_angle - current_angle + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;

        // self.player.rotate(angle_diff.clamp(-max_rotation, max_rotation));

        self.player.update(update_ctx.input, update_ctx.dt);

        let actions = update_ctx.input.actions().to_vec();
        for action in actions
        {
            ACTION_TABLE[action as usize](self, update_ctx);
        }
    }

    fn update(&mut self, update_ctx: &mut UpdateContext)
    {
        self.x = update_ctx.input.mouse_position().0 as f32;
        self.y = update_ctx.input.mouse_position().1 as f32;

        // self.button.update(update_ctx.input);
    }

    fn render(&self, render_ctx: &mut RenderContext)
    {
        // render_ctx.renderer.draw_texture(0, render_ctx.renderer.texture_matrix((render_ctx.renderer.virtual_size.0/2.0, render_ctx.renderer.virtual_size.1/2.0), (1.0, 1.0), 0.0, (1920.0, 1014.0)), 1, 0, 0);
        // render_ctx.renderer.draw_texture(0, render_ctx.renderer.texture_matrix((self.x, self.y), (0.5, 0.5), 0.0, (1920.0, 1014.0)), 1, 0, 1);

        // render_ctx.renderer.draw_texture(0, render_ctx.renderer.matrix((100.0, 100.0), (200.0, 200.0), 0.0), 1, 0, 0);
        self.player.draw(render_ctx, 1, 0);

        render_ctx.renderer.draw_texture(0, render_ctx.renderer.matrix((640.0, -300.0), (1920.0, 1080.0), 0.0), 2, 0, 0);
    }
}

impl App
{
    fn new() -> Self
    {
        let player = Player::new(Vec2::new(640.0, 360.0), 128.0, 128.0, 90.0_f32.to_radians(), 50.0_f32.to_radians());
        // player.set_action(event, action);

        Self
        {
            x: 0.0,
            y: 0.0,
            player
        }
    }
}

pub fn register_keys(input: &mut Input)
{
    input.add_key_binding(Key::KeyA, None, None, Some(Action::RotateLeft));
    input.add_key_binding(Key::KeyD, None, None, Some(Action::RotateRight));
    input.add_key_binding(Key::Space, Some(Action::PlaceCheckPoint), None, None);
}

fn main()
{
    pollster::block_on(game_loop(Box::new(App::new()), "Climbing Game", (1280, 720)));
}

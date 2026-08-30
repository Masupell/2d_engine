pub mod player;
pub mod rope;
pub mod wall;

use engine::*;

use crate::{player::Player, rope::Rope, wall::Wall};
// use rand::Rng;

type ActionFn = fn(&mut App, &mut UpdateContext);

const ACTION_TABLE: [ActionFn; Action::COUNT] =
[
    App::toggle_fullscreen,
    App::escape,
    App::mouse_left_pressed,
    App::mouse_left_released,
    App::mouse_left_hold,
    App::player_place_checkpoint,
    App::player_start_falling,
    App::player_move_up,
    App::player_move_left,
    App::player_move_right
];

struct App
{
    x: f32,
    y: f32,
    player: Player,
    rope: Rope,
    wall: Wall
}

impl App
{
    fn toggle_fullscreen(&mut self, ctx: &mut UpdateContext)
    {
        ctx.context.toggle_fullscreen();
    }

    fn escape(&mut self, _ctx: &mut UpdateContext)
    {

    }

    fn mouse_left_pressed(&mut self, _ctx: &mut UpdateContext)
    {

    }

    fn mouse_left_released(&mut self, ctx: &mut UpdateContext)
    {
        self.rope.add_anchor(ctx.graphics.renderer, ctx.graphics.device, ctx.graphics.queue, 10);
    }

    fn mouse_left_hold(&mut self, _ctx: &mut UpdateContext)
    {

    }


    fn player_place_checkpoint(&mut self, ctx: &mut UpdateContext)
    {
        self.rope.add_anchor(ctx.graphics.renderer, ctx.graphics.device, ctx.graphics.queue, 10);
    }

    fn player_start_falling(&mut self, _ctx: &mut UpdateContext) { self.player.start_falling(); }

    fn player_move_up(&mut self, _ctx: &mut UpdateContext) { self.player.move_up(); }
    fn player_move_left(&mut self, _ctx: &mut UpdateContext) { self.player.move_left(); }
    fn player_move_right(&mut self, _ctx: &mut UpdateContext) { self.player.move_right(); }
}

impl EngineEvent for App
{
    fn setup(&mut self, ctx: &mut Context, graphics: &mut GraphicsContext, input: &mut Input)
    {
        let player_texture = graphics.load_texture("src/image/player.png", FilterMode::Linear, FilterMode::Linear);
        self.player.set_texture(player_texture);
        register_keys(input);

        graphics.load_shader(Some("src/shaders/rope.wgsl"), None, PipeLineType::Normal);
        let pp_id = graphics.load_shader(Some("src/shaders/post_process.wgsl"), Some("src/shaders/post_process.wgsl"), PipeLineType::PostProcess);
        ctx.set_post_process_pipeline(pp_id);

        self.rope.build_mesh(graphics.renderer, graphics.device, graphics.queue);
    }

    fn physics_update(&mut self, update_ctx: &mut UpdateContext)
    {
        let (rope_anchor, rope_max_reach) = self.rope.current_reach();
        self.player.update(update_ctx.dt as f32, rope_anchor, rope_max_reach);

        self.rope.update(980.0, self.player.collision.pos, update_ctx.dt as f32); //980, as 100px = 1m
        self.rope.update_mesh(update_ctx.graphics.renderer, update_ctx.graphics.device, update_ctx.graphics.queue);

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
    }

    fn render(&self, render_ctx: &mut RenderContext)
    {
        self.wall.draw(render_ctx, 1, 0);
        self.player.draw(render_ctx, 1, 0);
        self.rope.draw(render_ctx, 1, 1);
    }
}

impl App
{
    fn new() -> Self
    {
        let player = Player::new(Vec2::new(0.0, 0.0), 128.0, 128.0, 30.0_f32.to_radians());
        let rope = Rope::new(Vec2::new(0.0, 0.0));

        Self
        {
            x: 0.0,
            y: 0.0,
            player,
            rope,
            wall: Wall::new(1280.0*2.0)
        }
    }
}

pub fn register_keys(input: &mut Input)
{
    input.add_key_binding(Key::Space, Some(Action::PlaceCheckPoint), None, None);
    input.add_key_binding(Key::KeyF, None, Some(Action::StartFalling), None);
    input.add_key_binding(Key::KeyW, None, None, Some(Action::MoveUp));
    input.add_key_binding(Key::KeyA, None, None, Some(Action::MoveLeft));
    input.add_key_binding(Key::KeyD, None, None, Some(Action::MoveRight));
}

fn main()
{
    pollster::block_on(game_loop(Box::new(App::new()), "Climbing Game", (1280, 720)));
}

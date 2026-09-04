pub mod player;
pub mod rope;
pub mod wall;
pub mod collectible;

use engine::{utility::DrawLayer, *};

use crate::{player::Player, rope::Rope, wall::Wall, collectible::{CollectibleManager, CollectibleKind}};
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
    App::player_move_right,
    App::toggle_rope_extending
];

struct App
{
    x: f32,
    y: f32,
    player: Player,
    rope: Rope,
    wall: Wall,
    collectibles: CollectibleManager,
    rope_extending: bool,
    rope_extending_toggle: no_if::button::Button
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

    }

    fn mouse_left_hold(&mut self, _ctx: &mut UpdateContext)
    {

    }


    fn player_place_checkpoint(&mut self, ctx: &mut UpdateContext)
    {
        const PLACE_TABLE: [fn(&mut App, &mut UpdateContext); 2] =
        [
            App::place_nothing,
            App::place_anchor
        ];

        let on_wall = self.wall.contains(self.player.collision.pos) as usize;
        let falling = !self.player.is_falling() as usize; // Later maybe instead of not allowing that, only dont, when player fell to much
        PLACE_TABLE[on_wall&falling](self, ctx);
    }
    fn place_nothing(&mut self, _: &mut UpdateContext) {}
    fn place_anchor(&mut self, ctx: &mut UpdateContext)
    {
        self.rope.add_anchor(ctx.graphics.renderer, ctx.graphics.device, ctx.graphics.queue, 1);
        self.player.score -= 2;
    }

    fn player_start_falling(&mut self, _ctx: &mut UpdateContext) { self.player.start_falling(); }

    fn player_move_up(&mut self, _ctx: &mut UpdateContext) { self.player.move_up(); }
    fn player_move_left(&mut self, _ctx: &mut UpdateContext) { self.player.move_left(); }
    fn player_move_right(&mut self, _ctx: &mut UpdateContext) { self.player.move_right(); }

    fn skip_rope_growth(&mut self, _: &mut UpdateContext, _: f32) {}
    fn do_rope_growth(&mut self, ctx: &mut UpdateContext, difference: f32)
    {
        self.rope.grow_active_segment(ctx.graphics.renderer, ctx.graphics.device, ctx.graphics.queue);
        self.player.rope_reserve -= difference;
    }

    fn toggle_rope_extending(&mut self, _: &mut UpdateContext)
    {
        self.rope_extending = !self.rope_extending;
    }
}

impl EngineEvent for App
{
    fn setup(&mut self, ctx: &mut Context, graphics: &mut GraphicsContext, input: &mut Input)
    {
        register_keys(input);

        let player_texture = graphics.load_texture("src/bin/game/assets/player.png", FilterMode::Linear, FilterMode::Linear);
        self.player.set_texture(player_texture);

        let rope_coil_texture = graphics.load_texture("src/bin/game/assets/rope_coil.png", FilterMode::Linear, FilterMode::Linear);
        self.collectibles.set_texture(CollectibleKind::RopeCoil, rope_coil_texture);
        for _ in 0..10
        {
            self.collectibles.spawn_rope_coil(&self.wall, self.player.collision.pos, 853.0, 2000.0, 200.0); // value in cm
        }

        let wall_border_texture = graphics.load_texture("src/bin/game/assets/border_right.png", FilterMode::Linear, FilterMode::Linear);
        self.wall.set_border_right_texture(wall_border_texture);

        let rope_toggle_button_texture = graphics.load_texture("src/image/button.png", FilterMode::Linear, FilterMode::Linear);
        self.rope_extending_toggle.set_texture(rope_toggle_button_texture);

        graphics.load_shader(Some("src/shaders/rope.wgsl"), None, PipeLineType::Normal);
        let pp_id = graphics.load_shader(Some("src/shaders/post_process.wgsl"), Some("src/shaders/post_process.wgsl"), PipeLineType::PostProcess);
        ctx.set_post_process_pipeline(pp_id);

        self.rope.build_mesh(graphics.renderer, graphics.device, graphics.queue);

        graphics.set_clear_color([0.13, 0.4, 0.76, 1.0]);
    }

    fn physics_update(&mut self, update_ctx: &mut UpdateContext)
    {
        let (rope_anchor, rope_max_reach) = self.rope.current_reach();
        self.player.update(update_ctx.dt as f32, rope_anchor, rope_max_reach, self.wall.get_bounds());

        self.collectibles.update(&self.wall, self.player.collision.pos, 800.0, update_ctx.dt as f32);
        self.collectibles.check_collection(&mut self.player, 100.0);

        const GROWTH_TABLE: [fn(&mut App, &mut UpdateContext, f32); 2] = [App::skip_rope_growth, App::do_rope_growth];
        let (has_rope, difference) = self.player.try_consume_rope_for_growth(rope_anchor, rope_max_reach, self.rope.segment_length);
        let should_grow = has_rope & self.rope_extending;
        GROWTH_TABLE[should_grow as usize](self, update_ctx, difference);

        self.rope.update(980.0, self.player.collision.pos, update_ctx.dt as f32); //1960 as 200px = 1m  x980, as 100px = 1m
        self.rope.update_mesh(update_ctx.graphics.renderer, update_ctx.graphics.device, update_ctx.graphics.queue);

        self.rope_extending_toggle.update(update_ctx.input);

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
        self.collectibles.draw(render_ctx, 2, 0);
        self.player.draw(render_ctx, 2, 0);
        self.rope.draw(render_ctx, 2, 1);

        let score_text = format!("Score: {}", self.player.score);
        let current_height_text = format!("Height: {:.2}m", -self.player.collision.pos.y/200.0);
        let rope_text = format!("Rope left: {}m\ntest newline?", self.player.rope_reserve/200.0);
        let (top_left, width, height) = render_ctx.graphics.renderer.text_bounds(&score_text, (5.0, 5.0), 48.0);
        let center = (top_left.0 + width * 0.5, top_left.1 + height * 0.5);
        render_ctx.graphics.renderer.draw_ui(0, render_ctx.graphics.renderer.ui_matrix(center, (width, height), 0.0), [0.0, 1.0, 0.0, 1.0], 4, 0);
        render_ctx.graphics.renderer.draw_text(render_ctx.graphics.device, render_ctx.graphics.queue, &score_text, (5.0, 5.0), 48.0, [1.0, 0.0, 1.0, 1.0], CoordSpace::Screen, DrawLayer::UI, 5, 0);
        render_ctx.graphics.renderer.draw_text(render_ctx.graphics.device, render_ctx.graphics.queue, &current_height_text, (5.0, 53.0), 48.0, [1.0, 1.0, 1.0, 1.0], CoordSpace::Screen, DrawLayer::UI, 5, 0);
        render_ctx.graphics.renderer.draw_text(render_ctx.graphics.device, render_ctx.graphics.queue, &rope_text, (5.0, 101.0), 48.0, [1.0, 1.0, 1.0, 1.0], CoordSpace::Screen, DrawLayer::UI, 5, 0);
        let (top_left, width, height) = render_ctx.graphics.renderer.text_bounds(&rope_text, (5.0, 101.0), 48.0);
        let center = (top_left.0 + width * 0.5, top_left.1 + height * 0.5);
        render_ctx.graphics.renderer.draw_ui(0, render_ctx.graphics.renderer.ui_matrix(center, (width, height), 0.0), [0.0, 1.0, 1.0, 1.0], 4, 0);

        render_ctx.graphics.renderer.draw(0, render_ctx.graphics.renderer.matrix((0.0, -100.0), (200.0, 200.0), 0.0), [0.0, 0.0, 1.0, 1.0], 1, 0);
        render_ctx.graphics.renderer.draw(0, render_ctx.graphics.renderer.matrix((0.0, -300.0), (200.0, 200.0), 0.0), [1.0, 0.0, 0.0, 1.0], 1, 0);
        render_ctx.graphics.renderer.draw(0, render_ctx.graphics.renderer.matrix((0.0, -500.0), (200.0, 200.0), 0.0), [0.0, 1.0, 0.0, 1.0], 1, 0);
        render_ctx.graphics.renderer.draw(0, render_ctx.graphics.renderer.matrix((0.0, -700.0), (200.0, 200.0), 0.0), [0.0, 0.0, 1.0, 1.0], 1, 0);
        render_ctx.graphics.renderer.draw(0, render_ctx.graphics.renderer.matrix((0.0, -900.0), (200.0, 200.0), 0.0), [1.0, 0.0, 0.0, 1.0], 1, 0);

        self.rope_extending_toggle.draw(render_ctx, 5, 0);
    }
}


// 200px = 1m
impl App
{
    fn new() -> Self
    {
        let player = Player::new(Vec2::new(0.0, 0.0), 128.0, 128.0, 30.0_f32.to_radians());
        let rope = Rope::new(Vec2::new(0.0, 0.0));
        let collectibles = CollectibleManager::new();
        let mut rope_extending_toggle = no_if::button::Button::new(Rect::new(10.0, 120.0, 100.0, 50.0));
        rope_extending_toggle.set_action(ButtonEvent::Click, Action::ToggleRopeExtending);

        Self
        {
            x: 0.0,
            y: 0.0,
            player,
            rope,
            wall: Wall::new(1280.0*2.0),
            collectibles,
            rope_extending: true,
            rope_extending_toggle
        }
    }
}

pub fn register_keys(input: &mut Input)
{
    input.add_key_binding(Key::Space, Some(Action::PlaceCheckPoint), None, None);
    // input.add_mouse_binding(Button::Left, Some(Action::PlaceCheckPoint), None, None);
    input.add_key_binding(Key::KeyF, None, Some(Action::StartFalling), None);
    input.add_key_binding(Key::KeyW, None, None, Some(Action::MoveUp));
    input.add_key_binding(Key::KeyA, None, None, Some(Action::MoveLeft));
    input.add_key_binding(Key::KeyD, None, None, Some(Action::MoveRight));
}

fn main()
{
    pollster::block_on(game_loop(Box::new(App::new()), "Climbing Game", (1280, 720)));
}

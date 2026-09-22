pub mod player;
pub mod rope;
pub mod wall;
pub mod collectible;
pub mod decorations;
pub mod hazard;

use engine::{utility::DrawLayer, *};

use crate::{collectible::{CollectibleKind, CollectibleManager}, decorations::DecorationSpawner, hazard::{HazardMovement, HazardSpawner, HazardState}, player::Player, rope::Rope, wall::Wall};
use rand::Rng;

type ActionFn = fn(&mut App, &mut UpdateContext);
type RenderFn = fn(&App, &mut RenderContext);

#[derive(Copy, Clone, PartialEq)]
enum GameState
{
    MainMenu,
    MainMenuSettings,
    Playing,
    Paused,
    Dead,
}
impl GameState { const COUNT: usize = 5; }


const GAME_UPDATE_TABLE: [ActionFn; GameState::COUNT] =
[
    App::update_main_menu,
    App::update_menu_settings,
    App::update_world,
    App::update_pause_menu,
    App::update_dead,
];

const GAME_RENDER_TABLE: [RenderFn; GameState::COUNT] =
[
    App::draw_main_menu,
    App::draw_main_menu_settings,
    App::draw_playing,
    App::draw_paused,
    App::draw_dead,
];


const fn base_actions() -> [ActionFn; Action::COUNT]
{
    let mut table: [ActionFn; Action::COUNT] = [App::no_op; Action::COUNT];
    table[Action::ToggleFullScreen as usize] = App::toggle_fullscreen;
    table
}

const fn main_menu_actions() -> [ActionFn; Action::COUNT]
{
    let mut table = base_actions();
    table[Action::StartGame as usize] = App::start_game;
    table[Action::OpenSettings as usize] = App::open_settings;
    table
}

const fn main_menu_settings_actions() -> [ActionFn; Action::COUNT]
{
    let mut table = base_actions();
    table[Action::Escape as usize] = App::back_to_main_menu;
    table[Action::BackToMainMenu as usize] = App::back_to_main_menu;
    table
}

const fn playing_actions() -> [ActionFn; Action::COUNT]
{
    let mut table = base_actions();
    table[Action::Escape as usize] = App::pause_game;
    table[Action::MouseLeftPressed as usize] = App::mouse_left_pressed;
    table[Action::MouseLeftReleased as usize] = App::mouse_left_released;
    table[Action::MouseLeftHold as usize] = App::mouse_left_hold;
    table[Action::ToggleWallShader as usize] = App::toggle_wall_shader;
    table[Action::PlaceCheckPoint as usize] = App::player_place_checkpoint;
    table[Action::StartFalling as usize] = App::player_start_falling;
    table[Action::MoveUp as usize] = App::player_move_up;
    table[Action::MoveLeft as usize] = App::player_move_left;
    table[Action::MoveRight as usize] = App::player_move_right;
    table[Action::ToggleRopeExtending as usize] = App::toggle_rope_extending;
    table
}

const fn paused_actions() -> [ActionFn; Action::COUNT]
{
    let mut table = base_actions();
    table[Action::Escape as usize] = App::resume_game;
    table[Action::ResumeGame as usize] = App::resume_game;
    table[Action::RestartGame as usize] = App::restart_game;
    table
}

const fn dead_actions() -> [ActionFn; Action::COUNT]
{
    let mut table = base_actions();
    table[Action::RestartGame as usize] = App::restart_game;
    table
}

const ACTION_TABLES: [[ActionFn; Action::COUNT]; GameState::COUNT] =
[
    main_menu_actions(),
    main_menu_settings_actions(),
    playing_actions(),
    paused_actions(),
    dead_actions(),
];


// const ACTION_TABLE: [ActionFn; Action::COUNT] =
// [
//     App::toggle_fullscreen,
//     App::escape,
//     App::mouse_left_pressed,
//     App::mouse_left_released,
//     App::mouse_left_hold,
//     App::toggle_wall_shader,
//     App::player_place_checkpoint,
//     App::player_start_falling,
//     App::player_move_up,
//     App::player_move_left,
//     App::player_move_right,
//     App::toggle_rope_extending
// ];

struct App
{
    x: f32,
    y: f32,
    player: Player,
    rope: Rope,
    wall: Wall,
    collectibles: CollectibleManager,
    rope_extending: bool,
    rope_extending_toggle: no_if::button::Button,
    current_wall_shader: usize,
    decorations: DecorationSpawner,
    hazards: HazardSpawner,
    game_state: GameState,
    restart_button: no_if::button::Button,
    blur_texture: usize,
    death_g_force: f32,
    menu_climb: f32,
    vignette_id: usize,
    blur_id: usize,
    start_button: no_if::button::Button
}

impl App
{
    fn no_op(&mut self, _ctx: &mut UpdateContext) {}

    fn toggle_fullscreen(&mut self, ctx: &mut UpdateContext)
    {
        ctx.context.toggle_fullscreen();
    }

    fn mouse_left_pressed(&mut self, _ctx: &mut UpdateContext)
    {

    }

    fn mouse_left_released(&mut self, _ctx: &mut UpdateContext)
    {

    }

    fn mouse_left_hold(&mut self, _ctx: &mut UpdateContext)
    {

    }


    fn start_game(&mut self, ctx: &mut UpdateContext)
    {
        self.game_state = GameState::Playing;
        ctx.context.set_post_process_pipeline(self.vignette_id);
    }

    fn open_settings(&mut self, _ctx: &mut UpdateContext) { self.game_state = GameState::MainMenuSettings; }

    fn back_to_main_menu(&mut self, ctx: &mut UpdateContext)
    {
        self.game_state = GameState::MainMenu;
        ctx.context.set_post_process_pipeline(self.blur_id);
        ctx.context.add_post_process_pipeline(self.vignette_id);
    }

    fn resume_game(&mut self, ctx: &mut UpdateContext)
    {
        self.game_state = GameState::Playing;
        ctx.context.set_post_process_pipeline(self.vignette_id);
    }

    fn pause_game(&mut self, ctx: &mut UpdateContext)
    {
        self.game_state = GameState::Paused;
        ctx.context.set_post_process_pipeline(self.blur_id);
        ctx.context.add_post_process_pipeline(self.vignette_id);
    }

    fn restart_game(&mut self, ctx: &mut UpdateContext)
    {
        self.player.reset();
        self.rope.reset_rope(ctx.graphics.renderer, ctx.graphics.device, ctx.graphics.queue, Vec2::ZERO);
        let mut rng = rand::rng();
        ctx.graphics.set_uniform("seed", UniformValue::Float(rng.random()));

        self.start_game(ctx);
    }

    fn no_state_change(&mut self, _ctx: &mut UpdateContext) {}
    fn kill_player(&mut self, ctx: &mut UpdateContext)
    {
        self.death_g_force = self.player.last_deceleration / 1960.0;
        self.game_state = GameState::Dead;

        ctx.context.set_post_process_pipeline(self.blur_id);
        ctx.context.add_post_process_pipeline(self.vignette_id);
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
        let add_anchor = self.rope.add_anchor(ctx.graphics.renderer, ctx.graphics.device, ctx.graphics.queue, 1);
        self.player.score -= 2 * add_anchor as i32;
    }

    fn player_start_falling(&mut self, _ctx: &mut UpdateContext) { self.player.start_falling(Vec2::ZERO); }

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

    fn skip_hit(&mut self, _direction: Vec2) {}
    fn apply_hit(&mut self, direction: Vec2)
    {
        self.player.start_falling(direction * 800.0);
    }

    fn toggle_wall_shader(&mut self, ctx: &mut UpdateContext)
    {
        self.current_wall_shader = (self.current_wall_shader + 1) % 3;

        const LOAD_SHADER: [fn(&mut App, &mut UpdateContext); 3] =
        [
            App::wall_shader_cracks,
            App::wall_shader_bands,
            App::wall_shader_fast
        ];

        LOAD_SHADER[self.current_wall_shader](self, ctx);
    }

    fn wall_shader_cracks(&mut self, ctx: &mut UpdateContext)
    {
        ctx.graphics.replace_shader_with_uniforms(Some("src/shaders/wall_shader/wall_shader.wgsl"), None, PipeLineType::Normal, &[("scale", UniformType::Float), ("band_height", UniformType::Float), ("tilt_strength", UniformType::Float), ("crack_density", UniformType::Float)], self.wall.get_current_shader_id());
        ctx.graphics.set_uniform("scale", UniformValue::Float(150.0));
        ctx.graphics.set_uniform("band_height", UniformValue::Float(200.0));
        ctx.graphics.set_uniform("tilt_strength", UniformValue::Float(1.0));
        ctx.graphics.set_uniform("crack_density", UniformValue::Float(0.1));
    }

    fn wall_shader_bands(&mut self, ctx: &mut UpdateContext)
    {
        ctx.graphics.replace_shader_with_uniforms(Some("src/shaders/wall_shader/wall_shader_bands.wgsl"), None, PipeLineType::Normal, &[("scale", UniformType::Float), ("band_height", UniformType::Float), ("tilt_strength", UniformType::Float), ("seed", UniformType::Float)], self.wall.get_current_shader_id());
        ctx.graphics.set_uniform("scale", UniformValue::Float(150.0));
        ctx.graphics.set_uniform("band_height", UniformValue::Float(200.0));
        ctx.graphics.set_uniform("tilt_strength", UniformValue::Float(1.0));
        let mut rng = rand::rng();
        ctx.graphics.set_uniform("seed", UniformValue::Float(rng.random()));
    }

    fn wall_shader_fast(&mut self, ctx: &mut UpdateContext)
    {
        ctx.graphics.replace_shader_with_uniforms(Some("src/shaders/wall_shader/wall_shader_fast.wgsl"), None, PipeLineType::Normal, &[("band_height", UniformType::Float)], self.wall.get_current_shader_id());
        ctx.graphics.set_uniform("band_height", UniformValue::Float(200.0));
    }
}

// Seperation, just all game-state functions
impl App
{
    fn update_main_menu(&mut self, ctx: &mut UpdateContext)
    {
        let x_value = (self.menu_climb * 0.05).sin() * 900.0;
        ctx.graphics.set_camera_pos((x_value, 0.0 - self.menu_climb * 60.0));
        self.menu_climb += ctx.dt as f32;
        self.decorations.maintain(&self.wall, Vec2::new(x_value, -self.menu_climb*60.0), -1.0, 400.0, 720.0, 20);

        self.start_button.update(ctx.input);
    }

    fn update_menu_settings(&mut self, _ctx: &mut UpdateContext) {}

    fn update_world(&mut self, update_ctx: &mut UpdateContext)
    {
        let (rope_anchor, rope_max_reach) = self.rope.current_reach();
        let nearby: Vec<_> = self.decorations.nearby_solid_rects(self.player.collision.pos).collect();
        self.player.update(update_ctx.dt as f32, rope_anchor, rope_max_reach, self.wall.get_bounds(), &nearby);

        self.collectibles.update(&self.wall, self.player.collision.pos, 800.0, update_ctx.dt as f32);
        self.collectibles.check_collection(&mut self.player, 100.0);

        const GROWTH_TABLE: [fn(&mut App, &mut UpdateContext, f32); 2] = [App::skip_rope_growth, App::do_rope_growth];
        let (has_rope, difference) = self.player.try_consume_rope_for_growth(rope_anchor, rope_max_reach, self.rope.segment_length);
        let should_grow = has_rope & self.rope_extending;
        GROWTH_TABLE[should_grow as usize](self, update_ctx, difference);

        self.rope.update(980.0, self.player.collision.pos, update_ctx.dt as f32); //1960 as 200px = 1m  x980, as 100px = 1m
        self.rope.reclaim_visible_splits(update_ctx.graphics.renderer, update_ctx.graphics.device, update_ctx.graphics.queue);
        self.rope.update_mesh(update_ctx.graphics.renderer, update_ctx.graphics.device, update_ctx.graphics.queue);

        self.decorations.maintain(&self.wall, self.player.collision.pos, self.player.direction_y(), 400.0, 720.0, 20);
        self.hazards.maintain(&self.wall, self.player.collision.pos, update_ctx.dt as f32);

        let (player_hit, knockback_dir) = self.hazards.check_hit(self.player.collision.pos, self.player.hit_radius());
        const HIT_TABLE: [fn(&mut App, Vec2); 2] = [App::skip_hit, App::apply_hit];
        HIT_TABLE[player_hit as usize](self, knockback_dir);

        self.rope_extending_toggle.update(update_ctx.input);

        const DEATH_TABLE: [fn(&mut App, &mut UpdateContext); 2] = [App::no_state_change, App::kill_player];
        DEATH_TABLE[self.player.is_beyond_recovery() as usize * (1-(self.game_state == GameState::Dead) as usize)](self, update_ctx);
    }

    fn update_pause_menu(&mut self, _ctx: &mut UpdateContext) {}

    fn update_dead(&mut self, ctx: &mut UpdateContext)
    {
        self.update_world(ctx);
        self.restart_button.update(ctx.input);
    }

    fn draw_main_menu(&self, render_ctx: &mut RenderContext)
    {
        self.wall.draw(render_ctx, 1);
        self.decorations.draw(render_ctx, 2, 0);

        // render_ctx.graphics.renderer.draw_tinted_texture(0, render_ctx.graphics.renderer.ui_matrix((640.0, 360.0), (1280.0, 720.0), 0.0), self.blur_texture, [0.5, 0.5, 0.5, 1.0], 3, 0);
        self.start_button.draw(render_ctx, 3, 0);
    }

    fn draw_main_menu_settings(&self, _render_ctx: &mut RenderContext) {}

    fn draw_world(&self, render_ctx: &mut RenderContext)
    {
        self.wall.draw(render_ctx, 1);
        self.collectibles.draw(render_ctx, 2, 0);
        self.decorations.draw(render_ctx, 2, 0);
        self.player.draw(render_ctx, 3, 0);
        self.rope.draw(render_ctx, 3, 1);
        self.hazards.draw(render_ctx, 3, 0);
    }

    fn draw_hud(&self, render_ctx: &mut RenderContext)
    {
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

        self.rope_extending_toggle.draw(render_ctx, 5, 0);
    }

    fn draw_playing(&self, render_ctx: &mut RenderContext)
    {
        self.draw_world(render_ctx);
        self.draw_hud(render_ctx);
    }

    fn draw_paused(&self, render_ctx: &mut RenderContext)
    {
        self.draw_world(render_ctx);

        // render_ctx.graphics.renderer.draw_texture(0, render_ctx.graphics.renderer.ui_matrix((640.0, 360.0), (1280.0, 720.0), 0.0), self.blur_texture, 5, 0);

        render_ctx.graphics.renderer.draw_text_centered_outline(render_ctx.graphics.device, render_ctx.graphics.queue, "Paused", (640.0, 100.0), 120.0, [0.7, 0.09, 0.09, 1.0], [0.0, 0.0, 0.0, 1.0], 2.0, CoordSpace::Screen, DrawLayer::UI, 6, 0);
    }

    fn draw_dead(&self, render_ctx: &mut RenderContext)
    {
        self.draw_world(render_ctx);

        // render_ctx.graphics.renderer.draw_texture(0, render_ctx.graphics.renderer.ui_matrix((640.0, 360.0), (1280.0, 720.0), 0.0), self.blur_texture, 5, 0);

        let score_str = format!("Score: {}", self.player.score);
        let g_force_str = format!("G-force: {}", self.death_g_force);

        render_ctx.graphics.renderer.draw_text_centered_outline(render_ctx.graphics.device, render_ctx.graphics.queue, "You died", (640.0, 200.0), 115.0, [0.43, 0.09, 0.09, 1.0], [0.0, 0.0, 0.0, 1.0], 2.0, CoordSpace::Screen, DrawLayer::UI, 6, 0);
        render_ctx.graphics.renderer.draw_text_centered_outline(render_ctx.graphics.device, render_ctx.graphics.queue, &score_str, (450.0, 300.0), 48.0, [0.7, 0.7, 0.7, 1.0], [0.0, 0.0, 0.0, 1.0], 1.0, CoordSpace::Screen, DrawLayer::UI, 6, 0);
        render_ctx.graphics.renderer.draw_text_centered_outline(render_ctx.graphics.device, render_ctx.graphics.queue, &g_force_str, (830.0, 300.0), 48.0, [0.7, 0.7, 0.7, 1.0], [0.0, 0.0, 0.0, 1.0], 1.0, CoordSpace::Screen, DrawLayer::UI, 6, 0);
        self.restart_button.draw(render_ctx, 6, 0);
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
        self.restart_button.set_texture(rope_toggle_button_texture);
        self.start_button.set_texture(rope_toggle_button_texture);

        let decorations_texture = graphics.load_texture("src/bin/game/assets/temp_decorations_atlas.png", FilterMode::Linear, FilterMode::Linear);
        self.decorations.set_texture(decorations_texture);
        self.decorations.add_variant((0.0, 0.0), (128.0, 128.0), 40.0, true); // rock
        self.decorations.add_variant((256.0, 0.0), (222.0, 159.0), 40.0, false); // grass 1
        self.decorations.add_variant((0.0, 256.0), (329.0, 159.0), 40.0, false); // grass 2

        let hazard_texture = graphics.load_texture("src/bin/game/assets/hazard_items.png", FilterMode::Linear, FilterMode::Linear);
        let warning_texture = graphics.load_texture("src/bin/game/assets/warning.png", FilterMode::Linear, FilterMode::Linear);
        self.hazards.set_hazard_texture(hazard_texture);
        self.hazards.set_warning_texture(warning_texture);
        self.hazards.add_kind(HazardMovement::FallFromTop, (0.0, 0.0), (298.0, 291.0), 256.0, 100.0, 980.0, 2.0, 128.0, HazardState::Tumbling);

        self.blur_texture = graphics.load_texture("src/bin/game/assets/blur.png", FilterMode::Linear, FilterMode::Linear);

        graphics.load_shader(Some("src/shaders/rope.wgsl"), None, PipeLineType::Normal);
        let rock_shader = graphics.load_shader_with_uniform(Some("src/shaders/wall_shader/wall_shader_bands.wgsl"), None, PipeLineType::Normal, &[("scale", UniformType::Float), ("band_height", UniformType::Float), ("tilt_strength", UniformType::Float), ("seed", UniformType::Float)]);
        graphics.set_uniform("scale", UniformValue::Float(150.0));
        graphics.set_uniform("band_height", UniformValue::Float(200.0));
        graphics.set_uniform("tilt_strength", UniformValue::Float(1.0));
        let mut rng = rand::rng();
        graphics.set_uniform("seed", UniformValue::Float(rng.random()));
        self.wall.set_rock_shader(rock_shader as u8);

        let vignette_pipeline = graphics.load_shader(Some("src/shaders/post_process.wgsl"), Some("src/shaders/post_process.wgsl"), PipeLineType::PostProcess);
        self.vignette_id = vignette_pipeline;
        ctx.set_post_process_pipeline(vignette_pipeline);

        graphics.set_uniform("radius", UniformValue::Float(3.0));
        let blur_pipeline = graphics.load_shader_with_uniform(Some("src/shaders/blur_post_process.wgsl"), Some("src/shaders/blur_post_process.wgsl"), PipeLineType::PostProcess, &[("radius", UniformType::Float)]);
        self.blur_id = blur_pipeline;
        ctx.add_post_process_pipeline(blur_pipeline);

        self.rope.build_mesh(graphics.renderer, graphics.device, graphics.queue);

        graphics.set_clear_color([0.13, 0.4, 0.76, 1.0]);
    }

    fn physics_update(&mut self, update_ctx: &mut UpdateContext)
    {
        GAME_UPDATE_TABLE[self.game_state as usize](self, update_ctx);

        let actions = update_ctx.input.actions().to_vec();
        for action in actions
        {
            // ACTION_TABLE[action as usize](self, update_ctx);
            ACTION_TABLES[self.game_state as usize][action as usize](self, update_ctx);
        }
    }

    fn update(&mut self, update_ctx: &mut UpdateContext)
    {
        self.x = update_ctx.input.mouse_position().0 as f32;
        self.y = update_ctx.input.mouse_position().1 as f32;
    }

    fn render(&self, render_ctx: &mut RenderContext)
    {
        GAME_RENDER_TABLE[self.game_state as usize](self, render_ctx);
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
        let mut rope_extending_toggle = no_if::button::Button::new(Rect::new(10.0, 202.0, 100.0, 50.0));
        rope_extending_toggle.set_action(ButtonEvent::Click, Action::ToggleRopeExtending);

        let mut restart_button = no_if::button::Button::new(Rect::new(590.0, 350.0, 100.0, 50.0));
        restart_button.set_action(ButtonEvent::Click, Action::RestartGame);

        let mut start_button = no_if::button::Button::new(Rect::new(590.0, 350.0, 100.0, 50.0));
        start_button.set_action(ButtonEvent::Click, Action::StartGame);


        Self
        {
            x: 0.0,
            y: 0.0,
            player,
            rope,
            wall: Wall::new(1280.0*2.0),
            collectibles,
            rope_extending: true,
            rope_extending_toggle,
            current_wall_shader: 1,
            decorations: DecorationSpawner::new(),
            hazards: HazardSpawner::new(3.0, 6.0),
            game_state: GameState::MainMenu,
            restart_button,
            blur_texture: 0,
            death_g_force: 0.0,
            menu_climb: 0.0,
            vignette_id: 0,
            blur_id: 0,
            start_button
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
    input.add_key_binding(Key::Tab, Some(Action::ToggleWallShader), None, None);

    input.add_key_binding(Key::Escape, Some(Action::Escape), None, None);
}

fn main()
{
    pollster::block_on(game_loop(Box::new(App::new()), "Climbing Game", (1280, 720)));
}

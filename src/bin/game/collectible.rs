use engine::*;
use rand::Rng;

use crate::player::Player;
use crate::wall::Wall;

// Currently all collectibles are the same
const IDLE_AMPLITUDE: f32 = 0.12; // +- size wobble
const IDLE_SPEED: f32 = 2.5;
const COLLECT_DURATION: f32 = 0.35;
const COLLECT_POP: f32 = 0.6; // small size increase when collecting
const BASE_SIZE: f32 = 110.0;

#[derive(Copy, Clone, PartialEq)]
pub enum CollectibleKind
{
    RopeCoil,
}

impl CollectibleKind { pub const COUNT: usize = 1; }

#[derive(Copy, Clone, PartialEq)]
enum CollectibleState
{
    Inactive,
    Idle,
    Collecting,
}

impl CollectibleState { const COUNT: usize = 3; }

type StateUpdateFn = fn(&mut Collectible, f32);

const STATE_UPDATE_TABLE: [StateUpdateFn; CollectibleState::COUNT] =
[
    Collectible::update_inactive,
    Collectible::update_idle,
    Collectible::update_collecting,
];

type ScaleFn = fn(&Collectible) -> f32;

const SCALE_TABLE: [ScaleFn; CollectibleState::COUNT] =
[
    Collectible::scale_inactive,
    Collectible::scale_idle,
    Collectible::scale_collecting,
];

type CollectEffectFn = fn(&mut Player, f32);

// Each collectible has its own
const COLLECT_EFFECT_TABLE: [CollectEffectFn; CollectibleKind::COUNT] =
[
    Collectible::apply_rope_coil,
];

pub struct Collectible
{
    kind: CollectibleKind,
    state: CollectibleState,
    pos: Vec2,
    value: f32,
    texture_id: usize,
    idle_phase: f32,
    collect_timer: f32,
}

impl Collectible
{
    fn inactive() -> Self
    {
        Self
        {
            kind: CollectibleKind::RopeCoil,
            state: CollectibleState::Inactive,
            pos: Vec2::ZERO,
            value: 0.0,
            texture_id: 0,
            idle_phase: 0.0,
            collect_timer: 0.0,
        }
    }

    fn activate(&mut self, kind: CollectibleKind, pos: Vec2, value: f32, texture_id: usize)
    {
        self.kind = kind;
        self.pos = pos;
        self.value = value;
        self.texture_id = texture_id;
        self.state = CollectibleState::Idle;
        self.idle_phase = 0.0;
        self.collect_timer = 0.0;
    }

    fn update(&mut self, dt: f32)
    {
        STATE_UPDATE_TABLE[self.state as usize](self, dt);
    }

    fn update_inactive(&mut self, _dt: f32) {}

    fn update_idle(&mut self, dt: f32)
    {
        self.idle_phase += IDLE_SPEED * dt;
    }

    fn update_collecting(&mut self, dt: f32)
    {
        self.collect_timer += dt;

        const NEXT_STATE: [CollectibleState; 2] = [CollectibleState::Collecting, CollectibleState::Inactive];
        let finished = self.collect_timer > COLLECT_DURATION;
        self.state = NEXT_STATE[finished as usize];
    }

    fn collect(&mut self, player: &mut Player)
    {
        COLLECT_EFFECT_TABLE[self.kind as usize](player, self.value);

        self.state = CollectibleState::Collecting;
        self.collect_timer = 0.0;
    }

    fn apply_rope_coil(player: &mut Player, amount: f32)
    {

    }

    fn current_scale(&self) -> f32
    {
        SCALE_TABLE[self.state as usize](self)
    }

    fn scale_inactive(&self) -> f32 { 0.0 }

    fn scale_idle(&self) -> f32
    {
        1.0 + self.idle_phase.sin() * IDLE_AMPLITUDE
    }

    fn scale_collecting(&self) -> f32
    {
        let t = (self.collect_timer / COLLECT_DURATION).min(1.0);
        let pop = (t * std::f32::consts::PI).sin() * COLLECT_POP;

        (1.0 - t + pop).max(0.0)
    }

    fn draw(&self, render_ctx: &mut RenderContext, z_index: u32, shader_id: u8)
    {
        let scale = self.current_scale();
        let size = (BASE_SIZE * scale, BASE_SIZE * scale);

        render_ctx.graphics.renderer.draw_texture(0, render_ctx.graphics.renderer.matrix((self.pos.x, self.pos.y), size, 0.0), self.texture_id, z_index, shader_id);
    }
}

pub struct CollectibleManager
{
    pool: Vec<Collectible>,
    texture_ids: [usize; CollectibleKind::COUNT],
}

impl CollectibleManager
{
    pub fn new() -> Self
    {
        Self
        {
            pool: Vec::new(),
            texture_ids: [0; CollectibleKind::COUNT],
        }
    }

    pub fn set_texture(&mut self, kind: CollectibleKind, texture_id: usize)
    {
        self.texture_ids[kind as usize] = texture_id;
    }

    pub fn update(&mut self, dt: f32)
    {
        self.pool.iter_mut().for_each(|c| c.update(dt));
    }

    pub fn draw(&self, render_ctx: &mut RenderContext, z_index: u32, shader_id: u8)
    {
        self.pool.iter().filter(|c| (c.state as usize) > 0).for_each(|c| c.draw(render_ctx, z_index, shader_id));
    }

    pub fn check_collection(&mut self, player: &mut Player, collect_radius: f32)
    {
        let player_pos = player.collision.pos;
        self.pool.iter_mut().filter(|c| (c.state == CollectibleState::Idle) & ((c.pos - player_pos).length() < collect_radius)).for_each(|c| c.collect(player));
    }

    fn spawn(&mut self, kind: CollectibleKind, pos: Vec2, value: f32)
    {
        let texture_id = self.texture_ids[kind as usize];

        // Either reuses the first Inactive collectible, or if no exist adds a new one
        // No filter(), to avoid borrowing issues
        let index = self.pool.iter().position(|c| c.state == CollectibleState::Inactive).unwrap_or_else(||
        {
            self.pool.push(Collectible::inactive());
            self.pool.len() - 1
        });
        self.pool[index].activate(kind, pos, value, texture_id);
    }

    // Very basic spawning
    pub fn spawn_rope_coil(&mut self, wall: &Wall, near_y: f32, spread: f32, value: f32)
    {
        let bounds = wall.get_bounds();
        let margin = 40.0;
        let mut rng = rand::rng();

        let x = rng.random_range((bounds.0 + margin)..(bounds.1 - margin)) as f32;
        let y = near_y - rng.random_range(0.0_f32..spread);

        self.spawn(CollectibleKind::RopeCoil, Vec2::new(x, y), value);
    }

    pub fn amount(&self) -> usize
    {
        self.pool.len()
    }
}

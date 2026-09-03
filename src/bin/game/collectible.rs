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

const DESPAWN_MARGIN: f32 = 1000.0;

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

    fn deactivate(&mut self)
    {
        self.state = CollectibleState::Inactive;
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
        player.add_rope_reserve(amount);
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

    pub fn update(&mut self, wall: &Wall, player_pos: Vec2, spread: f32, dt: f32)
    {
        self.pool.iter_mut().for_each(|c| c.update(dt));

        self.pool.iter_mut().filter(|c| (c.state == CollectibleState::Idle) & (c.pos.y > player_pos.y + DESPAWN_MARGIN)).for_each(|c| c.deactivate());

        let deficit = 10_usize.saturating_sub(self.active_amount());

        (0..deficit).for_each(|_| self.spawn_rope_coil(wall, player_pos, 853.0, spread, 200.0));
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
    pub fn spawn_rope_coil(&mut self, wall: &Wall, player_pos: Vec2, std_dev: f32, spread: f32, value: f32)
    {
        let bounds = wall.get_bounds();
        let margin = 40.0;
        let mut rng = rand::rng();

        let raw_x = sample_gaussian(&mut rng, player_pos.x, std_dev);
        let x = raw_x.clamp(bounds.0 + margin, bounds.1 - margin);//rng.random_range((bounds.0 + margin)..(bounds.1 - margin)) as f32;
        let y = player_pos.y - rng.random_range(0.0_f32..spread) - 500.0; // -500, so it spawns above screen

        self.spawn(CollectibleKind::RopeCoil, Vec2::new(x, y), value);
    }

    pub fn amount(&self) -> usize
    {
        self.pool.len()
    }

    pub fn active_amount(&self) -> usize
    {
        self.pool.iter().filter(|c| c.state == CollectibleState::Idle).count()
    }
}

// Box-muller transform
// std_dev/standard-deviation in the same units as the man (so not from 0 to 1)
fn sample_gaussian(rng: &mut impl Rng, mean: f32, std_dev: f32) -> f32
{
    let u1: f32 = rng.random_range(f32::MIN_POSITIVE..1.0);
    let u2: f32 = rng.random_range(0.0..1.0);

    let z0 = (-2.0 * u1.ln()).sqrt() * (std::f32::consts::TAU * u2).cos();

    mean + std_dev * z0
}

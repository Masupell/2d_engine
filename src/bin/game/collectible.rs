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

const TARGET_ACTIVE: usize = 15; // Total amount of collectibles
const SPAWN_STD_DEV: f32 = 853.0;
const SPAWN_ABOVE_SCREEN: f32 = 500.0;
const WALL_MARGIN: f32 = 40.0;

#[derive(Copy, Clone, PartialEq)]
pub enum CollectibleKind
{
    RopeCoil,
    Score,
    DashOrb
}
impl CollectibleKind { pub const COUNT: usize = 3; }

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
    Collectible::add_score_point,
    Collectible::apply_dash_orb
];

type CanCollectFn = fn(&Player) -> bool;

const CAN_COLLECT_TABLE: [CanCollectFn; CollectibleKind::COUNT] =
[
    Collectible::always_collectable,
    Collectible::always_collectable,
    Player::can_gain_dash
];

const NEXT_COLLECT_STATE: [CollectibleState; CollectibleKind::COUNT] =
[
    CollectibleState::Collecting,
    CollectibleState::Collecting,
    CollectibleState::Inactive
];

type TryCollectFn = fn(&mut Collectible, &mut Player, &mut Vec<CollectEvent>);

const TRY_COLLECT_TABLE: [TryCollectFn; 2] =
[
    Collectible::skip_collect,
    Collectible::collect
];

// What got collected this frame
#[derive(Copy, Clone)]
pub struct CollectEvent
{
    pub kind: CollectibleKind,
    pub pos: Vec2, // world position at collection
    pub size: (f32, f32) // draw size at collection
}

pub struct Collectible
{
    kind: CollectibleKind,
    state: CollectibleState,
    pos: Vec2,
    value: f32,
    texture_id: usize,
    shader_id: u8,
    aspect: f32,
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
            shader_id: 0,
            aspect: 1.0,
            idle_phase: 0.0,
            collect_timer: 0.0
        }
    }

    fn activate(&mut self, entry: &SpawnEntry, pos: Vec2)
    {
        self.kind = entry.kind;
        self.pos = pos;
        self.value = entry.value;
        self.texture_id = entry.texture_id;
        self.shader_id = entry.shader_id;
        self.aspect = entry.aspect;
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

    fn skip_collect(&mut self, _player: &mut Player, _events: &mut Vec<CollectEvent>) {}
    fn collect(&mut self, player: &mut Player, events: &mut Vec<CollectEvent>)
    {
        COLLECT_EFFECT_TABLE[self.kind as usize](player, self.value);
        events.push(CollectEvent { kind: self.kind, pos: self.pos, size: self.draw_size() });

        self.state = NEXT_COLLECT_STATE[self.kind as usize];
        self.collect_timer = 0.0;
    }

    fn always_collectable(_player: &Player) -> bool { true }

    fn apply_rope_coil(player: &mut Player, amount: f32)
    {
        player.add_rope_reserve(amount);
    }

    fn add_score_point(player: &mut Player, amount: f32)
    {
        player.score += amount as i32;
    }

    fn apply_dash_orb(player: &mut Player, amount: f32)
    {
        player.add_dash(amount as i32);
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

    fn draw_size(&self) -> (f32, f32)
    {
        let scale = self.current_scale();
        let fit = BASE_SIZE * scale / self.aspect.max(1.0);
        (fit * self.aspect, fit)
    }

    fn draw(&self, render_ctx: &mut RenderContext, z_index: u32)
    {
        let size = self.draw_size();
        render_ctx.graphics.renderer.draw_texture(0, render_ctx.graphics.renderer.matrix((self.pos.x, self.pos.y), size, 0.0), self.texture_id, z_index, self.shader_id);
    }
}

#[derive(Copy, Clone)]
struct SpawnEntry
{
    kind: CollectibleKind,
    texture_id: usize,
    shader_id: u8,
    aspect: f32,
    value: f32,
    weight: f32,  // spawnchance
    credit: f32  // how likely it is to actually spawn
}

pub struct CollectibleManager
{
    pool: Vec<Collectible>,
    entries: Vec<SpawnEntry>,
    events: Vec<CollectEvent>
}

impl CollectibleManager
{
    pub fn new() -> Self
    {
        Self
        {
            pool: Vec::new(),
            entries: Vec::new(),
            events: Vec::new()
        }
    }

    // spawn_chance from 0.0..=1.0, percentage of spawns
    pub fn add_kind(&mut self, kind: CollectibleKind, texture_id: usize, texture_size: (f32, f32), shader_id: u8, value: f32, spawn_chance: f32)
    {
        let aspect = texture_size.0 / texture_size.1;
        self.entries.push(SpawnEntry { kind, texture_id, shader_id, aspect, value, weight: spawn_chance.max(0.0), credit: 0.0 });
    }

    pub fn initialize_spawn(&mut self, wall: &Wall, player_pos: Vec2, count: usize, spread: f32)
    {
        let count = count * self.can_spawn() as usize;
        (0..count).for_each(|_| self.spawn_next(wall, player_pos, spread));
    }

    pub fn update(&mut self, wall: &Wall, player_pos: Vec2, spread: f32, dt: f32)
    {
        self.pool.iter_mut().for_each(|c| c.update(dt));

        self.pool.iter_mut().filter(|c| (c.state == CollectibleState::Idle) & (c.pos.y > player_pos.y + DESPAWN_MARGIN)).for_each(|c| c.deactivate());

        let deficit = TARGET_ACTIVE.saturating_sub(self.active_amount()) * self.can_spawn() as usize;

        (0..deficit).for_each(|_| self.spawn_next(wall, player_pos, spread));
    }

    pub fn draw(&self, render_ctx: &mut RenderContext, z_index: u32)
    {
        self.pool.iter().filter(|c| (c.state as usize) > 0).for_each(|c| c.draw(render_ctx, z_index));
    }

    pub fn check_collection(&mut self, player: &mut Player, collect_radius: f32)
    {
        self.events.clear();

        let player_pos = player.collision.pos;
        let events = &mut self.events;
        self.pool.iter_mut().filter(|c| (c.state == CollectibleState::Idle) & ((c.pos - player_pos).length() < collect_radius)).for_each(|c|
        {
            let allowed = CAN_COLLECT_TABLE[c.kind as usize](player);
            TRY_COLLECT_TABLE[allowed as usize](c, player, events);
        });
    }

    pub fn collected(&self) ->&[CollectEvent]
    {
        &self.events
    }

    fn total_weight(&self) -> f32
    {
        self.entries.iter().map(|e| e.weight).sum()
    }

    fn can_spawn(&self) -> bool
    {
        self.total_weight() > 0.0
    }

    // picks index based on weight, removes total weight from picked index, so it has less of a chance of being spawned next time
    fn next_entry_index(&mut self) -> usize
    {
        let total = self.total_weight();
        self.entries.iter_mut().for_each(|e| e.credit += e.weight);
        let index = self.entries.iter().enumerate().max_by(|a, b| a.1.credit.total_cmp(&b.1.credit)).map(|(i, _)| i).unwrap_or(0);
        self.entries[index].credit -= total;
        index
    }

    fn spawn_next(&mut self, wall: &Wall, player_pos: Vec2, spread: f32)
    {
        let index = self.next_entry_index();
        let entry = self.entries[index];
        let pos = random_spawn_pos(wall, player_pos, SPAWN_STD_DEV, spread);

        self.spawn(entry, pos);
    }

    fn spawn(&mut self, entry: SpawnEntry, pos: Vec2)
    {
        // Either reuses the first Inactive collectible, or if no exist adds a new one
        // No filter(), to avoid borrowing issues
        let index = self.pool.iter().position(|c| c.state == CollectibleState::Inactive).unwrap_or_else(||
        {
            self.pool.push(Collectible::inactive());
            self.pool.len() - 1
        });
        self.pool[index].activate(&entry, pos);
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

// Very basic spawning
fn random_spawn_pos(wall: &Wall, player_pos: Vec2, std_dev: f32, spread: f32) -> Vec2
{
    let bounds = wall.get_bounds();
    let mut rng = rand::rng();

    let raw_x = sample_gaussian(&mut rng, player_pos.x, std_dev);
    let x = raw_x.clamp(bounds.0 + WALL_MARGIN, bounds.1 - WALL_MARGIN);
    let y = player_pos.y - rng.random_range(0.0_f32..spread) - SPAWN_ABOVE_SCREEN;

    Vec2::new(x, y)
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

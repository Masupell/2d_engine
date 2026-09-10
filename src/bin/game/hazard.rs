use std::f32;

use engine::*;
use rand::Rng;

use crate::wall::Wall;

#[derive(Copy, Clone)]
pub enum HazardState
{
    Inactive,
    Warning,
    Active,
    Tumbling // only tumbling for now
}
impl HazardState { const COUNT: usize = 4; }

#[derive(Copy, Clone)]
pub enum HazardMovement
{
    FallFromTop,
    ShootFromLeft,
    ShootFromRight,
}
impl HazardMovement { const COUNT: usize = 3; }

#[derive(Copy, Clone)]
struct HazardKind
{
    movement: HazardMovement,
    active_rect: ((f32, f32), (f32, f32)),
    draw_height: f32,
    speed: f32,
    gravity: f32,
    warning_duration: f32,
    hit_radius: f32,
    on_hit: HazardState
}

struct Hazard
{
    state: HazardState,
    kind_index: usize,
    pos: Vec2,
    velocity: Vec2,
    warning_progress: f32,
    highest_y: f32,
    hit: bool,
    rotation: f32,
    angular_velocity: f32
}

impl Hazard
{
    fn inactive() -> Self
    {
        Self
        {
            state: HazardState::Inactive,
            kind_index: 0,
            pos: Vec2::ZERO,
            velocity: Vec2::ZERO,
            warning_progress: 0.0,
            highest_y: 0.0,
            hit: false,
            rotation: 0.0,
            angular_velocity: 0.0
        }
    }

    fn update(&mut self, kind: &HazardKind, camera_pos: (f32, f32), dt: f32)
    {
        const UPDATE_TABLE: [fn(&mut Hazard, &HazardKind, (f32, f32), f32); HazardState::COUNT] =
        [
            Hazard::update_inactive,
            Hazard::update_warning,
            Hazard::update_active,
            Hazard::update_tumbling
        ];

        UPDATE_TABLE[self.state as usize](self, kind, camera_pos, dt);
    }

    fn update_inactive(&mut self, _kind: &HazardKind, _camera_pos: (f32, f32), _dt: f32) {}

    fn update_warning(&mut self, kind: &HazardKind, camera_pos: (f32, f32), dt: f32)
    {
        self.warning_progress += dt;
        self.highest_y = self.highest_y.min(camera_pos.1);

        let should_activate = (self.warning_progress > kind.warning_duration) as usize;

        const ACTIVATE_TABLE: [fn(&mut Hazard); 2] = [Hazard::keep_warning, Hazard::activate];
        ACTIVATE_TABLE[should_activate](self);
    }

    fn keep_warning(&mut self) {}
    fn activate(&mut self)
    {
        self.state = HazardState::Active;
    }

    fn update_active(&mut self, kind: &HazardKind, _camera_pos: (f32, f32), dt: f32)
    {
        self.velocity.y += kind.gravity * dt;
        self.pos += self.velocity * dt;
    }

    // Only one kind of after hit right now
    fn update_tumbling(&mut self, kind: &HazardKind, _camera_pos: (f32, f32), dt: f32)
    {
        self.velocity.y += kind.gravity * dt;
        self.pos += self.velocity * dt;
        self.rotation += self.angular_velocity * dt;
    }

    fn draw(&self, render_ctx: &mut RenderContext, normal_tex_id: usize, warning_tex_id: usize, kind: &HazardKind, z_index: u32, shader_id: u8)
    {
        const DRAW_TABLE: [fn(&Hazard, &mut RenderContext, usize, usize, &HazardKind, u32, u8); HazardState::COUNT] =
        [
            Hazard::draw_nothing,
            Hazard::draw_warning,
            Hazard::draw_active,
            Hazard::draw_tumbling
        ];

        DRAW_TABLE[self.state as usize](self, render_ctx, normal_tex_id, warning_tex_id, kind, z_index, shader_id);
    }

    fn draw_nothing(&self, _render_ctx: &mut RenderContext, _normal_tex_id: usize, _warning_tex_id: usize, _kind: &HazardKind, _z_index: u32, _shader_id: u8) {}

    fn draw_warning(&self, render_ctx: &mut RenderContext, _normal_tex_id: usize, warning_tex_id: usize, kind: &HazardKind, z_index: u32, shader_id: u8)
    {
        const WARNING_POS_TABLE: [fn(Vec2, f32, (f32, f32), (f32, f32), (f32, f32)) -> Vec2; HazardMovement::COUNT] =
        [
            warning_pos_fall_from_top,
            warning_pos_shoot_from_left,
            warning_pos_shoot_from_right,
        ];

        // let progress = self.warning_progress / kind.warning_duration;
        let animation_progress = (self.warning_progress / 0.5).min(1.0);
        let shake = (animation_progress * 8.0 * std::f32::consts::PI).sin() * (1.0 - animation_progress) * 0.15;
        let grow = (animation_progress * std::f32::consts::PI).sin() * 0.1 + 1.0;

        let rect_pos = (0.0, 0.0);
        let rect_pos_2 = (329.0, 0.0);
        let rect_size = (329.0, 270.0);
        let aspect = rect_size.0 / rect_size.1;
        let size = (kind.draw_height * aspect, kind.draw_height);
        let grow_size = (size.0 * grow, size.1 * grow);

        let camera = render_ctx.graphics.renderer.camera_pos;
        let screen_pos = WARNING_POS_TABLE[kind.movement as usize](self.pos, self.highest_y, camera, (1280.0, 720.0), size);

        render_ctx.graphics.renderer.draw_texture_atlas(0, render_ctx.graphics.renderer.matrix((screen_pos.x, screen_pos.y), size, shake), warning_tex_id, rect_pos, rect_size, z_index, shader_id);
        render_ctx.graphics.renderer.draw_texture_atlas(0, render_ctx.graphics.renderer.matrix((screen_pos.x, screen_pos.y), grow_size, 0.0), warning_tex_id, rect_pos_2, rect_size, z_index, shader_id);
    }

    fn draw_active(&self, render_ctx: &mut RenderContext, normal_tex_id: usize, _warning_tex_id: usize, kind: &HazardKind, z_index: u32, shader_id: u8)
    {
        let (rect_pos, rect_size) = kind.active_rect;
        let aspect = rect_size.0 / rect_size.1;
        let size = (kind.draw_height * aspect, kind.draw_height);

        let rotation = self.velocity.y.atan2(self.velocity.x);

        render_ctx.graphics.renderer.draw_texture_atlas(0, render_ctx.graphics.renderer.matrix((self.pos.x, self.pos.y), size, rotation), normal_tex_id, rect_pos, rect_size, z_index, shader_id);
    }

    fn draw_tumbling(&self, render_ctx: &mut RenderContext, normal_tex_id: usize, _warning_tex_id: usize, kind: &HazardKind, z_index: u32, shader_id: u8)
    {
        let (rect_pos, rect_size) = kind.active_rect;
        let aspect = rect_size.0 / rect_size.1;
        let size = (kind.draw_height * aspect, kind.draw_height);

        render_ctx.graphics.renderer.draw_texture_atlas(0, render_ctx.graphics.renderer.matrix((self.pos.x, self.pos.y), size, self.rotation), normal_tex_id, rect_pos, rect_size, z_index, shader_id);
    }
}

fn warning_pos_fall_from_top(hazard_pos: Vec2, highest_y: f32, _camera: (f32, f32), virtual_size: (f32, f32), draw_size: (f32, f32)) -> Vec2
{
    let margin = 10.0;
    Vec2::new(hazard_pos.x, highest_y - virtual_size.1 * 0.5 + draw_size.1/2.0 + margin)//camera.1 - virtual_size.1 * 0.5 + draw_size.1/2.0 + margin)
}

fn warning_pos_shoot_from_left(hazard_pos: Vec2, _highest_y: f32, camera: (f32, f32), virtual_size: (f32, f32), draw_size: (f32, f32)) -> Vec2
{
    let margin = 10.0;
    Vec2::new(camera.0 - virtual_size.0 * 0.5 + draw_size.0/2.0 + margin, hazard_pos.y)
}

fn warning_pos_shoot_from_right(hazard_pos: Vec2, _highest_y: f32, camera: (f32, f32), virtual_size: (f32, f32), draw_size: (f32, f32)) -> Vec2
{
    let margin = 10.0;
    Vec2::new(camera.0 + virtual_size.0 * 0.5 - draw_size.0/2.0 - margin, hazard_pos.y)
}


fn setup_fall_from_top(player_pos: Vec2, bounds: (f32, f32), speed: f32, rng: &mut rand::rngs::ThreadRng) -> (Vec2, Vec2)
{
    let margin = 20.0;
    let x = fold(sample_flat_distribution(rng, player_pos.x, 1000.0, 0.3), bounds.0 + margin, bounds.1 - margin);
    let pos = Vec2::new(x, player_pos.y - 700.0);

    (pos, Vec2::new(0.0, speed))
}

fn setup_shoot_from_left(player_pos: Vec2, bounds: (f32, f32), speed: f32, rng: &mut rand::rngs::ThreadRng) -> (Vec2, Vec2)
{
    let y = player_pos.y + rng.random_range(-100.0..100.0);
    let pos = Vec2::new(bounds.0 - 30.0, y);

    (pos, Vec2::new(speed, 0.0))
}

fn setup_shoot_from_right(player_pos: Vec2, bounds: (f32, f32), speed: f32, rng: &mut rand::rngs::ThreadRng) -> (Vec2, Vec2)
{
    let y = player_pos.y + rng.random_range(-100.0..100.0);
    let pos = Vec2::new(bounds.1 + 30.0, y);

    (pos, Vec2::new(-speed, 0.0))
}

pub struct HazardSpawner
{
    pool: Vec<Hazard>,
    normal_texture_id: usize,
    warning_texture_id: usize,
    kinds: Vec<HazardKind>,
    spawn_timer: f32,
    pub min_spawn_interval: f32,
    pub max_spawn_interval: f32,
}

impl HazardSpawner
{
    pub fn new(min_spawn_interval: f32, max_spawn_interval: f32) -> Self
    {
        Self
        {
            pool: Vec::new(),
            normal_texture_id: 0,
            warning_texture_id: 0,
            kinds: Vec::new(),
            spawn_timer: min_spawn_interval,
            min_spawn_interval,
            max_spawn_interval,
        }
    }

    pub fn set_hazard_texture(&mut self, texture_id: usize)
    {
        self.normal_texture_id = texture_id;
    }

    pub fn set_warning_texture(&mut self, texture_id: usize)
    {
        self.warning_texture_id = texture_id;
    }

    pub fn add_kind(&mut self, movement: HazardMovement, active_rect_pos: (f32, f32), active_rect_size: (f32, f32), draw_height: f32, speed: f32, gravity: f32, warning_duration: f32, hit_radius: f32, on_hit: HazardState) -> usize
    {
        self.kinds.push(HazardKind
        {
            movement,
            active_rect: (active_rect_pos, active_rect_size),
            draw_height,
            speed,
            gravity,
            warning_duration,
            hit_radius,
            on_hit
        });

        self.kinds.len() - 1
    }

    pub fn draw(&self, render_ctx: &mut RenderContext, z_index: u32, shader_id: u8)
    {
        self.pool.iter().for_each(|h| h.draw(render_ctx, self.normal_texture_id, self.warning_texture_id, &self.kinds[h.kind_index], z_index, shader_id));
    }

    // true if player is hit (only simple distance check)
    pub fn check_hit(&mut self, player_pos: Vec2, player_radius: f32) -> bool
    {
        let hit_index = (0..self.pool.len()).find(|&i|
        {
            let is_active = (self.pool[i].state as usize) > 1;
            let kind = self.kinds[self.pool[i].kind_index];
            let in_range = (self.pool[i].pos - player_pos).length() < kind.hit_radius + player_radius;

            is_active & in_range
        });

        hit_index.into_iter().for_each(|i|
        {
            let kind = self.kinds[self.pool[i].kind_index];
            let offset = self.pool[i].pos - player_pos;
            let away_from_player = offset * (1.0 / offset.length().max(0.0001));

            self.pool[i].hit = true;
            self.pool[i].state = kind.on_hit;
            self.pool[i].velocity = self.pool[i].velocity * 0.4 + away_from_player * 250.0;
            self.pool[i].angular_velocity = self.pool[i].velocity.x * 0.02;
        });

        hit_index.is_some()
    }

    fn skip_spawn(&mut self, _wall: &Wall, _player_pos: Vec2) {}

    fn spawn_random(&mut self, wall: &Wall, player_pos: Vec2)
    {
        let mut rng = rand::rng();

        self.spawn_timer = rng.random_range(self.min_spawn_interval..self.max_spawn_interval);

        let kind_index = rng.random_range(0..self.kinds.len());
        let kind = self.kinds[kind_index];
        let bounds = wall.get_bounds();

        const SETUP_TABLE: [fn(Vec2, (f32, f32), f32, &mut rand::rngs::ThreadRng) -> (Vec2, Vec2); HazardMovement::COUNT] =
        [
            setup_fall_from_top,
            setup_shoot_from_left,
            setup_shoot_from_right,
        ];

        let (pos, velocity) = SETUP_TABLE[kind.movement as usize](player_pos, bounds, kind.speed, &mut rng);

        let index = self.pool.iter().position(|h| (h.state as usize) < 1).unwrap_or_else(||
        {
            self.pool.push(Hazard::inactive());
            self.pool.len() - 1
        });

        self.pool[index] = Hazard
        {
            state: HazardState::Warning,
            kind_index,
            pos,
            velocity,
            warning_progress: 0.0,
            highest_y: player_pos.y, // should be camera, but has no acces to it right now, and camera and player are the same for now anyways
            hit: false,
            rotation: 0.0,
            angular_velocity: 0.0
        };
    }

    // Right now all hazards use same spawn timer
    pub fn maintain(&mut self, wall: &Wall, player_pos: Vec2, dt: f32)
    {
        self.spawn_timer -= dt;

        let should_spawn = (self.spawn_timer < 0.0) as usize;
        const SPAWN_TABLE: [fn(&mut HazardSpawner, &Wall, Vec2); 2] = [HazardSpawner::skip_spawn, HazardSpawner::spawn_random];
        SPAWN_TABLE[should_spawn](self, wall, player_pos);

        (0..self.pool.len()).for_each(|i|
        {
            let kind = self.kinds[self.pool[i].kind_index];
            self.pool[i].update(&kind, (player_pos.x, player_pos.y), dt); // here also camera, but is player for now
        });

        const DESPAWN_DISTANCE: f32 = 1200.0;

        self.pool.iter_mut().filter(|h| (h.state as usize > 0) & ((h.pos - player_pos).length() > DESPAWN_DISTANCE)).for_each(|h| h.state = HazardState::Inactive);
    }
}


fn sample_flat_distribution(rng: &mut impl Rng, mean: f32, spread: f32, flatness: f32) -> f32
{
    const SAMPLE_TABLE: [fn(f32, f32) -> f32; 2] =
    [
        sample_flat_center,
        sample_flat_edge,
    ];

    let flat_probability = 2.0 * flatness / (1.0 + flatness);
    let sample_type = rng.random_bool(flat_probability as f64) as usize;

    let rand_value = rng.random_range(0.0..1.0);
    let magnitude = SAMPLE_TABLE[sample_type](rand_value, flatness);

    let sign = [1.0, -1.0][rng.random_bool(0.5) as usize];

    mean + spread * sign * magnitude
}

fn sample_flat_center(rand_value: f32, flatness: f32) -> f32
{
    rand_value*flatness
}

fn sample_flat_edge(rand_value: f32, flatness: f32) -> f32
{
    flatness + (1.0 - flatness) * (1.0 - (1.0 - rand_value).sqrt())
}

fn fold(value: f32, min: f32, max: f32) -> f32
{
    let width = max - min;
    let x = (value - min).rem_euclid(2.0 * width);
    min + width - (x - width).abs()
}

use engine::{no_if::vector::Vec2, *};

// Climbing: Move up, left and right
// Falling: Swing using left and right, press up when at bottom, to climb again
#[derive(Copy, Clone, PartialEq)]
enum MovementState
{
    Climbing,
    Falling,
}

impl MovementState { const COUNT: usize = 2; }

type StateUpdateFn = fn(&mut Player, f32, Vec2, f32);

const STATE_UPDATE_TABLE: [StateUpdateFn; MovementState::COUNT] =
[
    Player::update_climbing,
    Player::update_falling,
];

pub struct Player
{
    pub collision: Triangle,
    width: f32,
    height: f32,
    texture_id: usize,
    max_rotation: f32,

    velocity: Vec2,
    move_input: Vec2,
    state: MovementState,
    fall_origin: Vec2,

    pub speed: f32,
    pub swing_thrust: f32,
    pub gravity: f32,
    pub max_survivable_fall: f32,
    pub tilt_per_velocity: f32,
    pub tilt_smoothing: f32,
    pub recovery_tolerance: f32,

    pub score: i32,
    actions: [Option<Action>; PlayerEvent::COUNT],
}

impl Player
{
    pub fn new(center: Vec2, width: f32, height: f32, max_rotation: f32) -> Self
    {
        // Expects it in local coordinates
        let a = Vec2::new(0.0, -height/2.0); // top point
        let b = Vec2::new(width/2.0, height/2.0); // bottom-right
        let c = Vec2::new(-width/2.0, height/2.0); // bottom-left
        let collision: Triangle = Triangle::new(center, 0.0, a, b, c);

        Player
        {
            collision,
            width,
            height,
            texture_id: 0,
            max_rotation,
            velocity: Vec2::ZERO,
            move_input: Vec2::ZERO,
            state: MovementState::Climbing,
            fall_origin: Vec2::ZERO,
            speed: 300.0,
            swing_thrust: 900.0,
            gravity: 980.0,
            max_survivable_fall: 600.0,
            tilt_per_velocity: 0.0025,
            tilt_smoothing: 0.05,
            recovery_tolerance: 15.0,
            score: 0,
            actions: [None; PlayerEvent::COUNT]
        }
    }

    pub fn set_action(&mut self, event: ButtonEvent, action: Action)
    {
        self.actions[event as usize] = Some(action)
    }

    pub fn update(&mut self, dt: f32, rope_anchor: Vec2, rope_max_reach: f32)
    {
        STATE_UPDATE_TABLE[self.state as usize](self, dt, rope_anchor, rope_max_reach);
        self.update_tilt(dt);
        self.move_input = Vec2::ZERO;
    }

    fn update_tilt(&mut self, dt: f32)
    {
        let target_rotation = (self.velocity.x * self.tilt_per_velocity).clamp(-self.max_rotation, self.max_rotation);
        let catch_up = 1.0 - self.tilt_smoothing.powf(dt);
        self.collision.rotation += (target_rotation - self.collision.rotation) * catch_up;
    }

    fn update_climbing(&mut self, dt: f32, rope_anchor: Vec2, rope_max_reach: f32)
    {
        let input_len = self.move_input.length();
        let move_dir = self.move_input * (1.0 / input_len.max(1.0));

        self.velocity = move_dir * self.speed;
        self.collision.change_pos(self.velocity * dt);

        self.constrain_to_rope(rope_anchor, rope_max_reach);
    }

    fn update_falling(&mut self, dt: f32, rope_anchor: Vec2, rope_max_reach: f32)
    {
        let offset = self.collision.pos - rope_anchor;
        let distance = offset.length();
        let radial_dir = offset * (1.0 / distance.max(0.0001)); // see constrain_to_rope - same zero-offset guard
        let tangent_dir = Vec2::new(radial_dir.y, -radial_dir.x);

        let caught = (distance > rope_max_reach) as u32 as f32;

        let acceleration = Vec2::new(0.0, self.gravity) + tangent_dir * (self.move_input.x * self.swing_thrust * caught);

        self.velocity += acceleration * dt;
        // self.velocity *= self.drag.powf(dt);
        self.collision.change_pos(self.velocity * dt);

        let slack_deficit = self.constrain_to_rope(rope_anchor, rope_max_reach);

        const RECOVERY_TABLE: [MovementState; 2] = [MovementState::Falling, MovementState::Climbing];
        let at_bottom = slack_deficit > -self.recovery_tolerance;
        let w_pressed = self.move_input.y < 0.0;
        let recover = at_bottom & w_pressed;

        self.state = RECOVERY_TABLE[recover as usize];
    }

    fn constrain_to_rope(&mut self, anchor: Vec2, max_reach: f32) -> f32
    {
        let offset = self.collision.pos - anchor;
        let distance = offset.length();
        let radial_dir = offset * (1.0 / distance.max(0.0001));

        let slack_deficit = distance - max_reach;
        let excess = slack_deficit.max(0.0);
        self.collision.change_pos(radial_dir * -excess);

        let beyond_limit = (slack_deficit > 0.0) as u32 as f32;
        let outward_speed = (self.velocity.x * radial_dir.x + self.velocity.y * radial_dir.y).max(0.0);
        self.velocity -= radial_dir * outward_speed * beyond_limit;

        slack_deficit
    }

    pub fn move_up(&mut self)
    {
        self.move_input.y -= 1.0;
    }

    pub fn move_left(&mut self)
    {
        self.move_input.x -= 1.0;
    }

    pub fn move_right(&mut self)
    {
        self.move_input.x += 1.0;
    }

    pub fn start_falling(&mut self)
    {
        self.state = MovementState::Falling;
        self.fall_origin = self.collision.pos;
        self.velocity = Vec2::ZERO;
    }

    pub fn is_falling(&self) -> bool
    {
        self.state == MovementState::Falling // ==
    }

    pub fn is_beyond_recovery(&self) -> bool
    {
        let fallen = (self.collision.pos - self.fall_origin).length();
        fallen > self.max_survivable_fall
    }

    pub fn draw(&self, render_ctx: &mut RenderContext, z_index: u32, shader_id: u8)
    {
        render_ctx.graphics.renderer.draw_texture(0, render_ctx.graphics.renderer.matrix((self.collision.pos.x, self.collision.pos.y), (self.width, self.height), self.collision.rotation), self.texture_id, z_index, shader_id);
        render_ctx.graphics.set_camera_pos((self.collision.pos.x, self.collision.pos.y)); // Basic Camera
    }

    pub fn set_texture(&mut self, texture_id: usize)
    {
        self.texture_id = texture_id;
    }

    pub fn set_pos(&mut self, pos: Vec2)
    {
        self.collision.pos += pos;
    }

    // Would not change collision, so dont do that yet
    pub fn set_size(&mut self, size: (f32, f32))
    {
        self.width = size.0;
        self.height = size.1;
    }
}

#[derive(Copy, Clone)]
pub enum PlayerEvent
{
    Checkpoint,
    Hit,
    Fall,
    Death,
    None
}

impl PlayerEvent { pub const COUNT: usize = 5; }

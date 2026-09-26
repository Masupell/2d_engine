use engine::{no_if::vector::Vec2, *};


const JIGGLE_DURATION: f32 = 0.3;
const JIGGLE_FREQUENCY: f32 = 60.0;
const JIGGLE_AMPLITUDE: f32 = 8.0;

// Climbing: Move up, left and right
// Falling: Swing using left and right, press up when at bottom, to climb again
#[derive(Copy, Clone, PartialEq)]
enum MovementState
{
    Climbing,
    Falling,
}

impl MovementState { const COUNT: usize = 2; }

type StateUpdateFn = fn(&mut Player, f32, Vec2, f32, (f32, f32), &[(Vec2, (f32, f32))]);

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
    last_direction_y: f32,
    move_input: Vec2,
    state: MovementState,

    pub speed: f32,
    pub swing_thrust: f32,
    pub gravity: f32,
    pub max_survivable_deceleration: f32,
    pub tilt_per_velocity: f32,
    pub tilt_smoothing: f32,
    pub max_recoverable_vel: f32,
    pub last_deceleration: f32,

    pub rope_reserve: f32,
    pub rope_grow_tolerance: f32,

    pub score: i32,
    score_progress: f32,
    highest_y: f32,
    actions: [Option<Action>; PlayerEvent::COUNT],

    pub max_dashes: i32,
    pub dash: i32, // amounts of dashes at once
    pub dash_speed: f32,
    pub dash_duration: f32,
    dash_requested: bool,
    dash_dir: Vec2,
    dash_timer: f32,
    jiggle_timer: f32,

    stun_timer: f32,
    stun_shader: u8
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
            last_direction_y: -1.0,
            move_input: Vec2::ZERO,
            state: MovementState::Climbing,
            speed: 80.0,
            swing_thrust: 900.0,
            gravity: 1960.0,//980.0,
            max_survivable_deceleration: 1960.0*50.0, //50g
            tilt_per_velocity: 0.0025,
            tilt_smoothing: 0.05,
            max_recoverable_vel: 400.0, //px per second, 200 = 1m => 400px -> 2m/s
            last_deceleration: 0.0,
            rope_reserve: 1000.0,
            rope_grow_tolerance: 10.0,
            score: 0,
            score_progress: 0.0,
            highest_y: center.y,
            actions: [None; PlayerEvent::COUNT],
            max_dashes: 3,
            dash: 3,
            dash_speed: 1400.0,
            dash_duration: 0.15,
            dash_requested: false,
            dash_dir: Vec2::ZERO,
            dash_timer: 0.0,
            jiggle_timer: 0.0,
            stun_timer: 0.0,
            stun_shader: 0
        }
    }

    pub fn set_action(&mut self, event: ButtonEvent, action: Action)
    {
        self.actions[event as usize] = Some(action)
    }

    pub fn add_rope_reserve(&mut self, amount: f32)
    {
        self.rope_reserve += amount;
    }

    pub fn try_consume_rope_for_growth(&mut self, anchor: Vec2, max_reach: f32, segment_length: f32) -> (bool, f32)
    {
        let distance = (self.collision.pos - anchor).length();

        let pushing_against_limit = distance > (max_reach - self.rope_grow_tolerance);
        let is_climbing = !self.is_falling();
        let has_reserve = self.rope_reserve > segment_length;

        let should_grow = pushing_against_limit & is_climbing & has_reserve;

        (should_grow, segment_length * (should_grow as u32 as f32))
    }

    pub fn dash(&mut self)
    {
        self.dash_requested = true;
    }

    pub fn add_dash(&mut self, amount: i32)
    {
        self.dash = (self.dash + amount).min(self.max_dashes);
    }

    pub fn can_gain_dash(&self) -> bool
    {
        self.dash < self.max_dashes
    }

    pub fn dash_count(&self) -> i32
    {
        self.dash
    }

    pub fn dash_request(&mut self)
    {
        let input_len = self.move_input.length();
        let dir = self.move_input * (1.0/input_len.max(0.0001));
        let has_dir = input_len > 0.0;
        let has_charge = self.dash > 0;

        let start = self.dash_requested & has_dir & has_charge;
        let denied = self.dash_requested & !has_charge;

        let start_i = start as i32;
        let start_f = start_i as f32;
        let denied_f = denied as u32 as f32;

        self.dash -= start_i;
        self.dash_dir = dir * start_f + self.dash_dir * (1.0 - start_f);
        self.dash_timer = self.dash_duration * start_f + self.dash_timer * (1.0 - start_f);
        self.jiggle_timer = JIGGLE_DURATION * denied_f + self.jiggle_timer * (1.0 - denied_f);

        self.dash_requested = false;
    }

    fn apply_dash(&mut self, dt: f32)
    {
        let step = dt.min(self.dash_timer);
        self.collision.change_pos(self.dash_dir * (self.dash_speed * step));
        self.dash_timer -= step;
    }

    pub fn update(&mut self, dt: f32, rope_anchor: Vec2, rope_max_reach: f32, wall_bounds: (f32, f32), nearby_solids: &[(Vec2, (f32, f32))])
    {
        let awake = !self.stunned();
        self.move_input = self.move_input * (awake as u32 as f32);
        self.dash_requested &= awake;

        self.dash_request();
        self.apply_dash(dt);

        STATE_UPDATE_TABLE[self.state as usize](self, dt, rope_anchor, rope_max_reach, wall_bounds, nearby_solids);
        self.update_tilt(dt);
        self.move_input = Vec2::ZERO;
        self.jiggle_timer = (self.jiggle_timer - dt).max(0.0);

        let upward = (self.highest_y - self.collision.pos.y).max(0.0);
        self.highest_y = self.highest_y.min(self.collision.pos.y);
        self.score_progress += upward;
        let point = (self.score_progress >= 200.0) as i32;
        self.score += point;
        self.score_progress -= 200.0 * point as f32;

        self.stun_timer = (self.stun_timer - dt).max(0.0);
    }

    fn update_tilt(&mut self, dt: f32)
    {
        let target_rotation = (self.velocity.x * self.tilt_per_velocity).clamp(-self.max_rotation, self.max_rotation);
        let catch_up = 1.0 - self.tilt_smoothing.powf(dt);
        self.collision.rotation += (target_rotation - self.collision.rotation) * catch_up;
    }

    fn resolve_solid_collision(&mut self, rect_pos: Vec2, rect_size: (f32, f32), dt: f32)
    {
        self.collision.triangle_rect_mtv(rect_pos, rect_size).into_iter().for_each(|mtv|
        {
            self.collision.change_pos(mtv);

            let push_dir = mtv * (1.0 / mtv.length().max(0.0001));
            let speed_along_push = self.velocity.dot(push_dir);
            let inward_amount = (-speed_along_push).max(0.0);
            self.velocity += push_dir * inward_amount;
            self.last_deceleration = inward_amount / dt.max(0.0001);
        });
    }

    fn update_climbing(&mut self, dt: f32, rope_anchor: Vec2, rope_max_reach: f32, wall_bounds: (f32, f32), nearby_solids: &[(Vec2, (f32, f32))])
    {
        let input_len = self.move_input.length();
        let move_dir = self.move_input * (1.0 / input_len.max(1.0));

        self.velocity = move_dir * self.speed;
        let pressed_up = (self.move_input.y < 0.0) as i32 as f32;
        self.last_direction_y = -pressed_up + self.last_direction_y * (1.0 - pressed_up);
        self.collision.change_pos(self.velocity * dt);

        self.constrain_to_rope(rope_anchor, rope_max_reach, dt);
        self.collision.pos.x = self.collision.pos.x.clamp(wall_bounds.0, wall_bounds.1);

        nearby_solids.iter().for_each(|&(rect_pos, rect_size)| self.resolve_solid_collision(rect_pos, rect_size, dt));
    }

    fn update_falling(&mut self, dt: f32, rope_anchor: Vec2, rope_max_reach: f32, wall_bounds: (f32, f32), nearby_solids: &[(Vec2, (f32, f32))])
    {
        let offset = self.collision.pos - rope_anchor;
        let distance = offset.length();
        let radial_dir = offset * (1.0 / distance.max(0.0001)); // see constrain_to_rope - same zero-offset guard
        let tangent_dir = Vec2::new(radial_dir.y, -radial_dir.x);

        let caught = (distance > rope_max_reach) as u32 as f32;

        let acceleration = Vec2::new(0.0, self.gravity) + tangent_dir * (self.move_input.x * self.swing_thrust * caught);

        self.velocity += acceleration * dt;
        // self.velocity *= self.drag.powf(dt);
        let falling_down = (self.velocity.y > 0.0) as i32 as f32;
        let falling_up = (self.velocity.y < 0.0) as i32 as f32;
        self.last_direction_y = falling_down - falling_up;
        self.collision.change_pos(self.velocity * dt);

        self.constrain_to_rope(rope_anchor, rope_max_reach, dt);

        nearby_solids.iter().for_each(|&(rect_pos, rect_size)| self.resolve_solid_collision(rect_pos, rect_size, dt));

        const RECOVERY_TABLE: [MovementState; 2] = [MovementState::Falling, MovementState::Climbing];
        let recoverable_speed = self.velocity.length() < self.max_recoverable_vel;
        let w_pressed = self.move_input.y < 0.0;
        let in_wall = (self.collision.pos.x >= wall_bounds.0) & (self.collision.pos.x <= wall_bounds.1);
        let recover = recoverable_speed & w_pressed & in_wall;

        self.state = RECOVERY_TABLE[recover as usize];
    }

    fn constrain_to_rope(&mut self, anchor: Vec2, max_reach: f32, dt: f32) -> f32
    {
        let offset = self.collision.pos - anchor;
        let distance = offset.length();
        let radial_dir = offset * (1.0 / distance.max(0.0001));

        let slack_deficit = distance - max_reach;
        let excess = slack_deficit.max(0.0);
        self.collision.change_pos(radial_dir * -excess);

        let beyond_limit = (slack_deficit > 0.0) as u32 as f32;
        let outward_speed = (self.velocity.x * radial_dir.x + self.velocity.y * radial_dir.y).max(0.0);
        let removed_speed = outward_speed * beyond_limit;
        self.velocity -= radial_dir * removed_speed;
        self.last_deceleration = removed_speed / dt.max(0.0001);

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

    pub fn start_falling(&mut self, impulse: Vec2)
    {
        self.state = MovementState::Falling;
        self.velocity += impulse;
    }

    pub fn is_falling(&self) -> bool
    {
        self.state == MovementState::Falling // ==
    }

    pub fn direction_y(&self) -> f32
    {
        self.last_direction_y
    }

    pub fn is_beyond_recovery(&self) -> bool
    {
        self.last_deceleration > self.max_survivable_deceleration
    }

    fn jiggle_offset(&self) -> f32
    {
        let fade = self.jiggle_timer / JIGGLE_DURATION;
        (self.jiggle_timer * JIGGLE_FREQUENCY).sin() * JIGGLE_AMPLITUDE * fade
    }

    pub fn draw(&self, render_ctx: &mut RenderContext, z_index: u32, shader_id: u8)
    {
        let draw_x = self.collision.pos.x + self.jiggle_offset();

        render_ctx.graphics.renderer.draw_texture(0, render_ctx.graphics.renderer.matrix((draw_x, self.collision.pos.y), (self.width, self.height), self.collision.rotation), self.texture_id, z_index, shader_id);
        render_ctx.graphics.set_camera_pos((self.collision.pos.x, self.collision.pos.y)); // Basic Camera

        const STARS_SIZE: (f32, f32) = (150.0, 75.0);
        let fade = (self.stun_timer / 0.3).min(1.0);
        let head_offset = -(self.height * 0.5 + 15.0);
        let (sin, cos) = self.collision.rotation.sin_cos();
        let stars_pos = (self.collision.pos.x - head_offset * sin, self.collision.pos.y + head_offset * cos);

        render_ctx.graphics.renderer.draw_tinted_texture(0, render_ctx.graphics.renderer.matrix(stars_pos, STARS_SIZE, self.collision.rotation), 0, [1.0, 1.0, 1.0, fade], z_index+1, self.stun_shader);
    }

    pub fn set_texture(&mut self, texture_id: usize)
    {
        self.texture_id = texture_id;
    }

    pub fn set_pos(&mut self, pos: Vec2)
    {
        self.collision.pos = pos;
    }

    // Would not change collision, so dont do that yet
    pub fn set_size(&mut self, size: (f32, f32))
    {
        self.width = size.0;
        self.height = size.1;
    }

    // could just return 64.0 here, but wif it ever changes
    pub fn hit_radius(&self) -> f32
    {
        self.width.min(self.height) * 0.5
    }

    pub fn stun(&mut self, duration: f32)
    {
        self.stun_timer = self.stun_timer.max(duration);
    }

    pub fn stunned(&self) -> bool
    {
        self.stun_timer > 0.0
    }

    pub fn set_stun_shader(&mut self, shader_id: u8)
    {
        self.stun_shader = shader_id;
    }

    pub fn velocity(&self) -> Vec2
    {
        self.velocity
    }

    pub fn reset(&mut self)
    {
        self.set_pos(Vec2::ZERO);
        self.velocity = Vec2::ZERO;
        self.state = MovementState::Climbing;
        self.last_direction_y = -1.0;
        self.move_input = Vec2::ZERO;
        self.last_deceleration = 0.0;
        self.rope_reserve = 1000.0;
        self.score = 0;
        self.score_progress = 0.0;
        self.highest_y = 0.0;
        self.dash = 3;
        self.dash_requested = false;
        self.dash_dir = Vec2::ZERO;
        self.dash_timer = 0.0;
        self.jiggle_timer = 0.0;
        self.stun_timer = 0.0;
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

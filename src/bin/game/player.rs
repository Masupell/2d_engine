use engine::{no_if::vector::Vec2, *};

pub struct Player
{
    pub collision: Triangle,

    width: f32,
    height: f32,

    texture_id: usize,

    rotation_speed: f32,
    max_rotation: f32, // in both directions from 0 degrees (0 being up in my case)
    velocity: Vec2,
    pub acceleration: f32,
    pub gravity: f32,
    pub drag: f32, // 0..1, lower is 'stickier'

    pub score: i32,
    actions: [Option<Action>; PlayerEvent::COUNT],
}

impl Player
{
    pub fn new(center: Vec2, width: f32, height: f32, rotation_speed: f32, max_rotation: f32) -> Self
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
            rotation_speed,
            max_rotation,
            velocity: Vec2::ZERO,
            acceleration: 1500.0,
            gravity: 980.0,
            drag: 0.15,
            score: 0,
            actions: [None; PlayerEvent::COUNT]
        }
    }

    pub fn set_action(&mut self, event: ButtonEvent, action: Action)
    {
        self.actions[event as usize] = Some(action)
    }

    pub fn update(&mut self, input: &Input, dt: f32, rope_anchor: Vec2, rope_max_reach: f32)
    {
        let forward = Vec2::new(self.collision.rotation.sin(), -self.collision.rotation.cos());

        let acceleration = forward * self.acceleration + Vec2::new(0.0, self.gravity);
        self.velocity += acceleration * dt;
        self.velocity *= self.drag.powf(dt); // to be frame rate independent

        self.collision.change_pos(self.velocity * dt);
        self.constrain_to_rope(rope_anchor, rope_max_reach);
    }

    fn constrain_to_rope(&mut self, anchor: Vec2, max_reach: f32)
    {
        let offset = self.collision.pos - anchor;
        let distance = offset.length();
        let radial_dir = offset.normalize();

        let excess = (distance - max_reach).max(0.0);
        self.collision.change_pos(radial_dir * -excess);

        let outward_speed = (self.velocity.x * radial_dir.x + self.velocity.y * radial_dir.y).max(0.0);
        self.velocity -= radial_dir * outward_speed * ((excess > 0.0) as i32) as f32;
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

    // In radians
    pub fn rotate(&mut self, amount: f32)
    {
        let new_rotation = self.collision.rotation + amount;
        self.collision.rotation = new_rotation.clamp(-self.max_rotation, self.max_rotation);
    }

    pub fn rotate_left(&mut self, dt: f32)
    {
        self.rotate(-self.rotation_speed * dt);
    }

    pub fn rotate_right(&mut self, dt: f32)
    {
        self.rotate(self.rotation_speed * dt);
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

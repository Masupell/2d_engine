use engine::*;

pub struct Rope
{
    pub points: Vec<RopePoint>,

    pub anchor: Vec2,

    pub rest_length: f32,
    pub max_length: f32,

    pub segment_length: f32
}

pub struct RopePoint
{
    pub pos: Vec2,
    pub prev_pos: Vec2
}


impl RopePoint
{
    pub fn new(pos: Vec2) -> Self
    {
        Self
        {
            pos,
            prev_pos: pos
        }
    }

    pub fn update(&mut self, gravity: f32, dt: f32)
    {
        // move particle
        let new_pos = 2.0*self.pos - self.prev_pos + Vec2::new(0.0, gravity) * dt*dt;
        self.prev_pos = self.pos;
        self.pos = new_pos;
    }

    pub fn draw(&self, render_ctx: &mut crate::RenderContext, z_index: u32, shader_id: u8)
    {
        render_ctx.renderer.draw_texture(0, render_ctx.renderer.matrix((self.pos.x, self.pos.y), (25.0, 25.0), 0.0), 0, z_index, shader_id);
    }
}

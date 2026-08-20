use engine::*;

pub struct Rope
{
    pub points: Vec<RopePoint>,

    pub anchor: Vec2,

    pub rest_length: f32,
    pub max_length: f32,

    pub segment_length: f32
}

impl Rope
{
    pub fn new(top_point: Vec2) -> Self
    {
        let segment_length = 30.0;
        let mut points = Vec::new();

        let angle: f32 = 0.4;
        for i in 0..=10
        {
            let position = top_point + Vec2::new(angle.cos() * i as f32 * segment_length, angle.sin() * i as f32 * segment_length);

            points.push(RopePoint::new(position));
        }

        Rope
        {
            points,
            anchor: top_point,
            rest_length: segment_length,
            max_length: segment_length+10.0,
            segment_length
        }
    }

    pub fn update(&mut self, gravity: f32, dt: f32)
    {
        for i in 1..self.points.len()
        {
            self.points[i].update(gravity, dt);
        }

        for _ in 0..5
        {
            self.solve_constraints();
        }

        self.points[0].pos = self.anchor;
        self.points[0].prev_pos = self.anchor;
    }

    fn solve_constraints(&mut self)
    {
        for i in 0..self.points.len() - 1
        {
            let delta = self.points[i+1].pos - self.points[i].pos;
            let distance = delta.length();
            if distance == 0.0 { continue; }

            let difference = (distance - self.segment_length) / distance;
            let correction = delta * difference;

            if i == 0
            {
                self.points[i+1].pos -= correction
            }
            else
            {
                self.points[i].pos += correction * 0.5;
                self.points[i+1].pos -= correction * 0.5;
            }
        }
        self.points[0].pos = self.anchor;
    }

    pub fn draw(&self, render_ctx: &mut crate::RenderContext, z_index: u32, shader_id: u8)
    {
        for point in self.points.iter()
        {
            render_ctx.graphics.renderer.draw_texture(0, render_ctx.graphics.renderer.matrix((point.pos.x, point.pos.y), (25.0, 25.0), 0.0), 0, z_index, shader_id);
        }
    }
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
        render_ctx.graphics.renderer.draw_texture(0, render_ctx.graphics.renderer.matrix((self.pos.x, self.pos.y), (25.0, 25.0), 0.0), 0, z_index, shader_id);
    }
}

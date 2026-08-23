use engine::*;

pub struct Rope
{
    pub points: Vec<RopePoint>,

    pub anchor: Vec2,

    pub rest_length: f32,
    pub max_length: f32,

    pub segment_length: f32,

    mesh_builder: MeshBuilder,
    mesh_id: Option<usize>,
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
            segment_length,
            mesh_builder: MeshBuilder::new(MeshTopology::Triangles),
            mesh_id: None
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
        const CORRECTION_TABLE: [(f32, f32); 2] =
        [
            (0.5, 0.5),
            (0.0, 1.0),
        ];

        for i in 0..self.points.len() - 1
        {
            let delta = self.points[i+1].pos - self.points[i].pos;
            let distance = delta.length();
            if distance == 0.0 { continue; } // if statement

            let difference = (distance - self.segment_length) / distance;
            let correction = delta * difference;

            let table_index = (i == 0) as usize; // ==
            let (first_factor, second_factor) = CORRECTION_TABLE[table_index];

            self.points[i].pos += correction * first_factor;
            self.points[i+1].pos -= correction * second_factor;
        }
        self.points[0].pos = self.anchor;
    }

    pub fn draw(&self, render_ctx: &mut crate::RenderContext, z_index: u32, shader_id: u8)
    {
        self.mesh_id.into_iter().for_each(|mesh_id|
        {
            render_ctx.graphics.renderer.draw_mesh(mesh_id, 0, z_index, shader_id);
        });
    }

    // Currently just uses device nd queue directly for testing
    pub fn build_mesh(&mut self, renderer: &mut crate::Renderer, device: &wgpu::Device, queue: &wgpu::Queue)
    {
        self.mesh_builder.clear();

        let width = 10.0;
        let half_width = width * 0.5;

        self.build_body(half_width);
        self.build_cap(0, half_width, true);
        self.build_cap(self.points.len()-1, half_width, false);

        self.mesh_id = Some(self.mesh_builder.build(renderer, device, queue));
    }


    fn build_body(&mut self, half_width: f32)
    {
        for i in 0..self.points.len() - 1
        {
            let current = self.points[i].pos;
            let next = self.points[i + 1].pos;
            let direction = (next - current).normalize();
            let normal = Vec2::new(-direction.y, direction.x);

            let left_current = current + normal * half_width;
            let right_current = current - normal * half_width;

            let left_next = next + normal * half_width;
            let right_next = next - normal * half_width;

            self.add_triangle(left_current, right_current, left_next);
            self.add_triangle(right_current, right_next, left_next);
        }
    }

    fn build_cap(&mut self, index: usize, radius: f32, start: bool)
    {
        const CAP_SEGMENTS: usize = 8;

        let center = self.points[index].pos;

        let previous = index.saturating_sub(1);

        let next = (index + 1).min(self.points.len() - 1);

        let direction = (self.points[next].pos - self.points[previous].pos).normalize();

        const CAP_DIRECTION: [f32; 2] =
        [
            1.0,
            -1.0,
        ];

        let direction = direction * CAP_DIRECTION[start as usize];

        let normal = Vec2::new(-direction.y, direction.x);

        for i in 0..CAP_SEGMENTS
        {
            let t0 = i as f32 / CAP_SEGMENTS as f32;
            let t1 = (i + 1) as f32 / CAP_SEGMENTS as f32;

            let angle0 = -std::f32::consts::FRAC_PI_2 +  t0 * std::f32::consts::PI;
            let angle1 = -std::f32::consts::FRAC_PI_2 + t1 * std::f32::consts::PI;

            let p0 = center + direction * angle0.cos() * radius + normal * angle0.sin() * radius;
            let p1 = center + direction * angle1.cos() * radius + normal * angle1.sin() * radius;

            self.add_triangle(center, p0, p1);
        }
    }

    fn add_triangle(&mut self, a: Vec2, b: Vec2, c: Vec2)
    {
        self.mesh_builder.add_vertex(VertexPosition::World((a.x, a.y)), (0.0, 0.0));
        self.mesh_builder.add_vertex(VertexPosition::World((b.x, b.y)), (0.0, 0.0));
        self.mesh_builder.add_vertex(VertexPosition::World((c.x, c.y)), (0.0, 0.0));
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
}

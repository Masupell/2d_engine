use engine::*;

pub struct Rope
{
    pub segments: Vec<RopeSegment>,

    pub segment_length: f32,
    pub rest_length: f32,
    pub max_length: f32,

    width: f32,
    pub view_buffer: f32,
    view_bounds: (Vec2, Vec2),

    pub max_active_points: usize,
}

impl Rope
{
    pub fn new(top_point: Vec2) -> Self
    {
        let segment_length = 30.0;
        let angle: f32 = 0.4;
        let direction = Vec2::new(angle.cos(), angle.sin());

        Rope
        {
            segments: vec![RopeSegment::new(top_point, direction, segment_length, 1)],
            segment_length,
            rest_length: segment_length,
            max_length: segment_length + 15.0,
            width: 10.0,
            view_buffer: 200.0,
            view_bounds: (top_point, top_point),
            max_active_points: 25,
        }
    }

    fn compute_view_bounds(player_pos: Vec2, buffer: f32) -> (Vec2, Vec2)
    {
        let half_extent = Vec2::new(1280.0 * 0.5 + buffer, 720.0 * 0.5 + buffer); //1280.0, 720.0 - centered on player

        (player_pos - half_extent, player_pos + half_extent)
    }

    pub fn update(&mut self, gravity: f32, player_pos: Vec2, dt: f32)
    {
        let segment_length = self.segment_length;

        self.view_bounds = Self::compute_view_bounds(player_pos, self.view_buffer);
        let (view_min, view_max) = self.view_bounds;

        self.segments.iter_mut().filter(|segment| segment.in_view(view_min, view_max)).for_each(|segment| segment.update(gravity, player_pos, segment_length, dt));
    }

    fn split_at_current_end(&mut self, renderer: &mut crate::Renderer, device: &wgpu::Device, queue: &wgpu::Queue, extra_points: usize)
    {
        let width = self.width;
        let segment_length = self.segment_length;
        let previous = self.segments.last_mut().unwrap();

        let anchor_pos = previous.end_pos();
        let direction = previous.end_direction(); // previous is better here, then actual player position
        previous.end_anchor = Some(anchor_pos);

        let mut segment = RopeSegment::new(anchor_pos, direction, segment_length, extra_points);

        let reused_id = renderer.reserve_mesh_slot();
        reused_id.into_iter().for_each(|id| segment.use_reused_mesh_slot(id));

        segment.build_mesh(renderer, device, queue, width);

        self.segments.push(segment);
    }

    pub fn add_anchor(&mut self, renderer: &mut crate::Renderer, device: &wgpu::Device, queue: &wgpu::Queue, extra_points: usize)
    {
        self.split_at_current_end(renderer, device, queue, extra_points);
    }

    // In case I need the anchor positions
    pub fn anchor_positions(&self) -> impl Iterator<Item = Vec2> + '_
    {
        self.segments.iter().map(|segment| segment.anchor_pos)
    }

    // Where the player is currently tethered from + the max stretch length, if everythings straight
    pub fn current_reach(&self) -> (Vec2, f32)
    {
        let active = self.segments.last().unwrap();
        let max_reach = (active.points.len()-1) as f32 * self.max_length;

        (active.anchor_pos, max_reach)
    }

    fn no_op_after_growth(&mut self, _renderer: &mut crate::Renderer, _device: &wgpu::Device, _queue: &wgpu::Queue, _split_index: usize) {}
    fn split_after_growth(&mut self, renderer: &mut crate::Renderer, device: &wgpu::Device, queue: &wgpu::Queue, split_index: usize)
    {
        let width = self.width;
        let active = self.segments.last_mut().unwrap();

        let mut new_segment = active.split_off_front(split_index);
        active.build_mesh(renderer, device, queue, width);

        let reused_id = renderer.reserve_mesh_slot();
        reused_id.into_iter().for_each(|id| new_segment.use_reused_mesh_slot(id));

        new_segment.build_mesh(renderer, device, queue, width);

        self.segments.push(new_segment);
    }

    pub fn grow_active_segment(&mut self, renderer: &mut crate::Renderer, device: &wgpu::Device, queue: &wgpu::Queue)
    {
        let segment_length = self.segment_length;
        let width = self.width;
        let max_active_points = self.max_active_points;
        let (view_min, view_max) = self.view_bounds;

        let active = self.segments.last_mut().unwrap();

        active.push_point(segment_length);
        active.build_mesh(renderer, device, queue, width);

        let too_long = active.points.len() > max_active_points;
        let split_index = active.find_offscreen_split_index(view_min, view_max);

        let should_split = too_long & split_index.is_some();

        const AFTER_GROWTH_TABLE: [fn(&mut Rope, &mut crate::Renderer, &wgpu::Device, &wgpu::Queue, usize); 2] =
        [
            Rope::no_op_after_growth,
            Rope::split_after_growth,
        ];

        AFTER_GROWTH_TABLE[should_split as usize](self, renderer, device, queue, split_index.unwrap_or(0));
    }

    // Removes anchors that got placed for optimization, keeps player-placed ones
    pub fn reclaim_visible_splits(&mut self, renderer: &mut crate::Renderer, device: &wgpu::Device, queue: &wgpu::Queue)
    {
        let (view_min, view_max) = self.view_bounds;
        let width = self.width;

        let merge_index = self.segments.iter().position(|segment| segment.auto_split & RopeSegment::point_in_view(segment.anchor_pos, view_min, view_max));

        merge_index.into_iter().for_each(|i|
        {
            self.merge_with_previous(i, renderer);
            self.segments[i - 1].build_mesh(renderer, device, queue, width);
        });
    }

    fn merge_with_previous(&mut self, index: usize, renderer: &mut crate::Renderer)
    {
        let removed = self.segments.remove(index);

        removed.mesh_id.into_iter().for_each(|id| renderer.free_mesh(id));

        let previous = &mut self.segments[index - 1];

        previous.points.pop();
        previous.points.extend(removed.points);

        previous.end_anchor = removed.end_anchor;
    }

    pub fn draw(&self, render_ctx: &mut crate::RenderContext, z_index: u32, shader_id: u8)
    {
        let (view_min, view_max) = self.view_bounds;
        self.segments.iter().filter(|segment| segment.in_view(view_min, view_max)).for_each(|segment| segment.draw(render_ctx, z_index, shader_id));
    }

    // Only builds meshes for newly added segements
    pub fn build_mesh(&mut self, renderer: &mut crate::Renderer, device: &wgpu::Device, queue: &wgpu::Queue)
    {
        let width = self.width;
        self.segments.iter_mut().filter(|segment| segment.mesh_id.is_none()).for_each(|segment| segment.build_mesh(renderer, device, queue, width));
    }

    // Only updates positions of segements in view
    pub fn update_mesh(&mut self, renderer: &mut crate::Renderer, device: &wgpu::Device, queue: &wgpu::Queue)
    {
        let width = self.width;
        let (view_min, view_max) = self.view_bounds;

        self.segments.iter_mut().filter(|segment| segment.in_view(view_min, view_max)).for_each(|segment| segment.update_mesh(renderer, device, queue, width));
    }
}

pub struct RopeSegment
{
    pub points: Vec<RopePoint>,
    pub anchor_pos: Vec2,

    // None, when following the player, Some(Vec2) when attached to anchor
    end_anchor: Option<Vec2>,

    // If it is a segment, created by player or auto-created
    auto_split: bool,

    mesh_builder: MeshBuilder,
    mesh_id: Option<usize>,
}

impl RopeSegment
{
    fn new(anchor_pos: Vec2, direction: Vec2, segment_length: f32, extra_points: usize) -> Self
    {
        let points = (0..=extra_points).map(|i| RopePoint::new(anchor_pos + direction * (i as f32 * segment_length))).collect();

        Self
        {
            points,
            anchor_pos,
            end_anchor: None,
            auto_split: false,
            mesh_builder: MeshBuilder::new(MeshTopology::Triangles),
            mesh_id: None,
        }
    }

    fn use_reused_mesh_slot(&mut self, mesh_id: usize)
    {
        self.mesh_builder = MeshBuilder::with_mesh_id(MeshTopology::Triangles, mesh_id);
    }

    fn split_off_front(&mut self, split_index: usize) -> RopeSegment
    {
        let split_pos = self.points[split_index].pos;

        let tail_points = self.points.split_off(split_index);
        self.points.push(RopePoint::new(split_pos));

        self.end_anchor = Some(split_pos);

        RopeSegment
        {
            points: tail_points,
            anchor_pos: split_pos,
            end_anchor: None,
            auto_split: true,
            mesh_builder: MeshBuilder::new(MeshTopology::Triangles),
            mesh_id: None,
        }
    }

    fn point_in_view(pos: Vec2, view_min: Vec2, view_max: Vec2) -> bool
    {
        let in_x = (pos.x > view_min.x) & (pos.x < view_max.x);
        let in_y = (pos.y > view_min.y) & (pos.y < view_max.y);

        in_x & in_y
    }

    // Searches from player point down, to the anchor thats no longer in view
    fn find_offscreen_split_index(&self, view_min: Vec2, view_max: Vec2) -> Option<usize>
    {
        self.points.iter().enumerate().filter(|(_, point)| !Self::point_in_view(point.pos, view_min, view_max)).map(|(i, _)| i).last()
    }

    // Bounding box over entire segment
    fn bounds(&self) -> (Vec2, Vec2)
    {
        self.points.iter().fold(
            (Vec2::new(f32::INFINITY, f32::INFINITY), Vec2::new(f32::NEG_INFINITY, f32::NEG_INFINITY)),
            |(min, max), point|
            (
                Vec2::new(min.x.min(point.pos.x), min.y.min(point.pos.y)),
                Vec2::new(max.x.max(point.pos.x), max.y.max(point.pos.y)),
            )
        )
    }

    fn in_view(&self, view_min: Vec2, view_max: Vec2) -> bool
    {
        let (seg_min, seg_max) = self.bounds();

        let overlap_x = (seg_min.x < view_max.x) & (seg_max.x > view_min.x);
        let overlap_y = (seg_min.y < view_max.y) & (seg_max.y > view_min.y);

        overlap_x & overlap_y
    }

    fn end_pos(&self) -> Vec2
    {
        self.points.last().unwrap().pos
    }

    fn push_point(&mut self, segment_length: f32)
    {
        self.points.push(RopePoint::new(self.end_pos() + self.end_direction() * segment_length));
    }

    // Need at least one new point when calling
    fn end_direction(&self) -> Vec2
    {
        let last = self.points.len() - 1;
        let prev = last.saturating_sub(1);

        (self.points[last].pos - self.points[prev].pos).normalize()
    }

    fn update(&mut self, gravity: f32, player_pos: Vec2, segment_length: f32, dt: f32)
    {
        let last = self.points.len() - 1;
        let tail_target = self.end_anchor.unwrap_or(player_pos);

        self.points.iter_mut().skip(1).for_each(|point| point.update(gravity, dt));

        let iterations = self.points.len().clamp(5, 20);
        (0..iterations).for_each(|i| SOLVE_PASS_TABLE[i % 2](self, segment_length));

        self.points[0].pos = self.anchor_pos;
        self.points[0].prev_pos = self.anchor_pos;

        self.points[last].pos = tail_target;
        self.points[last].prev_pos = tail_target;
    }

    fn solve_pair(&mut self, i: usize, last: usize, segment_length: f32)
    {
        const CORRECTION_TABLE: [(f32, f32); 4] =
        [
            (0.5, 0.5),
            (0.0, 1.0),
            (1.0, 0.0),
            (0.0, 0.0)
        ];

        const STIFFNESS: f32 = 0.25;

        let delta = self.points[i + 1].pos - self.points[i].pos;
        let distance = delta.length();

        let stretch = (distance - segment_length).max(0.0);
        let correction = delta.normalize() * stretch * STIFFNESS;

        let first_fixed = 1 - i.min(1);
        let second_fixed = 1 - (last - 1 - i).min(1);

        let index = first_fixed + second_fixed * 2;
        let (first_factor, second_factor) = CORRECTION_TABLE[index];

        self.points[i].pos += correction * first_factor;
        self.points[i + 1].pos -= correction * second_factor;
    }

    fn solve_forward(&mut self, segment_length: f32)
    {
        let last = self.points.len() - 1;

        for i in 0..last
        {
            self.solve_pair(i, last, segment_length);
        }
    }

    fn solve_backward(&mut self, segment_length: f32)
    {
        let last = self.points.len() - 1;

        for i in (0..last).rev()
        {
            self.solve_pair(i, last, segment_length);
        }
    }

    fn draw(&self, render_ctx: &mut crate::RenderContext, z_index: u32, shader_id: u8)
    {
        self.mesh_id.into_iter().for_each(|mesh_id|
        {
            render_ctx.graphics.renderer.draw_mesh(mesh_id, 0, z_index, shader_id);
        });
    }

    fn build_mesh(&mut self, renderer: &mut crate::Renderer, device: &wgpu::Device, queue: &wgpu::Queue, width: f32)
    {
        self.mesh_builder.clear();

        let half_width = width * 0.5;

        self.build_body(half_width);
        self.build_cap(0, half_width, true);
        self.build_cap(self.points.len() - 1, half_width, false);

        self.mesh_id = Some(self.mesh_builder.build(renderer, device, queue));
    }

    fn update_mesh(&mut self, renderer: &mut crate::Renderer, device: &wgpu::Device, queue: &wgpu::Queue, width: f32)
    {
        let half_width = width * 0.5;

        self.update_body(half_width);
        let body_vertex_count = (self.points.len() - 1) * 6;
        let cap_vertex_count = 8 * 3; // 8 segments, 3 vertices each
        self.update_cap(0, half_width, true, body_vertex_count);
        self.update_cap(self.points.len() - 1, half_width, false, body_vertex_count + cap_vertex_count);

        self.mesh_builder.update_vertices(renderer, device, queue);
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

            let x0 = i as f32;
            let x1 = (i + 1) as f32;

            self.add_triangle(left_current, right_current, left_next, Vec2::new(x0, 0.0), Vec2::new(x0, 1.0), Vec2::new(x1, 0.0));
            self.add_triangle(right_current, right_next, left_next, Vec2::new(x0, 1.0), Vec2::new(x1, 1.0), Vec2::new(x1, 0.0));
        }
    }

    fn update_body(&mut self, half_width: f32)
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

            let base = i * 6;

            self.mesh_builder.set_vertex_position(base, (left_current.x, left_current.y));
            self.mesh_builder.set_vertex_position(base+1, (right_current.x, right_current.y));
            self.mesh_builder.set_vertex_position(base+2, (left_next.x, left_next.y));
            self.mesh_builder.set_vertex_position(base+3, (right_current.x, right_current.y));
            self.mesh_builder.set_vertex_position(base+4, (right_next.x, right_next.y));
            self.mesh_builder.set_vertex_position(base+5, (left_next.x, left_next.y));
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
        let uv_x: [f32; 2] =
        [
            (self.points.len() - 1) as f32,
            0.0
        ];
        let x = uv_x[start as usize];

        let direction = direction * CAP_DIRECTION[start as usize];

        let normal = Vec2::new(-direction.y, direction.x);

        for i in 0..CAP_SEGMENTS
        {
            let t0 = i as f32 / CAP_SEGMENTS as f32;
            let t1 = (i + 1) as f32 / CAP_SEGMENTS as f32;

            let angle0 = -std::f32::consts::FRAC_PI_2 + t0 * std::f32::consts::PI;
            let angle1 = -std::f32::consts::FRAC_PI_2 + t1 * std::f32::consts::PI;

            let p0 = center + direction * angle0.cos() * radius + normal * angle0.sin() * radius;
            let p1 = center + direction * angle1.cos() * radius + normal * angle1.sin() * radius;

            let v0 = 0.5 + (angle0.sin() * 0.5) * CAP_DIRECTION[(1 - (start as i32)).abs() as usize];
            let v1 = 0.5 + (angle1.sin() * 0.5) * CAP_DIRECTION[(1 - (start as i32)).abs() as usize];

            self.add_triangle(center, p0, p1, Vec2::new(x, 0.5), Vec2::new(x, v0), Vec2::new(x, v1));
        }
    }

    fn update_cap(&mut self, index: usize, radius: f32, start: bool, vertex_offset: usize)
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

            let angle0 = -std::f32::consts::FRAC_PI_2 + t0 * std::f32::consts::PI;
            let angle1 = -std::f32::consts::FRAC_PI_2 + t1 * std::f32::consts::PI;

            let p0 = center + direction * angle0.cos() * radius + normal * angle0.sin() * radius;
            let p1 = center + direction * angle1.cos() * radius + normal * angle1.sin() * radius;

            let base = vertex_offset + i * 3;
            self.mesh_builder.set_vertex_position(base, (center.x, center.y));
            self.mesh_builder.set_vertex_position(base+1, (p0.x, p0.y));
            self.mesh_builder.set_vertex_position(base+2, (p1.x, p1.y));
        }
    }

    fn add_triangle(&mut self, a: Vec2, b: Vec2, c: Vec2, aa: Vec2, bb: Vec2, cc: Vec2)
    {
        self.mesh_builder.add_vertex((a.x, a.y), (aa.x, aa.y));
        self.mesh_builder.add_vertex((b.x, b.y), (bb.x, bb.y));
        self.mesh_builder.add_vertex((c.x, c.y), (cc.x, cc.y));
    }
}

type SolvePassFn = fn(&mut RopeSegment, f32);

const SOLVE_PASS_TABLE: [SolvePassFn; 2] =
[
    RopeSegment::solve_forward,
    RopeSegment::solve_backward,
];

pub struct RopePoint
{
    pub pos: Vec2,
    pub prev_pos: Vec2
}

impl RopePoint
{
    pub fn new(pos: Vec2) -> Self
    {
        Self { pos, prev_pos: pos }
    }

    pub fn update(&mut self, gravity: f32, dt: f32)
    {
        let new_pos = 2.0 * self.pos - self.prev_pos + Vec2::new(0.0, gravity) * dt * dt;
        self.prev_pos = self.pos;
        self.pos = new_pos;
    }
}

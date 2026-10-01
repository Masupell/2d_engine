use engine::{utility::DrawLayer, *};

pub const SEGMENTS: usize = 5;
pub const HEAD_RADIUS: f32 = 42.0;
pub const HEAD_LIFT: f32 = 19.0; // to make the rope be in the mouth

const RADII: [f32; SEGMENTS] = [HEAD_RADIUS, 37.0, 33.0, 29.0, 25.0];
const OVERLAP: f32 = 0.65; // smaller value for more, it gets multiplied
const DAMPING: f32 = 0.95;
const ITERATIONS: usize = 4;

const WIGGLE_AMOUNT: f32 = 5.0;
const WIGGLE_PER_PIXEL: f32 = 0.06;
const WIGGLE_PHASE: f32 = 1.3;
const LOOK_DISTANCE: f32 = 150.0;
const GROUND_FRICTION: f32 = 0.5;

pub const NO_GROUND: f32 = 1.0e30;

const BODY_CLIMB_SPEED: f32 = 220.0;
const LIE_SPEED: f32 = 0.01; // lower is faster
const LIE_VELOCITY: f32 = 0.2;

pub fn circle_rect_push(center: Vec2, radius: f32, rect_pos: Vec2, rect_size: (f32, f32)) -> Vec2
{
    let half = Vec2::new(rect_size.0 * 0.5, rect_size.1 * 0.5);
    let offset = center - rect_pos;

    let closest = rect_pos + Vec2::new(offset.x.clamp(-half.x, half.x), offset.y.clamp(-half.y, half.y));
    let away = center - closest;
    let distance = away.length();
    let outside_push = away * ((radius - distance).max(0.0) / distance.max(0.0001));

    let depth_x = half.x - offset.x.abs() + radius;
    let depth_y = half.y - offset.y.abs() + radius;
    let inside_push = [Vec2::new(0.0, depth_y * offset.y.signum()), Vec2::new(depth_x * offset.x.signum(), 0.0)][(depth_x < depth_y) as usize];

    let center_inside = (offset.x.abs() < half.x) & (offset.y.abs() < half.y);
    [outside_push, inside_push][center_inside as usize]
}

pub struct Caterpillar
{
    points: [Vec2; SEGMENTS],
    previous: [Vec2; SEGMENTS],
    crawl: f32, // head travel distance
    facing: f32 // 1 = right; -1 = left
}

// distance between i and segment before it
fn spacing(i: usize) -> f32
{
    (RADII[i - 1] + RADII[i]) * OVERLAP
}

fn over_the_top(angle: f32) -> f32
{
    angle - std::f32::consts::TAU * (angle >= std::f32::consts::FRAC_PI_2) as u32 as f32
}

impl Caterpillar
{
    pub fn new(head: Vec2) -> Self
    {
        let mut body = Self
        {
            points: [head; SEGMENTS],
            previous: [head; SEGMENTS],
            crawl: 0.0,
            facing: 1.0
        };
        body.reset(head);
        body
    }

    pub fn reset(&mut self, head: Vec2)
    {
        let head = head - Vec2::new(0.0, HEAD_LIFT);
        let mut y = head.y;

        self.points[0] = head;
        for i in 1..SEGMENTS
        {
            y += spacing(i);
            self.points[i] = Vec2::new(head.x, y);
        }

        self.previous = self.points;
        self.crawl = 0.0;
    }

    pub fn update(&mut self, head: Vec2, gravity: f32, dt: f32, solids: &[(Vec2, (f32, f32))], ground_y: f32, ground_half_width: f32)
    {
        let head = head - Vec2::new(0.0, HEAD_LIFT);

        let moved = head - self.points[0];
        self.crawl += moved.length();

        let direction = (moved.x > 0.3) as i32 as f32 - (moved.x < -0.3) as i32 as f32;
        self.facing = [self.facing, direction][(direction != 0.0) as usize];

        let head_on_ground = ((head.y + RADII[0]) >= ground_y - 2.0) & (head.x.abs() <= ground_half_width);
        let max_lift = BODY_CLIMB_SPEED * dt / ITERATIONS as f32;

        self.points[0] = head;
        self.previous[0] = head;

        for i in 1..SEGMENTS
        {
            let velocity = (self.points[i] - self.previous[i]) * DAMPING;
            self.previous[i] = self.points[i];
            self.points[i] = self.points[i] + velocity + Vec2::new(0.0, gravity * dt * dt);
        }

        for _ in 0..ITERATIONS
        {
            for i in 1..SEGMENTS
            {
                let offset = self.points[i] - self.points[i - 1];
                let distance = offset.length().max(0.0001);
                self.points[i] = self.points[i - 1] + offset * (spacing(i) / distance);
            }

            for i in 1..SEGMENTS
            {
                self.collide_solids(i, solids);
                self.collide_ground(i, ground_y, ground_half_width, head_on_ground, max_lift);
            }
        }

        let grounded = head_on_ground as u32 as f32;
        let lie = (1.0 - LIE_SPEED.powf(dt)) * grounded;

        for i in 1..SEGMENTS
        {
            let parent = self.points[i - 1];

            let lying = Vec2::new(-self.facing * spacing(i), RADII[i - 1] - RADII[i]);
            let on_ledge = ((parent.x + lying.x).abs() <= ground_half_width) as u32 as f32;

            let current = self.points[i] - parent;
            let current_angle = over_the_top(current.y.atan2(current.x));
            let target_angle = over_the_top(lying.y.atan2(lying.x));

            let angle = current_angle + (target_angle - current_angle) * lie * on_ledge;
            let rotated = parent + Vec2::new(angle.cos(), angle.sin()) * spacing(i);

            let delta = (rotated - self.points[i]) * grounded;
            self.points[i] += delta;
            self.previous[i] += delta * (1.0 - LIE_VELOCITY);
        }
    }

    fn collide_solids(&mut self, i: usize, solids: &[(Vec2, (f32, f32))])
    {
        for &(rect_pos, rect_size) in solids
        {
            let push = circle_rect_push(self.points[i], RADII[i], rect_pos, rect_size);
            self.points[i] += push;
            self.previous[i] += push;
        }
    }

    fn collide_ground(&mut self, i: usize, ground_y: f32, ground_half_width: f32, head_on_ground: bool, max_lift: f32)
    {
        let radius = RADII[i];
        let over_ledge = (self.points[i].x.abs() <= ground_half_width) as u32 as f32;
        let from_above = (self.previous[i].y + radius <= ground_y + 1.0) as u32 as f32;
        let pull_up = head_on_ground as u32 as f32;

        let penetration = (self.points[i].y + radius - ground_y).max(0.0) * over_ledge * from_above.max(pull_up);

        let push = [penetration.min(max_lift), penetration][(from_above > 0.0) as usize];
        self.points[i].y -= push;


        let touching = (penetration > 0.0) as u32 as f32;
        self.previous[i].y = self.previous[i].y * (1.0 - touching) + self.points[i].y * touching;
        let keep_x = 1.0 - touching * (1.0 - GROUND_FRICTION);
        self.previous[i].x = self.points[i].x - (self.points[i].x - self.previous[i].x) * keep_x;
    }

    pub fn head_center(&self) -> Vec2
    {
        self.points[0]
    }

    // (center, radius) of each segment
    pub fn circles(&self) -> [(Vec2, f32); SEGMENTS]
    {
        std::array::from_fn(|i| (self.points[i], RADII[i]))
    }

    // bounding box of entire body
    pub fn bounds(&self) -> (Vec2, Vec2)
    {
        let max_radius = Vec2::new(HEAD_RADIUS, HEAD_RADIUS);
        let min = self.points.iter().fold(self.points[0], |m, p| Vec2::new(m.x.min(p.x), m.y.min(p.y)));
        let max = self.points.iter().fold(self.points[0], |m, p| Vec2::new(m.x.max(p.x), m.y.max(p.y)));
        (min - max_radius, max + max_radius)
    }

    pub fn draw(&self, render_ctx: &mut RenderContext, offset: Vec2, look_target: Vec2, layer: DrawLayer, z_index: u32, shader_id: u8)
    {
        let to_target = look_target - self.head_center();
        let distance = to_target.length();
        let look = to_target * ((distance / LOOK_DISTANCE).min(1.0) / distance.max(0.0001));
        let (look_x, look_y) = (look.x * 0.5 + 0.5, look.y * 0.5 + 0.5);

        for i in (0..SEGMENTS).rev()
        {
            let not_head = (i > 0) as u32 as f32;
            let wiggle = (self.crawl * WIGGLE_PER_PIXEL - i as f32 * WIGGLE_PHASE).sin() * WIGGLE_AMOUNT * not_head;

            let pos = self.points[i] + offset + Vec2::new(wiggle, 0.0);
            let diameter = RADII[i] * 2.0;

            let stripe = (i % 2) as f32;
            let part = 1.0 - not_head;

            let transform = render_ctx.graphics.renderer.matrix((pos.x, pos.y), (diameter, diameter), 0.0);
            render_ctx.graphics.renderer.draw_mesh_transformed(0, 0, transform, Some([stripe, part, look_x, look_y]), layer, z_index - 1, shader_id);
        }
    }
}

use crate::no_if::vector::Vec2;

#[derive(Copy, Clone)]
pub struct Rect
{
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Rect
{
    pub fn new(x: f64, y: f64, width: f64, height: f64) -> Self
    {
        Self
        {
            x,
            y,
            width,
            height,
        }
    }

    pub fn new_from_center(x: f64, y: f64, width: f64, height: f64) -> Self
    {
        let corner_x = x-width/2.0;
        let corner_y = y-height/2.0;

        Self
        {
            x: corner_x,
            y: corner_y,
            width,
            height
        }
    }

    pub fn update_from_center_pos(&mut self, new_x: f64, new_y: f64)
    {
        let corner_x = new_x-self.width/2.0;
        let corner_y = new_y-self.height/2.0;
        self.x = corner_x;
        self.y = corner_y;
    }

    pub fn contains(&self, point: (f64, f64)) -> bool
    {
        point.0 >= self.x
            && point.0 <= self.x + self.width
            && point.1 >= self.y
            && point.1 <= self.y + self.height
    }

    pub fn contains_x(&self, x: f64) -> bool
    {
        x >= self.x && x <= self.x + self.width
    }
}


#[derive(Copy, Clone)]
pub struct Triangle
{
    // Center Points
    pub pos: Vec2,
    pub rotation: f32,

    // local space vertices
    pub a: Vec2,
    pub b: Vec2,
    pub c: Vec2
}

impl Triangle
{
    pub fn new(pos: Vec2, rotation: f32, a: Vec2, b: Vec2, c: Vec2) -> Self
    {
        Triangle
        {
            pos,
            rotation,
            a,
            b,
            c
        }
    }

    // Gives one of the three vertices of the triangle in worldposition
    pub fn world_point(&self, point: Vec2) -> Vec2
    {
        let rotated = point.rotated(-self.rotation);
        rotated + self.pos
    }

    pub fn contains(&self, point: Vec2) -> bool
    {
        let translated = point-self.pos;
        let local = translated.rotated(-self.rotation);

        let ab = Vec2::cross_points(self.a, self.b, local);
        let bc = Vec2::cross_points(self.b, self.c, local);
        let ca = Vec2::cross_points(self.c, self.a, local);

        (ab >= 0.0 && bc >= 0.0 && ca >= 0.0) || (ab <= 0.0 && bc <= 0.0 && ca <= 0.0)
    }

    pub fn set_position(&mut self, position: Vec2)
    {
        self.pos = position;
    }

    pub fn change_pos(&mut self, change: Vec2)
    {
        self.pos += change;
    }

    pub fn move_by(&mut self, movement: Vec2)
    {
        self.pos += movement;
    }

    pub fn rotate(&mut self, amount: f32)
    {
        self.rotation = (self.rotation + amount).rem_euclid(std::f32::consts::TAU); // Technically dont need wraping, but still
    }

    // mtv: Minimum-translation-vector
    // Only for overlapping a reactngle with no rotation, so just the basic rect
    pub fn triangle_rect_mtv(&self, rect_pos: Vec2, rect_size: (f32, f32)) -> Option<Vec2>
    {
        let tri = [self.world_point(self.a), self.world_point(self.b), self.world_point(self.c)]; // so it is in world_pos

        let half = (rect_size.0 * 0.5, rect_size.1 * 0.5);
        let rect =
        [
            Vec2::new(rect_pos.x - half.0, rect_pos.y - half.1),
            Vec2::new(rect_pos.x + half.0, rect_pos.y - half.1),
            Vec2::new(rect_pos.x + half.0, rect_pos.y + half.1),
            Vec2::new(rect_pos.x - half.0, rect_pos.y + half.1),
        ];

        let edge_normal = |p0: Vec2, p1: Vec2|
        {
            let edge = p1 - p0;
            Vec2::new(-edge.y, edge.x).normalize()
        };

        let axes =
        [
            edge_normal(tri[0], tri[1]),
            edge_normal(tri[1], tri[2]),
            edge_normal(tri[2], tri[0]),
            Vec2::new(1.0, 0.0),
            Vec2::new(0.0, 1.0),
        ];

        let center_dir = self.pos - rect_pos;

        axes.iter().map(|&axis|
        {
            let (tri_min, tri_max) = Self::project_min_max_tri(&tri, axis);
            let (rect_min, rect_max) = Self::project_min_max_rect(&rect, axis);
            let overlap = tri_max.min(rect_max) - tri_min.max(rect_min);
            let sign = (center_dir.dot(axis) > 0.0) as i32 as f32 * 2.0 - 1.0;
            (overlap, axis * sign)
        }).min_by(|a, b| a.0.partial_cmp(&b.0).unwrap()).filter(|(overlap, _)| *overlap > 0.0).map(|(overlap, axis)| axis * overlap)
    }

    fn project_min_max_tri(points: &[Vec2; 3], axis: Vec2) -> (f32, f32)
    {
        let p0 = points[0].dot(axis);
        let p1 = points[1].dot(axis);
        let p2 = points[2].dot(axis);
        (p0.min(p1).min(p2), p0.max(p1).max(p2))
    }

    fn project_min_max_rect(points: &[Vec2; 4], axis: Vec2) -> (f32, f32)
    {
        let p0 = points[0].dot(axis);
        let p1 = points[1].dot(axis);
        let p2 = points[2].dot(axis);
        let p3 = points[3].dot(axis);
        (p0.min(p1).min(p2).min(p3), p0.max(p1).max(p2).max(p3))
    }
}

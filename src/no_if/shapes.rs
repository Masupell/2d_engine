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

    pub fn contains(&self, point: (f64, f64)) -> bool
    {
        point.0 >= self.x
            && point.0 <= self.x + self.width
            && point.1 >= self.y
            && point.1 <= self.y + self.height
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
}

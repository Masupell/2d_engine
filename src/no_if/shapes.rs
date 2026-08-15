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
    pub x: f32,
    pub y: f32,
    pub rotation: f32,

    // local space vertices
    pub a: (f32, f32),
    pub b: (f32, f32),
    pub c: (f32, f32)
}

impl Triangle
{
    pub fn new(x: f32, y: f32, rotation: f32, a: (f32, f32), b: (f32, f32), c: (f32, f32)) -> Self
    {
        Triangle
        {
            x,
            y,
            rotation,
            a,
            b,
            c
        }
    }

    // Gives one of the three vertices of the triangle in worldposition
    fn world_point(&self, point: (f32, f32)) -> (f32, f32)
    {
        let rotated = rotate_point(point, self.rotation);

        (
            rotated.0 + self.x,
            rotated.1 + self.y,
        )
    }

    pub fn contains(&self, point: (f32, f32)) -> bool
    {
        let translated = (point.0 - self.x, point.1 - self.y);
        let local = rotate_point(translated, -self.rotation);

        let ab = cross(self.a, self.b, local);
        let bc = cross(self.b, self.c, local);
        let ca = cross(self.c, self.a, local);

        (ab >= 0.0 && bc >= 0.0 && ca >= 0.0) || (ab <= 0.0 && bc <= 0.0 && ca <= 0.0)
    }

    pub fn set_position(&mut self, position: (f32, f32))
    {
        self.x = position.0;
        self.y = position.1;
    }

    pub fn move_by(&mut self, movement: (f32, f32))
    {
        self.x += movement.0;
        self.y += movement.1;
    }

    pub fn rotate(&mut self, amount: f32)
    {
        self.rotation = (self.rotation + amount).rem_euclid(std::f32::consts::TAU); // Technically dont need wraping, but still
    }
}

fn rotate_point(point: (f32, f32), angle: f32) -> (f32, f32)
{
    let (sin, cos) = angle.sin_cos();
    (
        point.0 * cos - point.1 * sin,
        point.0 * sin + point.1 * cos
    )
}

fn cross(a: (f32, f32), b: (f32, f32), p: (f32, f32)) -> f32
{
    (b.0 - a.0) * (p.1 - a.1) - (b.1 - a.1) * (p.0 - a.0)
}

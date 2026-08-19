use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Vec2
{
    pub x: f32,
    pub y: f32,
}

impl Vec2
{
    pub const ZERO: Self = Self { x: 0.0, y: 0.0 };
    pub const ONE: Self = Self { x: 1.0, y: 1.0 };
    pub const X: Self = Self { x: 1.0, y: 0.0 };
    pub const Y: Self = Self { x: 0.0, y: 1.0 };

    pub fn new(x: f32, y: f32) -> Self
    {
        Self { x, y }
    }

    pub fn new_from_tuple(point: (f32, f32)) -> Self
    {
        Self { x: point.0, y: point.1 }
    }

    pub fn length_squared(self) -> f32
    {
        self.x * self.x + self.y * self.y
    }

    pub fn length(self) -> f32
    {
        self.length_squared().sqrt()
    }

    pub fn normalized(self) -> Self
    {
        self / self.length()
    }

    pub fn dot(self, other: Self) -> f32
    {
        self.x * other.x + self.y * other.y
    }

    // 2d cross product
    pub fn cross(self, other: Self) -> f32
    {
        self.x * other.y - self.y * other.x
    }

    pub fn distance(self, other: Self) -> f32
    {
        (self - other).length()
    }

    pub fn distance_squared(self, other: Self) -> f32
    {
        (self - other).length_squared()
    }

    pub fn lerp(self, other: Self, t: f32) -> Self
    {
        self + (other - self) * t
    }

    pub fn perpendicular(self) -> Self
    {
        Self::new(-self.y, self.x)
    }

    pub fn rotated(self, angle: f32) -> Self
    {
        let (sin, cos) = angle.sin_cos();

        Self::new(self.x * cos - self.y * sin, self.x * sin + self.y * cos)
    }


    // takes three points and turns them into two vectors, thn performs cross product
    pub fn cross_points(a: Vec2, b: Vec2, p: Vec2) -> f32
    {
        (b - a).cross(p - a)
    }

}

impl Add for Vec2
{
    type Output = Self;

    fn add(self, rhs: Self) -> Self
    {
        Self::new(self.x + rhs.x, self.y + rhs.y)
    }
}

impl AddAssign for Vec2
{
    fn add_assign(&mut self, rhs: Self)
    {
        self.x += rhs.x;
        self.y += rhs.y;
    }
}

impl Sub for Vec2
{
    type Output = Self;

    fn sub(self, rhs: Self) -> Self
    {
        Self::new(self.x - rhs.x, self.y - rhs.y)
    }
}

impl SubAssign for Vec2
{
    fn sub_assign(&mut self, rhs: Self)
    {
        self.x -= rhs.x;
        self.y -= rhs.y;
    }
}

impl Mul<f32> for Vec2
{
    type Output = Self;

    fn mul(self, rhs: f32) -> Self
    {
        Self::new(self.x * rhs, self.y * rhs)
    }
}

impl MulAssign<f32> for Vec2
{
    fn mul_assign(&mut self, rhs: f32)
    {
        self.x *= rhs;
        self.y *= rhs;
    }
}

impl Div<f32> for Vec2
{
    type Output = Self;

    fn div(self, rhs: f32) -> Self
    {
        Self::new(self.x / rhs, self.y / rhs)
    }
}

impl DivAssign<f32> for Vec2
{
    fn div_assign(&mut self, rhs: f32)
    {
        self.x /= rhs;
        self.y /= rhs;
    }
}

impl Neg for Vec2
{
    type Output = Self;

    fn neg(self) -> Self
    {
        Self::new(-self.x, -self.y)
    }
}

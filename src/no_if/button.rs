use crate::input::Action;

pub struct Button
{
    pub rect: Rect,
    pub action: Action
}

impl Button
{
    pub fn new(rect: Rect, action: Action) -> Self
    {
        Button
        {
            rect,
            action
        }
    }

    pub fn update(&self, input: &mut crate::Input)
    {
        let clicked = input.actions().contains(&Action::MouseLeftPressed);

        if clicked && self.rect.contains(input.mouse_position())
        {
            input.add_action(self.action);
        }
    }
}


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

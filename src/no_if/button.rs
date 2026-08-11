use crate::input::Action;

pub struct Button
{
    pub rect: Rect,
    pub actions: [Option<Action>; ButtonEvent::COUNT]
}

impl Button
{
    pub fn new(rect: Rect) -> Self
    {
        Button
        {
            rect,
            actions: [None; ButtonEvent::COUNT]
        }
    }

    pub fn set_action(&mut self, event: ButtonEvent, action: Action)
    {
        self.actions[event as usize] = Some(action)
    }

    pub fn update(&self, input: &mut crate::Input)
    {
        let clicked = input.actions().contains(&Action::MouseLeftPressed);
        if self.rect.contains(input.mouse_position())
        {
            if clicked
            {
                self.actions[ButtonEvent::Click as usize].into_iter().for_each(|action| input.add_action(action));
                return;
            }
            self.actions[ButtonEvent::Hover as usize].into_iter().for_each(|action| input.add_action(action));
        }
    }
}

#[derive(Copy, Clone)]
pub enum ButtonEvent
{
    Hover,
    Click
}

impl ButtonEvent
{
    pub const COUNT: usize = 2;
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

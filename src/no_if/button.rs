use crate::input::Action;

pub struct Button
{
    pub rect: Rect,
    pub actions: [Option<Action>; ButtonEvent::COUNT],
    was_inside: bool
}

impl Button
{
    pub fn new(rect: Rect) -> Self
    {
        Button
        {
            rect,
            actions: [None; ButtonEvent::COUNT],
            was_inside: false
        }
    }

    pub fn set_action(&mut self, event: ButtonEvent, action: Action)
    {
        self.actions[event as usize] = Some(action)
    }

    pub fn update(&mut self, input: &mut crate::Input)
    {
        const EVENT_TABLE: [[[ButtonEvent; 2]; 2]; 2] =
        [
            // was_inside = false
            [
                // not in rect
                [ButtonEvent::None, ButtonEvent::None],
                // in rect
                [ButtonEvent::Hover, ButtonEvent::Click],
            ],
            // was_inside = true
            [
                // not in rect
                [ButtonEvent::Unhover, ButtonEvent::None],
                // in rect
                [ButtonEvent::None, ButtonEvent::Click],
            ]
        ];
        let inside = self.rect.contains(input.mouse_position());
        // [was it inside?][is it still inside?][is it clicked (as bool, false is 0 therefore not clicked and hovered instead)]
        let event = EVENT_TABLE[self.was_inside as usize][inside as usize][input.actions().contains(&Action::MouseLeftPressed) as usize];
        self.actions[event as usize].into_iter().for_each(|action| input.add_action(action));
        self.was_inside = inside;
    }
}

#[derive(Copy, Clone)]
pub enum ButtonEvent
{
    Hover,
    Click,
    Unhover,
    None
}

impl ButtonEvent
{
    pub const COUNT: usize = 4;
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

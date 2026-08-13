use crate::no_if::action::Action;

pub struct Button
{
    pub rect: Rect, // Tope left Corner definition
    pub actions: [Option<Action>; ButtonEvent::COUNT],
    texture_id: usize,
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
            texture_id: 0,
            was_inside: false
        }
    }

    pub fn set_action(&mut self, event: ButtonEvent, action: Action)
    {
        self.actions[event as usize] = Some(action)
    }

    pub fn update(&mut self, input: &mut crate::Input)
    {
        const EVENT_TABLE: [[[ButtonEvent; 3]; 2]; 2] =
        [
            // was_inside = false
            [
                // not in rect
                [ButtonEvent::None, ButtonEvent::None, ButtonEvent::None],
                // in rect
                [ButtonEvent::Hover, ButtonEvent::Click, ButtonEvent::Released],
            ],
            // was_inside = true
            [
                // not in rect
                [ButtonEvent::Unhover, ButtonEvent::None, ButtonEvent::None],
                // in rect
                [ButtonEvent::None, ButtonEvent::Click, ButtonEvent::Released],
            ]
        ];
        // None, Click, Released
        let inside = self.rect.contains(input.mouse_position());
        // [was it inside?][is it still inside?][is it clicked (as bool, false is 0 therefore not clicked and hovered instead) + 2x is it released(2x1=1)]
        let pressed = input.actions().contains(&Action::MouseLeftPressed) as usize;
        let released = input.actions().contains(&Action::MouseLeftReleased) as usize;
        // if somehow pressed and released was active in the same frame, it would give an index error (could add a 4th state for that, but its fine for now)
        let mouse_event = pressed + released*2;
        let event = EVENT_TABLE[self.was_inside as usize][inside as usize][mouse_event];
        self.actions[event as usize].into_iter().for_each(|action| input.add_action(action));
        self.was_inside = inside;
    }

    pub fn draw(&self, render_ctx: &mut crate::RenderContext, z_index: u32, shader_id: u8)
    {
        let center = ((self.rect.x + self.rect.width/2.0) as f32, (self.rect.y + self.rect.height/2.0) as f32);
        render_ctx.renderer.draw_texture(0, render_ctx.renderer.matrix(center, (self.rect.width as f32, self.rect.height as f32), 0.0), self.texture_id, z_index, shader_id);
    }

    pub fn set_texture(&mut self, texture_id: usize)
    {
        self.texture_id = texture_id;
    }

    pub fn set_pos(&mut self, pos: (f32, f32))
    {
        self.rect.x = pos.0 as f64;
        self.rect.y = pos.1 as f64;
    }

    pub fn set_size(&mut self, size: (f32, f32))
    {
        self.rect.width = size.0 as f64;
        self.rect.height = size.1 as f64;
    }

    pub fn set_size_centered(&mut self, size: (f32, f32))
    {
        let center_x = self.rect.x + self.rect.width / 2.0;
        let center_y = self.rect.y + self.rect.height / 2.0;

        self.rect.width = size.0 as f64;
        self.rect.height = size.1 as f64;

        self.rect.x = center_x - self.rect.width / 2.0;
        self.rect.y = center_y - self.rect.height / 2.0;
    }
}

#[derive(Copy, Clone)]
pub enum ButtonEvent
{
    Click,
    Released,
    Hover,
    Unhover,
    None
}

impl ButtonEvent
{
    pub const COUNT: usize = 5;
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

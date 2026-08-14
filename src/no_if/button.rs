use crate::no_if::{action::Action, rect::Rect};

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

pub struct Button
{
    pub rect: Rect, // Tope left Corner definition
    pub actions: [Option<Action>; ButtonEvent::COUNT],
    texture_id: usize,
    was_inside: bool,
    hover_enabled: bool,
    normal_size: (f64, f64),
    hover_scale: f32
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
            was_inside: false,
            hover_enabled: true,
            normal_size: (rect.width, rect.height),
            hover_scale: 1.1
        }
    }

    pub fn set_action(&mut self, event: ButtonEvent, action: Action)
    {
        self.actions[event as usize] = Some(action)
    }

    pub fn update(&mut self, input: &mut crate::Input)
    {
        // None, Click, Released
        let inside = self.rect.contains(input.mouse_position());
        // [was it inside?][is it still inside?][is it clicked (as bool, false is 0 therefore not clicked and hovered instead) + 2x is it released(2x1=1)]
        let pressed = input.actions().contains(&Action::MouseLeftPressed) as usize;
        let released = input.actions().contains(&Action::MouseLeftReleased) as usize;
        // if somehow pressed and released was active in the same frame, it would give an index error (could add a 4th state for that, but its fine for now)
        let mouse_event = pressed + released*2;
        let event = EVENT_TABLE[self.was_inside as usize][inside as usize][mouse_event];
        self.handle_hover_event(event);
        self.actions[event as usize].into_iter().for_each(|action| input.add_action(action));
        self.was_inside = inside;
    }

    pub fn draw(&self, render_ctx: &mut crate::RenderContext, z_index: u32, shader_id: u8)
    {
        let center = ((self.rect.x + self.rect.width/2.0) as f32, (self.rect.y + self.rect.height/2.0) as f32);
        render_ctx.renderer.draw_texture(0, render_ctx.renderer.matrix(center, (self.rect.width as f32, self.rect.height as f32), 0.0), self.texture_id, z_index, shader_id);
    }

    fn handle_hover_event(&mut self, event: ButtonEvent)
    {
        let scale_table: [[Option<f32>; ButtonEvent::COUNT]; 2] =
        [
            [None, None, None, None, None], // Hover disabled
            [None, None, Some(self.hover_scale), Some(1.0), None] // Hover enabled
        ];

        scale_table[self.hover_enabled as usize][event as usize].into_iter().for_each(|scale|
        {
            self.set_size_centered((self.normal_size.0 as f32 * scale, self.normal_size.1 as f32 * scale));
        });
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

    pub fn handle_hover(&mut self, enabled: bool)
    {
        self.hover_enabled = enabled;
    }

    pub fn set_hover_scale(&mut self, scale: f32)
    {
        self.hover_scale = scale;
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

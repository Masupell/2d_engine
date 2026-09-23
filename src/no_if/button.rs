use crate::no_if::action::Action;

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
    pos: (f32, f32), // center
    size: (f32, f32),
    pub actions: [Option<Action>; ButtonEvent::COUNT],
    texture_id: usize,
    atlas_pos: (f32, f32),
    atlas_size: (f32, f32),
    was_inside: bool,
    hover_enabled: bool,
    normal_size: (f32, f32),
    hover_scale: f32
}

impl Button
{
    pub fn new(pos: (f32, f32), size: (f32, f32)) -> Self
    {
        Button
        {
            pos,
            size,
            actions: [None; ButtonEvent::COUNT],
            texture_id: 0,
            atlas_pos: (0.0, 0.0),
            atlas_size: (0.0, 0.0),
            was_inside: false,
            hover_enabled: true,
            normal_size: size,
            hover_scale: 1.1
        }
    }

    pub fn set_action(&mut self, event: ButtonEvent, action: Action)
    {
        self.actions[event as usize] = Some(action)
    }

    pub fn set_pos(&mut self, pos: (f32, f32))
    {
        self.pos = pos;
    }

    pub fn update(&mut self, input: &mut crate::Input)
    {
        let top_left = (self.pos.0 - self.size.0/2.0, self.pos.1 - self.size.1/2.0);
        // None, Click, Released
        let inside = contained((input.mouse_position().0 as f32, input.mouse_position().1 as f32), top_left, self.size);
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
        render_ctx.graphics.renderer.draw_texture_atlas_ui(0, render_ctx.graphics.renderer.ui_matrix(self.pos, (self.size.0, self.size.1), 0.0), self.texture_id, self.atlas_pos, self.atlas_size, z_index, shader_id);
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
             self.size = (self.normal_size.0 * scale, self.normal_size.1 * scale);
        });
    }

    pub fn set_texture(&mut self, texture_id: usize)
    {
        self.texture_id = texture_id;
    }

    pub fn set_atlas_rect(&mut self, pos: (f32, f32), size: (f32, f32))
    {
        self.atlas_pos = pos;
        self.atlas_size = size;
    }

    pub fn set_size(&mut self, size: (f32, f32))
    {
        self.size = (size.0, size.1);
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

pub fn contained(point: (f32, f32), top_left: (f32, f32), size: (f32, f32)) -> bool
{
    point.0 >= top_left.0
        && point.0 <= top_left.0 + size.0
        && point.1 >= top_left.1
        && point.1 <= top_left.1 + size.1
}

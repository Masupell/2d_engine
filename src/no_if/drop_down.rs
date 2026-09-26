use crate::no_if::action::Action;
use crate::utility::{CoordSpace, DrawLayer};

pub struct Dropdown
{
    pos: (f32, f32),
    size: (f32, f32),
    labels: Vec<String>,
    actions: Vec<Action>,
    is_open: bool,
    selected: usize,
    hovered: usize,
    backgrounds: [[f32; 4]; 2], // normal and hover
    text_color: [f32; 4],
    font_size: f32,
}

impl Dropdown
{
    const CLICK_HANDLERS: [fn(&mut Self, &mut crate::Input, usize); 4] =
    [
        Self::no_click,
        Self::outside_click,
        Self::header_click,
        Self::option_click,
    ];

    pub fn new(size: (f32, f32), header_text: impl Into<String>) -> Self
    {
        Self
        {
            pos: (0.0, 0.0),
            size,
            labels: vec![header_text.into()],
            actions: Vec::new(),
            is_open: false,
            selected: 0,
            hovered: 0,
            backgrounds: [[0.6, 0.1, 0.0, 1.0], [0.8, 0.15, 0.0, 1.0]],
            text_color: [1.0, 1.0, 1.0, 1.0],
            font_size: size.1,
        }
    }

    pub fn add_option(&mut self, text: impl Into<String>, action: Action)
    {
        self.labels.push(text.into());
        self.actions.push(action);
    }

    pub fn set_pos(&mut self, pos: (f32, f32))
    {
        self.pos = pos;
    }

    pub fn set_selected(&mut self, index: usize)
    {
        self.selected = index;
    }

    fn option_count(&self) -> usize
    {
        self.actions.len()
    }

    fn visible_rows(&self) -> usize
    {
        1 + self.is_open as usize * self.option_count()
    }

    fn display_label(&self) -> usize
    {
        let valid = (self.selected < self.option_count()) as usize;
        valid * (self.selected + 1)
    }

    fn hover(&self, mouse: (f32, f32)) -> usize
    {
        let left = self.pos.0 - self.size.0 * 0.5;
        let top = self.pos.1 - self.size.1 * 0.5;

        let row = ((mouse.1 - top) / self.size.1).floor();

        let in_x = (mouse.0 >= left) & (mouse.0 <= left + self.size.0);
        let in_y = (row >= 0.0) & (row < self.visible_rows() as f32);

        (in_x & in_y) as usize * (row as usize + 1)
    }

    pub fn update(&mut self, input: &mut crate::Input)
    {
        let mouse = (input.mouse_position().0 as f32, input.mouse_position().1 as f32);
        let hovered = self.hover(mouse);
        self.hovered = hovered;

        let clicked = input.actions().contains(&Action::MouseLeftPressed) as usize;
        let handler = clicked * (1 + hovered.min(2));

        Self::CLICK_HANDLERS[handler](self, input, hovered);
    }

    fn no_click(&mut self, _input: &mut crate::Input, _hovered: usize) {}

    fn outside_click(&mut self, _input: &mut crate::Input, _hovered: usize)
    {
        self.is_open = false;
    }

    fn header_click(&mut self, _input: &mut crate::Input, _hovered: usize)
    {
        self.is_open = !self.is_open;
    }

    fn option_click(&mut self, input: &mut crate::Input, hovered: usize)
    {
        let i = hovered - 2;
        self.selected = i;
        input.add_action(self.actions[i]);
        self.is_open = false;
    }

    pub fn draw(&self, render_ctx: &mut crate::RenderContext, z_index: u32)
    {
        for row in 0..self.visible_rows()
        {
            let is_header = (row == 0) as usize;
            let label = row + is_header * self.display_label();
            let hovered = (self.hovered == row + 1) as usize;

            self.draw_row(render_ctx, row, &self.labels[label], hovered, z_index);
        }
    }

    fn draw_row(&self, render_ctx: &mut crate::RenderContext, row: usize, text: &str, hovered: usize, z_index: u32)
    {
        let center = (self.pos.0, self.pos.1 + row as f32 * self.size.1);
        let background = self.backgrounds[hovered];

        render_ctx.graphics.renderer.draw_ui(0, render_ctx.graphics.renderer.ui_matrix(center, self.size, 0.0), background, z_index, 0);
        render_ctx.graphics.renderer.draw_text_centered(render_ctx.graphics.device, render_ctx.graphics.queue, text, center, self.font_size, self.text_color, 0.0, CoordSpace::Screen, DrawLayer::UI, z_index + 1, 0);
    }
}

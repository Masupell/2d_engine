use crate::no_if::action::Action;
use crate::utility::{CoordSpace, DrawLayer};

const CHECK_SCALE: f32 = 0.55; // size 0 0.0->1.0 entire checkbox
const LABEL_PADDING: f32 = 0.35; // percentage of boxsize (35%)

pub struct Checkbox
{
    pos: (f32, f32),
    size: (f32, f32),
    label: String,
    checked: bool,
    hovered: bool,
    actions: [Action; 2],
    box_backgrounds: [[f32; 4]; 2],
    check_colors: [[f32; 4]; 2], // unchecked, checked
    outline_color: [f32; 4],
    outline_thickness: f32,
    text_color: [f32; 4],
    font_size: f32,
}

impl Checkbox
{
    const CLICK_HANDLERS: [fn(&Self, &mut crate::Input); 2] =
    [
        Self::on_no_toggle,
        Self::on_toggle,
    ];

    pub fn new(size: (f32, f32), label: impl Into<String>, on_checked: Action, on_unchecked: Action) -> Self
    {
        Self
        {
            pos: (0.0, 0.0),
            size,
            label: label.into(),
            checked: false,
            hovered: false,
            actions: [on_unchecked, on_checked],
            box_backgrounds: [[0.6, 0.1, 0.0, 0.9], [0.8, 0.15, 0.0, 0.9]],
            check_colors: [[0.0, 0.0, 0.0, 0.0], [1.0, 1.0, 1.0, 1.0]],
            outline_color: [1.0, 1.0, 1.0, 1.0],
            outline_thickness: 2.0,
            text_color: [1.0, 1.0, 1.0, 1.0],
            font_size: size.1 * 0.8,
        }
    }

    pub fn set_pos(&mut self, pos: (f32, f32))
    {
        self.pos = pos;
    }

    // Sets the state without firing an action (e.g. when loading settings)
    pub fn set_checked(&mut self, checked: bool)
    {
        self.checked = checked;
    }

    pub fn is_checked(&self) -> bool
    {
        self.checked
    }

    fn inside(&self, point: (f32, f32)) -> bool
    {
        let left = self.pos.0 - self.size.0 * 0.5;
        let top = self.pos.1 - self.size.1 * 0.5;

        (point.0 >= left) & (point.0 <= left + self.size.0) & (point.1 >= top) & (point.1 <= top + self.size.1)
    }

    pub fn update(&mut self, input: &mut crate::Input)
    {
        let mouse = (input.mouse_position().0 as f32, input.mouse_position().1 as f32);
        self.hovered = self.inside(mouse);

        let clicked = input.actions().contains(&Action::MouseLeftPressed);
        let toggled = self.hovered & clicked;

        self.checked ^= toggled; // xor, basically 'if toggled => checked = !checked'
        Self::CLICK_HANDLERS[toggled as usize](self, input);
    }

    fn on_no_toggle(&self, _input: &mut crate::Input) {}

    fn on_toggle(&self, input: &mut crate::Input)
    {
        input.add_action(self.actions[self.checked as usize]);
    }

    pub fn draw(&self, render_ctx: &mut crate::RenderContext, z_index: u32)
    {
        let graphics = &mut render_ctx.graphics;

        let box_side = self.size.1;
        let left = self.pos.0 - self.size.0 * 0.5;
        let box_center = (left + box_side * 0.5, self.pos.1);
        let check_side = box_side * CHECK_SCALE;

        graphics.renderer.draw_ui(0, graphics.renderer.ui_matrix(box_center, (box_side, box_side), 0.0), self.box_backgrounds[self.hovered as usize], z_index, 0);
        graphics.renderer.draw_ui(0, graphics.renderer.ui_matrix(box_center, (check_side, check_side), 0.0), self.check_colors[self.checked as usize], z_index + 1, 0);

        graphics.renderer.draw_rect_outline(box_center, (box_side, box_side), self.outline_thickness, self.outline_color, 0.0, CoordSpace::Screen, DrawLayer::UI, z_index + 2, 0);

        let text_pos = (left + box_side * (1.0 + LABEL_PADDING), self.pos.1 - self.font_size * 0.5);
        graphics.renderer.draw_text(graphics.device, graphics.queue, &self.label, text_pos, self.font_size, self.text_color, 0.0, CoordSpace::Screen, DrawLayer::UI, z_index + 1, 0);
    }
}

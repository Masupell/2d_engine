use std::str::FromStr;

use crate::no_if::action::Action;
use crate::utility::{CoordSpace, DrawLayer};

const CURSOR_BLINK: f32 = 0.5;
const TEXT_PADDING: f32 = 0.3;
const CURSOR_WIDTH: f32 = 2.0;
const CURSOR_HEIGHT: f32 = 0.7;

const REJECT_DURATION: f32 = 0.3;
const SHAKE_AMPLITUDE: f32 = 4.0;
const SHAKE_FREQUENCY: f32 = 60.0;

// const CURSOR_SPACE: f32 = CURSOR_WIDTH * 3.0;

pub fn default_charset() -> String
{
    (' '..='~').collect()
}

pub fn digits_charset() -> String
{
    ('0'..='9').collect()
}

pub fn number_charset() -> String
{
    "0123456789.-".to_string()
}

#[derive(Copy, Clone)]
pub enum TextAlign
{
    Left,
    Center,
    Right,
}
impl TextAlign { pub const COUNT: usize = 3; }

pub struct InputField
{
    pos: (f32, f32), // center
    size: (f32, f32),
    text: String,
    placeholder: String,
    charset: String,
    max_length: usize,

    focused: bool,
    hovered: bool,
    blink: f32,
    on_submit: Option<Action>,

    backgrounds: [[f32; 4]; 2],
    hover_tint: [f32; 4],
    outline_colors: [[f32; 4]; 2],
    outline_thickness: f32,
    text_color: [f32; 4],
    placeholder_color: [f32; 4],
    font_size: f32,

    saved_text: String,
    reject_timer: f32,
    error_color: [f32; 4],

    align: TextAlign
}

impl InputField
{
    const SUBMIT_HANDLERS: [fn(&Self, &mut crate::Input); 2] = [Self::on_no_submit, Self::on_submit];
    const CAPTURE_HANDLERS: [fn(&mut crate::Input); 2] = [Self::no_capture, crate::Input::request_text_capture];
    const FOCUS_HANDLERS: [fn(&mut Self); 2] = [Self::no_change, Self::save_text];
    const CANCEL_HANDLERS: [fn(&mut Self); 2] = [Self::no_change, Self::restore_text];

    pub fn new(size: (f32, f32), charset: impl Into<String>) -> Self
    {
        Self
        {
            pos: (0.0, 0.0),
            size,
            text: String::new(),
            placeholder: String::new(),
            charset: charset.into(),
            max_length: 32,
            focused: false,
            hovered: false,
            blink: 0.0,
            on_submit: None,
            backgrounds: [[0.1, 0.1, 0.1, 0.85], [0.18, 0.18, 0.18, 0.95]],
            hover_tint: [0.05, 0.05, 0.05, 0.0],
            outline_colors: [[0.6, 0.6, 0.6, 1.0], [1.0, 1.0, 1.0, 1.0]],
            outline_thickness: 2.0,
            text_color: [1.0, 1.0, 1.0, 1.0],
            placeholder_color: [0.6, 0.6, 0.6, 1.0],
            font_size: size.1 * 0.7,
            saved_text: String::new(),
            reject_timer: 0.0,
            error_color: [0.95, 0.25, 0.2, 1.0],
            align: TextAlign::Left
        }
    }

    pub fn set_pos(&mut self, pos: (f32, f32)) { self.pos = pos; }
    pub fn set_placeholder(&mut self, placeholder: impl Into<String>) { self.placeholder = placeholder.into(); }
    pub fn set_max_length(&mut self, max_length: usize) { self.max_length = max_length; }
    pub fn set_on_submit(&mut self, action: Action) { self.on_submit = Some(action); }
    pub fn set_align(&mut self, align: TextAlign) { self.align = align; }

    fn text_x(&self, pos: (f32, f32), width: f32) -> f32
    {
        let inner_left = pos.0 - self.size.0 * 0.5 + self.size.1 * TEXT_PADDING;
        let inner_right = pos.0 + self.size.0 * 0.5 - self.size.1 * TEXT_PADDING;
        let content = width;// + CURSOR_SPACE;
        let aligned = [inner_left, pos.0 - content * 0.5, inner_right - content][self.align as usize];
        aligned.max(inner_left)
    }

    pub fn set_text(&mut self, text: &str)
    {
        let charset = &self.charset;
        self.text = text.chars().filter(|c| charset.contains(*c)).take(self.max_length).collect();
    }

    pub fn text(&self) -> &str
    {
        &self.text
    }

    pub fn value<T: FromStr>(&self) -> Option<T>
    {
        self.text.trim().parse().ok()
    }

    pub fn is_focused(&self) -> bool
    {
        self.focused
    }

    fn inside(&self, point: (f32, f32)) -> bool
    {
        let left = self.pos.0 - self.size.0 * 0.5;
        let top = self.pos.1 - self.size.1 * 0.5;

        (point.0 >= left) & (point.0 <= left + self.size.0) & (point.1 >= top) & (point.1 <= top + self.size.1)
    }


    pub fn update(&mut self, input: &mut crate::Input, dt: f32, endless: bool)
    {
        const UPDATE_TABLE: [fn(&mut InputField, &mut crate::Input, f32); 2] = [InputField::do_update, InputField::no_update];
        UPDATE_TABLE[endless as usize](self, input, dt);
    }

    pub fn no_update(&mut self, _input: &mut crate::Input, _dt: f32) {}
    pub fn do_update(&mut self, input: &mut crate::Input, dt: f32)
    {
        let mouse = input.mouse_position_f32();
        self.hovered = self.inside(mouse);

        let was_focused = self.focused;
        let clicked = input.actions().contains(&Action::MouseLeftPressed);
        self.focused = [self.focused, self.hovered][clicked as usize];

        Self::FOCUS_HANDLERS[(self.focused & !was_focused) as usize](self);
        let focused = self.focused as usize;

        let removals = input.backspaces() as usize * focused;
        (0..removals).for_each(|_| { self.text.pop(); });

        let room = self.max_length.saturating_sub(self.text.chars().count()) * focused;
        let charset = &self.charset;
        let added: String = input.typed_text().chars().filter(|c| charset.contains(*c)).take(room).collect();
        let changed = (removals > 0) | !added.is_empty();
        self.text.push_str(&added);

        let typed_count = input.typed_text().chars().count() * focused;
        let rejected = typed_count > added.chars().count();
        self.reject_timer = [(self.reject_timer - dt).max(0.0), REJECT_DURATION][rejected as usize];

        let cancelled = self.focused & input.esc_pressed();
        Self::CANCEL_HANDLERS[cancelled as usize](self);

        let clicked_away = clicked & !self.hovered & was_focused;
        let submitted = self.focused & input.enter_pressed() & !cancelled;
        Self::SUBMIT_HANDLERS[(submitted | clicked_away) as usize](self, input);

        self.focused &= !(submitted | cancelled);

        self.blink = ((self.blink + dt) % (CURSOR_BLINK * 2.0)) * (!changed) as u32 as f32;
        Self::CAPTURE_HANDLERS[self.focused as usize](input);
    }

    fn no_change(&mut self) {}

    fn save_text(&mut self)
    {
        self.saved_text.clone_from(&self.text);
    }

    fn restore_text(&mut self)
    {
        self.text.clone_from(&self.saved_text);
    }

    fn on_no_submit(&self, _input: &mut crate::Input) {}
    fn on_submit(&self, input: &mut crate::Input)
    {
        self.on_submit.into_iter().for_each(|action| input.add_action(action));
    }

    fn no_capture(_input: &mut crate::Input) {}

    pub fn draw(&self, render_ctx: &mut crate::RenderContext, z_index: u32, endless: bool)
    {
        const DRAW_TABLE: [fn(&InputField, &mut crate::RenderContext, u32); 2] = [InputField::do_draw, InputField::no_draw];
        DRAW_TABLE[endless as usize](self, render_ctx, z_index);
    }

    pub fn no_draw(&self, _render_ctx: &mut crate::RenderContext, _z_index: u32) {}
    pub fn do_draw(&self, render_ctx: &mut crate::RenderContext, z_index: u32)
    {
        let graphics = &mut render_ctx.graphics;
        let focused = self.focused as usize;

        let reject = self.reject_timer / REJECT_DURATION;
        let shake = (self.reject_timer * SHAKE_FREQUENCY).sin() * SHAKE_AMPLITUDE * reject;
        let pos = (self.pos.0 + shake, self.pos.1);

        let base = self.backgrounds[focused];
        let tint = self.hovered as u32 as f32;
        let background = std::array::from_fn::<f32, 4, _>(|i| base[i] + self.hover_tint[i] * tint);
        graphics.renderer.draw_ui(0, graphics.renderer.ui_matrix(pos, self.size, 0.0), background, z_index, 0);

        let normal_outline = self.outline_colors[focused];
        let outline = std::array::from_fn::<f32, 4, _>(|i| normal_outline[i] + (self.error_color[i] - normal_outline[i]) * reject);
        graphics.renderer.draw_rect_outline(pos, self.size, self.outline_thickness, outline, 0.0, CoordSpace::Screen, DrawLayer::UI, z_index + 2, 0);

        let text_width = graphics.renderer.text_size(&self.text, self.font_size).0;
        let placeholder_width = graphics.renderer.text_size(&self.placeholder, self.font_size).0;

        let text_left = self.text_x(pos, text_width);
        let text_y = pos.1 - self.font_size * 0.5;
        graphics.renderer.draw_text(graphics.device, graphics.queue, &self.text, (text_left, text_y), self.font_size, self.text_color, 0.0, CoordSpace::Screen, DrawLayer::UI, z_index + 1, 0);

        let empty = self.text.is_empty() as u32 as f32;
        let placeholder_color = [self.placeholder_color[0], self.placeholder_color[1], self.placeholder_color[2], self.placeholder_color[3] * empty];
        let placeholder_left = self.text_x(pos, placeholder_width);
        graphics.renderer.draw_text(graphics.device, graphics.queue, &self.placeholder, (placeholder_left, text_y), self.font_size, placeholder_color, 0.0, CoordSpace::Screen, DrawLayer::UI, z_index + 1, 0);

        let visible = (self.focused & (self.blink < CURSOR_BLINK)) as u32 as f32;
        let cursor_pos = (text_left + text_width + CURSOR_WIDTH * 2.0, pos.1);
        let cursor_color = [self.text_color[0], self.text_color[1], self.text_color[2], self.text_color[3] * visible];
        graphics.renderer.draw_ui(0, graphics.renderer.ui_matrix(cursor_pos, (CURSOR_WIDTH, self.size.1 * CURSOR_HEIGHT), 0.0), cursor_color, z_index + 1, 0);
    }
}

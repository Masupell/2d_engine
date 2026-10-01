use engine::{no_if::button::contained, utility::DrawLayer, *};

use crate::caterpillar::{Caterpillar, HEAD_LIFT, HEAD_RADIUS};
use crate::player::PlayerSkin;

const CARD_SIZE: (f32, f32) = (240.0, 240.0);
const CARD_CENTERS: [(f32, f32); 2] = [(710.0, 345.0), (970.0, 345.0)];
const GROUND_BELOW_CENTER: f32 = 85.0;
const TITLE_POS: (f32, f32) = (840.0, 195.0);
const LABEL_BELOW_CARD: f32 = 145.0;

const WALK_SPEED: f32 = 90.0;
const WALK_RANGE: f32 = 45.0;
const DASH_SPEED: f32 = 600.0;
const DASH_TIME: f32 = 0.15;
const GRAVITY: f32 = 1960.0;
const TILT_PER_VELOCITY: f32 = 0.0025;
const MAX_TILT: f32 = 0.52;
const TILT_SMOOTHING: f32 = 0.05;
const CLIMBER_SIZE: f32 = 128.0;

const CARD_COLOR: [[f32; 4]; 2] = [[0.15, 0.12, 0.10, 0.85], [0.22, 0.18, 0.15, 0.9]];
const OUTLINE_COLOR: [[f32; 4]; 2] = [[0.5, 0.45, 0.4, 1.0], [0.95, 0.8, 0.3, 1.0]];
const OUTLINE_WIDTH: [f32; 2] = [2.0, 5.0];
const GROUND_COLOR: [f32; 4] = [0.25, 0.48, 0.17, 1.0];
const TEXT_COLOR: [f32; 4] = [0.85, 0.85, 0.85, 1.0];

pub struct CharacterSelect
{
    selected: usize,
    unlocked: [bool; 2],
    hovered: [bool; 2],

    x: f32,
    velocity: f32,
    tilt: f32,
    input_x: f32,
    dash_timer: f32,

    body: Caterpillar,
    climber_texture: usize,
    body_shader: u8,
}

impl CharacterSelect
{
    pub fn new() -> Self
    {
        Self
        {
            selected: 0,
            unlocked: [true, false],
            hovered: [false; 2],
            x: 0.0,
            velocity: 0.0,
            tilt: 0.0,
            input_x: 0.0,
            dash_timer: 0.0,
            body: Caterpillar::new(Self::caterpillar_head(0.0)),
            climber_texture: 0,
            body_shader: 0,
        }
    }

    pub fn set_climber_texture(&mut self, texture_id: usize)
    {
        self.climber_texture = texture_id;
    }

    pub fn set_body_shader(&mut self, shader_id: u8)
    {
        self.body_shader = shader_id;
    }

    pub fn set_caterpillar_unlocked(&mut self, unlocked: bool)
    {
        self.unlocked[1] = unlocked;
    }

    pub fn skin(&self) -> PlayerSkin
    {
        [PlayerSkin::Climber, PlayerSkin::CaterPillar][self.selected]
    }

    pub fn move_left(&mut self) { self.input_x -= 1.0; }
    pub fn move_right(&mut self) { self.input_x += 1.0; }
    pub fn dash(&mut self) { self.dash_timer = DASH_TIME; }

    fn caterpillar_head(x: f32) -> Vec2
    {
        Vec2::new(x, -(HEAD_RADIUS - HEAD_LIFT))
    }

    fn top_left(center: (f32, f32)) -> (f32, f32)
    {
        (center.0 - CARD_SIZE.0 * 0.5, center.1 - CARD_SIZE.1 * 0.5)
    }

    fn ground(center: (f32, f32)) -> (f32, f32)
    {
        (center.0, center.1 + GROUND_BELOW_CENTER)
    }

    pub fn update(&mut self, mouse: (f32, f32), clicked: bool, dt: f32)
    {
        let dashing = (self.dash_timer > 0.0) as u32 as f32;
        let direction = self.input_x.clamp(-1.0, 1.0);

        self.velocity = direction * WALK_SPEED + direction * DASH_SPEED * dashing;
        self.x = (self.x + self.velocity * dt).clamp(-WALK_RANGE, WALK_RANGE);
        self.dash_timer = (self.dash_timer - dt).max(0.0);
        self.input_x = 0.0;

        let target_tilt = (self.velocity * TILT_PER_VELOCITY).clamp(-MAX_TILT, MAX_TILT);
        self.tilt += (target_tilt - self.tilt) * (1.0 - TILT_SMOOTHING.powf(dt));

        self.body.update(Self::caterpillar_head(self.x), GRAVITY, dt, &[], 0.0, CARD_SIZE.0 * 0.5);

        self.hovered = std::array::from_fn(|i| contained(mouse, Self::top_left(CARD_CENTERS[i]), CARD_SIZE));
        let picked: usize = (0..2).map(|i| (clicked & self.hovered[i] & self.unlocked[i]) as usize * (i + 1)).sum();
        self.selected = [self.selected, 0, 1][picked];
    }

    pub fn draw(&self, render_ctx: &mut RenderContext, mouse: (f32, f32), z_index: u32)
    {
        render_ctx.graphics.renderer.draw_text_centered(render_ctx.graphics.device, render_ctx.graphics.queue, "Character:", TITLE_POS, 30.0, TEXT_COLOR, 0.0, CoordSpace::Screen, DrawLayer::UI, z_index, 0);

        for i in 0..2
        {
            let center = CARD_CENTERS[i];
            let selected = (self.selected == i) as usize;
            let renderer = &mut *render_ctx.graphics.renderer;

            let background = renderer.ui_matrix(center, CARD_SIZE, 0.0);
            renderer.draw_ui(0, background, CARD_COLOR[self.hovered[i] as usize], z_index, 0);

            let ground = Self::ground(center);
            let ground_line = renderer.ui_matrix((ground.0, ground.1 + 4.0), (CARD_SIZE.0 - 30.0, 8.0), 0.0);
            renderer.draw_ui(0, ground_line, GROUND_COLOR, z_index + 1, 0);

            renderer.draw_rect_outline(center, CARD_SIZE, OUTLINE_WIDTH[selected], OUTLINE_COLOR[selected], 0.0, CoordSpace::Screen, DrawLayer::UI, z_index + 4, 0);

            let label = ["Climber", "Caterpillar"][i];
            let label_pos = (center.0, center.1 + LABEL_BELOW_CARD);
            render_ctx.graphics.renderer.draw_text_centered(render_ctx.graphics.device, render_ctx.graphics.queue, label, label_pos, 26.0, TEXT_COLOR, 0.0, CoordSpace::Screen, DrawLayer::UI, z_index, 0);
        }

        // CLimber Preview
        let climber_ground = Self::ground(CARD_CENTERS[0]);
        let climber_pos = (climber_ground.0 + self.x, climber_ground.1 - CLIMBER_SIZE * 0.5);
        let climber_transform = render_ctx.graphics.renderer.ui_matrix(climber_pos, (CLIMBER_SIZE, CLIMBER_SIZE), self.tilt);
        render_ctx.graphics.renderer.draw_mesh_transformed(0, self.climber_texture, climber_transform, Some([1.0; 4]), DrawLayer::UI, z_index + 2, 0);

        // Caterpillar preview
        let caterpillar_ground = Self::ground(CARD_CENTERS[1]);
        let renderer = &render_ctx.graphics.renderer;
        let offset = Vec2::new
        (
            renderer.camera_pos.0 + caterpillar_ground.0 - renderer.virtual_size.0 * 0.5,
            renderer.camera_pos.1 + caterpillar_ground.1 - renderer.virtual_size.1 * 0.5,
        );
        let look_target = Vec2::new(mouse.0 - caterpillar_ground.0, mouse.1 - caterpillar_ground.1); // card space, like the body
        self.body.draw(render_ctx, offset, look_target, DrawLayer::UI, z_index + 3, self.body_shader);

        let locked = (!self.unlocked[1]) as u32 as f32;
        let center = CARD_CENTERS[1];

        let cover = render_ctx.graphics.renderer.ui_matrix(center, CARD_SIZE, 0.0);
        render_ctx.graphics.renderer.draw_ui(0, cover, [0.05, 0.05, 0.05, 0.75 * locked], z_index + 5, 0);

        let text_color = [TEXT_COLOR[0], TEXT_COLOR[1], TEXT_COLOR[2], locked];
        render_ctx.graphics.renderer.draw_text_centered(render_ctx.graphics.device, render_ctx.graphics.queue, "Reach the top\n    to unlock", (center.0, center.1 - 15.0), 26.0, text_color, 0.0, CoordSpace::Screen, DrawLayer::UI, z_index + 6, 0);
    }
}

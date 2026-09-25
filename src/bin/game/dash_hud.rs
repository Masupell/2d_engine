use engine::{utility::DrawLayer, *};

const FIRST_SLOT: (f32, f32) = (45.0, 190.0);
const SLOT_SPACING: f32 = 85.0;
const ORB_SIZE: f32 = 46.0;
const FRAME_SIZE: f32 = 54.0;
const RECT_SIZE: f32 = 44.0;
const FRAME_THICKNESS: f32 = 5.0;
const FRAME_COLOR: [f32; 4] = [0.7, 0.85, 1.0, 0.85];
const RECT_COLOR: [f32; 4] = [0.7, 0.85, 1.0, 0.25];
const SLOT_ALPHA: [f32; 2] = [0.13, 1.0];

const FLIGHT_DURATION: f32 = 0.75;
const FLIGHT_ARC: f32 = -80.0;
const FLIGHT_GROWTH: f32 = 1.0;

struct FlyingOrb
{
    start: (f32, f32),
    start_size: f32,
    progress: f32,
}

pub struct DashHud
{
    flying: Vec<FlyingOrb>,
    shader_id: u8,
}

impl DashHud
{
    pub fn new() -> Self
    {
        Self
        {
            flying: Vec::new(),
            shader_id: 0,
        }
    }

    pub fn set_shader(&mut self, shader_id: u8)
    {
        self.shader_id = shader_id;
    }

    pub fn launch(&mut self, screen_pos: (f32, f32), start_size: f32)
    {
        self.flying.push(FlyingOrb { start: screen_pos, start_size, progress: 0.0 });
    }

    pub fn update(&mut self, dt: f32)
    {
        let step = dt / FLIGHT_DURATION;
        self.flying.iter_mut().for_each(|orb| orb.progress += step);
        self.flying.retain(|orb| orb.progress < 1.0);
    }

    pub fn reset(&mut self)
    {
        self.flying.clear();
    }

    fn slot_center(index: usize) -> (f32, f32)
    {
        (FIRST_SLOT.0 + index as f32 * SLOT_SPACING, FIRST_SLOT.1)
    }

    // Ease-out along the line, bulging sideways, ends exactly on the slot at slot size
    fn flight_pose(orb: &FlyingOrb, target: (f32, f32)) -> ((f32, f32), f32)
    {
        let t = orb.progress.min(1.0);
        let eased = 1.0 - (1.0 - t).powi(3);

        let delta = (target.0 - orb.start.0, target.1 - orb.start.1);
        let length = (delta.0 * delta.0 + delta.1 * delta.1).sqrt().max(0.0001);
        let side = (-delta.1 / length, delta.0 / length);
        let bulge = (eased * std::f32::consts::PI).sin();

        let pos =
        (
            orb.start.0 + delta.0 * eased + side.0 * bulge * FLIGHT_ARC,
            orb.start.1 + delta.1 * eased + side.1 * bulge * FLIGHT_ARC,
        );
        let size = (orb.start_size + (ORB_SIZE - orb.start_size) * eased) * (1.0 + bulge * FLIGHT_GROWTH);

        (pos, size)
    }

    fn draw_orb(&self, render_ctx: &mut RenderContext, center: (f32, f32), size: f32, alpha: f32, z_index: u32)
    {
        let transform = render_ctx.graphics.renderer.ui_matrix(center, (size, size), 0.0);
        render_ctx.graphics.renderer.draw_tinted_texture_ui(0, transform, 0, [1.0, 1.0, 1.0, alpha], z_index, self.shader_id);
    }

    pub fn draw(&self, render_ctx: &mut RenderContext, dash_count: i32, slot_count: i32, z_index: u32)
    {
        let filled = (dash_count - self.flying.len() as i32).max(0) as usize;

        (0..slot_count.max(0) as usize).for_each(|i|
        {
            let center = Self::slot_center(i);
            render_ctx.graphics.renderer.draw_ui(0, render_ctx.graphics.renderer.ui_matrix(center, (RECT_SIZE, RECT_SIZE), std::f32::consts::FRAC_PI_4), RECT_COLOR, z_index, 0);
            render_ctx.graphics.renderer.draw_rect_outline(center, (FRAME_SIZE, FRAME_SIZE), FRAME_THICKNESS, FRAME_COLOR, std::f32::consts::FRAC_PI_4, CoordSpace::Screen, DrawLayer::UI, z_index, 0);


            let alpha = SLOT_ALPHA[(i < filled) as usize];
            self.draw_orb(render_ctx, center, ORB_SIZE, alpha, z_index + 1);
        });

        self.flying.iter().enumerate().for_each(|(k, orb)|
        {
            let (pos, size) = Self::flight_pose(orb, Self::slot_center(filled + k));
            self.draw_orb(render_ctx, pos, size, 1.0, z_index + 2);
        });
    }
}

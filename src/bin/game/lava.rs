use engine::*;

const WAVE_MARGIN: f32 = 30.0;
const SCREEN_MARGIN: f32 = 40.0;

// Quad goes from a bit above surface to the bottom of the screen, so the player cant fall 'under' the lava
pub struct Lava
{
    surface_y: f32,
    start_y: f32,
    highest_player_y: f32,
    pub rise_speed: f32,
    pub max_lag: f32, // to stay always close to the player
    shader_id: u8
}

impl Lava
{
    pub fn new(start_y: f32) -> Self
    {
        Self
        {
            surface_y: start_y,
            start_y,
            highest_player_y: 0.0,
            rise_speed: 35.0, // 0.175 m/s
            max_lag: 900.0, // 4.5m below
            shader_id: 0
        }
    }

    pub fn set_shader(&mut self, shader_id: u8)
    {
        self.shader_id = shader_id;
    }

    pub fn reset(&mut self)
    {
        self.surface_y = self.start_y;
        self.highest_player_y = 0.0;
    }

    pub fn surface_y(&self) -> f32
    {
        self.surface_y
    }

    pub fn update(&mut self, player_pos: Vec2, dt: f32, rising: bool)
    {
        self.highest_player_y = self.highest_player_y.min(player_pos.y);

        let rise = self.rise_speed * dt * rising as u32 as f32;

        self.surface_y = (self.surface_y - rise).min(self.highest_player_y + self.max_lag);
    }

    pub fn touches(&self, pos: Vec2, radius: f32) -> bool
    {
        pos.y + radius > self.surface_y
    }

    pub fn draw(&self, render_ctx: &mut RenderContext, z_index: u32)
    {
        let camera = render_ctx.graphics.renderer.camera_pos;
        let view = render_ctx.graphics.renderer.virtual_size;

        let top = self.surface_y - WAVE_MARGIN;
        let bottom = camera.1 + view.1 * 0.5 + SCREEN_MARGIN;

        let height = (bottom - top).max(0.0);

        let center = (camera.0, top + height * 0.5);
        let size = (view.0 + SCREEN_MARGIN * 2.0, height);

        render_ctx.graphics.renderer.draw(0, render_ctx.graphics.renderer.matrix(center, size, 0.0), [1.0, 1.0, 1.0, 1.0], z_index, self.shader_id);
    }
}

use engine::*;

const WAVE_MARGIN: f32 = 240.0;
const SCREEN_MARGIN: f32 = 40.0;

// Splashes, sent to the shader as one mat4 (4 columns = 4 splashes)
const SPLASH_SLOTS: usize = 4;
const EMPTY_SPLASH: [f32; 4] = [0.0, -1000.0, 0.0, 0.0]; // start time far in the past -> invisible
const SPLASH_SCALE: f32 = 0.075; // strength = sqrt(mass * speed) * scale

// Quad goes from a bit above surface to the bottom of the screen, so the player cant fall 'under' the lava
pub struct Lava
{
    surface_y: f32,
    start_y: f32,
    highest_player_y: f32,
    pub rise_speed: f32,
    pub max_lag: f32, // to stay always close to the player
    shader_id: u8,
    time: f32, // same time as in main
    splashes: [[f32; 4]; SPLASH_SLOTS], // (x, start_time, strength, unused)
    next_splash: usize // ringbuffer, always replaces oldest splash
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
            shader_id: 0,
            time: 0.0,
            splashes: [EMPTY_SPLASH; SPLASH_SLOTS],
            next_splash: 0
        }
    }

    pub fn set_shader(&mut self, shader_id: u8)
    {
        self.shader_id = shader_id;
    }

    pub fn shader_id(&self) -> u8
    {
        self.shader_id
    }

    pub fn reset(&mut self)
    {
        self.surface_y = self.start_y;
        self.highest_player_y = 0.0;
        self.splashes = [EMPTY_SPLASH; SPLASH_SLOTS];
        self.next_splash = 0;
    }

    pub fn surface_y(&self) -> f32
    {
        self.surface_y
    }

    pub fn splash_uniform(&self) -> [[f32; 4]; 4]
    {
        self.splashes
    }

    pub fn update(&mut self, player_pos: Vec2, dt: f32, rising: bool, time: f32)
    {
        self.time = time;
        self.highest_player_y = self.highest_player_y.min(player_pos.y);

        let rise = self.rise_speed * dt * rising as u32 as f32;

        self.surface_y = (self.surface_y - rise).min(self.highest_player_y + self.max_lag);
    }

    pub fn check_entry(&mut self, pos: Vec2, velocity: Vec2, radius: f32, mass: f32, dt: f32)
    {
        let bottom = pos.y + radius;
        let previous_bottom = bottom - velocity.y * dt;

        let entered = (previous_bottom <= self.surface_y) & (bottom > self.surface_y);

        const SPLASH_TABLE: [fn(&mut Lava, f32, f32, f32); 2] = [Lava::no_splash, Lava::add_splash];
        SPLASH_TABLE[entered as usize](self, pos.x, velocity.y, mass);
    }

    fn no_splash(&mut self, _x: f32, _speed: f32, _mass: f32) {}
    fn add_splash(&mut self, x: f32, speed: f32, mass: f32)
    {
        let momentum = mass * speed.abs(); //kg*(px/s)
        let strength = momentum.sqrt() * SPLASH_SCALE;

        self.splashes[self.next_splash] = [x, self.time, strength, 0.0];
        self.next_splash = (self.next_splash + 1) % SPLASH_SLOTS;
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

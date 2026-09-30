use engine::*;

// Endless mode
pub const NO_SUMMIT: f32 = -1.0e30;

const SEGMENT_HEIGHT: f32 = 864.0; // side wall
const BORDER_WIDTH: f32 = 80.0;

// same as in shader
const CAP_HEIGHT: f32 = 90.0;   // whole quad
const CAP_SURFACE: f32 = 50.0;  // ground line, measured from the quad's top = exactly at summit_y
const CAP_OVERHANG: f32 = 24.0; // how far the ledge sticks out past the borders on each side

pub struct Wall
{
    pub width: f32,
    bounds: Rect,
    border_right: usize,
    rock_shader: u8,
    cap_shader: u8,
    summit_y: f32, // world y of the top of the wall (y points down, so higher = more negative)
}

impl Wall
{
    pub fn new(width: f32) -> Self
    {
        Self
        {
            width,
            bounds: Rect::new_from_center(0.0, 0.0, width as f64, 0.0),
            border_right: 0,
            rock_shader: 0,
            cap_shader: 0,
            summit_y: NO_SUMMIT,
        }
    }

    pub fn set_border_right_texture(&mut self, texture_id: usize)
    {
        self.border_right = texture_id;
    }

    pub fn set_rock_shader(&mut self, shader_id: u8)
    {
        self.rock_shader = shader_id;
    }

    pub fn set_cap_shader(&mut self, shader_id: u8)
    {
        self.cap_shader = shader_id;
    }

    // NO_SUMMIT for endless
    pub fn set_summit_y(&mut self, summit_y: f32)
    {
        self.summit_y = summit_y;
    }

    pub fn summit_y(&self) -> f32
    {
        self.summit_y
    }

    // left and right side of horizontal
    pub fn get_bounds(&self) -> (f32, f32)
    {
        (self.bounds.x as f32, (self.bounds.x+self.bounds.width) as f32)
    }

    pub fn contains(&self, pos: Vec2) -> bool
    {
        self.bounds.contains_x(pos.x as f64)
    }

    // The part of a vertical strip [top, bottom] that is below the summit.
    // Returns (center_y, height, cut) where cut is the fraction (0..1) cut off at the top.
    // Endless: summit is far above everything, so nothing gets cut.
    fn clip_to_summit(&self, top: f32, bottom: f32) -> (f32, f32, f32)
    {
        let visible_top = top.max(self.summit_y);
        let height = (bottom - visible_top).max(0.0);
        let cut = ((visible_top - top) / (bottom - top)).min(1.0);

        (visible_top + height * 0.5, height, cut)
    }

    pub fn draw(&self, render_ctx: &mut RenderContext, z_index: u32)
    {
        let camera = render_ctx.graphics.renderer.camera_pos;

        let left_x = -self.width * 0.5 - BORDER_WIDTH * 0.5;
        let right_x = self.width * 0.5 + BORDER_WIDTH * 0.5;

        // Everything still follows the camera, but nothing goes above the summit
        let (center_y, height, _) = self.clip_to_summit(camera.1 - SEGMENT_HEIGHT * 0.5, camera.1 + SEGMENT_HEIGHT * 0.5);

        // Left border
        render_ctx.graphics.renderer.draw(0, render_ctx.graphics.renderer.matrix((left_x, center_y), (BORDER_WIDTH, height), 0.0), [0.59, 0.56, 0.51, 1.0], z_index, 0);

        // Right border: two tiles, each cut at the summit on its own
        let offset = camera.1.rem_euclid(SEGMENT_HEIGHT);
        let y1 = camera.1 - offset;
        let y2 = y1 + SEGMENT_HEIGHT;
        self.draw_border_tile(render_ctx, right_x, y1, z_index);
        self.draw_border_tile(render_ctx, right_x, y2, z_index);

        // Rock
        render_ctx.graphics.renderer.draw(0, render_ctx.graphics.renderer.matrix((0.0, center_y), (self.width, height), 0.0), [0.47, 0.45, 0.4, 1.0], z_index, self.rock_shader);

        // Summit cap (grass ledge), covers the straight cut edge.
        // Same z as the wall: it's drawn after it because its pipeline was loaded later (higher id sorts later).
        // Endless: it sits at NO_SUMMIT, far outside of any view.
        let cap_width = self.width + 2.0 * BORDER_WIDTH + 2.0 * CAP_OVERHANG;
        let cap_center_y = self.summit_y - CAP_SURFACE + CAP_HEIGHT * 0.5;
        render_ctx.graphics.renderer.draw(0, render_ctx.graphics.renderer.matrix((0.0, cap_center_y), (cap_width, CAP_HEIGHT), 0.0), [1.0, 1.0, 1.0, 1.0], z_index, self.cap_shader);
    }

    // One right border tile. Cut at the summit by showing only the lower part of the texture.
    fn draw_border_tile(&self, render_ctx: &mut RenderContext, x: f32, center_y: f32, z_index: u32)
    {
        let (visible_center, height, cut) = self.clip_to_summit(center_y - SEGMENT_HEIGHT * 0.5, center_y + SEGMENT_HEIGHT * 0.5);

        let texture_size = render_ctx.graphics.renderer.texture_size(self.border_right);
        let rect_pos = (0.0, cut * texture_size.1);
        let rect_size = (texture_size.0, (1.0 - cut) * texture_size.1);

        render_ctx.graphics.renderer.draw_texture_atlas(0, render_ctx.graphics.renderer.matrix((x, visible_center), (BORDER_WIDTH, height), 0.0), self.border_right, rect_pos, rect_size, z_index, 0);
    }

    pub fn get_current_shader_id(&self) -> usize
    {
        self.rock_shader as usize
    }
}

use engine::*;

pub struct Wall
{
    pub width: f32,
    bounds: Rect,
    border_right: usize,
    rock_shader: u8
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
            rock_shader: 0
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

    // left and right side of horizontal
    pub fn get_bounds(&self) -> (f32, f32)
    {
        (self.bounds.x as f32, (self.bounds.x+self.bounds.width) as f32)
    }

    pub fn contains(&self, pos: Vec2) -> bool
    {
        self.bounds.contains_x(pos.x as f64)
    }

    pub fn draw(&self, render_ctx: &mut RenderContext, z_index: u32)
    {
        let camera = render_ctx.graphics.renderer.camera_pos;
        let border_width = 80.0;

        let left_x = -self.width * 0.5 - border_width * 0.5;
        let right_x = self.width * 0.5 + border_width * 0.5;

        // Border
        render_ctx.graphics.renderer.draw(0, render_ctx.graphics.renderer.matrix((left_x, camera.1), (border_width, 864.0), 0.0,), [0.59, 0.56, 0.51, 1.0], z_index, 0,);

        let offset = camera.1.rem_euclid(864.0); // Not sure if this counts as no if?

        let y1 = camera.1 - offset;
        let y2 = y1 + 864.0;

        render_ctx.graphics.renderer.draw_texture(0, render_ctx.graphics.renderer.matrix((right_x, y1), (border_width, 864.0), 0.0), self.border_right, z_index, 0);
        render_ctx.graphics.renderer.draw_texture(0, render_ctx.graphics.renderer.matrix((right_x, y2), (border_width, 864.0), 0.0), self.border_right, z_index, 0);
        // render_ctx.graphics.renderer.draw(0, render_ctx.graphics.renderer.matrix((right_x, camera.1), (border_width, 864.0), 0.0,), [0.34, 0.32, 0.29, 1.0], z_index, shader_id,);

        // [0.40, 0.30, 0.22, 1.0]
        // [0.20, 0.15, 0.12, 1.0]

        render_ctx.graphics.renderer.draw(0, render_ctx.graphics.renderer.matrix((0.0, camera.1), (self.width, 864.0), 0.0), [0.47, 0.45, 0.4, 1.0], z_index, self.rock_shader);
    }
}

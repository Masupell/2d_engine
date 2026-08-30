use engine::*;

pub struct Wall
{
    pub width: f32
}

impl Wall
{
    pub fn new(width: f32) -> Self
    {
        Self { width }
    }

    pub fn draw(&self, render_ctx: &mut RenderContext, z_index: u32, shader_id: u8)
    {
        let camera = render_ctx.graphics.renderer.camera_pos;
        let border_width = 80.0;

        let left_x = -self.width * 0.5 - border_width * 0.5;
        let right_x = self.width * 0.5 + border_width * 0.5;

        // Border
        render_ctx.graphics.renderer.draw(0, render_ctx.graphics.renderer.matrix((left_x, camera.1), (border_width, 864.0), 0.0,), [0.59, 0.56, 0.51, 1.0], z_index, shader_id,);
        render_ctx.graphics.renderer.draw(0, render_ctx.graphics.renderer.matrix((right_x, camera.1), (border_width, 864.0), 0.0,), [0.34, 0.32, 0.29, 1.0], z_index, shader_id,);

        // [0.40, 0.30, 0.22, 1.0]
        // [0.20, 0.15, 0.12, 1.0]

        render_ctx.graphics.renderer.draw(0, render_ctx.graphics.renderer.matrix((0.0, camera.1), (self.width, 864.0), 0.0), [0.47, 0.45, 0.4, 1.0], z_index, shader_id);
    }
}

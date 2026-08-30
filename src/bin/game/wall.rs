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
        render_ctx.graphics.renderer.draw(0, render_ctx.graphics.renderer.matrix((0.0, camera.1), (self.width, 864.0), 0.0), [0.47, 0.45, 0.4, 1.0], z_index, shader_id);
    }
}

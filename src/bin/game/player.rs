use engine::*;

pub struct Player
{
    rect: Rect,
    speed: f32,
    rotation: f32,
    texture_id: usize,
    pub score: i32,
    pub height: f32,
}

impl Player
{
    pub fn new(rect: Rect) -> Self
    {
        Player
        {
            rect,
            speed: 0.0,
            rotation: 0.0,
            texture_id: 0,
            score: 0,
            height: 0.0
        }
    }

    pub fn draw(&self, render_ctx: &mut RenderContext, z_index: u32, shader_id: u8)
    {
        let center = ((self.rect.x + self.rect.width/2.0) as f32, (self.rect.y + self.rect.height/2.0) as f32);
        render_ctx.renderer.draw_texture(0, render_ctx.renderer.matrix(center, (self.rect.width as f32, self.rect.height as f32), 0.0), self.texture_id, z_index, shader_id);
    }

    pub fn set_texture(&mut self, texture_id: usize)
    {
        self.texture_id = texture_id;
    }

    pub fn set_pos(&mut self, pos: (f32, f32))
    {
        self.rect.x = pos.0 as f64;
        self.rect.y = pos.1 as f64;
    }

    pub fn set_size(&mut self, size: (f32, f32))
    {
        self.rect.width = size.0 as f64;
        self.rect.height = size.1 as f64;
    }
}

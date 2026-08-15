use engine::*;

pub struct Player
{
    pub collision: Triangle,

    width: f32,
    height: f32,

    texture_id: usize,

    pub speed: f32,
    pub score: i32,
}

impl Player
{
    pub fn new(center: (f32, f32), width: f32, height: f32) -> Self
    {
        // Expects it in local coordinates
        let a = (0.0, -height/2.0); // top point
        let b = (width/2.0, height/2.0); // bottom-right
        let c = (-width/2.0, height/2.0); // bottom-left
        let collision: Triangle = Triangle::new(center.0, center.1, 0.0, a, b, c);

        Player
        {
            collision,
            width,
            height,
            texture_id: 0,
            speed: 0.0,
            score: 0
        }
    }

    pub fn update(&mut self, input: &Input)
    {
        let inside = self.collision.contains(input.mouse_position_f32());
        if inside
        {
            println!("Mouse inside Triangle");
        }
    }

    pub fn draw(&self, render_ctx: &mut RenderContext, z_index: u32, shader_id: u8)
    {
        render_ctx.renderer.draw_texture(0, render_ctx.renderer.matrix((self.collision.x, self.collision.y), (self.width, self.height), self.collision.rotation), self.texture_id, z_index, shader_id);
    }

    pub fn set_texture(&mut self, texture_id: usize)
    {
        self.texture_id = texture_id;
    }

    pub fn set_pos(&mut self, pos: (f32, f32))
    {
        self.collision.x = pos.0;
        self.collision.y = pos.1;
    }

    // In radians
    pub fn rotate(&mut self, amount: f32)
    {
        self.collision.rotate(amount);
    }

    // Would not change collision, so dont do that yet
    pub fn set_size(&mut self, size: (f32, f32))
    {
        self.width = size.0;
        self.height = size.1;
    }
}

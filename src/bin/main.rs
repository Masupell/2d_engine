use engine::{asset_manager::TextureHandle, *};
// use rand::Rng;


struct App { texture: TextureHandle, x: f32, y: f32}

impl EngineEvent for App 
{
    fn setup(&mut self, ctx: &mut Context)
    {
        ctx.toggle_vsync();
        // ctx.toggle_fullscreen();
        // ctx.assets.load_texture("src/image/owl.jpg", |id|
        // {
        //     println!("Texture loaded! ID = {}", id);
        // });
        // self.texture = ctx.assets.textures.get_texture(1);
        // loader.load_texture("src/image/owl.jpg");
        // loader.load_shader(Some("src/shaders/test.wgsl"), None);
        // loader.load_shader(Some("src/shaders/post_process.wgsl"), Some("src/shaders/post_process.wgsl"));
    }

    fn update(&mut self, update_ctx: &mut UpdateContext)
    {
        if update_ctx.input.is_key_pressed(winit::keyboard::KeyCode::F11)
        {
            update_ctx.context.toggle_fullscreen();
        }
        if update_ctx.input.is_key_pressed(winit::keyboard::KeyCode::KeyQ)
        {
            update_ctx.context.toggle_vsync();
        }
        if update_ctx.input.is_key_pressed(winit::keyboard::KeyCode::KeyW)
        {
            self.texture = update_ctx.context.assets.request_texture("src/image/owl.jpg", |id|
            {
                println!("Texture loaded! ID = {}", id.0);
            });
        }
        
        self.x = update_ctx.input.mouse_position().0 as f32;
        self.y = update_ctx.input.mouse_position().1 as f32;
        // self.texture = update_ctx.context.assets.textures.get_texture(1);
    }

    fn render(&self, render_ctx: &mut RenderContext)
    {
        if let Some(texture) = render_ctx.context.assets.textures.get_texture(&self.texture)
        {
            render_ctx.renderer.draw_texture(0, render_ctx.renderer.texture_matrix((render_ctx.renderer.virtual_size.0/2.0, render_ctx.renderer.virtual_size.1/2.0), (1.0, 1.0), 0.0, (1920.0, 1014.0)), texture.clone(), 0, 0);
        }
        // render_ctx.renderer.draw_texture(0, render_ctx.renderer.texture_matrix((self.x, self.y), (0.5, 0.5), 0.0, (1920.0, 1014.0)), 1, 0, 1);
    }
}

impl App
{
    fn new() -> Self 
    {
        Self {texture: TextureHandle::default(), x: 0.0, y: 0.0}
    }
}

fn main() 
{
    pollster::block_on(game_loop(Box::new(App::new()), "Performance", (1280, 720)));
}
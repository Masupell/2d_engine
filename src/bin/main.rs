use engine::*;
// use rand::Rng;


struct App { x: f32, y: f32}

impl EngineEvent for App 
{
    fn setup(&mut self, loader: &mut dyn state::Loader) 
    {
        loader.load_texture("src/image/owl.jpg");
        loader.load_shader(Some("src/shaders/test.wgsl"), None);
        loader.load_shader(Some("src/shaders/post_process.wgsl"), Some("src/shaders/post_process.wgsl"));
    }

    fn update(&mut self, input: &Input, _dt: f64) 
    {
        self.x = input.mouse_position().0 as f32;
        self.y = input.mouse_position().1 as f32;
    }

    fn render(&self, renderer: &mut Renderer) 
    {
        renderer.draw_texture(0, renderer.texture_matrix((renderer.virtual_size.0/2.0, renderer.virtual_size.1/2.0), (1.0, 1.0), 0.0, (1920.0, 1014.0)), 1, 0, 0);
        renderer.draw_texture(0, renderer.texture_matrix((self.x, self.y), (0.5, 0.5), 0.0, (1920.0, 1014.0)), 1, 0, 1);
    }
}

impl App
{
    fn new() -> Self 
    {
        Self {x: 0.0, y: 0.0}
    }
}

fn main() 
{
    pollster::block_on(game_loop(Box::new(App::new()), "Performance", (1280, 720)));
}
use engine::*;
// use rand::Rng;


struct App {}

impl EngineEvent for App 
{
    fn setup(&mut self, loader: &mut dyn state::Loader) 
    {
        loader.load_texture("src/image/owl.jpg");
        loader.load_shader("src/shaders/test.wgsl");
    }

    fn update(&mut self, _input: &Input, _dt: f64) {}

    fn render(&self, renderer: &mut Renderer) 
    {
        renderer.draw_texture(0, renderer.texture_matrix((renderer.virtual_size.0/2.0, renderer.virtual_size.1/2.0), (1.0, 1.0), 0.0, (1920.0, 1014.0)), 1, 0, 0);
        renderer.draw_texture(0, renderer.texture_matrix((renderer.virtual_size.0/2.0, renderer.virtual_size.1/2.0), (0.5, 0.5), 0.0, (1920.0, 1014.0)), 1, 0, 1);
    }
}

impl App
{
    fn new() -> Self 
    {
        Self {}
    }
}

fn main() 
{
    pollster::block_on(game_loop(Box::new(App::new()), "Performance", (1280, 720)));
}
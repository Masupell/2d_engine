use engine::*;
use rand::Rng;

struct Quad 
{
    pos: (f32, f32),
    size: f32,
    color: [f32; 4],
    vel: (f32, f32),
}

struct App 
{
    quads: Vec<Quad>,
    width: f32,
    height: f32,
}

impl EngineEvent for App 
{
    fn setup(&mut self, _loader: &mut dyn state::Loader) {}

    fn update(&mut self, _input: &Input, dt: f64) 
    {
        for quad in &mut self.quads 
        {
            quad.pos.0 += quad.vel.0 * dt as f32;
            quad.pos.1 += quad.vel.1 * dt as f32;

            if quad.pos.0 < 0.0 || quad.pos.0 + quad.size > self.width 
            {
                quad.vel.0 *= -1.0;
            }
            if quad.pos.1 < 0.0 || quad.pos.1 + quad.size > self.height 
            {
                quad.vel.1 *= -1.0;
            }
        }
    }

    fn render(&self, renderer: &mut Renderer) 
    {
        for quad in &self.quads 
        {
            renderer.draw(0, renderer.matrix(quad.pos, (quad.size, quad.size), 0.0), quad.color, 1);
        }
    }
}

impl App
{
    fn new(width: f32, height: f32, count: usize) -> Self 
    {
        let mut rng = rand::rng();
        let mut quads = Vec::with_capacity(count);

        for _ in 0..count 
        {
            let size = rng.random_range(10.0..40.0);
            let pos = (rng.random_range(0.0..(width - size)), rng.random_range(0.0..(height - size)));
            let color = 
            [
                rng.random_range(0.0..1.0),
                rng.random_range(0.0..1.0),
                rng.random_range(0.0..1.0),
                1.0,
            ];
            let vel = (rng.random_range(-100.0..100.0), rng.random_range(-100.0..100.0));
            quads.push(Quad { pos, size, color, vel });
        }

        Self { quads, width, height }
    }
}

fn main() 
{
    pollster::block_on(game_loop(Box::new(App::new(1280.0, 720.0, 50000)), "Performance", (1280, 720)));
}
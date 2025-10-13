use std::f32::consts::PI;

use engine::*;
use rand::Rng;
use winit::{event::MouseButton, keyboard::KeyCode};

struct App 
{
    quads: Vec<((f32, f32), f32, [f32; 4])>
}

impl EngineEvent for App
{
    fn setup(&mut self, loader: &mut dyn state::Loader) 
    {

    }
    
    fn update(&mut self, input: &Input, dt: f64) 
    {

    }
    fn render(&self, renderer: &mut Renderer) //Column-major layout
    {  
        for ((x, y), size, color) in &self.quads
        {
            renderer.draw(
                0, // your white 1x1 texture
                renderer.matrix((*x, *y), (*size, *size), 0.0),
                *color,
                1, // instance count or layer, adjust if needed
            );
        }
    }
}

impl App
{
    fn new() -> Self
    {
        let mut rng = rand::rng();
        let mut quads = Vec::with_capacity(10000);
        for _ in 0..10000
        {
            let x = rng.random_range(0.0..1280.0);
            let y = rng.random_range(0.0..720.0);
            let size = rng.random_range(10.0..40.0);
            let color = 
            [
                rng.random_range(0.0..1.0),
                rng.random_range(0.0..1.0),
                rng.random_range(0.0..1.0),
                1.0,
            ];
            quads.push(((x, y), size, color));
        }
        
        Self 
        { 
            quads
        }
    }
}

fn main()
{
    pollster::block_on(game_loop(Box::new(App::new()), "Animate", (1280, 720)))
}
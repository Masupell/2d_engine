use std::collections::HashMap;

use engine::*;
use rand::Rng;

use crate::wall::Wall;

// Hysteresis added on top of distance_from_player before a decoration
// behind the player gets recycled - see maintain(). A normal bit of drift
// right at the boundary doesn't immediately despawn something still worth
// having around.
const DESPAWN_BUFFER: f32 = 100.0;

const MIN_SCALE: f32 = 0.8;
const MAX_SCALE: f32 = 1.3;

const CELL_SIZE: f32 = 200.0;

// Max amount it tries to spawn without overlapping
const MAX_SPAWN_ATTEMPTS: usize = 6;

#[derive(Clone, Copy)]
struct Variant
{
    rect_pos: (f32, f32),
    rect_size: (f32, f32),
    draw_height: f32,
    solid: bool,
}

struct Decoration
{
    active: bool,
    pos: Vec2,
    rect_pos: (f32, f32),
    rect_size: (f32, f32),
    draw_size: (f32, f32), // aspect corrected
    flip_x: f32,
    solid: bool,
}

impl Decoration
{
    fn inactive() -> Self
    {
        Self
        {
            active: false,
            pos: Vec2::ZERO,
            rect_pos: (0.0, 0.0),
            rect_size: (0.0, 0.0),
            draw_size: (0.0, 0.0),
            flip_x: 1.0,
            solid: false,
        }
    }

    fn draw(&self, render_ctx: &mut RenderContext, texture_id: usize, z_index: u32, shader_id: u8)
    {
        let size = (self.draw_size.0 * self.flip_x, self.draw_size.1);
        render_ctx.graphics.renderer.draw_texture_atlas(0, render_ctx.graphics.renderer.matrix((self.pos.x, self.pos.y), size, 0.0), texture_id, self.rect_pos, self.rect_size, z_index, shader_id);
    }
}

fn rects_overlap(pos_a: Vec2, size_a: (f32, f32), pos_b: Vec2, size_b: (f32, f32)) -> bool
{
    let overlap_x = (pos_a.x - pos_b.x).abs() < (size_a.0 + size_b.0) * 0.5;
    let overlap_y = (pos_a.y - pos_b.y).abs() < (size_a.1 + size_b.1) * 0.5;

    overlap_x & overlap_y
}

pub struct DecorationSpawner
{
    pool: Vec<Decoration>,
    texture_id: usize,
    variants: Vec<Variant>,
    // Cell coord -> indices into `pool`, covering every active decoration
    // (not just solid ones) - used both for spawn-time overlap avoidance
    // and, filtered by .solid, for the player collision query. Rebuilt
    // once per maintain() call from the current active set (after
    // despawning, before spawning), then kept in sync incrementally as
    // spawn_random adds new ones within that same call.
    grid: HashMap<(i32, i32), Vec<usize>>,
}

impl DecorationSpawner
{
    pub fn new() -> Self
    {
        Self
        {
            pool: Vec::new(),
            texture_id: 0,
            variants: Vec::new(),
            grid: HashMap::new(),
        }
    }

    pub fn set_texture(&mut self, texture_id: usize)
    {
        self.texture_id = texture_id;
    }

    pub fn add_variant(&mut self, rect_pos: (f32, f32), rect_size: (f32, f32), draw_height: f32, solid: bool)
    {
        self.variants.push(Variant { rect_pos, rect_size, draw_height, solid });
    }

    pub fn draw(&self, render_ctx: &mut RenderContext, z_index: u32, shader_id: u8)
    {
        self.pool.iter().filter(|d| d.active).for_each(|d| d.draw(render_ctx, self.texture_id, z_index, shader_id));
    }

    fn active_count(&self) -> usize
    {
        self.pool.iter().filter(|d| d.active).count()
    }

    fn cell_coord(pos: Vec2) -> (i32, i32)
    {
        ((pos.x / CELL_SIZE).floor() as i32, (pos.y / CELL_SIZE).floor() as i32)
    }

    fn grid_insert(&mut self, index: usize, pos: Vec2)
    {
        let cell = Self::cell_coord(pos);
        self.grid.entry(cell).or_insert_with(Vec::new).push(index);
    }

    fn rebuild_grid(&mut self)
    {
        self.grid.clear();

        self.pool.iter().enumerate().filter(|(_, d)| d.active).for_each(|(index, d)|
        {
            let cell = Self::cell_coord(d.pos);
            self.grid.entry(cell).or_insert_with(Vec::new).push(index);
        });
    }

    fn nearby_indices(&self, pos: Vec2) -> impl Iterator<Item = usize> + '_
    {
        let (cell_x, cell_y) = Self::cell_coord(pos);

        (-1..=1).flat_map(move |dy|
        {
            (-1..=1).filter_map(move |dx| self.grid.get(&(cell_x + dx, cell_y + dy)))
        }).flatten().copied()
    }

    fn overlaps_any(&self, pos: Vec2, size: (f32, f32)) -> bool
    {
        self.nearby_indices(pos).any(|index| rects_overlap(pos, size, self.pool[index].pos, self.pool[index].draw_size))
    }

    pub fn nearby_solid_rects(&self, pos: Vec2) -> impl Iterator<Item = (Vec2, (f32, f32))> + '_
    {
        self.nearby_indices(pos).filter(|&index| self.pool[index].solid).map(|index| (self.pool[index].pos, self.pool[index].draw_size))
    }

    fn spawn(&mut self, pos: Vec2, variant: Variant, scale: f32, flip_x: f32)
    {
        let index = self.pool.iter().position(|d| !d.active).unwrap_or_else(||
        {
            self.pool.push(Decoration::inactive());
            self.pool.len() - 1
        });

        let aspect = variant.rect_size.0 / variant.rect_size.1;
        let height = variant.draw_height * scale;
        let draw_size = (height * aspect, height);

        self.pool[index] = Decoration
        {
            active: true,
            pos,
            rect_pos: variant.rect_pos,
            rect_size: variant.rect_size,
            draw_size,
            flip_x,
            solid: variant.solid,
        };

        self.grid_insert(index, pos);
    }

    fn spawn_random(&mut self, wall: &Wall, player_pos: Vec2, dir: f32, distance_from_player: f32, spread: f32)
    {
        let bounds = wall.get_bounds();
        let margin = 20.0;
        let mut rng = rand::rng();

        let variant_index = rng.random_range(0..self.variants.len());
        let variant = self.variants[variant_index];
        let scale = rng.random_range(MIN_SCALE..MAX_SCALE);

        let aspect = variant.rect_size.0 / variant.rect_size.1;
        let height = variant.draw_height * scale;
        let footprint = (height * aspect, height);

        let near = player_pos.y + dir * distance_from_player;
        let far = near + dir * spread;

        let y_min = near.min(far);
        let y_max = near.max(far);

        const FLIP_TABLE: [f32; 2] = [1.0, -1.0];
        let flip_x = FLIP_TABLE[rng.random_range(0..2)];

        let pos = (0..MAX_SPAWN_ATTEMPTS).map(|_| Vec2::new(rng.random_range(bounds.0 + margin..bounds.1 - margin), rng.random_range(y_min..y_max))).find(|candidate| !self.overlaps_any(*candidate, footprint)).unwrap_or_else(|| Vec2::new(rng.random_range(bounds.0 + margin..bounds.1 - margin), rng.random_range(y_min..y_max)));

        self.spawn(pos, variant, scale, flip_x);
    }

    pub fn maintain(&mut self, wall: &Wall, player_pos: Vec2, direction_y: f32, distance_from_player: f32, spread: f32, target_count: usize)
    {
        let moving_down = (direction_y > 0.0) as i32 as f32;
        let dir = moving_down * 2.0 - 1.0;

        let despawned = self.pool.iter_mut().filter(|d| d.active & (((d.pos.y - player_pos.y) * dir) < -distance_from_player)).map(|d| d.active = false).count();

        let deficit = target_count.saturating_sub(self.active_count());

        let needs_rebuild = (despawned > 0) | (deficit > 0);
        (0..needs_rebuild as usize).for_each(|_| self.rebuild_grid());

        (0..deficit).for_each(|_| self.spawn_random(wall, player_pos, dir, distance_from_player, spread));
    }
}

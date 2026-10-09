use crate::{renderer::{Renderer, MODE_COLOR, MODE_TEXTURE, WHITE_TEXTURE}, utility::{CoordSpace, DrawLayer, FULL_UV_RECT}};

pub const QUAD_MESH: usize = 0;

#[derive(Clone, Copy)]
pub struct Draw
{
    pub pos: (f32, f32), // center
    pub size: (f32, f32),
    pub rotation: f32,
    pub color: [f32; 4],
    pub texture: usize,
    pub uv_rect: [f32; 4],
    pub outline: f32, // 0.0 = filled
    pub mesh: usize,
    pub space: CoordSpace,
    pub layer: DrawLayer,
    pub z: u32,
    pub shader: u8
}

impl Default for Draw
{
    fn default() -> Self
    {
        Self
        {
            pos: (0.0, 0.0),
            size: (1.0, 1.0),
            rotation: 0.0,
            color: [1.0, 1.0, 1.0, 1.0],
            texture: WHITE_TEXTURE,
            uv_rect: FULL_UV_RECT,
            outline: 0.0,
            mesh: 0,
            space: CoordSpace::World,
            layer: DrawLayer::World,
            z: 0,
            shader: 0
        }
    }
}

impl Draw
{
    pub fn rect(pos: (f32, f32), size: (f32, f32), rotation: f32, color: [f32; 4]) -> Self
    {
        Self { pos, size, rotation, color, ..Default::default() }
    }

    pub fn sprite(texture: usize, pos: (f32, f32), size: (f32, f32), rotation: f32) -> Self
    {
        Self { pos, size, rotation, texture, ..Default::default() }
    }

    pub fn world_mesh(mesh: usize) -> Self
    {
        Self { mesh, size: (1.0, -1.0), ..Default::default() }
    }

    pub fn ui(self) -> Self
    {
        Self { space: CoordSpace::Screen, layer: DrawLayer::UI, ..self }
    }
}


#[derive(Clone, Copy, PartialEq, Debug)]
pub enum TextAlign
{
    TopLeft,
    Center
}

#[derive(Clone, Copy)]
pub struct Text<'a>
{
    pub text: &'a str,
    pub pos: (f32, f32),
    pub height: f32, // virtual pixel size
    pub color: [f32; 4],
    // (color, width), only looks good for small widths
    pub outline: Option<([f32; 4], f32)>,
    pub align: TextAlign,
    pub font: usize,
    pub rotation: f32,
    pub space: CoordSpace,
    pub layer: DrawLayer,
    pub z: u32,
    pub shader: u8
}

impl Default for Text<'_>
{
    fn default() -> Self
    {
        Self
        {
            text: "",
            pos: (0.0, 0.0),
            height: 32.0,
            color: [1.0, 1.0, 1.0, 1.0],
            outline: None,
            align: TextAlign::TopLeft,
            font: 0,
            rotation: 0.0,
            space: CoordSpace::World,
            layer: DrawLayer::World,
            z: 0,
            shader: 0
        }
    }
}

impl<'a> Text<'a>
{
    pub fn new(text: &'a str, pos: (f32, f32), height: f32, color: [f32; 4]) -> Self
    {
        Self { text, pos, height, color, ..Default::default() }
    }

    pub fn center(self) -> Self
    {
        Self { align: TextAlign::Center, ..self }
    }

    pub fn ui(self) -> Self
    {
        Self { space: CoordSpace::Screen, layer: DrawLayer::UI, ..self }
    }
}


impl Renderer
{
    // basically for a post-proceses effect with a NormalWithScreen pipeline
    pub fn draw_fullscreen(&mut self, texture_id: usize, color: [f32; 4], z_index: u32, id: u8)
    {
        let transform = self.matrix((self.virtual_size.0 * 0.5, self.virtual_size.1 * 0.5), self.view_size, 0.0);
        self.push_command(0, transform, texture_id, color, MODE_TEXTURE, FULL_UV_RECT, CoordSpace::Screen, DrawLayer::World, z_index, id);
    }

    pub fn draw(&mut self, d: Draw)
    {
        if d.outline > 0.0
        {
            self.push_rect_outline(&d);
            return;
        }
        let transform = self.matrix(d.pos, d.size, d.rotation);
        // always using MODE_TEXTURE right now
        self.push_command(d.mesh, transform, d.texture, d.color, MODE_TEXTURE, d.uv_rect, d.space, d.layer, d.z, d.shader);
    }

    pub fn draw_text(&mut self, t: Text)
    {
        let pos = match t.align
        {
            TextAlign::TopLeft => t.pos,
            TextAlign::Center =>
            {
                let (width, height) = self.text_size_with_font(t.font, t.text, t.height);
                (t.pos.0 - width * 0.5, t.pos.1 - height * 0.5)
            }
        };

        if let Some((outline_color, outline_width)) = t.outline
        {
            const DIRECTIONS: [(f32, f32); 8] =
            [
                (-1.0, -1.0), (0.0, -1.0), (1.0, -1.0),
                (-1.0,  0.0),              (1.0,  0.0),
                (-1.0,  1.0), (0.0,  1.0), (1.0,  1.0),
            ];

            let steps = outline_width.ceil().max(1.0) as i32;
            for step in 1..=steps
            {
                let radius = outline_width * step as f32 / steps as f32;
                for (dx, dy) in DIRECTIONS
                {
                    let offset_pos = (pos.0 + dx * radius, pos.1 + dy * radius);
                    self.push_text(&t, offset_pos, outline_color);
                }
            }
        }

        self.push_text(&t, pos, t.color);
    }

    fn push_text(&mut self, t: &Text, pos: (f32, f32), color: [f32; 4])
    {
        let atlas = &self.fonts[t.font];
        let scale = t.height / atlas.native_size;
        let texture_id = atlas.texture_id;
        let ascent = atlas.ascent;
        let line_height = atlas.line_height;

        let baseline_pos = (pos.0, pos.1 + ascent * scale);
        let (width, height) = self.text_size_with_font(t.font, t.text, t.height);
        let center = (pos.0 + width * 0.5, pos.1 + height * 0.5);
        let origin = rotate_point_around(baseline_pos, center, t.rotation);

        let cos = t.rotation.cos();
        let sin = t.rotation.sin();

        let mut cursor_y = 0.0;
        for line in t.text.lines()
        {
            let mut cursor_x = 0.0;
            for ch in line.chars()
            {
                let Some(glyph) = self.fonts[t.font].glyphs.get(&ch) else { continue; };

                let (offset, size, uv_min, uv_max, advance) = (glyph.offset, glyph.size, glyph.uv_min, glyph.uv_max, glyph.advance);

                if size[0] > 0.0 && size[1] > 0.0
                {
                    let y_top = cursor_y - offset[1];
                    let local = (cursor_x + offset[0] + size[0] * 0.5, y_top - size[1] * 0.5);

                    let world = (origin.0 + (cos * local.0 + sin * local.1) * scale, origin.1 + (sin * local.0 - cos * local.1) * scale);
                    let glyph_size = (size[0] * scale, size[1] * scale);

                    let transform = self.matrix(world, glyph_size, t.rotation);
                    let uv_rect = [uv_min[0], uv_min[1], uv_max[0] - uv_min[0], uv_max[1] - uv_min[1]];

                    self.push_command(QUAD_MESH, transform, texture_id, color, MODE_TEXTURE, uv_rect, t.space, t.layer, t.z, t.shader);
                }

                cursor_x += advance;
            }
            cursor_y -= line_height;
        }
    }

    // no anti-aliasing right now
    fn push_rect_outline(&mut self, d: &Draw)
    {
        let half = (d.size.0 * 0.5, d.size.1 * 0.5);
        let t = d.outline.min(half.0).min(half.1);
        let inner_height = d.size.1 - 2.0 * t;

        let edges =
        [
            ((0.0, -half.1 + t * 0.5), (d.size.0, t)), // top
            ((0.0,  half.1 - t * 0.5), (d.size.0, t)), // bottom
            ((-half.0 + t * 0.5, 0.0), (t, inner_height)), // left
            (( half.0 - t * 0.5, 0.0), (t, inner_height)), // right
        ];

        let cos = d.rotation.cos();
        let sin = d.rotation.sin();

        for (offset, edge_size) in edges
        {
            let rotated = (offset.0 * cos - offset.1 * sin, offset.0 * sin + offset.1 * cos);
            let pos = (d.pos.0 + rotated.0, d.pos.1 + rotated.1);

            let transform = self.matrix(pos, edge_size, d.rotation);
            self.push_command(QUAD_MESH, transform, WHITE_TEXTURE, d.color, MODE_COLOR, FULL_UV_RECT, d.space, d.layer, d.z, d.shader);
        }
    }



    pub fn texture_size(&self, texture_id: usize) -> (f32, f32)
    {
        self.textures[texture_id].size
    }

    pub fn text_size(&self, text: &str, height_px: f32) -> (f32, f32)
    {
        self.text_size_with_font(0, text, height_px)
    }

    pub fn text_size_with_font(&self, font_id: usize, text: &str, height_px: f32) -> (f32, f32)
    {
        let atlas = &self.fonts[font_id];
        let scale = height_px / atlas.native_size;

        let width = self.measure_text_width_with_font(font_id, text, height_px);

        let line_count = text.lines().count().max(1);
        let scaled_line_height = atlas.line_height * scale;
        let height = height_px + (line_count - 1) as f32 * scaled_line_height;

        (width, height)
    }

    pub fn measure_text_width(&self, text: &str, height_px: f32) -> f32
    {
        self.measure_text_width_with_font(0, text, height_px)
    }

    pub fn measure_text_width_with_font(&self, font_id: usize, text: &str, height_px: f32) -> f32
    {
        let atlas = &self.fonts[font_id];
        let scale = height_px / atlas.native_size;

        let width = text.lines().map(|line| line.chars().filter_map(|ch| atlas.glyphs.get(&ch)).map(|glyph| glyph.advance).sum::<f32>()).fold(0.0_f32, f32::max);
        width * scale
    }



    // to position ui-elements at the actual view_screen borders (if not ScaleMode::LetterBox)
    // (left, top, right, bottom)
    pub fn ui_bounds(&self) -> (f32, f32, f32, f32)
    {
        let extra_x = (self.view_size.0 - self.virtual_size.0) * 0.5;
        let extra_y = (self.view_size.1 - self.virtual_size.1) * 0.5;

        (-extra_x, -extra_y, self.virtual_size.0 + extra_x, self.virtual_size.1 + extra_y)
    }

    pub fn pixel_matrix(&self, pos: (f32, f32), size: (f32, f32), rotation: f32) -> [[f32; 4]; 4]
    {
        let to_virtual = (self.virtual_size.0 / self.window_size.0, self.virtual_size.1 / self.window_size.1);

        let virtual_pos = (pos.0 * to_virtual.0, pos.1 * to_virtual.1);
        let virtual_size = (size.0 * to_virtual.0, size.1 * to_virtual.1);

        self.ui_matrix(virtual_pos, virtual_size, rotation)
    }

    // stays with virtual size, centered in view_size
    // so, left of it would be negative
    pub fn ui_matrix(&self, pos: (f32, f32), size: (f32, f32), rotation: f32) -> [[f32; 4]; 4]
    {
        let world_pos =
        (
            self.camera_pos.0 + pos.0 - self.virtual_size.0 * 0.5,
            self.camera_pos.1 + pos.1 - self.virtual_size.1 * 0.5 // +, - => (0,0) is top-left, -,+ => (0,0) is bottom-left
        );

        self.matrix(world_pos, size, rotation)
    }

    // Still draws with pixels, but this time everything gets drawn like it looks with the original screen-size, so resized looks the same (in relation to each other)
    // If using this, when trying to use the windowsize, use virtual_size instead of window_size
    // Because everything here is in relation to the original "virtual" size, not the actual window size
    pub fn matrix(&self, pos: (f32, f32), size: (f32, f32), rotation: f32) -> [[f32; 4]; 4]
    {
        let cos = rotation.cos();
        let sin = rotation.sin();

        [
            [cos*size.0, sin*size.0, 0.0, 0.0],
            [sin*size.1, -cos*size.1, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [pos.0, pos.1, 0.0, 1.0]
        ]
    }

    // Size in relative to the original, not pixels
    pub fn texture_matrix(&self, pos: (f32, f32), scale: (f32, f32), rotation: f32, texture_size: (f32, f32)) -> [[f32; 4]; 4]
    {
        self.matrix(pos, (texture_size.0 * scale.0, texture_size.1 * scale.1), rotation)
    }

    // virtual pixels to world_position
    pub fn screen_to_world(&self, screen: (f32, f32)) -> (f32, f32)
    {
        let scale = self.pixels_per_unit * self.camera_zoom;
        let dx = (screen.0 - self.virtual_size.0 * 0.5) / scale;
        let dy = (screen.1 - self.virtual_size.1 * 0.5) / scale;

        let (sin, cos) = self.camera_rotation.sin_cos();
        (
            self.camera_pos.0 + dx * cos - dy * sin,
            self.camera_pos.1 + dx * sin + dy * cos
        )
    }

    // world_position to virtual pixels
    pub fn world_to_screen(&self, world: (f32, f32)) -> (f32, f32)
    {
        let scale = self.pixels_per_unit * self.camera_zoom;
        let dx = world.0 - self.camera_pos.0;
        let dy = world.1 - self.camera_pos.1;

        let (sin, cos) = self.camera_rotation.sin_cos();
        (
            (dx * cos + dy * sin) * scale + self.virtual_size.0 * 0.5,
            (-dx * sin + dy * cos) * scale + self.virtual_size.1 * 0.5
        )
    }
}

// pivot point to rotate around (matrix uses the center, text uses top-left)
fn rotate_point_around(point: (f32, f32), pivot: (f32, f32), rotation: f32) -> (f32, f32)
{
    let cos = rotation.cos();
    let sin = rotation.sin();
    let dx = point.0 - pivot.0;
    let dy = point.1 - pivot.1;

    (pivot.0 + dx * cos - dy * sin, pivot.1 + dx * sin + dy * cos)
}

use std::collections::HashMap;

use ab_glyph::{point, Font, FontArc, PxScale};
use anyhow::{Ok, anyhow};

pub fn rasterize_char(font_path: &str, char: char) -> std::result::Result<(Vec<u8>, usize, usize), anyhow::Error>
{
    let font_data = std::fs::read(font_path)?;

    let font = FontArc::try_from_vec(font_data)?;

    let scale = PxScale::from(64.0);

    let glyph = font.glyph_id(char).with_scale_and_position(scale, point(0.0, 0.0));

    let outline = font.outline_glyph(glyph).ok_or_else(|| anyhow!("Could not outline glyph '{}'", char))?;

    let bounds = outline.px_bounds();
    let width = bounds.width().ceil() as usize;
    let height = bounds.height().ceil() as usize;
    println!("Width: {}, Heigth: {}", width, height);

    let mut bitmap = vec![0u8; width * height];
    outline.draw(|x, y, c| 
    { 
        let xx = x as usize;
        let yy = y as usize;

        if xx < width && yy < height
        {
            bitmap[yy * width + xx] = (c * 255.0) as u8;
        }
    });

    Ok((bitmap, width, height))
}


pub struct Glyph
{
    pub uv_min: [f32; 2], // top left (uv coordinate in texture)
    pub uv_max: [f32; 2], // top right ("-")
    pub size: [f32; 2], // width and height of bitmap
    pub offset: [f32; 2], // offset of baseline (like down or Up, forp or l)
    pub advance: f32, // Space after Glyph
}


pub fn rasterize_text(font_path: &str, text: &str, text_scale: f32) -> std::result::Result<(Vec<u8>, usize, usize, Vec<Glyph>), anyhow::Error>
{
    let font_data = std::fs::read(font_path)?;
    let font = FontArc::try_from_vec(font_data)?;
    let scale = PxScale::from(text_scale);

    let mut bitmaps = Vec::new();
    let mut glyphs: Vec<Glyph> = Vec::new();

    let mut total_width = 0;
    let mut total_height = 0;

    let mut offset: [f32; 2] = [0.0, 0.0];

    for char in text.chars()
    {
        let glyph = font.glyph_id(char).with_scale_and_position(scale, point(0.0, 0.0));

        println!("{}", char);

        let outline = font.outline_glyph(glyph).ok_or_else(|| anyhow!("Could not outline glyph '{}'", char))?;

        let bounds = outline.px_bounds();
        let width = bounds.width().ceil() as usize;
        let height = bounds.height().ceil() as usize;

        offset = [bounds.min.x, bounds.min.y];

        // let advance = 

        let mut bitmap = vec![0u8; width * height];
        outline.draw(|x, y, c| 
        { 
            let xx = x as usize;
            let yy = y as usize;

            if xx < width && yy < height
            {
                bitmap[yy * width + xx] = (c * 255.0) as u8;
            }
        });

        bitmaps.push((bitmap, width, height));
        total_width += width; // Just a horizontal texture strip for now
        total_height = total_height.max(height);
    }

    let mut atlas = vec![0u8; total_width * total_height];

    let mut x_cursor = 0;
    for (bitmap, width, height) in bitmaps
    {
        for y in 0..height
        {
            for x in 0..width
            {
                let src = bitmap[y * width + x];
                let dst_x = x_cursor + x;
                let dst_y = y; // important, only works for horizontal texture right now
                atlas[dst_y * total_width + dst_x] = src;
            }
        }

        let uv_min = [x_cursor as f32 / total_width as f32, 0.0];
        let uv_max = [(x_cursor + width) as f32 / total_width as f32, height as f32 / total_height as f32];

        let glyph = Glyph
        {
            uv_min,
            uv_max,
            size: [width as f32, height as f32],
            offset,
            advance: 10.0
        };
        glyphs.push(glyph);
        x_cursor += width;
    }


    Ok((atlas, total_width, total_height, glyphs))
}

// Does not have individual characters, just gets drawn as one texture
// Better if there is no need for individual character-change
// Also just in one line
pub fn rasterize_static_text(font_path: &str, text: &str, text_scale: f32) -> std::result::Result<(Vec<u8>, usize, usize), anyhow::Error>
{
    let font_data = std::fs::read(font_path)?;
    let font = FontArc::try_from_vec(font_data)?;
    let scale = PxScale::from(text_scale);

    let mut bitmaps = Vec::new();

    let mut total_width = 0;
    let mut total_height = 0;

    let mut min_y = f32::MAX;
    let mut max_y = f32::MIN;

    let mut glyph_offsets: Vec<usize> = Vec::new();

    for char in text.chars()
    {
        let glyph = font.glyph_id(char).with_scale_and_position(scale, point(0.0, 0.0));

        let outline = font.outline_glyph(glyph).ok_or_else(|| anyhow!("Could not outline glyph '{}'", char))?;

        let bounds = outline.px_bounds();
        let width = bounds.width().ceil() as usize;
        let height = bounds.height().ceil() as usize;

        min_y = min_y.min(bounds.min.y);
        max_y = max_y.max(bounds.max.y);

        let mut bitmap = vec![0u8; width * height];
        outline.draw(|x, y, c| 
        { 
            let xx = x as usize;
            let yy = y as usize;

            if xx < width && yy < height
            {
                bitmap[yy * width + xx] = (c * 255.0) as u8;
            }
        });

        bitmaps.push((bitmap, width, height));
        total_width += width; // Here it is supposed to be horizontal, it is just text

        glyph_offsets.push(height as usize);

        println!("max_y: {}, min_y: {}", max_y, min_y);
        println!("Char '{}', height: {}", char, height);
    }
    // let y_offset = -min_y.ceil() as usize;
    total_height = (max_y - min_y).ceil() as usize;

    let mut atlas = vec![0u8; total_width * total_height];

    let mut x_cursor = 0;

    for ((bitmap, width, height), y_offset) in bitmaps.into_iter().zip(glyph_offsets.into_iter())
    {
        for y in 0..height
        {
            for x in 0..width
            {
                let src = bitmap[y * width + x];
                let dst_x = x_cursor + x;
                let dst_y = y + (total_height-y_offset)/2; // important, only works for horizontal texture right now
                atlas[dst_y * total_width + dst_x] = src;
            }
        }
        x_cursor += width;
    }


    Ok((atlas, total_width, total_height))
}


// For Preloading Fonts, good when often used and not changed, like in game-engines
// Not so good for graphics application like gimp, because I can not resize the font after loading it
// pub struct FontAtlas
// {
//     pub texture: wgpu::BindGroup,
//     pub glyphs: HashMap<char, Glyph>
// }

// impl FontAtlas
// {
//     pub fn load_glyphs(font_path: &str)
//     {
//         let charset: Vec<char> = (' '..='~').collect();

//         let font_data = std::fs::read(font_path)?;
//         let font = FontArc::try_from_vec(font_data)?;


//     }
// }
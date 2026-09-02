use std::collections::HashMap;

use ab_glyph::{Font, FontArc, PxScale, ScaleFont, point};
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
pub struct FontAtlas
{
    pub texture_id: usize,
    pub glyphs: HashMap<char, Glyph>,
    pub line_height: f32
}

pub fn rasterize_font_atlas(font_path: &str, charset: &str, size: f32) -> std::result::Result<(Vec<u8>, usize, usize, HashMap<char, Glyph>, f32), anyhow::Error>
{
    let font_data = std::fs::read(font_path)?;
    let font = FontArc::try_from_vec(font_data)?;
    let px_scale = PxScale::from(size);
    let scaled_font = font.as_scaled(px_scale);

    struct RasterizedGlyph
    {
        bitmap: Vec<u8>,
        width: usize,
        height: usize,
        bearing_x: f32,
        bearing_y: f32,
        advance: f32,
    }

    let mut rasterized: Vec<(char, RasterizedGlyph)> = Vec::new();
    let mut cell_width: usize = 1;
    let mut cell_height: usize = 1;

    for ch in charset.chars()
    {
        let glyph_id = scaled_font.glyph_id(ch);
        let advance = scaled_font.h_advance(glyph_id);
        let positioned = glyph_id.with_scale_and_position(px_scale, point(0.0, 0.0));

        let (bitmap, width, height, bearing_x, bearing_y) = match font.outline_glyph(positioned)
        {
            Some(outline) =>
            {
                let bounds = outline.px_bounds();
                let width = bounds.width().ceil().max(1.0) as usize;
                let height = bounds.height().ceil().max(1.0) as usize;

                let mut bitmap = vec![0u8; width * height];
                outline.draw(|x, y, coverage|
                {
                    let xx = x as usize;
                    let yy = y as usize;

                    if xx < width && yy < height
                    {
                        bitmap[yy * width + xx] = (coverage * 255.0) as u8;
                    }
                });

                (bitmap, width, height, bounds.min.x, bounds.min.y)
            }
            None => (Vec::new(), 0, 0, 0.0, 0.0),
        };

        cell_width = cell_width.max(width);
        cell_height = cell_height.max(height);

        rasterized.push((ch, RasterizedGlyph { bitmap, width, height, bearing_x, bearing_y, advance }));
    }

    // Padding, so it wont combine through linear filtering at edge with neighbouring glyph
    const PADDING: usize = 2;
    let padded_width = cell_width + PADDING;
    let padded_height = cell_height + PADDING;

    let columns = (rasterized.len() as f32).sqrt().ceil().max(1.0) as usize;
    let rows = (rasterized.len() + columns - 1) / columns;

    let atlas_width = columns * padded_width;
    let atlas_height = rows * padded_height;

    let mut atlas = vec![0u8; atlas_width * atlas_height];
    let mut glyphs = HashMap::new();

    for (index, (ch, glyph)) in rasterized.into_iter().enumerate()
    {
        let column = index % columns;
        let row = index / columns;
        let origin_x = column * padded_width;
        let origin_y = row * padded_height;

        for y in 0..glyph.height
        {
            for x in 0..glyph.width
            {
                atlas[(origin_y + y) * atlas_width + (origin_x + x)] = glyph.bitmap[y * glyph.width + x];
            }
        }

        let uv_min = [origin_x as f32 / atlas_width as f32, origin_y as f32 / atlas_height as f32];
        let uv_max = [(origin_x + glyph.width) as f32 / atlas_width as f32, (origin_y + glyph.height) as f32 / atlas_height as f32];

        glyphs.insert(ch, Glyph
        {
            uv_min,
            uv_max,
            size: [glyph.width as f32, glyph.height as f32],
            offset: [glyph.bearing_x, glyph.bearing_y],
            advance: glyph.advance,
        });
    }

    let line_height = scaled_font.height() + scaled_font.line_gap();

    Ok((atlas, atlas_width, atlas_height, glyphs, line_height))
}

// impl FontAtlas
// {
//     pub fn load_glyphs(font_path: &str)
//     {
//         let charset: Vec<char> = (' '..='~').collect();

//         let font_data = std::fs::read(font_path)?;
//         let font = FontArc::try_from_vec(font_data)?;


//     }
// }

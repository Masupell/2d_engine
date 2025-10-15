use std::sync::Arc;

use image::GenericImageView;
use anyhow::*;
use wgpu::TextureView;

pub struct Texture
{
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
    pub bind_group: Arc<wgpu::BindGroup>
}

impl Texture
{
    pub fn white(device: &wgpu::Device, queue: &wgpu::Queue, layout: &wgpu::BindGroupLayout) -> Result<Self>
    {
        let pixel: [u8; 4] = [255, 255, 255, 255];
        let img = image::DynamicImage::ImageRgba8(image::ImageBuffer::from_raw(1, 1, pixel.to_vec()).unwrap());
        Self::from_image(device, queue, &img, Some("White"), layout)
    }
    
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, path: &str, layout: &wgpu::BindGroupLayout) -> Result<Self>
    {
        let img = image::open(path)?;
        Self::from_image(device, queue, &img, None, layout)
    }
    
    pub fn from_bytes(device: &wgpu::Device, queue: &wgpu::Queue, bytes: &[u8], label: &str, layout: &wgpu::BindGroupLayout) -> Result<Self>
    {
        let img = image::load_from_memory(bytes)?;
        Self::from_image(device, queue, &img, Some(label), layout)
    }

    pub fn from_image(device: &wgpu::Device, queue: &wgpu::Queue, img: &image::DynamicImage, label: Option<&str>, layout: &wgpu::BindGroupLayout) -> Result<Self>
    {
        let rgba = img.to_rgba8();
        let dimensions = img.dimensions();

        let size = wgpu::Extent3d
        {
            width: dimensions.0,
            height: dimensions.1,
            depth_or_array_layers: 1,
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor
        {
            label,
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[]
        });

        queue.write_texture(
            wgpu::TexelCopyTextureInfo
            {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &rgba,
            wgpu::TexelCopyBufferLayout
            {
                offset: 0,
                bytes_per_row: Some(4 * dimensions.0),
                rows_per_image: Some(dimensions.1),
            },
            size
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor
        {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let bind_group = bind_group(device, layout, &view, &sampler);

        Ok(Self { texture, view, sampler, bind_group: Arc::new(bind_group) })
    }

    // Right now pretty much almost the exact same code as from_image, but to lazy to combine into one right now
    pub fn from_alpha_bitmap(device: &wgpu::Device, queue: &wgpu::Queue, bitmap: &[u8], width: usize, height: usize, label: Option<&str>, layout: &wgpu::BindGroupLayout) -> Result<Self>
    {
        let mut rgba =  Vec::with_capacity(width * height * 4);
        for &alpha in bitmap 
        {
            rgba.extend_from_slice(&[255, 255, 255, alpha]);
        }

        let size = wgpu::Extent3d
        {
            width: width as u32,
            height: height as u32,
            depth_or_array_layers: 1,
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor
        {
            label,
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[]
        });

        queue.write_texture(
            wgpu::TexelCopyTextureInfo
            {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &rgba,
            wgpu::TexelCopyBufferLayout
            {
                offset: 0,
                bytes_per_row: Some(4 * width as u32),
                rows_per_image: Some(height as u32),
            },
            size
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor
        {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let bind_group = bind_group(device, layout, &view, &sampler);

        Ok(Self { texture, view, sampler, bind_group: Arc::new(bind_group) })
    }


    pub fn screen_texture(device: &wgpu::Device, width: u32, height: u32, layout: &wgpu::BindGroupLayout) -> Self
    {
        let size = wgpu::Extent3d
        {
            width: width,
            height: height,
            depth_or_array_layers: 1,
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor
        {
            label: Some("Screen Texture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[]
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor
        {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let bind_group = bind_group(device, layout, &view, &sampler);

        Self { texture, view, sampler, bind_group: Arc::new(bind_group) }
    }


    // bindgroup_layout
    pub fn bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout
    {
        let texture_bindgroup_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor
        {
            label: Some("Texture Bind Group Layout"),
            entries: 
            &[
                wgpu::BindGroupLayoutEntry
                {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture 
                    {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry
                {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None
                }
            ]
        });
        texture_bindgroup_layout
    }
}

fn bind_group(device: &wgpu::Device, bindgroup_layout: &wgpu::BindGroupLayout, view: &TextureView, sampler: &wgpu::Sampler) -> wgpu::BindGroup
{
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor
    {
        label: Some("Diffuse Bind Group"),
        layout: bindgroup_layout,
        entries:
        &[
            wgpu::BindGroupEntry
            {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(view),
            },
            wgpu::BindGroupEntry
            {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(sampler),
            }
        ]
    });
    bind_group
}
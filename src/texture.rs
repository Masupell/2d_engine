use std::sync::Arc;
use anyhow::*;


#[derive(Copy, Clone, PartialEq, Debug)]
pub enum FilterMode
{
    Nearest,
    Linear
}

impl FilterMode
{
    pub fn into_wgpu(self) -> wgpu::FilterMode
    {
        match self
        {
            Self::Nearest => wgpu::FilterMode::Nearest,
            Self::Linear => wgpu::FilterMode::Linear
        }
    }
}

// Quick struct, to serve as step towards final texture struct
pub struct TextureEntry
{
    pub bind_group: Arc<wgpu::BindGroup>,
    pub size: (f32, f32)
}

// Right now, it is just used as a hlper, that returns things, but I need to change it, so that it does everything texture related
pub struct Texture
{
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler
}

impl Texture
{
    pub fn white(device: &wgpu::Device, queue: &wgpu::Queue, mag_filter: FilterMode, min_filter: FilterMode) -> Result<Self>
    {
        Ok(Self::from_rgba8(device, queue, &[255, 255, 255, 255], 1, 1, mag_filter, min_filter, Some("White")))
    }

    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, path: &str, mag_filter: FilterMode, min_filter: FilterMode) -> Result<Self>
    {
        let img = image::open(path)?;
        Self::from_image(device, queue, &img, mag_filter, min_filter, None)
    }

    pub fn from_bytes(device: &wgpu::Device, queue: &wgpu::Queue, bytes: &[u8], mag_filter: FilterMode, min_filter: FilterMode) -> Result<Self>
    {
        let img = image::load_from_memory(bytes)?;
        Self::from_image(device, queue, &img, mag_filter, min_filter, None)
    }

    pub fn from_image(device: &wgpu::Device, queue: &wgpu::Queue, img: &image::DynamicImage, mag_filter: FilterMode, min_filter: FilterMode, label: Option<&str>) -> Result<Self>
    {
        let rgba = img.to_rgba8();
        let (width, height) = rgba.dimensions();

        Ok(Self::from_rgba8(device, queue, &rgba, width, height, mag_filter, min_filter, label))
    }

    // Right now pretty much almost the exact same code as from_image, but to lazy to combine into one right now
    pub fn from_alpha_bitmap(device: &wgpu::Device, queue: &wgpu::Queue, bitmap: &[u8], width: usize, height: usize, mag_filter: FilterMode, min_filter: FilterMode, label: Option<&str>) -> Result<Self>
    {
        let mut rgba =  Vec::with_capacity(width * height * 4);
        for &alpha in bitmap
        {
            rgba.extend_from_slice(&[255, 255, 255, alpha]);
        }

        Ok(Self::from_rgba8(device, queue, &rgba, width as u32, height as u32, mag_filter, min_filter, label))
    }


    pub fn from_rgba8(device: &wgpu::Device, queue: &wgpu::Queue, rgba: &[u8], width: u32, height: u32, mag_filter: FilterMode, min_filter: FilterMode, label: Option<&str>) -> Self
    {
        debug_assert_eq!(rgba.len(), (width * height * 4) as usize, "rgba data doesn't match {width}x{height}");

        let size = wgpu::Extent3d
        {
            width,
            height,
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
            rgba,
            wgpu::TexelCopyBufferLayout
            {
                offset: 0,
                bytes_per_row: Some(4 * width),
                rows_per_image: Some(height),
            },
            size
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor
        {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: mag_filter.into_wgpu(),
            min_filter: min_filter.into_wgpu(),
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        Self { texture, view, sampler }
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

    // bind_group
    pub fn bind_group(&self, device: &wgpu::Device, bindgroup_layout: &wgpu::BindGroupLayout) -> wgpu::BindGroup
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
                    resource: wgpu::BindingResource::TextureView(&self.view),
                },
                wgpu::BindGroupEntry
                {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                }
            ]
        });
        // self.bind_group = Some(bind_group);
        bind_group
    }
}

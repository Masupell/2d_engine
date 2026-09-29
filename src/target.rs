use std::sync::Arc;

use crate::texture::TextureEntry;

// id for set_target, texture_id the normal texture_id
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct TargetHandle
{
    pub id: usize,
    pub texture_id: usize,
}

pub(crate) struct RenderTarget
{
    pub(crate) scale: f32,
    pub(crate) format: wgpu::TextureFormat,
    pub(crate) format_slot: usize,
    pub(crate) texture: wgpu::Texture,
    pub(crate) view: wgpu::TextureView,
    pub(crate) size: (u32, u32),
    pub(crate) texture_id: usize,
    pub(crate) clear: Option<wgpu::Color>,
    pub(crate) snapshot: Option<Snapshot>
}

// copy of a target (or screen)
pub(crate) struct Snapshot
{
    pub(crate) texture: wgpu::Texture,
    pub(crate) view: wgpu::TextureView,
    pub(crate) entry: TextureEntry
}

pub(crate) fn create_snapshot(device: &wgpu::Device, texture_layout: &wgpu::BindGroupLayout, sampler: &wgpu::Sampler, format: wgpu::TextureFormat, size: (u32, u32)) -> Snapshot
{
    let (texture, view, entry) = create_target_texture(device, texture_layout, sampler, format, size);
    Snapshot { texture, view, entry }
}

// true if format can be a render target here: renderable, sampleable, filterable (linear sampler), blendable
pub fn format_is_usable(format: wgpu::TextureFormat, device_features: wgpu::Features) -> bool
{
    let features = format.guaranteed_format_features(device_features);
    let usages = wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::COPY_DST;
    let flags = wgpu::TextureFormatFeatureFlags::FILTERABLE | wgpu::TextureFormatFeatureFlags::BLENDABLE;

    features.allowed_usages.contains(usages) & features.flags.contains(flags)
}

pub(crate) fn scaled_size(screen_size: (u32, u32), scale: f32) -> (u32, u32)
{
    (((screen_size.0 as f32 * scale).round() as u32).max(1), ((screen_size.1 as f32 * scale).round() as u32).max(1))
}

pub(crate) fn create_target_texture(device: &wgpu::Device, texture_layout: &wgpu::BindGroupLayout, sampler: &wgpu::Sampler, format: wgpu::TextureFormat, size: (u32, u32)) -> (wgpu::Texture, wgpu::TextureView, TextureEntry)
{
    let texture = device.create_texture(&wgpu::TextureDescriptor
    {
        label: Some("Render Target"),
        size: wgpu::Extent3d { width: size.0, height: size.1, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });

    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor
    {
        label: Some("Render Target Bind Group"),
        layout: texture_layout,
        entries:
        &[
            wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&view) },
            wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(sampler) },
        ],
    });

    let entry = TextureEntry { bind_group: Arc::new(bind_group), size: (size.0 as f32, size.1 as f32) };

    (texture, view, entry)
}

// Currently the same for all targets
pub(crate) fn create_target_sampler(device: &wgpu::Device) -> wgpu::Sampler
{
    device.create_sampler(&wgpu::SamplerDescriptor
    {
        label: Some("Render Target Sampler"),
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::FilterMode::Nearest,
        ..Default::default()
    })
}

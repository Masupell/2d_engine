use std::{fs, sync::Arc};

#[derive(Clone)]
pub struct ShaderModuleHandle
{
    pub module: Arc<wgpu::ShaderModule>,
    pub entry: String
}

impl ShaderModuleHandle
{
    fn from_source(device: &wgpu::Device, source: &str, entry: &str) -> Self
    {
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor
        {
           label: None,
           source: wgpu::ShaderSource::Wgsl(source.into())
        });
        Self { module: Arc::new(module), entry: entry.to_string() }
    }

    pub fn from_path(device: &wgpu::Device, path: &str, entry: &str) -> Self
    {
        let source = fs::read_to_string(path).unwrap_or_else(|e| panic!("Failed to read shader '{}': {}", path, e));
        Self::from_source(device, &source, entry)
    }

    pub fn default_vertex(device: &wgpu::Device) -> Self
    {
        Self::from_source(device, include_str!("shaders/shader.wgsl"), "vs_main")
    }

    pub fn default_fragment(device: &wgpu::Device) -> Self
    {
        Self::from_source(device, include_str!("shaders/shader.wgsl"), "fs_main")
    }
}

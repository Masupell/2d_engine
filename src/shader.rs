use std::{fs, sync::Arc};

#[derive(Clone)]
pub struct ShaderModuleHandle
{
    pub module: Arc<wgpu::ShaderModule>,
    pub entry: String
}

impl ShaderModuleHandle
{
    pub(crate) fn from_input(device: &wgpu::Device, input: ShaderInput, entry: &str) -> Self
    {
        match input
        {
            ShaderInput::File(path) => Self::from_path(device, path, entry),
            ShaderInput::Code(code) => Self::from_source(device, code, entry),
        }
    }

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

#[derive(Copy, Clone)]
pub enum ShaderInput<'a>
{
    File(&'a str), // path
    Code(&'a str), // wgsl source
}

// basically 'include_wgsl!', but my shader needs just the string, can change that eventually
#[macro_export]
macro_rules! shader
{
    ($path:literal) => { $crate::shader::ShaderInput::Code(include_str!($path)) };
}

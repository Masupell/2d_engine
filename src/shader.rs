use std::fs;

pub struct Shader
{
    pub module: wgpu::ShaderModule,
    pub vs_entry: String,
    pub fs_entry: String
}

impl Shader
{
    pub fn new(device: &wgpu::Device, path: &str, vs_entry: &str, fs_entry: &str) -> Self
    {
        let source = fs::read_to_string(path).unwrap();
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor
        {
            label: Some(path),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });

        Self
        {
            module,
            vs_entry: vs_entry.to_string(),
            fs_entry: fs_entry.to_string()
        }
    }

    pub fn default(device: &wgpu::Device) -> Self 
    {
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor
        {
            label: Some("Default Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/shader.wgsl").into()),
        });

        Self 
        { 
            module, 
            vs_entry: "vs_main".into(), 
            fs_entry: "fs_main".into()
        }
    }
}
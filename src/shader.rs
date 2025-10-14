use std::fs;

pub struct Shader
{
    pub vertex_module: wgpu::ShaderModule,
    pub fragment_module: wgpu::ShaderModule,
    pub vs_entry: String,
    pub fs_entry: String
}

impl Shader
{
    // pub fn new(device: &wgpu::Device, path: &str, vs_entry: &str, fs_entry: &str) -> Self
    // {
    //     let source = fs::read_to_string(path).unwrap();
    //     let module = device.create_shader_module(wgpu::ShaderModuleDescriptor
    //     {
    //         label: Some(path),
    //         source: wgpu::ShaderSource::Wgsl(source.into()),
    //     });

    //     Self
    //     {
    //         module,
    //         vs_entry: vs_entry.to_string(),
    //         fs_entry: fs_entry.to_string()
    //     }
    // }

    pub fn default(device: &wgpu::Device) -> Self 
    {
        let vertex_module = device.create_shader_module(wgpu::ShaderModuleDescriptor
        {
            label: Some("Default Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/shader.wgsl").into()),
        });

        let fragment_module = device.create_shader_module(wgpu::ShaderModuleDescriptor
        {
            label: Some("Default Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/shader.wgsl").into()),
        });

        Self 
        { 
            vertex_module,
            fragment_module, 
            vs_entry: "vs_main".into(), 
            fs_entry: "fs_main".into()
        }
    }

    pub fn new_fragment(&mut self, device: &wgpu::Device, path: &str, entry: &str)
    {
        let source = fs::read_to_string(path).unwrap();
        let fragment = device.create_shader_module(wgpu::ShaderModuleDescriptor
        {
            label: Some("Fragment"),
            source: wgpu::ShaderSource::Wgsl(source.into())
        });
        self.fragment_module = fragment;
        self.fs_entry = entry.to_string();
    }

    pub fn new_vertex(&mut self, device: &wgpu::Device, path: &str, entry: &str)
    {
        let source = fs::read_to_string(path).unwrap();
        let vertex = device.create_shader_module(wgpu::ShaderModuleDescriptor
        {
            label: Some("Vertex"),
            source: wgpu::ShaderSource::Wgsl(source.into())
        });
        self.vertex_module = vertex;
        self.vs_entry = entry.to_string();
    }
}
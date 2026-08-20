use crate::{Renderer, utility::{MeshData, Vertex}};

pub struct MeshBuilder
{
    data: MeshData,
    mesh_id: Option<usize>,
    topology: MeshTopology
}

impl MeshBuilder
{
    pub fn new(topology: MeshTopology) -> Self
    {
        Self
        {
            data: MeshData::new(),
            mesh_id: None,
            topology
        }
    }

    pub fn clear(&mut self)
    {
        self.data.vertices.clear();
        self.data.indices.clear();
    }

    pub fn add_vertex(&mut self, vertex: Vertex) -> u16
    {
        self.data.add_vertex(vertex)
    }

    pub fn add_triangle(&mut self, a: u16, b: u16, c: u16)
    {
        self.data.add_triangle(a, b, c);
    }

    pub fn build(&mut self, renderer: &mut Renderer, device: &wgpu::Device, queue: &wgpu::Queue) -> usize
    {
        match self.topology
        {
            MeshTopology::Triangles =>
            {
                self.build_triangle_indices();
            }
            MeshTopology::TriangleStrip =>
            {
                self.build_triangle_strip_indices();
            }
        }
        let id = match self.mesh_id
        {
            Some(mesh_id) =>
            {
                renderer.update_mesh(device, queue, mesh_id, &self.data);
                mesh_id
            }
            None =>
            {
                let mesh_id = renderer.create_mesh(device, &self.data);
                self.mesh_id = Some(mesh_id);
                mesh_id
            }
        };
        id
    }

    fn build_triangle_indices(&mut self)
    {
        self.data.indices.clear();

        for i in (0..self.data.vertices.len()).step_by(3)
        {
            if i + 2 >= self.data.vertices.len()
            {
                break;
            }

            self.data.indices.extend_from_slice(&[i as u16, (i+1) as u16, (i+2) as u16]);
        }
    }

    // Doesnt actually do a triangle strip at the moment
    fn build_triangle_strip_indices(&mut self)
    {
        self.data.indices.clear();
        if self.data.vertices.len() < 3 { return; }

        for i in 0..self.data.vertices.len() - 2
        {
            if i % 2 == 0
            {
                self.data.indices.extend_from_slice(&[i as u16, (i+1) as u16, (i+2) as u16]);
            }
            else
            {
                self.data.indices.extend_from_slice(&[(i + 1) as u16, i as u16, (i + 2) as u16]);
            }
        }
    }
}


pub enum MeshTopology
{
    Triangles,
    TriangleStrip
}

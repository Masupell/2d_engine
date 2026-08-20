use crate::{Renderer, utility::{MeshData, Vertex}};

pub struct MeshBuilder
{
    vertices: Vec<BuilderVertex>,
    mesh_id: Option<usize>,
    topology: MeshTopology
}

impl MeshBuilder
{
    pub fn new(topology: MeshTopology) -> Self
    {
        Self
        {
            vertices: Vec::new(),
            mesh_id: None,
            topology
        }
    }

    pub fn clear(&mut self)
    {
        self.vertices.clear();
    }

    pub fn add_vertex(&mut self, position: VertexPosition, uv: (f32, f32))
    {
        self.vertices.push(BuilderVertex { position, uv });
    }

    pub fn build(&mut self, renderer: &mut Renderer, device: &wgpu::Device, queue: &wgpu::Queue) -> usize
    {
        let mut data = MeshData::new();

        for vertex in &self.vertices
        {
            let position = self.convert_position(renderer, vertex.position);
            data.add_vertex(Vertex::new([position.0, position.1, 0.0], [vertex.uv.0, vertex.uv.1]));
        }

        match self.topology
        {
            MeshTopology::Triangles =>
            {
                self.build_triangle_indices(&mut data);
            }
            MeshTopology::TriangleStrip =>
            {
                self.build_triangle_strip_indices(&mut data);
            }
        }
        let id = match self.mesh_id
        {
            Some(mesh_id) =>
            {
                renderer.update_mesh(device, queue, mesh_id, &data);
                mesh_id
            }
            None =>
            {
                let mesh_id = renderer.create_mesh(device, &data);
                self.mesh_id = Some(mesh_id);
                mesh_id
            }
        };
        id
    }

    fn build_triangle_indices(&mut self, data: &mut MeshData)
    {
        for i in (0..self.vertices.len()).step_by(3)
        {
            if i + 2 >= self.vertices.len()
            {
                break;
            }

            data.add_triangle(i as u16, (i+1) as u16, (i+2) as u16);
        }
    }

    // Doesnt actually do a triangle strip at the moment
    fn build_triangle_strip_indices(&mut self, data: &mut MeshData)
    {
        if self.vertices.len() < 3 { return; }

        for i in 0..self.vertices.len() - 2
        {
            if i % 2 == 0
            {
                data.add_triangle(i as u16, (i+1) as u16, (i+3) as u16);
            }
            else
            {
                data.add_triangle((i+1) as u16, i as u16, (i+2) as u16);
            }
        }
    }

    fn convert_position(&self, renderer: &Renderer, position: VertexPosition) -> (f32, f32)
    {
        let (x, y, width, height) = match position
        {
            VertexPosition::Screen((x, y)) =>
            {
                (x, y, renderer.window_size.0, renderer.window_size.1)
            }
            VertexPosition::Virtual((x, y)) =>
            {
                (x, y, renderer.virtual_size.0, renderer.virtual_size.1)
            }
        };

        let ndc_x = (x/width) * 2.0 - 1.0;
        let ndc_y = (y/height) * 2.0;

        (ndc_x, ndc_y)
    }
}

#[derive(Copy, Clone)]
struct BuilderVertex
{
    position: VertexPosition,
    uv: (f32, f32)
}

pub enum MeshTopology
{
    Triangles,
    TriangleStrip
}

#[derive(Copy, Clone)]
pub enum VertexPosition
{
    Screen((f32, f32)),
    Virtual((f32, f32))
}

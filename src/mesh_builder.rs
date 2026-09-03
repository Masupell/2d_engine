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

    pub fn with_mesh_id(topology: MeshTopology, mesh_id: usize) -> Self
    {
        Self
        {
            vertices: Vec::new(),
            mesh_id: Some(mesh_id),
            topology
        }
    }


    pub fn clear(&mut self)
    {
        self.vertices.clear();
    }

    pub fn add_vertex(&mut self, position: (f32, f32), uv: (f32, f32))
    {
        self.vertices.push(BuilderVertex { position, uv });
    }

    pub fn set_vertex_position(&mut self, index: usize, position: (f32, f32))
    {
        self.vertices[index].position = position;
    }

    pub fn vertex_count(&self) -> usize
    {
        self.vertices.len()
    }

    pub fn build(&mut self, renderer: &mut Renderer, device: &wgpu::Device, queue: &wgpu::Queue) -> usize
    {
        let data = self.build_data();
        match self.mesh_id
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
        }
    }

    pub fn update_vertices(&mut self, renderer: &mut Renderer, device: &wgpu::Device, queue: &wgpu::Queue)
    {
        let mesh_id = self.mesh_id;
        mesh_id.into_iter().for_each(|mesh_id|
        {
            let vertices = self.build_vertices();
            renderer.update_mesh_vertices(device, queue, mesh_id, &vertices);
        });
    }

    fn build_data(&mut self) -> MeshData
    {
        let mut data = MeshData::new();

        let vertices = self.build_vertices();

        for vertex in vertices
        {
            data.add_vertex(vertex);
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

        data
    }

    fn build_vertices(&self) -> Vec<Vertex>
    {
        self.vertices.iter().map(|vertex|
        {
            Vertex::new([vertex.position.0, vertex.position.1, 0.0], [vertex.uv.0, vertex.uv.1])
        }).collect()
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
                data.add_triangle(i as u16, (i+1) as u16, (i+2) as u16);
            }
            else
            {
                data.add_triangle((i+1) as u16, i as u16, (i+2) as u16);
            }
        }
    }

    // fn convert_position(&self, renderer: &Renderer, position: VertexPosition) -> (f32, f32)
    // {
    //     match position
    //     {
    //         VertexPosition::Screen((x, y)) =>
    //         {
    //             let to_virtual = (renderer.virtual_size.0 / renderer.window_size.0, renderer.virtual_size.1 / renderer.window_size.1);
    //             self.anchor_to_camera(renderer, (x * to_virtual.0, y * to_virtual.1))
    //         }
    //         VertexPosition::Virtual((x, y)) =>
    //         {
    //             self.anchor_to_camera(renderer, (x, y))
    //         }
    //         VertexPosition::World((x, y)) =>
    //         {
    //             (x, y)
    //         }
    //     }
    // }

    // fn anchor_to_camera(&self, renderer: &Renderer, (x, y): (f32, f32)) -> (f32, f32)
    // {
    //     (
    //         renderer.camera_pos.0 + x - renderer.virtual_size.0 * 0.5,
    //         renderer.camera_pos.1 + y - renderer.virtual_size.1 * 0.5
    //     )
    // }
}

#[derive(Copy, Clone)]
struct BuilderVertex
{
    position: (f32, f32),
    uv: (f32, f32)
}

pub enum MeshTopology
{
    Triangles,
    TriangleStrip
}

// #[derive(Copy, Clone)]
// pub enum VertexPosition
// {
//     Screen((f32, f32)), // in actual window pixels
//     Virtual((f32, f32)), // the virtual resolution, basically the resolutin I gave in the beginning, screen_virtual, need to be rebuild every frame currently to stay there
//     World((f32, f32)) // in world coordinates (like if the player is at -500), uses virtual size
// }


#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Sphere {
    pub c: [f32; 3],
    pub r: f32
}

#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: Mat4<f32> = cgmath::Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.0,
    0.0, 0.0, 0.5, 1.0,
);

// Isocahedron
const X: f32 = 0.525731112119133606;
const Z: f32 = 0.850650808352039932;

use crate::{math::{Vec3, Mat4}, vertex::WorldSpaceVertex};
pub const isocahedron_vert: &'static [Vec3<f32>] = &[
    Vec3::new(-X, 0.0, Z), Vec3::new(-X, 0.0, Z),
    Vec3::new(X, 0.0, Z), Vec3::new(X, 0.0, Z),
    Vec3::new(-X, 0.0, -Z), Vec3::new(-X, 0.0, -Z),
    Vec3::new(X, 0.0, -Z), Vec3::new(X, 0.0, -Z),
    
    Vec3::new(0.0, Z, X), Vec3::new(0.0, Z, X),
    Vec3::new(0.0, Z, -X), Vec3::new(0.0, Z, -X),
    Vec3::new(0.0, -Z, X), Vec3::new(0.0, -Z, X),
    Vec3::new(0.0, -Z, -X), Vec3::new(0.0, -Z, -X),

    Vec3::new(Z, X, 0.0), Vec3::new(Z, X, 0.0),
    Vec3::new(-Z, X, 0.0), Vec3::new(-Z, X, 0.0),
    Vec3::new(Z, -X, 0.0), Vec3::new(Z, -X, 0.0),
    Vec3::new(-Z, -X, 0.0), Vec3::new(-Z, -X, 0.0)
];

pub const isocahedron_indices: &'static [u16] = &[
   0,4,1, 0,9,4, 9,5,4, 4,5,8, 4,8,1,
   8,10,1, 8,3,10, 5,3,8, 5,2,3, 2,7,3,
   7,10,3, 7,6,10, 7,11,6, 11,0,6, 0,1,6,
   6,1,10, 9,0,11, 9,11,2, 9,2,5, 7,2,11
];

use crate::{
    ecs,
    physics::Physics,
    world::Game
};

use std::sync::atomic::AtomicUsize;
struct AtomicId<T>(AtomicUsize, std::marker::PhantomData<T>);

impl<T> AtomicId<T> {
    const fn new() -> Self {
        Self(AtomicUsize::new(0), std::marker::PhantomData)
    }

    fn incr(&mut self) -> usize {
        self.0.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    }
}

#[derive(Debug)]
#[derive(PartialEq, Eq, Clone)]
pub enum MeshLabel {
    Cube,
    Isocahedron,
    Object {
        name: &'static str,
        path: String,
    }
}
#[derive(Debug)]
#[derive(PartialEq, Eq, Clone)]
pub struct MeshDesc {
    pub num_indices: usize,
    pub start_idx: u32,

    pub base_vertex_idx: i32,

    pub ty: MeshLabel
}

use autodiff::Zero;
use core_engine::Component;
#[derive(Component)]
pub enum Render {
    // A basic 3d mesh
    Mesh(Mesh),
    Orbit {
        // The color of the orbit
        color: [f32; 4],
    }
}

static mut mesh_idx: AtomicId<Mesh> = AtomicId::new();
use std::rc::Rc;
pub struct Mesh {
    // Id of the mesh
    id: usize,

    desc: Rc<MeshDesc>,
    model: Mat4<f32>,
    transform: Transform,
}

pub struct Transform {
    pub scale: f32,
    pub translation: Vec3<f32>,
    pub rotation: Quaternion<f32>,
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            scale: 1.0,
            translation: Vec3::zero(),
            rotation: Quaternion::zero()
        }
    }
}

impl From<&Transform> for Mat4<f32> {
    fn from(tr: &Transform) -> Mat4<f32> {
        let s = Mat4::from_scale(tr.scale);
        let t = Mat4::from_translation(tr.translation);
        let r: Mat4<f32> = tr.rotation.into();

        t * r * s
    }
}

use cgmath::{Matrix2, Quaternion, SquareMatrix};
impl Mesh {
    pub fn new(desc: Rc<MeshDesc>, tr: Transform) -> Self {
        let id = unsafe { mesh_idx.incr() };
        Self {
            id,
            transform: tr,
            model: Mat4::identity(),
            desc,
        }
    }
}

use ecs::System;
pub struct RenderingSystem;
impl System for RenderingSystem {
    fn run(&self, game: &mut Game, _: &std::time::Instant) {
        let world = &mut game.world;
        // Looping over the renderable objects
        let mut spheres = vec![];
        for (physic, render) in world.query_mut::<(Physics, Render)>() {
            let p = &physic.p;

            // If the object has moved
            //if physic.has_moved {
                match render {
                    Render::Mesh(mesh) => {
                        println!("mesh {:?} has moved", mesh.id);
                        // 1. Recompute its model matrix
                        mesh.transform.translation = *p;
                        mesh.model = (&mesh.transform).into();
                        // 2. Send it to the GPU queue
                        game.model_mat_uniform.write(&game.queue, (mesh.id * wgpu::BIND_BUFFER_ALIGNMENT as usize) as wgpu::BufferAddress, &mesh.model);
                    },
                    _ => (),
                }
            //}        
        }
        game.spheres_uniform.write(&game.queue, 0, &spheres);

        let frame = game.swap_chain
            .get_current_frame()
            .unwrap()
            .output;

        let mut encoder = game
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[wgpu::RenderPassColorAttachment {
                    // Draw to the current view frame
                    view: &frame.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.01,
                            g: 0.01,
                            b: 0.01,
                            a: 1.0,
                        }),
                        store: true,
                    },
                }],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &game.depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: true,
                    }),
                    stencil_ops: None,
                }),
            });

            // 1. Select the shader pipeline
            render_pass.set_pipeline(&game.galaxy_render_pipeline);
            // 2. Bind the group of uniforms (textures, data, etc...)
            render_pass.set_bind_group(0, &game.galaxy_bind_group, &[]);
            // 3. Set the vertex and index buffers
            render_pass.set_vertex_buffer(0, game.vertex_galaxy_buffer.slice(..));
            render_pass.set_index_buffer(game.index_galaxy_buffer.slice(..), wgpu::IndexFormat::Uint16);
            // 4. Draw on the render pass
            render_pass.draw_indexed(0..game.num_galaxy_indices, 0, 0..1);

            // 1. Select the shader pipeline
            render_pass.set_pipeline(&game.planet_render_pipeline);
            for render in world.query::<Render>() {
                match render {
                    Render::Mesh(mesh) => {
                        dbg!(mesh.id);

                        // 2. Bind the group of uniforms (textures, data, etc...)
                        render_pass.set_bind_group(0, &game.planet_bind_group, &[(mesh.id as u32) * wgpu::BIND_BUFFER_ALIGNMENT as u32]);
                        // 3. Set the vertex and index buffers
                        render_pass.set_vertex_buffer(0, game.vertex_mesh_buffer.slice(..));
                        render_pass.set_index_buffer(game.index_mesh_buffer.slice(..), wgpu::IndexFormat::Uint32);
                        // 4. Draw on the render pass
                        let start_idx = mesh.desc.start_idx;
                        let end_idx = mesh.desc.start_idx + (mesh.desc.num_indices as u32);
                        //dbg!(&mesh.desc);
                        render_pass.draw_indexed(start_idx..end_idx, mesh.desc.base_vertex_idx, 0..1);
                    },
                    Render::Orbit { .. } => {
                        todo!()
                    }
                }
            }
        }

        game.queue.submit(std::iter::once(encoder.finish()));
    }
}
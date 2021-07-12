extern crate byte_slice_cast;

use std::iter;
use std::time::Instant;
use wgpu::util::DeviceExt;

use winit::{
    event_loop::ControlFlow,
    event::WindowEvent
};

use crate::render::Mesh;
use crate::resources::{Loader, MeshLoader};
use crate::shared::Shared;
use crate::pipelines::{GalaxyPipeline, PlanetPipeline, RenderPipeline};
use crate::camera::Camera;
use crate::input::InputGameState;
use crate::uniform::UniformBuffer;
use crate::camera::CameraData;
use crate::vertex::ClipSpaceVertex;
pub struct Game {
    surface: wgpu::Surface,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    sc_desc: wgpu::SwapChainDescriptor,
    pub swap_chain: wgpu::SwapChain,
    pub size: winit::dpi::PhysicalSize<u32>,
    // Ray tracing rendering pipeline
    pub galaxy_render_pipeline: wgpu::RenderPipeline,
    pub planet_render_pipeline: wgpu::RenderPipeline,

    pub vertex_galaxy_buffer: wgpu::Buffer,
    pub index_galaxy_buffer: wgpu::Buffer,
    pub num_galaxy_indices: u32,

    pub vertex_mesh_buffer: wgpu::Buffer,
    pub index_mesh_buffer: wgpu::Buffer,
    pub num_mesh_indices: u32,

    map_tex: Texture,
    gnomonic_projection_tex: Texture,
    pub depth_texture: Texture,

    galaxy_bind_group_layout: wgpu::BindGroupLayout,
    pub galaxy_bind_group: wgpu::BindGroup,
    // Standard rasterizer rendering pipeline
    pub planet_bind_group_layout: wgpu::BindGroupLayout,
    pub planet_bind_group: wgpu::BindGroup,
    
    // Uniforms (can be send to multiple rendering pipelines)
    // Camera uniform for rotating the sky in the skybox shader
    pub skybox_camera_uniform: UniformBuffer<CameraData>,
    // Viewport camera matrix
    pub camera_uniform: UniformBuffer<Mat4<f32>>,
    // Perspective projection matrix
    pub proj_mat_uniform: UniformBuffer<Mat4<f32>>,
    window_size_uniform: UniformBuffer<Vec2<f32>>,
    time_uniform: UniformBuffer<f32>,
    pub model_mat_uniform: UniformBuffer<Mat4<f32>>,
    pub spheres_uniform: UniformBuffer<Vec<Sphere>>,

    pub clock: std::time::Instant,
    pub world: Shared<World>,
    pub spacecraft: Entity,

    pub input: InputGameState,

    mesh_loader: MeshLoader,
}

use cgmath::{Deg, Quaternion, Zero};
use crate::ecs::Entity;
use crate::ecs::{World, SystemManager};
use crate::physics::{
    Physics,
};
use crate::projection::*;
use crate::texture::Texture;
use crate::triangulation::Triangulation;
use cgmath::InnerSpace;
use crate::math::Vec2;
use crate::orbit::{Orbit, OrbitData};

fn generate_position<P: Projection<f32>>(size: usize) -> Vec<f32> {
    let (w, h) = (size as f32, size as f32);
    let mut data = vec![];
    for y in 0..(h as u32) {
        for x in 0..(w as u32) {
            let xy = Vec2::new(x, y);
            let clip_xy = Vec2::new(
                2.0 * ((xy.x as f32) / (w as f32)) - 1.0,
                2.0 * ((xy.y as f32) / (h as f32)) - 1.0,
            );
            if let Some(pos) = P::clip_to_world_space(&clip_xy) {
                let pos = pos.truncate().normalize();
                data.extend(&[pos.x as f32, pos.y as f32, pos.z as f32, 1.0]);
            } else {
                data.extend(&[1.0, 1.0, 1.0, 1.0]);
            }
        }
    }

    data
}

pub fn create_position_texture<P: Projection<f32>>(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    size: usize,
) -> Texture {
    let texels: Vec<f32> = generate_position::<P>(size);

    let dimensions = (size, size);
    Texture::from_raw_bytes::<f32>(&device, &queue, &texels, dimensions, "position")
}
use crate::{
    math::{Mat4, Vec3},
    vertex::Vertex,
    render::{Sphere, Render, Transform},
};
use cgmath::SquareMatrix;
use winit::window::Window;
use crate::input::KeyId;
impl Game {
    pub async fn new(window: &Window) -> Self {
        let size = window.inner_size();

        // The instance is a handle to our GPU
        // BackendBit::PRIMARY => Vulkan + Metal + DX12 + Browser WebGPU
        let instance = wgpu::Instance::new(wgpu::BackendBit::PRIMARY);
        let surface = unsafe { instance.create_surface(window) };
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
            })
            .await
            .unwrap();
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    features: wgpu::Features::empty(),
                    limits: wgpu::Limits::default(),
                },
                None, // Trace path
            )
            .await
            .unwrap();

        let sc_desc = wgpu::SwapChainDescriptor {
            usage: wgpu::TextureUsage::RENDER_ATTACHMENT,
            format: adapter.get_swap_chain_preferred_format(&surface).unwrap(),
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
        };
        let swap_chain = device.create_swap_chain(&surface, &sc_desc);

        // Texture loading
        let gnomonic_projection_tex = create_position_texture::<Gnomonic>(&device, &queue, 512);

        let bytes = include_bytes!("../img/map.png");
        let img = image::load_from_memory(bytes).unwrap();
        let map_tex = Texture::from_image(&device, &queue, &img, "map.png");

        let depth_texture = Texture::create_depth_texture(&device, &sc_desc, "depth texture");

        // Uniform buffer
        let skybox_camera_uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: wgpu::BIND_BUFFER_ALIGNMENT,
            usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
            mapped_at_creation: false,
        });

        let time_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: wgpu::BIND_BUFFER_ALIGNMENT,
            usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
            mapped_at_creation: false,
        });

        let window_size_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: wgpu::BIND_BUFFER_ALIGNMENT,
            usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
            mapped_at_creation: false,
        });

        let spheres = [
            Sphere {
                c: [5.0, 5.0, 5.0],
                r: 1.0
            },
            Sphere {
                c: [5.0, -5.0, 5.0],
                r: 2.0
            },
            Sphere {
                c: [5.0, -5.0, -5.0],
                r: 0.5
            }
        ].into();
        let spheres_uniform = UniformBuffer::new(&device, wgpu::BIND_BUFFER_ALIGNMENT);
        spheres_uniform.write(&queue, 0, &spheres);

        let model_mat_uniform = UniformBuffer::new(&device, wgpu::BIND_BUFFER_ALIGNMENT * 10);

        let window_size_uniform = UniformBuffer::new(&device, wgpu::BIND_BUFFER_ALIGNMENT);
        let time_uniform = UniformBuffer::new(&device, wgpu::BIND_BUFFER_ALIGNMENT);

        let camera_uniform = UniformBuffer::new(&device, wgpu::BIND_BUFFER_ALIGNMENT);
        let skybox_camera_uniform = UniformBuffer::new(&device, wgpu::BIND_BUFFER_ALIGNMENT);

        let proj_mat_uniform = UniformBuffer::new(&device, wgpu::BIND_BUFFER_ALIGNMENT);

        let galaxy_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStage::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStage::FRAGMENT,
                        ty: wgpu::BindingType::Sampler {
                            comparison: false,
                            filtering: true,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStage::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStage::FRAGMENT,
                        ty: wgpu::BindingType::Sampler {
                            comparison: false,
                            filtering: true,
                        },
                        count: None,
                    },
                    // rot matrix uniform
                    skybox_camera_uniform.desc_layout(4, wgpu::ShaderStage::FRAGMENT, false),
                    // window size uniform
                    window_size_uniform.desc_layout(5, wgpu::ShaderStage::VERTEX, false),
                    // time uniform
                    time_uniform.desc_layout(6, wgpu::ShaderStage::VERTEX, false),

                    // spherical objects uniform
                    spheres_uniform.desc_layout(7, wgpu::ShaderStage::FRAGMENT, false),
                ],
                label: Some("galaxy_bind_group_layout"),
            });

        let galaxy_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &galaxy_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&gnomonic_projection_tex.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&gnomonic_projection_tex.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&map_tex.view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&map_tex.sampler),
                },
                skybox_camera_uniform.desc(4),
                window_size_uniform.desc(5),
                time_uniform.desc(6),
                spheres_uniform.desc(7),
            ],
            label: Some("galaxy_bind_group"),
        });

        let planet_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                // model matrix uniform
                model_mat_uniform.desc_layout(0, wgpu::ShaderStage::VERTEX, true),
                // view matrix uniform
                camera_uniform.desc_layout(1, wgpu::ShaderStage::VERTEX, false),
                // proj matrix uniform
                proj_mat_uniform.desc_layout(2, wgpu::ShaderStage::VERTEX, false),
                // window size uniform
                window_size_uniform.desc_layout(3, wgpu::ShaderStage::VERTEX, false),
                // time uniform
                time_uniform.desc_layout(4, wgpu::ShaderStage::VERTEX, false),
            ],
            label: Some("planet_bind_group_layout"),
        });

        let planet_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &planet_bind_group_layout,
            entries: &[
                model_mat_uniform.desc(0),
                camera_uniform.desc(1),
                proj_mat_uniform.desc(2),
                window_size_uniform.desc(3),
                time_uniform.desc(4),
            ],
            label: Some("planet_bind_group"),
        });


        // uniform buffer
        let galaxy_render_pipeline = GalaxyPipeline::new(
            &device, 
            &sc_desc,
            wgpu::include_spirv!("shaders/allsky.vert.spv"),
            wgpu::include_spirv!("shaders/allsky.frag.spv"),
            &[&galaxy_bind_group_layout],
        );
        let planet_render_pipeline = PlanetPipeline::new(
            &device, 
            &sc_desc,
            wgpu::include_spirv!("shaders/planet.vert.spv"),
            wgpu::include_spirv!("shaders/planet.frag.spv"),
            &[&planet_bind_group_layout],
        );

        let (vertices_galaxy, indices_galaxy) = Triangulation::create::<Gnomonic>();

        let vertex_galaxy_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices_galaxy),
            usage: wgpu::BufferUsage::VERTEX,
        });
        let index_galaxy_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(&indices_galaxy),
            usage: wgpu::BufferUsage::INDEX,
        });
        let num_galaxy_indices = indices_galaxy.len() as u32;

        let clock = Instant::now();

        // Mesh descriptions
        let mut mesh_loader = MeshLoader::new();
        let planet_desc = std::rc::Rc::new(
            mesh_loader.load("./assets/isocahedron.gltf", "planet").unwrap()
        );
        let vertex_mesh_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: unsafe {
                crate::uniform::any_as_u8_slice(&mesh_loader.vertices)
            },
            usage: wgpu::BufferUsage::VERTEX,
        });
        let num_mesh_indices = dbg!(mesh_loader.indices.len() as u32);

        let index_mesh_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: unsafe {
                crate::uniform::any_as_u8_slice(&mesh_loader.indices)
            },
            usage: wgpu::BufferUsage::INDEX,
        });

        // Init ECS systems
        let mut world = Shared::new(World::new());
        let sun = Entity::new(&mut world);
        let planet = Entity::new(&mut world);
        let moon = Entity::new(&mut world);
        let spacecraft = Entity::new(&mut world);

        {
            // sun
            sun
                .add(Physics {
                    mu: 1000.0,
                    p: Vec3::new(0.0, 0.0, 0.0),
                    v: Vec3::zero(),
                    has_moved: false,
                }, &mut world)
                .add(Render::Mesh(Mesh::new(planet_desc.clone(), Transform::default())), &mut world);
        }

        {
            // planet
            planet
                .add(
                    Physics::new_static(&Vec3::zero(), 20.0),
                    &mut world
                )
                .add(Orbit::new(
                        world.clone(),
                        sun,
                        Deg(50.0).into(),
                        Deg(30.0).into(),
                        Deg(15.0).into(),
                        5.0,
                        1.0,
                        0.6,
                    ),
                    &mut world,
                )
                .add(Render::Mesh(Mesh::new(planet_desc.clone(), Transform { scale: 0.5, translation: Vec3::zero(), rotation: Quaternion::zero() })), &mut world);
        }

        {
            // moon
            moon
                .add(
                    Physics::new_static(&Vec3::zero(), 10.0),
                    &mut world
                )
                .add(
                    Orbit::new(
                        world.clone(),
                        planet,
                        Deg(50.0).into(),
                        Deg(30.0).into(),
                        Deg(-90.0).into(),
                        2.0,
                        1.0,
                        0.1,
                    ),
                    &mut world,
                )
                .add(Render::Mesh(Mesh::new(planet_desc, Transform { scale: 0.2, translation: Vec3::zero(), rotation: Quaternion::zero() })), &mut world);
        }
        
        {
            // spacecraft
            spacecraft
                .add(
                    Physics::new_static(&Vec3::zero(), 1e-3),
                    &mut world
                )
                .add(Orbit::new(
                        world.clone(),
                        sun,
                        Deg(50.0).into(),
                        Deg(30.0).into(),
                        Deg(0.0).into(),
                        10.0,
                        10000.0,
                        0.01,
                    ),
                    &mut world,
                )
                .add(Camera {
                        data: CameraData::default(),
                        active: true
                    },
                    &mut world
                );
        }

        let input = InputGameState::new();
        let mut app = Self {
            surface,
            device,
            queue,
            sc_desc,
            swap_chain,
            size,

            mesh_loader,

            galaxy_render_pipeline,
            planet_render_pipeline,

            vertex_galaxy_buffer,
            index_galaxy_buffer,
            num_galaxy_indices,

            vertex_mesh_buffer,
            index_mesh_buffer,
            num_mesh_indices,

            gnomonic_projection_tex,
            map_tex,

            galaxy_bind_group_layout,
            galaxy_bind_group,

            planet_bind_group_layout,
            planet_bind_group,

            depth_texture,

            // uniforms
            window_size_uniform,
            camera_uniform,
            skybox_camera_uniform,
            model_mat_uniform,
            time_uniform,
            spheres_uniform,
            proj_mat_uniform,
            clock,

            // The world containing the ECS data
            world,
            // A pointer to the spacecraft entity
            spacecraft,

            // Game input listeners
            input,
        };
        app.resize::<Gnomonic>(size);

        app
    }

    pub fn register_inputs(&mut self, event: &WindowEvent) {
        self.input.register_inputs(event);
    }

    pub fn resize<P: Projection<f32>>(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.size = new_size;
        self.sc_desc.width = new_size.width;
        self.sc_desc.height = new_size.height;
        self.swap_chain = self.device.create_swap_chain(&self.surface, &self.sc_desc);

        let ndc = P::compute_ndc_to_clip_factor(self.size.width as f32, self.size.height as f32);
        self.window_size_uniform.write(&self.queue, 0, &ndc);

        let aspect = (self.size.width as f32) / (self.size.height as f32);
        self.proj_mat_uniform.write(&self.queue, 0, &(crate::render::OPENGL_TO_WGPU_MATRIX * cgmath::perspective(Deg(90.0), aspect, 0.1, 100.0)));

        self.depth_texture = Texture::create_depth_texture(&self.device, &self.sc_desc, "depth_texture");
    }

    pub fn update(&mut self, systems: &mut SystemManager) {
        let elapsed = self.clock.elapsed().as_secs_f32();
        self.time_uniform.write(&self.queue, 0, &elapsed);

        systems.run(self);
    }

    pub fn render(&mut self) -> Result<(), wgpu::SwapChainError> {
        Ok(())
    }
}
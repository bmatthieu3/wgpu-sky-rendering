extern crate byte_slice_cast;

use std::iter;
use std::time::Instant;
use wgpu::util::DeviceExt;

use winit::event::WindowEvent;

pub const NUM_PROJECTIONS: i32 = 6;
use crate::uniform::Uniform;
pub struct Game {
    surface: wgpu::Surface,
    device: wgpu::Device,
    pub queue: wgpu::Queue,
    sc_desc: wgpu::SwapChainDescriptor,
    swap_chain: wgpu::SwapChain,
    pub size: winit::dpi::PhysicalSize<u32>,
    // Ray tracing rendering pipeline
    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    num_indices: u32,
    map_tex: Texture,
    gnomonic_projection_tex: Texture,

    texture_bind_group_layout: wgpu::BindGroupLayout,
    diffuse_bind_group: wgpu::BindGroup,
    // Standard rasterizer rendering pipeline
    
    // Uniforms (can be send to multiple rendering pipelines)
    pub rot_mat_uniform: Uniform<Mat4<f32>>,
    window_size_uniform: Uniform<Vec2<f32>>,
    time_uniform: Uniform<f32>,
    pub spheres_uniform: Uniform<Vec<Sphere>>,

    pub clock: std::time::Instant,
    pub world: World,
}

use crate::ecs::Entity;
use crate::ecs::{World, SystemManager};
use crate::orbit::{
    Physics,
    UpdatePhysicsSystem,
    OrbitData,
};
use crate::projection::*;
use crate::texture::Texture;
use crate::triangulation::Triangulation;
use cgmath::InnerSpace;
use crate::math::Vec2;

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
    render::{Sphere, Render, Id},
};
use winit::window::Window;
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

        // Uniform buffer
        let rot_mat_buf = device.create_buffer(&wgpu::BufferDescriptor {
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
        let spheres_uniform = Uniform::new(&device);
        spheres_uniform.write(&queue, &spheres);

        let window_size_uniform = Uniform::new(&device);
        let time_uniform = Uniform::new(&device);
        let rot_mat_uniform = Uniform::new(&device);

        let texture_bind_group_layout =
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
                    rot_mat_uniform.desc_layout(4, wgpu::ShaderStage::FRAGMENT),
                    // window size uniform
                    window_size_uniform.desc_layout(5, wgpu::ShaderStage::VERTEX),
                    // time uniform
                    time_uniform.desc_layout(6, wgpu::ShaderStage::VERTEX),

                    // spherical objects uniform
                    spheres_uniform.desc_layout(7, wgpu::ShaderStage::FRAGMENT),
                ],
                label: Some("texture_bind_group_layout"),
            });

        let diffuse_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &texture_bind_group_layout,
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
                rot_mat_uniform.desc(4),
                window_size_uniform.desc(5),
                time_uniform.desc(6),
                spheres_uniform.desc(7),
            ],
            label: Some("diffuse_bind_group"),
        });

        // uniform buffer
        let vs_shader =
            device.create_shader_module(&wgpu::include_spirv!("shaders/allsky.vert.spv"));
        let fs_shader =
            device.create_shader_module(&wgpu::include_spirv!("shaders/allsky.frag.spv"));
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&texture_bind_group_layout],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &vs_shader,
                entry_point: "main",
                buffers: &[Vertex::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &fs_shader,
                entry_point: "main",
                targets: &[wgpu::ColorTargetState {
                    format: sc_desc.format,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent::REPLACE,
                        alpha: wgpu::BlendComponent::REPLACE,
                    }),
                    write_mask: wgpu::ColorWrite::ALL,
                }],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                // Setting this to anything other than Fill requires Features::NON_FILL_POLYGON_MODE
                polygon_mode: wgpu::PolygonMode::Fill,
                // Requires Features::DEPTH_CLAMPING
                clamp_depth: false,
                // Requires Features::CONSERVATIVE_RASTERIZATION
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
        });

        let (vertices, indices) = Triangulation::create::<Gnomonic>();

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsage::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsage::INDEX,
        });
        let num_indices = indices.len() as u32;

        let clock = Instant::now();
        let id_proj = 5;

        // Init ECS systems
        let mut world = World::new();
        let sun = Entity::new(&mut world);
        let planet = Entity::new(&mut world);
        let moon = Entity::new(&mut world);

        sun.add(&mut world,  Physics::Static {
                mu: 100.0,
                p: Vec3::new(0.0, 0.0, 0.0)
            })
            .add(&mut world, Render::Sphere(
                Sphere {
                    c: [0.0, 0.0, 0.0],
                    r: 1.0
                }
            ));
        planet.add(&mut world,  Physics::orbit(
            sun,
                10.0,
            OrbitData::Elliptical {
                    a: 5.0,
                    e: 0.8,
                    w: 0.0
                }
            ))
            .add(&mut world, Render::Sphere(
                Sphere {
                    c: [5.0, 0.0, 0.0],
                    r: 0.1
                }
            ));
        moon.add(&mut world,  Physics::orbit(
            planet,
                10.0,
                OrbitData::Elliptical {
                    a: 1.0,
                    e: 0.9,
                    w: 0.0
                }
            ))
            .add(&mut world, Render::Sphere(
                Sphere {
                    c: [5.0, 0.0, 0.0],
                    r: 0.05
                }
            ));

        let mut app = Self {
            surface,
            device,
            queue,
            sc_desc,
            swap_chain,
            size,
            render_pipeline,
            vertex_buffer,
            index_buffer,
            num_indices,

            gnomonic_projection_tex,
            map_tex,

            texture_bind_group_layout,
            diffuse_bind_group,

            // uniforms
            window_size_uniform,
            rot_mat_uniform,
            time_uniform,
            spheres_uniform,
            clock,

            world,
        };
        app.resize::<Gnomonic>(size);

        app
    }

    pub fn resize<P: Projection<f32>>(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.size = new_size;
        self.sc_desc.width = new_size.width;
        self.sc_desc.height = new_size.height;
        self.swap_chain = self.device.create_swap_chain(&self.surface, &self.sc_desc);

        let ndc = P::compute_ndc_to_clip_factor(self.size.width as f32, self.size.height as f32);
        self.window_size_uniform.write(&self.queue, &ndc);
    }

    #[allow(unused_variables)]
    pub fn input(&mut self, event: &WindowEvent) -> bool {
        false
    }

    pub fn update(&mut self, systems: &mut SystemManager) {
        systems.run(self);

        let elapsed = self.clock.elapsed().as_secs_f32();
        self.time_uniform.write(&self.queue, &elapsed);
    }

    pub fn render(&mut self) -> Result<(), wgpu::SwapChainError> {
        let frame = self.swap_chain.get_current_frame()?.output;

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[wgpu::RenderPassColorAttachment {
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
                depth_stencil_attachment: None,
            });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.diffuse_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..self.num_indices, 0, 0..1);
        }

        self.queue.submit(iter::once(encoder.finish()));

        Ok(())
    }
}
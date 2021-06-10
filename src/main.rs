extern crate byte_slice_cast;

use std::iter;
use std::time::Instant;
use wgpu::util::DeviceExt;
use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

mod texture;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    position: [f32; 2],
}

impl Vertex {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::InputStepMode::Vertex,
            attributes: &[wgpu::VertexAttribute {
                offset: 0,
                shader_location: 0,
                format: wgpu::VertexFormat::Float32x2,
            }],
        }
    }
}

const NUM_PROJECTIONS: i32 = 6;

struct State {
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    sc_desc: wgpu::SwapChainDescriptor,
    swap_chain: wgpu::SwapChain,
    size: winit::dpi::PhysicalSize<u32>,
    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    num_indices: u32,
    // NEW!
    #[allow(dead_code)]
    projection_textures: [texture::Texture; NUM_PROJECTIONS as usize],
    map_texture: texture::Texture,
    texture_bind_group_layout: wgpu::BindGroupLayout,
    diffuse_bind_group: wgpu::BindGroup,

    // uniforms
    rot_mat_buf: wgpu::Buffer,
    window_size_buf: wgpu::Buffer,

    clock: std::time::Instant,
}
mod angle;
mod math;
mod projection;
mod triangulation;
use crate::projection::*;
use crate::texture::Texture;
use crate::triangulation::Triangulation;
use cgmath::InnerSpace;
use math::Vec2;
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

use crate::math::Mat4;
impl State {
    async fn new(window: &Window) -> Self {
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
        let projection_textures = [
            create_position_texture::<Aitoff>(&device, &queue, 512),
            create_position_texture::<Ortho>(&device, &queue, 512),
            create_position_texture::<Mollweide>(&device, &queue, 512),
            create_position_texture::<Mercator>(&device, &queue, 512),
            create_position_texture::<AzimuthalEquidistant>(&device, &queue, 512),
            create_position_texture::<Gnomonic>(&device, &queue, 512),
        ];
        let bytes = include_bytes!("../img/map.png");
        let img = image::load_from_memory(bytes).unwrap();
        let map_texture = texture::Texture::from_image(&device, &queue, &img, "map.png");

        // Uniform buffer
        let rot_mat_buf = device.create_buffer(&wgpu::BufferDescriptor {
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
                    wgpu::BindGroupLayoutEntry {
                        binding: 4,
                        visibility: wgpu::ShaderStage::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: wgpu::BufferSize::new(
                                std::mem::size_of::<Mat4<f32>>() as _,
                            ),
                        },
                        count: None,
                    },
                    // window size uniform
                    wgpu::BindGroupLayoutEntry {
                        binding: 5,
                        visibility: wgpu::ShaderStage::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: wgpu::BufferSize::new(
                                std::mem::size_of::<Vec2<f32>>() as wgpu::BufferAddress,
                            ),
                        },
                        count: None,
                    },
                ],
                label: Some("texture_bind_group_layout"),
            });

        let diffuse_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&projection_textures[0].view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&projection_textures[0].sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&map_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&map_texture.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &rot_mat_buf,
                        offset: 0,
                        size: wgpu::BufferSize::new(
                            std::mem::size_of::<Mat4<f32>>() as wgpu::BufferAddress
                        ),
                    }),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &window_size_buf,
                        offset: 0,
                        size: wgpu::BufferSize::new(
                            std::mem::size_of::<Vec2<f32>>() as wgpu::BufferAddress
                        ),
                    }),
                },
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

        let (vertices, indices) = Triangulation::create::<Aitoff>();

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

            projection_textures,
            map_texture,

            texture_bind_group_layout,
            diffuse_bind_group,

            // uniforms
            window_size_buf,
            rot_mat_buf,
            clock,
        };
        app.resize::<Aitoff>(size);

        app
    }

    fn resize<P: Projection<f32>>(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.size = new_size;
        self.sc_desc.width = new_size.width;
        self.sc_desc.height = new_size.height;
        self.swap_chain = self.device.create_swap_chain(&self.surface, &self.sc_desc);

        let ndc = P::compute_ndc_to_clip_factor(self.size.width as f32, self.size.height as f32);
        self.queue.write_buffer(
            &self.window_size_buf,
            0,
            bytemuck::bytes_of(&[ndc.x, ndc.y]),
        );
    }

    #[allow(unused_variables)]
    fn input(&mut self, event: &WindowEvent) -> bool {
        false
    }

    fn update(&mut self) {
        let elapsed = self.clock.elapsed().as_secs_f32();
        let rot = Mat4::from_angle_y(cgmath::Rad(elapsed));
        let rot: &[[f32; 4]; 4] = rot.as_ref();

        self.queue
            .write_buffer(&self.rot_mat_buf, 0, bytemuck::bytes_of(rot));
    }

    fn set_projection(&mut self, idx: usize) {
        // Update the vertex and index buffers
        let (vertices, indices) = match idx {
            0 => Triangulation::create::<Aitoff>(),
            1 => Triangulation::create::<Ortho>(),
            2 => Triangulation::create::<Mollweide>(),
            3 => Triangulation::create::<Mercator>(),
            4 => Triangulation::create::<AzimuthalEquidistant>(),
            5 => Triangulation::create::<Gnomonic>(),
            _ => unimplemented!(),
        };

        self.vertex_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Vertex Buffer"),
                contents: bytemuck::cast_slice(&vertices),
                usage: wgpu::BufferUsage::VERTEX,
            });
        self.index_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Index Buffer"),
                contents: bytemuck::cast_slice(&indices),
                usage: wgpu::BufferUsage::INDEX,
            });
        self.num_indices = indices.len() as u32;

        // Update the uniforms
        let aspect = match idx {
            0 => {
                Aitoff::compute_ndc_to_clip_factor(self.size.width as f32, self.size.height as f32)
            }
            1 => Ortho::compute_ndc_to_clip_factor(self.size.width as f32, self.size.height as f32),
            2 => Mollweide::compute_ndc_to_clip_factor(
                self.size.width as f32,
                self.size.height as f32,
            ),
            3 => Mercator::compute_ndc_to_clip_factor(
                self.size.width as f32,
                self.size.height as f32,
            ),
            4 => AzimuthalEquidistant::compute_ndc_to_clip_factor(
                self.size.width as f32,
                self.size.height as f32,
            ),
            5 => Gnomonic::compute_ndc_to_clip_factor(
                self.size.width as f32,
                self.size.height as f32,
            ),
            _ => unimplemented!(),
        };
        self.queue.write_buffer(
            &self.window_size_buf,
            0,
            bytemuck::bytes_of(&[aspect.x, aspect.y]),
        );

        // Update the bind group with the texture position from the current projection
        self.diffuse_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &self.texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(
                        &self.projection_textures[idx].view,
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(
                        &self.projection_textures[idx].sampler,
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&self.map_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&self.map_texture.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &self.rot_mat_buf,
                        offset: 0,
                        size: wgpu::BufferSize::new(
                            std::mem::size_of::<Mat4<f32>>() as wgpu::BufferAddress
                        ),
                    }),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &self.window_size_buf,
                        offset: 0,
                        size: wgpu::BufferSize::new(
                            std::mem::size_of::<Vec2<f32>>() as wgpu::BufferAddress
                        ),
                    }),
                },
            ],
            label: Some("diffuse_bind_group"),
        });
    }

    fn render(&mut self) -> Result<(), wgpu::SwapChainError> {
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

fn main() {
    env_logger::init();
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("allsky projections")
        .build(&event_loop).unwrap();

    use futures::executor::block_on;
    // Since main can't be async, we're going to need to block
    let mut state = block_on(State::new(&window));

    #[cfg(target_arch = "wasm32")]
    {
        use winit::platform::web::WindowExtWebSys;

        let canvas = window.canvas();

        let window = web_sys::window().unwrap();
        let document = window.document().unwrap();
        let body = document.body().unwrap();

        body.append_child(&canvas)
            .expect("Append canvas to HTML body");
    }

    let mut count: i32 = 0;

    event_loop.run(move |event, _, control_flow| {
        match event {
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == window.id() => {
                if !state.input(event) {
                    match event {
                        WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                        WindowEvent::KeyboardInput { input, .. } => match input {
                            KeyboardInput {
                                state: ElementState::Pressed,
                                virtual_keycode: Some(VirtualKeyCode::Escape),
                                ..
                            } => *control_flow = ControlFlow::Exit,
                            KeyboardInput {
                                state: ElementState::Pressed,
                                virtual_keycode: Some(VirtualKeyCode::Left),
                                ..
                            } => {
                                count += 1;
                                count %= NUM_PROJECTIONS;

                                state.set_projection(count as usize);
                            }
                            KeyboardInput {
                                state: ElementState::Pressed,
                                virtual_keycode: Some(VirtualKeyCode::Right),
                                ..
                            } => {
                                count -= 1;
                                if count < 0 {
                                    count += NUM_PROJECTIONS;
                                }
                                count %= NUM_PROJECTIONS;

                                state.set_projection(count as usize);
                            }
                            _ => {}
                        },
                        WindowEvent::Resized(physical_size) => match count {
                            0 => state.resize::<Aitoff>(*physical_size),
                            1 => state.resize::<Ortho>(*physical_size),
                            2 => state.resize::<Mollweide>(*physical_size),
                            3 => state.resize::<Mercator>(*physical_size),
                            4 => state.resize::<AzimuthalEquidistant>(*physical_size),
                            5 => state.resize::<Gnomonic>(*physical_size),
                            _ => unimplemented!(),
                        },
                        WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                            // new_inner_size is &mut so w have to dereference it twice
                            match count {
                                0 => state.resize::<Aitoff>(**new_inner_size),
                                1 => state.resize::<Ortho>(**new_inner_size),
                                2 => state.resize::<Mollweide>(**new_inner_size),
                                3 => state.resize::<Mercator>(**new_inner_size),
                                4 => state.resize::<AzimuthalEquidistant>(**new_inner_size),
                                5 => state.resize::<Gnomonic>(**new_inner_size),
                                _ => unimplemented!(),
                            }
                        }
                        _ => {}
                    }
                }
            }
            Event::RedrawRequested(_) => {
                state.update();
                match state.render() {
                    Ok(_) => {}
                    // Recreate the swap_chain if lost
                    Err(wgpu::SwapChainError::Lost) => match count {
                        0 => state.resize::<Aitoff>(state.size),
                        1 => state.resize::<Ortho>(state.size),
                        2 => state.resize::<Mollweide>(state.size),
                        3 => state.resize::<Mercator>(state.size),
                        4 => state.resize::<AzimuthalEquidistant>(state.size),
                        5 => state.resize::<Gnomonic>(state.size),
                        _ => unimplemented!(),
                    },
                    // The system is out of memory, we should probably quit
                    Err(wgpu::SwapChainError::OutOfMemory) => *control_flow = ControlFlow::Exit,
                    // All other errors (Outdated, Timeout) should be resolved by the next frame
                    Err(e) => eprintln!("{:?}", e),
                }
            }
            Event::MainEventsCleared => {
                // RedrawRequested will only trigger once, unless we manually
                // request it.
                window.request_redraw();
            }
            _ => {}
        }
    });
}

#[cfg(target_arch = "wasm32")]
mod wasm {
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen(start)]
    pub fn run() {
        console_log::init_with_level(log::Level::Debug);

        super::main();
    }
}

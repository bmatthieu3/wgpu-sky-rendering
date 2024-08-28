extern crate byte_slice_cast;

#[cfg(target_arch="wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch="wasm32")]
extern crate console_error_panic_hook;

use std::iter;

use wgpu::util::DeviceExt;
use winit::{
    event::*,
    event_loop::EventLoop,
    window::{Window, WindowBuilder},
};
use winit::keyboard::PhysicalKey;
use winit::keyboard::KeyCode;
use winit::window::Fullscreen;
mod texture;
mod vertex;
mod time;

use time::Clock;
use vertex::Vertex;
use texture::Texture;
use crate::math::Vec4;
const NUM_PROJECTIONS: i32 = 6;

struct State<'a> {
    surface: wgpu::Surface<'a>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,

    // The window must be declared after the surface so
    // it gets dropped after it as the surface contains
    // unsafe references to the window's resources.
    window: &'a Window,

    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    num_indices: u32,

    map_texture: texture::Texture,
    texture_bind_group_layout: wgpu::BindGroupLayout,
    diffuse_bind_group: wgpu::BindGroup,

    // uniforms
    rot_mat_buf: wgpu::Buffer,
    window_size_buf: wgpu::Buffer,

    clock: Clock,
}

mod angle;
mod math;
mod projection;
mod triangulation;
use crate::projection::*;
use crate::triangulation::Triangulation;
use math::Vec2;
use crate::math::Vec3;
fn generate_position<P: Projection<f32>>(size: u32) -> Vec<u8> {
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
                let pos = Vec3::new(
                    pos.x * 0.5 + 0.5,
                    pos.y * 0.5 + 0.5,
                    pos.z * 0.5 + 0.5,
                );

                data.extend(&[(pos.x * 256.0) as u8, (pos.y * 256.0) as u8, (pos.z * 256.0) as u8, 255]);
            } else {
                data.extend(&[255, 255, 255, 255]);
            }
        }
    }

    data
}

pub fn create_position_texture<P: Projection<f32>>(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    size: u32,
) -> Texture {
    let texels = generate_position::<P>(size);
    let bytes = texels.as_slice();

    let dimensions = (size, size, 1);
    let num_bytes_per_pixel = 4;
    Texture::from_raw_bytes::<u8>(&device, &queue, Some(bytes), dimensions, num_bytes_per_pixel, "position")
}

use crate::math::Mat4;
impl<'a> State<'a> {
    async fn new(window: &'a Window) -> Self {
        let size = window.inner_size();

        // The instance is a handle to our GPU
        // BackendBit::all => Vulkan + Metal + DX12 + Browser WebGPU
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            #[cfg(not(target_arch="wasm32"))]
            backends: wgpu::Backends::PRIMARY,
            #[cfg(target_arch="wasm32")]
            backends: wgpu::Backends::GL,
            ..Default::default()
        });

        let surface = instance.create_surface(window).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    required_features: wgpu::Features::empty(),
                    // favor performane over the memory usage
                    memory_hints: Default::default(),
                    // WebGL doesn't support all of wgpu's features, so if
                    // we're building for the web, we'll have to disable some.
                    required_limits: if cfg!(target_arch = "wasm32") {
                        wgpu::Limits {
                            max_texture_dimension_3d: 512,
                            ..wgpu::Limits::downlevel_webgl2_defaults()
                        }
                    } else {
                        wgpu::Limits::default()
                    },
                    label: None,
                },
                None, // Trace path
            )
            .await
            .unwrap();

        let surface_caps = surface.get_capabilities(&adapter);
        // Shader code in this tutorial assumes an sRGB surface texture. Using a different
        // one will result in all the colors coming out darker. If you want to support non
        // sRGB surfaces, you'll need to account for that when drawing to the frame.
        let surface_format = surface_caps.formats.iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![surface_format.add_srgb_suffix()],
            desired_maximum_frame_latency: 2,
        };

        /*let bytes = include_bytes!("../img/map.png");
        let img = image::load_from_memory(bytes).unwrap();
        let map_texture = texture::Texture::from_image(&device, &queue, &img, "map.png");*/

        let map_texture = Texture::from_raw_bytes::<u8>(
            &device,
            &queue,
            None,
            (512, 512, 12),
            4,
            "base HEALPix cells"
        );

        let tiles = [
            include_bytes!("../img/Npix0.jpg").to_vec(),
            include_bytes!("../img/Npix1.jpg").to_vec(),
            include_bytes!("../img/Npix2.jpg").to_vec(),
            include_bytes!("../img/Npix3.jpg").to_vec(),
            include_bytes!("../img/Npix4.jpg").to_vec(),
            include_bytes!("../img/Npix5.jpg").to_vec(),
            include_bytes!("../img/Npix6.jpg").to_vec(),
            include_bytes!("../img/Npix7.jpg").to_vec(),
            include_bytes!("../img/Npix8.jpg").to_vec(),
            include_bytes!("../img/Npix9.jpg").to_vec(),
            include_bytes!("../img/Npix10.jpg").to_vec(),
            include_bytes!("../img/Npix11.jpg").to_vec()
        ];

        for (idx, tile_bytes) in tiles.iter().enumerate() {
            let rgba_tile = image::load_from_memory(&tile_bytes).unwrap().to_rgba8();
            map_texture.write_data(
                &queue,
                (0, 0, idx as u32),
                &rgba_tile,
                (512, 512, 1)
            );
        }

        // Uniform buffer
        let rot_mat_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("rot matrix uniform"),
            size: 64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let window_size_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("window size uniform"),
            size: 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D3,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                    // rot matrix uniform
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
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
                        binding: 3,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: wgpu::BufferSize::new(
                                std::mem::size_of::<Vec4<f32>>() as wgpu::BufferAddress,
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
                    resource: wgpu::BindingResource::TextureView(&map_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&map_texture.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &rot_mat_buf,
                        offset: 0,
                        size: wgpu::BufferSize::new(
                            std::mem::size_of::<Mat4<f32>>() as wgpu::BufferAddress
                        ),
                    }),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &window_size_buf,
                        offset: 0,
                        size: wgpu::BufferSize::new(
                            16
                        ),
                    }),
                },
            ],
            label: Some("diffuse_bind_group"),
        });

        // uniform buffer
        let vs_shader =
            device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("allsky vert shader"),
                source: wgpu::ShaderSource::Glsl {
                    shader: include_str!("shaders/allsky.vert").into(),
                    stage: naga::ShaderStage::Vertex,
                    defines: Default::default()
                }
            });
        let fs_shader =
            device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("allsky frag shader"),
                source: wgpu::ShaderSource::Glsl {
                    shader: include_str!("shaders/allsky.frag").into(),
                    stage: naga::ShaderStage::Fragment,
                    defines: Default::default()
                },
            });

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
                compilation_options: Default::default(),
                buffers: &[Vertex::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &fs_shader,
                entry_point: "main",
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                // Setting this to anything other than Fill requires Features::NON_FILL_POLYGON_MODE
                polygon_mode: wgpu::PolygonMode::Fill,
                // Requires Features::DEPTH_CLIP_CONTROL
                unclipped_depth: false,
                // Requires Features::CONSERVATIVE_RASTERIZATION
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None, // 5.
            cache: None, // 6.
        });

        let (vertices, indices) = Triangulation::create::<Aitoff>();

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        let num_indices = indices.len() as u32;

        let clock = Clock::now();
        let mut app = Self {
            surface,
            device,
            queue,
            config,
            size,
            window,
            render_pipeline,
            vertex_buffer,
            index_buffer,
            num_indices,

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

    fn resize<P: Projection<f32>>(&mut self, mut new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            #[cfg(target_arch="wasm32")] {
                new_size.width = new_size.width.min(wgpu::Limits::downlevel_webgl2_defaults().max_texture_dimension_2d);
                new_size.height = new_size.height.min(wgpu::Limits::downlevel_webgl2_defaults().max_texture_dimension_2d);    
            }

            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }

        let ndc = P::compute_ndc_to_clip_factor(self.size.width as f32, self.size.height as f32);
        self.queue.write_buffer(
            &self.window_size_buf,
            0,
            bytemuck::bytes_of(&[ndc.x, ndc.y, 0.0, 0.0]),
        );
    }

    #[allow(unused_variables)]
    fn input(&mut self, event: &WindowEvent) -> bool {
        false
    }

    fn update(&mut self) {
        let elapsed = self.clock.elapsed_as_secs();

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
                usage: wgpu::BufferUsages::VERTEX,
            });
        self.index_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Index Buffer"),
                contents: bytemuck::cast_slice(&indices),
                usage: wgpu::BufferUsages::INDEX,
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
            bytemuck::bytes_of(&[aspect.x, aspect.y, 0.0, 0.0]),
        );

        // Update the bind group with the texture position from the current projection
        self.diffuse_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &self.texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&self.map_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.map_texture.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &self.rot_mat_buf,
                        offset: 0,
                        size: wgpu::BufferSize::new(
                            std::mem::size_of::<Mat4<f32>>() as wgpu::BufferAddress
                        ),
                    }),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &self.window_size_buf,
                        offset: 0,
                        size: wgpu::BufferSize::new(
                            16
                        ),
                    }),
                },
            ],
            label: Some("diffuse_bind_group"),
        });
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let size = self.window.inner_size();
        if size.width == 0 || size.height == 0 {
            return Ok(())
        }

        if let Ok(frame) = self.surface.get_current_texture() {
            let view = frame.texture.create_view(&wgpu::TextureViewDescriptor {
                format: Some(self.config.format.add_srgb_suffix()),
                ..Default::default()
            });
    
            let mut encoder = self
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Render Encoder"),
                });
    
            {
                let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Render Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: 0.01,
                                g: 0.01,
                                b: 0.01,
                                a: 1.0,
                            }),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    occlusion_query_set: None,
                    timestamp_writes: None,
                });
    
                render_pass.set_pipeline(&self.render_pipeline);
                render_pass.set_bind_group(0, &self.diffuse_bind_group, &[]);
                render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
                render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                render_pass.draw_indexed(0..self.num_indices, 0, 0..1);
            }
    
            self.queue.submit(iter::once(encoder.finish()));
            frame.present();
        }

        Ok(())
    }
}

#[cfg_attr(target_arch="wasm32", wasm_bindgen(start))]
pub async fn run() {
    #[cfg(target_arch = "wasm32")]
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    #[cfg(target_arch = "wasm32")]
    console_log::init_with_level(log::Level::Warn).expect("Couldn't initialize logger");
    #[cfg(not(target_arch = "wasm32"))]
    env_logger::init();

    let event_loop = EventLoop::new().unwrap();
    let mut builder = WindowBuilder::new();

    #[cfg(target_arch = "wasm32")]
    {   
        use wasm_bindgen::JsCast;
        use winit::platform::web::WindowBuilderExtWebSys;
        let canvas = web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .get_element_by_id("canvas")
            .unwrap()
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .unwrap();

        builder = builder.with_canvas(Some(canvas));
    }
    let window = builder.with_title("allsky projections")
        .build(&event_loop).unwrap();

    // Winit prevents sizing with CSS, so we have to set
    // the size manually when on web.
    #[cfg(target_arch = "wasm32")]
    {
        use winit::dpi::LogicalSize;
        let _ = window.request_inner_size(LogicalSize::new(768, 512));
    }

    let mut state = State::new(&window).await;

    let mut count: i32 = 0;

    event_loop.run(move |event, control_flow| {
        match event {
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == state.window.id() => {
                if !state.input(event) {
                    match event {
                        #[cfg(not(target_arch="wasm32"))]
                        WindowEvent::CloseRequested
                        | WindowEvent::KeyboardInput {
                            event:
                                KeyEvent {
                                    state: ElementState::Pressed,
                                    physical_key: PhysicalKey::Code(KeyCode::Escape),
                                    ..
                                },
                            ..
                        } => control_flow.exit(),
                        WindowEvent::KeyboardInput {
                            event:
                                KeyEvent {
                                    state: ElementState::Pressed,
                                    physical_key: PhysicalKey::Code(KeyCode::ArrowLeft),
                                    ..
                                },
                            ..
                        } => {
                            count += 1;
                            count %= NUM_PROJECTIONS;

                            state.set_projection(count as usize);
                        },
                        WindowEvent::KeyboardInput {
                            event:
                                KeyEvent {
                                    state: ElementState::Pressed,
                                    physical_key: PhysicalKey::Code(KeyCode::Enter),
                                    ..
                                },
                            ..
                        } => {
                            // toggle fullscreen
                            state.window.set_fullscreen(Some(Fullscreen::Borderless(None)));
                        },
                        WindowEvent::KeyboardInput {
                            event:
                                KeyEvent {
                                    state: ElementState::Pressed,
                                    physical_key: PhysicalKey::Code(KeyCode::ArrowRight),
                                    ..
                                },
                            ..
                        } => {
                            count -= 1;
                                if count < 0 {
                                    count += NUM_PROJECTIONS;
                                }
                                count %= NUM_PROJECTIONS;

                                state.set_projection(count as usize);
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
                        WindowEvent::RedrawRequested => {
                            state.update();
                            match state.render() {
                                Ok(_) => {}
                                // Reconfigure the surface if lost
                                Err(wgpu::SurfaceError::Lost) => match count {
                                    0 => state.resize::<Aitoff>(state.size),
                                    1 => state.resize::<Ortho>(state.size),
                                    2 => state.resize::<Mollweide>(state.size),
                                    3 => state.resize::<Mercator>(state.size),
                                    4 => state.resize::<AzimuthalEquidistant>(state.size),
                                    5 => state.resize::<Gnomonic>(state.size),
                                    _ => unimplemented!(),
                                },
                                // The system is out of memory, we should probably quit
                                Err(wgpu::SurfaceError::OutOfMemory) => control_flow.exit(),
                                // All other errors (Outdated, Timeout) should be resolved by the next frame
                                Err(e) => { eprintln!("{}", e); },
                            }
                        }
                        /*WindowEvent::ScaleFactorChanged { scale_factor, inner_size_writer } => {
                            state.inner_size
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
                        }*/
                        _ => {}
                    }
                }
            }
            // ... at the end of the WindowEvent block
            Event::AboutToWait => {
                // RedrawRequested will only trigger once unless we manually
                // request it.
                state.window.request_redraw();
            }
            _ => {}
        }
    }).unwrap();
}


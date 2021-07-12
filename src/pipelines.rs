use std::collections::HashMap;

pub struct RenderingPipeline {
    pipelines: HashMap<&'static str, wgpu::RenderPipeline>
}

use std::path::Path;
pub trait RenderPipeline {
    // Vertex type
    type VertexType: Vertex;

    fn new(
        // The device for which the pipeline is created
        device: &wgpu::Device,
        // Describes how the output attachment
        sc_desc: &wgpu::SwapChainDescriptor,
        // The vertex shader descriptor
        vertex_shader_desc: wgpu::ShaderModuleDescriptor,
        // The fragment shader descriptor
        frag_shader_desc: wgpu::ShaderModuleDescriptor,
        // A bunch of bind group layouts
        bind_group_layouts: &[&wgpu::BindGroupLayout],
    ) -> wgpu::RenderPipeline;
}

use crate::vertex::{ClipSpaceVertex, Vertex, WorldSpaceVertex};
pub struct GalaxyPipeline;
impl RenderPipeline for GalaxyPipeline {
    type VertexType = ClipSpaceVertex;

    fn new(
        // The device for which the pipeline is created
        device: &wgpu::Device,
        // Describes how the output attachment
        sc_desc: &wgpu::SwapChainDescriptor,
        // The vertex shader descriptor
        vertex_shader_desc: wgpu::ShaderModuleDescriptor,
        // The fragment shader descriptor
        frag_shader_desc: wgpu::ShaderModuleDescriptor,
        // A bunch of bind group layouts
        bind_group_layouts: &[&wgpu::BindGroupLayout],
    ) -> wgpu::RenderPipeline {
        let vs_shader = device.create_shader_module(&vertex_shader_desc);
        let fs_shader = device.create_shader_module(&frag_shader_desc);
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: bind_group_layouts,
                push_constant_ranges: &[],
            });

        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Galaxy Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &vs_shader,
                entry_point: "main",
                buffers: &[Self::VertexType::desc()],
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
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: false,
                depth_compare: wgpu::CompareFunction::Less, // 1.
                stencil: wgpu::StencilState::default(), // 2.
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
        })
    }
}

pub struct PlanetPipeline;
impl RenderPipeline for PlanetPipeline {
    type VertexType = WorldSpaceVertex;

    fn new(
        // The device for which the pipeline is created
        device: &wgpu::Device,
        // Describes how the output attachment
        sc_desc: &wgpu::SwapChainDescriptor,
        // The vertex shader descriptor
        vertex_shader_desc: wgpu::ShaderModuleDescriptor,
        // The fragment shader descriptor
        frag_shader_desc: wgpu::ShaderModuleDescriptor,
        // A bunch of bind group layouts
        bind_group_layouts: &[&wgpu::BindGroupLayout],
    ) -> wgpu::RenderPipeline {
        let vs_shader = device.create_shader_module(&vertex_shader_desc);
        let fs_shader = device.create_shader_module(&frag_shader_desc);
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: bind_group_layouts,
                push_constant_ranges: &[],
            });

        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Planet Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &vs_shader,
                entry_point: "main",
                buffers: &[Self::VertexType::desc()],
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
            // Describes how us writing to the depth/stencil buffer
            // will work. Since this is water, we need to read from the
            // depth buffer both as a texture in the shader, and as an
            // input attachment to do depth-testing. We don't write, so
            // depth_write_enabled is set to false. This is called
            // RODS or read-only depth stencil.
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less, // 1.
                stencil: wgpu::StencilState::default(), // 2.
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
        })
    }
}
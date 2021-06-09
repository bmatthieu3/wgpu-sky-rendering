use crate::texture::Texture;
use crate::vertex::Vertex;

pub struct TexturedObjectRenderDesc;
impl TexturedObjectRenderDesc {
    pub fn initialize(
        // The device for which the pipeline is created for
        device: &wgpu::Device,
        sc_desc: &wgpu::SwapChainDescriptor,
    
        bind_group_layouts: &[&wgpu::BindGroupLayout],
    ) -> wgpu::RenderPipeline {
        let vs_module = device.create_shader_module(&wgpu::include_spirv!("shaders/allsky.vert.spv"));
        let fs_module = device.create_shader_module(&wgpu::include_spirv!("shaders/allsky.frag.spv"));
        let render_pipeline_layout = device.create_pipeline_layout(
            &wgpu::PipelineLayoutDescriptor {
                label: Some("Render to color attachment"),
                bind_group_layouts: bind_group_layouts,
                push_constant_ranges: &[],
            }
        );
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render to color attachment pipeline"),
            // The "layout" is what uniforms will be needed.
            layout: Some(&render_pipeline_layout),
            // Vertex shader and input buffers
            vertex: wgpu::VertexState {
                module: &vs_module,
                entry_point: "main",
                // Layout of our vertices. This should match the structs
                // which are uploaded to the GPU. This should also be
                // ensured by tagging on either a `#[repr(C)]` onto a
                // struct, or a `#[repr(transparent)]` if it only contains
                // one item, which is itself `repr(C)`.
                buffers: &[
                    Vertex::desc()
                ],
            },
            // Fragment shader and output targets
            fragment: Some(wgpu::FragmentState {
                module: &fs_module,
                entry_point: "main",
                // Describes how the colour will be interpolated
                // and assigned to the output attachment.
                targets: &[sc_desc.format.into()],
            }),
            // How the triangles will be rasterized. This is more important
            // for the terrain because of the beneath-the water shot.
            // This is also dependent on how the triangles are being generated.
            primitive: wgpu::PrimitiveState {
                // What kind of data are we passing in?
                topology: wgpu::PrimitiveTopology::TriangleList,
                front_face: wgpu::FrontFace::Ccw,
                ..Default::default()
            },
            // Describes how us writing to the depth/stencil buffer
            // will work. Since this is water, we need to read from the
            // depth buffer both as a texture in the shader, and as an
            // input attachment to do depth-testing. We don't write, so
            // depth_write_enabled is set to false. This is called
            // RODS or read-only depth stencil.
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth24PlusStencil8,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            // depth_stencil: None,
            // No multisampling is used.
            multisample: wgpu::MultisampleState::default(),
        });

        pipeline
    }
}

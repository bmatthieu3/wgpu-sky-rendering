use image::GenericImageView;

pub struct Texture {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
}
use byte_slice_cast::*;

pub trait TextureFormat: ToByteSlice {
    const WGPU_FORMAT: wgpu::TextureFormat;
}

impl TextureFormat for f32 {
    const WGPU_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba32Float;
}
impl TextureFormat for u8 {
    const WGPU_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
}
impl Texture {
    fn from_bytes_rgba(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        rgba: &[u8],
        bytes_per_row: u32,
        dimensions: (u32, u32),
        label: &str,
    ) -> Self {
        let size = wgpu::Extent3d {
            width: dimensions.0,
            height: dimensions.1,
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsage::SAMPLED | wgpu::TextureUsage::COPY_DST,
        });

        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            rgba,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(std::num::NonZeroU32::new(bytes_per_row).unwrap()),
                rows_per_image: None,
            },
            size,
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        Self {
            texture,
            view,
            sampler,
        }
    }

    pub fn from_raw_bytes<T: TextureFormat>(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        data: &[T],
        dimensions: (usize, usize),
        label: &str,
    ) -> Self {
        let data = T::to_byte_slice(data);
        let bytes_per_row = (dimensions.0 as u32) * 4 * (std::mem::size_of::<T>() as u32);

        Self::from_bytes_rgba(
            device,
            queue,
            T::WGPU_FORMAT,
            data,
            bytes_per_row,
            (dimensions.0 as u32, dimensions.1 as u32),
            label,
        )
    }

    // rgba images
    pub fn from_image(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        img: &image::DynamicImage,
        label: &str,
    ) -> Self {
        let rgba = img.as_rgba8().unwrap();
        let dimensions = img.dimensions();
        let bytes_per_row = (dimensions.0 as u32) * 4 * (std::mem::size_of::<u8>() as u32);
        Self::from_bytes_rgba(
            device,
            queue,
            wgpu::TextureFormat::Rgba8UnormSrgb,
            rgba,
            bytes_per_row,
            dimensions,
            label,
        )
    }

    pub fn create_depth_texture(device: &wgpu::Device, sc_desc: &wgpu::SwapChainDescriptor, label: &str) -> Self {
        let size = wgpu::Extent3d { // 2.
            width: sc_desc.width,
            height: sc_desc.height,
            depth_or_array_layers: 1,
        };
        let desc = wgpu::TextureDescriptor {
            label: Some(label),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsage::RENDER_ATTACHMENT // 3.
                | wgpu::TextureUsage::SAMPLED,
        };
        let texture = device.create_texture(&desc);

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(
            &wgpu::SamplerDescriptor { // 4.
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                address_mode_w: wgpu::AddressMode::ClampToEdge,
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                mipmap_filter: wgpu::FilterMode::Nearest,
                compare: Some(wgpu::CompareFunction::LessEqual), // 5.
                lod_min_clamp: -100.0,
                lod_max_clamp: 100.0,
                ..Default::default()
            }
        );

        Self { texture, view, sampler }
    }
}

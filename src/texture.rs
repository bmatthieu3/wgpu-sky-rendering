use image::GenericImageView;

pub struct Texture {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
    num_bytes_per_pixel: usize,
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
use log::info;
impl Texture {
    fn from_bytes_rgba(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        rgba: Option<&[u8]>,
        dimensions: (u32, u32, u32),
        num_bytes_per_pixel: usize,
        label: &str,
    ) -> Self {
        let dimension = if dimensions.2 == 1 {
            wgpu::TextureDimension::D2
        } else {
            info!("{:?}", dimensions);
            wgpu::TextureDimension::D3
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            // All textures are stored as 3D, we represent our 2D texture
            // by setting depth to 1.
            size: wgpu::Extent3d {
                width: dimensions.0,
                height: dimensions.1,
                depth_or_array_layers: dimensions.2,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension,
            format,
            // TEXTURE_BINDING tells wgpu that we want to use this texture in shaders
            // COPY_DST means that we want to copy data to this texture
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

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

        let texture = Self {
            texture,
            view,
            sampler,
            num_bytes_per_pixel,
        };

        if let Some(rgba) = rgba {
            texture.write_data(queue, (0, 0, 0), rgba, dimensions);
        }

        texture
    }

    pub fn write_data(
        &self,
        queue: &wgpu::Queue,
        origin: (u32, u32, u32),
        data: &[u8],
        dimensions: (u32, u32, u32),
    ) {
        let size = wgpu::Extent3d {
            width: dimensions.0,
            height: dimensions.1,
            depth_or_array_layers: dimensions.2,
        };
        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: origin.0,
                    y: origin.1,
                    z: origin.2,
                },
                aspect: wgpu::TextureAspect::All,
            },
            data,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some((self.num_bytes_per_pixel as u32) * dimensions.0),
                rows_per_image: Some(dimensions.1),
            },
            size,
        );
    }

    pub fn from_raw_bytes<T: TextureFormat>(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bytes: Option<&[T]>,
        dimensions: (u32, u32, u32),
        num_bytes_per_pixel: usize,
        label: &str,
    ) -> Self {
        let bytes = bytes.and_then(|bytes| Some(T::to_byte_slice(bytes)));
        Self::from_bytes_rgba(
            device,
            queue,
            T::WGPU_FORMAT,
            bytes,
            dimensions,
            num_bytes_per_pixel,
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

        Self::from_bytes_rgba(
            device,
            queue,
            wgpu::TextureFormat::Rgba8UnormSrgb,
            Some(rgba),
            (dimensions.0, dimensions.1, 1),
            4,
            label,
        )
    }
}

use std::num::NonZeroU64;

pub trait ToByteSlice {
    unsafe fn any_as_u8_slice(&self) -> &[u8];
}

impl<T> ToByteSlice for Vec<T>
where 
    T: UniformData
{
    unsafe fn any_as_u8_slice(&self) -> &[u8] {
        std::slice::from_raw_parts(
            self.as_ptr() as *const u8,
            std::mem::size_of::<T>() * self.len(),
        )
    }
}

pub struct UniformBuffer<D>
where
    D: UniformData
{
    _data: std::marker::PhantomData<D>,
    buf: wgpu::Buffer
}

impl<D> UniformBuffer<D>
where
    D: UniformData
{
    pub fn new(device: &wgpu::Device) -> Self {
        let buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: wgpu::BIND_BUFFER_ALIGNMENT,
            usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            buf,
            _data: std::marker::PhantomData,
        }
    }

    pub fn write(&self, q: &wgpu::Queue, data: &D) {
        q.write_buffer(
            &self.buf,
            0,
            unsafe { data.any_as_u8_slice() }
        );
    }

    pub fn desc_layout(&self, binding: u32, visibility: wgpu::ShaderStage) -> wgpu::BindGroupLayoutEntry {
        wgpu::BindGroupLayoutEntry {
            binding,
            visibility,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: D::min_binding_size(),
            },
            count: None,
        }
    }

    pub fn desc(&self, binding: u32) -> wgpu::BindGroupEntry {
        wgpu::BindGroupEntry {
            binding,
            resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                buffer: &self.buf,
                offset: 0,
                size: D::min_binding_size(),
            }),
        }
    }
}

pub trait UniformData: Sized + ToByteSlice {
    fn min_binding_size() -> Option<NonZeroU64>;
}

// Implement the primitives as uniforms
trait Uniform {}

impl Uniform for f32 {}
impl Uniform for i16 {}
impl Uniform for i32 {}
impl Uniform for u32 {}
impl Uniform for u8 {}

impl<T> ToByteSlice for T
where
    T: Uniform
{
    unsafe fn any_as_u8_slice(&self) -> &[u8] {
        std::slice::from_raw_parts(
            (self as *const Self) as *const u8,
            std::mem::size_of::<Self>(),
        )
    }
}

impl<T> UniformData for T
where
    T: Uniform
{
    fn min_binding_size() -> Option<NonZeroU64> {
        wgpu::BufferSize::new(
            std::mem::size_of::<Self>() as wgpu::BufferAddress,
        )
    }
}

// All vec of uniforms are uniforms
const MAX_OBJECT_SIZE: usize = 10;
impl<S> UniformData for Vec<S>
where
    S: UniformData
{
    fn min_binding_size() -> Option<NonZeroU64> {
        wgpu::BufferSize::new(
            (MAX_OBJECT_SIZE*std::mem::size_of::<S>()) as wgpu::BufferAddress,
        )
    }
}

// Math uniforms
use crate::math::{Vec2, Vec3, Vec4, Mat3, Mat4};
impl<T> Uniform for Mat3<T> {}
impl<T> Uniform for Mat4<T> {}
impl<T> Uniform for Vec2<T> {}
impl<T> Uniform for Vec3<T> {}
impl<T> Uniform for Vec4<T> {}

// Rendering primitives for ray-tracing
use crate::render::Sphere;
impl Uniform for Sphere {}
use crate::camera::CameraData;
impl Uniform for CameraData {}





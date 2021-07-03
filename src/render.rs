
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Sphere {
    pub c: [f32; 3],
    pub r: f32
}

use crate::ecs;
use core_engine::Component;
pub struct Id<T>(usize, std::marker::PhantomData<T>);
impl<T> Id<T> {
    pub fn new(idx: usize) -> Self {
        Self(idx, std::marker::PhantomData)
    }
}
#[derive(Component)]
pub enum Render {
    Sphere(Sphere),
}

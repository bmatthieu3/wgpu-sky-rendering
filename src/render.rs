
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Sphere {
    pub c: [f32; 3],
    pub r: f32
}

use crate::{
    ecs,
    physics::Physics,
    world::Game
};
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

use ecs::System;
pub struct RenderSystem;
impl System for RenderSystem {
    fn run(&self, game: &mut Game, _: &std::time::Duration) {
        let world = &mut game.world;
        
        // Looping over the renderable objects
        let mut spheres = vec![];
        for (physic, render) in world.query_mut::<(Physics, Render)>() {
            let p = &physic.p;

            match render {
                Render::Sphere(s) => {
                    s.c = [p.x as f32, p.y as f32, p.z as f32];
                    spheres.push(*s);
                },
                _ => todo!()
            }
        }
        game.spheres_uniform.write(&game.queue, &spheres);
    }
}
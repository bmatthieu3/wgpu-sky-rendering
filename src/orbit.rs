use autodiff::Float;
pub enum OrbitData<T>
where
    T: Float
{
    Elliptical {
        // semi-major axis
        a: T,
        // eccentricity
        e: T,
        // argument of periapis,
        w: T
    },
    Circular {
        // radius of the orbit 
        R: T,
        // argument of the ascending node
        w: T
    },
    Parabolic, // TODO
    Hyperbolic, // TODO
}

use super::{
    math::{
        Vec3,
        resolve_numerically
    },
    core_engine::Component,
    ecs,
};

/* Physical constants */
// Constant of gravitation (G)
const G: f64 = 	6.67259e-11;
// Acceleration of gravity (g)
const g: f64 = 	9.80665; // m.s-2
// Astronomical unit (AU)
const AU: f64 = 149597870.0; // km
// Mass of Sun
const m_sun: f64 = 1.9891e30; // kg
// Mass of Earth
const m_earth: f64 = 5.9737e24; // kg
// Mass of Moon
const m_moon: f64 = 7.348e22; // kg

use ecs::Entity;
#[derive(Component)]
pub enum Physics {
    Orbit {
        // body around which orbiting
        primary_body: Entity,
        // position vector
        p: Vec3<f64>,
        // velocity magnitude
        v: f64,
        // true anomaly (angular distance of the body past the point of periapsis) in radians
        nu: f64,
        // distance to its primary body
        r: f64,
        // mu = m * G product of the body
        mu: f64,
        // zenith angle (angle between position and the velocity vector)
        gamma: f64,
        // orbit data
        data: OrbitData<f64>,
    },
    // The body is static
    Static {
        // position vector
        p: Vec3<f64>,
        // mu = m * G product of the body
        mu: f64,
    }
}

impl Physics {
    pub fn orbit(primary_body: Entity, mu: f64, data: OrbitData<f64>) -> Self {
        Self::Orbit {
            primary_body,
            mu,
            p: Vec3::zero(),
            v: 0.0,
            nu: 0.0,
            r: 0.0,
            gamma: 0.0,
            data
        }
    }

    pub fn static_object(mu: f64, p: Vec3<f64>) -> Self {
        Self::Static {
            p,
            mu,
        }
    }
}

use std::time;
use autodiff::*;
use ecs::System;
use crate::{
    render::Render,
    world::Game
};
pub struct UpdatePhysicsSystem;
impl System for UpdatePhysicsSystem {
    fn run(&self, game: &mut Game, t: &std::time::Duration) {
        let t = t.as_secs_f64();
        let mut world = &mut game.world;
        let prim_bodies_physics = world.query::<(Physics, Render)>()
            .map(|(physic, _)| {
                match physic {
                    Physics::Orbit { primary_body, ..} => {
                        let physic_prim_body = primary_body.get::<Physics>(world).unwrap();
                        match physic_prim_body {
                            &Physics::Orbit { p, mu, ..} => (mu, p),
                            &Physics::Static { p, mu } => (mu, p)
                        }
                    },
                    Physics::Static { p, mu} => {
                        // no primary body found
                        (*mu, *p)
                    }
                }
            })
            .collect::<Vec<_>>();

        // 1. TODO: change the orbital data if thrust power is applied
        // 2. Looping over the physics components to update the satellite positions.
        let mut spheres = vec![];
        for (idx, (physic, render)) in world.query_mut::<(Physics, Render)>().enumerate() {
            match physic {
                Physics::Orbit { p, gamma, v, nu, r, data, .. } => {
                    match *data {
                        OrbitData::Elliptical { a, e, w} => {
                            let E0 = 0.0;
                            let M0 = E0 - e*E0.sin();
                            let t0 = 0.0;
        
                            // mean motion
                            // Get the mu of the body around which it is orbiting
                            let (mu_prim_body, pos_prim_body) = prim_bodies_physics[idx];
                            dbg!(pos_prim_body);
                            let n = (mu_prim_body/(a*a*a)).sqrt();
            
                            let M = M0 + n * (t - t0);
                            let kep_eq = |x: F1| { x - e*x.sin() - M };
            
                            // Newton's method to compute e from the kepler equation
                            let err = 1e-5;
                            let E = resolve_numerically(M, &kep_eq, err);
            
                            // true anomaly
                            *nu = 2.0*((E*0.5).tan() * ((1.0+e)/(1.0-e)).sqrt()).atan();
                            dbg!(*nu);
                            let nu_c = (*nu).cos();
                            let nu_s = (*nu).sin();
                            // Update:
                            // 1. the position of the satellite
                            *r = a*(1.0 - e*e)/(1.0 + e*nu_c);
                            // 2. the zenith angle
                            let flight_path_angle = (e*nu_s/(1.0 + e*nu_c)).atan();
                            *gamma = std::f64::consts::PI/2.0 - flight_path_angle;
                            // 3. the magnitude of velocity
                            *v = (mu_prim_body*((2.0/(*r)) - 1.0/a)).sqrt();
            
                            // compute longitude
                            let x = a*E.cos();
                            let y = (*r)*nu_s;
                            let z = 0.0; // In the equator plane
            
                            *p = dbg!(Vec3::new(x, y, z) + pos_prim_body);
                            match render {
                                Render::Sphere(s) => {
                                    s.c = [p.x as f32, p.y as f32, p.z as f32];
                                    spheres.push(*s);
                                },
                                _ => unimplemented!()
                            }
                        },
                        _ => unimplemented!()
                    }
                },
                Physics::Static { .. } => (),
                _ => unimplemented!()
            }
        }
        
        game.spheres_uniform.write(&game.queue, &dbg!(spheres));
    }
}
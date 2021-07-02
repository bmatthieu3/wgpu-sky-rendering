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

#[derive(Component)]
pub struct Physics {
    // position vector
    p: Vec3<f64>,
    // velocity magnitude
    v: f64,
    // true anomaly (angular distance of the body past the point of periapsis) in radians
    nu: f64,
    // distance to its primary body
    r: f64,
    // zenith angle (angle between position and the velocity vector)
    gamma: f64, 
    // orbit data
    orbit: OrbitData<f64>,
}

impl Physics {
    pub fn new(orbit: OrbitData<f64>) -> Self {
        Self {
            orbit,
            p: Vec3::zero(),
            v: 0.0,
            nu: 0.0,
            r: 0.0,
            gamma: 0.0,
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
        // 1. TODO: change the orbital data if thrust power is applied
        // 2. Looping over the physics components to update the satellite positions.
        //let mut spheres = game.spheres_uniform.get_mut();
        for (p, r) in world.query_mut::<(Physics, Render)>() {
            match p.orbit {
                OrbitData::Elliptical { a, e, w} => {
                    let E0 = 0.0;
                    let M0 = E0 - e*E0.sin();
                    let t0 = 0.0;
    
                    let mu_earth = m_earth*1000.0*G;
                    // mean motion
                    let n = (mu_earth/(a*a*a)).sqrt();
    
                    let M = M0 + n * (t - t0);
                    let kep_eq = |x: F1| { x - e*x.sin() - M };
    
                    // Newton's method to compute e from the kepler equation
                    let err = 1e-5;
                    let E = resolve_numerically(M, &kep_eq, err);
    
                    // true anomaly
                    p.nu = ((E.cos() - e)/(1.0 - e*E.cos())).acos();
    
                    // Update:
                    // 1. the position of the satellite
                    p.r = a*(1.0 - e*e)/(1.0 + e*p.nu.cos());
                    // 2. the zenith angle
                    let flight_path_angle = (e*p.nu.sin()/(1.0 + e*p.nu.cos())).atan();
                    p.gamma = std::f64::consts::PI/2.0 - flight_path_angle;
                    // 3. the magnitude of velocity
                    p.v = (mu_earth*((2.0/p.r) - 1.0/a)).sqrt();
    
                    // compute longitude
                    let x = a*E.cos();
                    let y = p.r*p.nu.sin();
                    let z = 0.0; // In the equator plane
    
                    p.p = dbg!(Vec3::new(x, y, z));

                    /*match r {
                        Render::Sphere(idx) => {
                            get_mut()
                        },
                        _ => unimplemented!()
                    }*/
                },
                _ => unimplemented!()
            }
        }

        //game.spheres_uniform
    }
}
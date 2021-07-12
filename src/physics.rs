/* Physical constants */

use std::borrow::BorrowMut;

use super::{
    math::{
        Vec3,
    },
    core_engine::Component,
    ecs,
};

use cgmath::Zero;
#[derive(Debug, Clone)]
#[derive(Component)]
pub struct Physics {
    // position vector
    pub p: Vec3<f32>,
    // velocity vector
    pub v: Vec3<f32>,
    // mu = m * G product of the body
    pub mu: f32,
    // a flag to tell if the physics has been updated
    // during the current frame
    pub has_moved: bool,
}

impl Physics {
    pub fn new_static(p: &Vec3<f32>, mu: f32) -> Self {
        Self {
            p: *p,
            v: Vec3::zero(),
            mu,
            has_moved: false,
        }
    }
}

impl Default for Physics {
    fn default() -> Self {
        Self {
            p: Vec3::zero(),
            v: Vec3::zero(),
            mu: 1.0,
            has_moved: false,
        }
    }
}

use ecs::System;
use crate::{input::KeyId, orbit::OrbitData, render::Render, world::Game};

use crate::orbit::Orbit;
pub struct UpdatePhysicsSystem;
impl System for UpdatePhysicsSystem {
    fn run(&self, game: &mut Game, t: &std::time::Instant) {
        println!("run physic system");
        let world = &mut game.world;

        // 1. Reset the physics has_moved flag
        for physics in world.query_mut::<Physics>() {
            physics.has_moved = false;
        }

        /*// 2. Update entities being in orbit
        for (physics, orbit) in world.query_mut::<(Physics, Orbit)>() {
            orbit.update(t, physics);
        }*/
    } 
}

pub struct SpacecraftCommandSystem;
impl System for SpacecraftCommandSystem {
    fn run(&self, game: &mut Game, t: &std::time::Instant) {
        let world = game.world.clone();
        let spacecraft = &mut game.spacecraft;
        if game.input.is_key_pressed(&KeyId::Up) {
            println!("key up pressed");

            if let (Some(p), Some(o)) = (spacecraft.get_mut::<Physics>(&mut world.clone()), spacecraft.get_mut::<Orbit>(&mut world.clone())) {
                p.v *= 1.01;
                //o.set_velocity(&p.v);
            }
        }
    }
}

#[derive(Component)]
pub struct PowerCommand {
    // A force vector
    F: Vec3<f64>
}

/*// Constant of gravitation (G)
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

use autodiff::Float;
#[derive(Debug, Clone)]
pub enum OrbitData<T>
where
    T: Float
{
    Elliptical {
        // semi-major axis
        a: T,
        // eccentricity
        e: T,
    },
    Circular {
        // radius of the orbit 
        r: T,
    },
    Hyperbolic {
        // semi-major axis < 0
        a: T,
        // eccentricity
        e: T,
    }
}

use crate::ecs::Entity;
#[derive(Debug, Clone)]
pub struct Orbit {
    prim_body: Entity,
    // position vector
    p: Vec3<f64>,
    // velocity direction vector
    pub d: Vec3<f64>,
    // orbit data
    data: OrbitData<f64>,
    // time of creation of the orbit
    t0: std::time::Duration,
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

    // radius of the influence sphere
    rp: f64,

    // Eccentric anomaly E0
    e0: f64,
}

use cgmath::InnerSpace;
use itertools::Position;
use crate::math::Rotation;
impl Orbit {
    // Define a orbit from:
    // - the mass (or product G*M of the star: mu)
    // - the orbit data
    pub fn from_orbital_geometry(
        // World
        world: &ecs::World,
        // Reference body entity
        prim_body: Entity,
        // Mass of the orbiting body
        mu: f64,
        // Trajectory of the orbiting body around the reference body
        data: OrbitData<f64>,
        time: &std::time::Duration
    ) -> Self {
        let mut orbit = Self {
            prim_body,
            mu,
            p: Vec3::zero(),
            d: Vec3::zero(),
            nu: 0.0,
            r: 0.0,
            v: 0.0,
            gamma: 0.0,
            data,
            rp: 0.0,
            e0: 0.0,
            t0: *time,
        };

        // Set the object at the periapsis of its orbit
        // periapsis <=> nu = 0.0

        orbit.set_true_anomaly(world, 0.0);

        orbit
    }

    // Define an orbit from:
    // - a primary body characteristics
    // - the position of the satellite
    // - the velocity of the satellite
    // - the mass (mu) of the satellite
    //
    // This method is valid only if self is under the influence
    // of prim_body. This is true when the distance between the
    // satellite and prim_body is less than the sphere influence radius
    // of prim_body.
    pub fn from_physical_characteristics(
        // World for retrieving reference body characteristics
        world: &ecs::World,
        // Reference body entity
        prim_body: Entity,
        // Position of the orbiting body
        p: &Vec3<f64>,
        // Velocity of the orbiting body
        v: &Vec3<f64>,
        // Mass of the orbiting body
        mu: f64,
        time: &std::time::Duration,
    ) -> Self {
        let physics_r = prim_body.get::<Physics>(world).unwrap();
        let MG = physics_r.mu;
        let pr = &physics_r.p;

        let r = (p - pr).magnitude();

        let gamma = p.angle(&v);
        let v = v.magnitude();
        // The spacecraft/planet can escape the gravitational attranction of its primary body
        // This occurs when the object is launched at a speed Vesc >= sqrt(2*mu/r)
        let v_esc = (2.0*MG/r).sqrt();
        let mut orbit = if v >= v_esc {
            // Hyperbolic orbit type
            println!("hyperbola trajectory");

            // Compute:
            // 1. The semi-major axis a
            let a = 1.0/((2.0/r) - v*v/MG);

            // 2. The angle momentum
            let gamma_c = gamma.cos();
            let gamma_s = gamma.sin();

            let h = r*v*gamma_s;
            // 3. The true anomaly
            let nu = v*gamma_c*h/((h*h/r) - MG);
            // 4. The eccentricity
            let e = v*gamma_c*h/(MG*nu.sin());

            let data = OrbitData::Hyperbolic { a, e };

            // 5. Eccentric anomaly
            let nu_c = nu.cos();
            let e0 = ((e + nu_c)/(1.0 + e*nu_c)).acosh();

            Self {
                prim_body,
                mu,
                nu,
                r,
                v,
                gamma,
                data,

                // This will be updated
                p: Vec3::zero(),
                d: Vec3::zero(),
                rp: 0.0,
                t0: *time,
                e0
            }
            
        } else {
            // Elliptical orbit type
            println!("Elliptical trajectory");

            let c = r*v*v/MG;
            let gamma_c = gamma.cos();
            let gamma_s = gamma.sin();

            // 1. True anomaly (angle from periapsis to the point)
            let nu = (c*gamma_c*gamma_s/(c*gamma_s*gamma_s - 1.0)).atan();

            // 2. Eccentricity
            let cst = c - 1.0;
            let e = (cst*cst*gamma_s*gamma_s + gamma_c*gamma_c).sqrt();
            // 3. Semi-major axis
            let a = 1.0/((2.0/r) - v*v/MG);

            // 4. Elliptical orbit trajectory
            let data = OrbitData::Elliptical { a, e };

            // 5. Eccentric anomaly
            let nu_c = nu.cos();
            let e0 = ((e + nu_c)/(1.0 + e*nu_c)).acos();

            Self {
                prim_body,
                mu,
                nu,
                r,
                v,
                gamma,
                data,

                // This will be updated
                p: Vec3::zero(),
                d: Vec3::zero(),
                rp: 0.0,
                t0: *time,
                e0
            }
        };

        // Update:
        // - The position vector
        // - The velocity vector
        orbit.update_physic_data(pr);

        // - The sphere of influence of the body
        orbit.rp = orbit.r * (mu/MG).pow(0.4);
        orbit
    }

    // At a given zenith angle and position, set the velocity
    pub fn set_velocity(&mut self, world: &ecs::World, v: f64) {
        // The spacecraft/planet can escape the gravitational attraction of its primary body
        // This occurs when the object is launched at a speed Vesc >= sqrt(2*mu/r)
        let MG = self.prim_body.get::<Physics>(world)
            .unwrap()
            .mu;

        let v_esc = (2.0*MG/self.r).sqrt();
        let nu = if v >= v_esc {
            // Hyperbolic orbit type
            println!("hyperbola trajectory");
            // Compute:
            // 1. The semi-major axis a
            let a = 1.0/((2.0/self.r) - v*v/MG);

            // 2. The angle momentum
            let gamma_c = self.gamma.cos();
            let gamma_s = self.gamma.sin();

            let h = self.r*v*gamma_s;
            // 3. The true anomaly
            let nu = v*gamma_c*h/((h*h/self.r) - MG);
            // 4. the eccentricity
            let e = v*gamma_c*h/(MG*nu.sin());
            // Hyperbolic orbit trajectory
            self.data = OrbitData::Hyperbolic { a, e };

            nu
        } else {
            // Elliptical orbit type
            println!("elliptical trajectory");

            let c = self.r*v*v/MG;
            let gc = self.gamma.cos();
            let gs = self.gamma.sin();

            // Compute true anomaly (angle from periapsis to the point)
            let nu = (c*gc*gs/(c*gs*gs - 1.0)).atan();

            // Ellipse eccentricity
            let cst = c - 1.0;
            let e = (cst*cst*gs*gs + gc*gc).sqrt();
            // Ellipse semi-major axis
            let a = 1.0/((2.0/self.r) - v*v/MG);
            // Elliptical orbit trajectory
            self.data = OrbitData::Elliptical { a, e };

            nu
        };

        // Set the true anomaly
        self.set_true_anomaly(world, nu);
    }

    // Update the orbit with regards to the time elapsed
    //
    // Pass as extra parameters:
    // - The origin, i.e. the origin position from which the orbit is computed
    // - The mass of the origin
    fn update(&mut self, world: &ecs::World, time: &std::time::Duration) {
        let MG = self.prim_body.get::<Physics>(world)
            .unwrap()
            .mu;

        let t = time.as_secs_f64();
        let nu = match self.data {
            OrbitData::Elliptical { a, e} => {
                // mean motion
                // Get the mu of the body around which it is orbiting
                let n = (MG/(a*a*a)).sqrt();
                self.e0 = 0.0;
                let m0 = self.e0 - e*self.e0.sin();
                let t0 = self.t0.as_secs_f64() * 0.0;
                let m = m0 + n * (t - t0);
                let kep_eq = |x: F1| { x - e*x.sin() - m };

                // Newton's method to compute e from the kepler equation
                let err = 1e-5;
                let e_nu = resolve_numerically(m, &kep_eq, err);

                // True anomaly
                2.0*((e_nu*0.5).tan() * ((1.0+e)/(1.0-e)).sqrt()).atan()
            },
            OrbitData::Circular { r , .. } => {
                let n = (MG/(r*r*r)).sqrt();
                let m0 = self.e0;
                let t0 = self.t0.as_secs_f64();
                m0 + n * (t - t0)
            },
            OrbitData::Hyperbolic { a, e} => {
                todo!()
            },
        };

        self.set_true_anomaly(world, nu);
    }

    pub fn set_true_anomaly(&mut self, world: &ecs::World, nu: f64) {
        let (MG, origin) = {
            let p = self.prim_body.get::<Physics>(world).unwrap();
            (p.mu, &p.p)
        };

        self.nu = nu;

        let nu_c = nu.cos();
        let nu_s = nu.sin();

        match self.data {
            OrbitData::Elliptical { a, e } => {
                // Update:
                // 1. the position of the satellite
                self.r = a*(1.0 - e*e)/(1.0 + e*nu_c);
                // 2. the zenith angle
                let flight_path_angle = (e*nu_s/(1.0 + e*nu_c)).atan();
                self.gamma = std::f64::consts::PI/2.0 - flight_path_angle;
                // 3. the magnitude of velocity
                self.v = (MG*((2.0/self.r) - 1.0/a)).sqrt();
                // 4. eccentric anomaly
                self.e0 = ((e + nu_c)/(1.0 + e*nu_c)).acos();
            },
            OrbitData::Circular { r , .. } => {
                // Update:
                // 1. the position of the satellite
                self.r = r;
                // 2. the zenith angle
                self.gamma = std::f64::consts::PI/2.0;
                // 3. the magnitude of velocity
                self.v = (MG/self.r).sqrt();
                // 4. eccentric anomaly
                self.e0 = self.nu;
            },
            OrbitData::Hyperbolic { a, e, .. } => {
                /*// Update:
                // 1. the position of the satellite
                self.r = a*(1.0 - e*e)/(e*nu_c + 1.0);
                // 2. the zenith angle
                let flight_path_angle = (e*nu_s/(1.0 + e*nu_c)).atan();
                self.gamma = std::f64::consts::PI/2.0 - flight_path_angle;
                // 3. the magnitude of velocity
                self.v = (MG*((2.0/self.r) - 1.0/a)).sqrt();
                // 4. eccentric anomaly
                self.e0 = ((e + nu_c)/(1.0 + e*nu_c)).acosh();*/
                unreachable!();
            }
        }

        // Update:
        // - The position vector
        // - The velocity vector
        self.update_physic_data(origin);

        // - The sphere of influence of the body
        self.rp = self.r * (self.mu/MG).pow(0.4);
    }

    fn update_physic_data(&mut self, origin: &Vec3<f64>) {
        let nu_c = self.nu.cos();
        let nu_s = self.nu.sin();

        let p = match self.data {
            OrbitData::Elliptical { a, e} => {
                let E_c = (e + nu_c)/(1.0 + e*nu_c);
                let z = a*E_c;
                let x = self.r*nu_s;
                let y = 0.0; // In the equator plane

                Vec3::new(x, y, z)
            },
            OrbitData::Circular { r } => {
                let z = r * nu_c;
                let x = r * nu_s;
                let y = 0.0;

                Vec3::new(x, y, z)
            },
            OrbitData::Hyperbolic { a, e, ..} => {
                let z = self.r * nu_c;
                let x = self.r * nu_s;
                let y = 0.0;

                Vec3::new(x, y, z)
            }
        };

        // Position
        self.p = p + origin;
        // Velocity
        let er = p.normalize();
        self.d = er.rotate(self.gamma, Vec3::unit_y());
    }
}

#[derive(Debug, Clone)]
pub enum State {
    // Describe an entity in orbit relatively to another entity
    Orbit(Orbit),
    // Describe an entity moving in space with no gravitional forces
    // from any other entities
    Escaped {
        // The mass characteristic of the object
        mu: f64,
        // Its position
        p: Vec3<f64>,
        // Velocity magnitude
        v: Vec3<f64>,
    }
}

pub struct PhysicState {
    // The position
    pub p: Vec3<f64>,
    // The velocity
    pub v: Vec3<f64>,
    // The mass,
    pub mu: f64
}

use cgmath::num_traits::Pow;
impl State {
    /*fn create_from_position_velocity(world: &ecs::World, p: &Vec3<f64>, v: &Vec3<f64>, mu: f64) -> State {
        let mut prim_body = None;
        let mut min_rp2 = std::f64::MAX;
        for (entity, physic_r) in world.query_with_entity::<Physics>() {
            match physic_r {
                Physics::Static { .. } => (),
                Physics::Dynamic(State::Escaped { .. }) => (),
                Physics::Dynamic(State::Orbit(o)) => {
                    let d2 = (physic_r.get_position() - *p).magnitude2();
                    let rp2 = o.rp*o.rp;

                    if d2 < rp2 {
                        // self is under the influence of p
                        if rp2 < min_rp2 {
                            min_rp2 = rp2;
                            prim_body = Some(entity);
                        }
                    }
                }
            }
        }

        if let Some(prim_body) = prim_body {
            State::Orbit(
                Orbit::from_physical_characteristics(world, prim_body, p, v, mu)
            )
        } else {
            State::Escaped { p: *p, v: *v, mu }
        }
    }*/

    pub fn get_characteristics(&self) -> PhysicState {
        match self {
            State::Escaped { p, v, mu } => PhysicState { p: *p, v: *v, mu: *mu },
            State::Orbit(o) => PhysicState {p: o.p, v: o.d*o.v, mu: o.mu}
        }
    }
}

use autodiff::*;
use ecs::System;
use crate::{
    render::Render,
    world::Game
};*/
/*
pub struct UpdatePhysicsSystem;
impl System for UpdatePhysicsSystem {
    fn run(&self, game: &mut Game, t: &std::time::Duration) {
        let world = &mut game.world;
        println!("run physic system");

        // 1. Retrieve if it is under the influence of a primary body
        let mut prim_bodies = vec![];
        for (e1, ph1) in world.query_with_entity::<Physics>() {
            match ph1 {
                // The body is static, we will not move it so
                // nothing to do here
                Physics::Static { .. } => (),
                // If it is dynamic it can be either in:
                // - Escaped mode. But it can enter a region where it is influenced
                //   by a new body, so we must check for it
                // - Orbit mode. But it can exits the region of influence of its primary body, acquire
                //   another one or remain as it is.
                Physics::Dynamic(s1) => {
                    let PhysicState { p: p1, mu, .. } = s1.get_characteristics();
                    // New primary body entity
                    let mut prim_body = None;
                    let mut min_rp2 = std::f64::MAX;

                    for (e2, ph2) in world.query_with_entity::<Physics>() {
                        if e1 != e2 {
                            match ph2 {
                                Physics::Static { p: p2, .. } => {
                                    let rp2 = (p1 - *p2).magnitude2();
                                    if rp2 < min_rp2 {
                                        min_rp2 = rp2;
                                        prim_body = Some(e2);
                                    }
                                },
                                Physics::Dynamic(State::Escaped { .. }) => println!("sdfkjsfd"),
                                Physics::Dynamic(State::Orbit(o)) => {
                                    //dbg!(e1, e2);
                                    //dbg!(p1);
                                    let d2 = ((p1 - o.p).magnitude2());
                                    let rp2 = (o.rp*o.rp);
                
                                    if d2 < rp2 {
                                        // self is under the influence of p
                                        if rp2 < min_rp2 {
                                            min_rp2 = rp2;
                                            prim_body = Some(e2);
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // 2. Once we know if e1 is under the
                    // influence of a primary body or not
                    // we can change the state
                    prim_bodies.push((e1, prim_body));
                }
            }
        }

        // 2. Change the state of the entities having a physic component
        // prim_bodies stores the couple of physical entities with their primary body
        for (e1, e2) in prim_bodies.into_iter() {
            let ph1 = e1.get::<Physics>(world)
                .unwrap();
            let mut new_phy = if let Some(prim_body) = e2 {
                match ph1 {
                    Physics::Static { .. } => unreachable!(),
                    Physics::Dynamic(State::Escaped { p, v, mu }) => {
                        panic!();
                        Physics::Dynamic(
                            State::Orbit(
                                Orbit::from_physical_characteristics(world, prim_body, p, v, *mu, t)
                            )
                        )
                    },
                    Physics::Dynamic(State::Orbit(o)) => {
                        if o.prim_body != prim_body {
                            // The primary body has changed!
                            dbg!(e1, prim_body);
                            Physics::Dynamic(
                                State::Orbit(
                                    Orbit::from_physical_characteristics(world, o.prim_body, &o.p, &(o.d*o.v), o.mu, t)
                                )
                            )
                        } else {
                            ph1.clone()
                        }
                        //ph1.clone()
                    }
                }
            } else {
                // No primary associated
                match ph1 {
                    Physics::Static { .. } => unreachable!(),
                    // It is in escaped mode
                    Physics::Dynamic(State::Escaped { .. } ) => {
                        ph1.clone()
                    },
                    // The body did exit the sphere influence of its primary body
                    Physics::Dynamic(State::Orbit(o)) => {
                        // The primary body has changed!
                        panic!("escaped");
                        Physics::Dynamic(
                            State::Escaped { p: o.p, v: o.d*o.v, mu: o.mu }
                        )
                    }
                }
            };
            
            // update with time
            match &mut new_phy {
               // We do nothing
               Physics::Static { .. } => (),
               // It is in escaped mode. We will apply basic newton
               Physics::Dynamic(State::Escaped { .. } ) => (),
               // The body is in an orbit around another body
               // we apply the kepler equations
               Physics::Dynamic(State::Orbit(o)) => {
                   println!("in orbit");
                   let pr = o.prim_body.get::<Physics>(world).unwrap();
                   o.update(pr.get_mu(), pr.get_position(), t);
               }
            }

            e1.set::<Physics>(world, new_phy);
        }
    }
}

*/
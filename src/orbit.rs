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

use autodiff::{F1, Float};
use num_traits::Pow;
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

use crate::{ecs::Entity, physics::Physics};
use crate::shared::Shared;
use crate::ecs::World;
use cgmath::Rad;
#[derive(Debug)]
#[derive(Component)]
pub struct Orbit {
    world: Shared<World>,
    // The entity to which the object is orbiting
    primary_body: Entity,
    // longitude of the ascending node
    big_omega: Rad<f32>,
    // argument of the periapsis
    omega: Rad<f32>,
    // inclinaison
    i: Rad<f32>,
    // semi major axis
    a: f32,
    // time of periapsis passage
    tau: f32,
    // eccentricity
    e: f32,

    // Motion period
    T: f32,
    // n
    n: f32,
    // normal to the elliptical plane
    h: Vec3<f32>
}

use crate::world::Game;
use ecs::System;
pub struct UpdateInOrbitObjectsSystem;
impl System for UpdateInOrbitObjectsSystem {
    fn run(&self, game: &mut Game, t: &std::time::Instant) {
        for (physics, orbit) in game.world.clone().query_mut::<(Physics, Orbit)>() {
            // See 4.67 to 4.70 equation to calculate the true anomaly (noted theta)
            // as a function of the time elapsed
            // https://farside.ph.utexas.edu/teaching/celestial/Celestial/node33.html
            let M = orbit.n * (t.elapsed().as_secs_f32() - orbit.tau);

            // Newton's method to compute E from the kepler equation
            let kep_eq = |x: F1| { x - orbit.e*x.sin() - M };
            let E = resolve_numerically(M as f64, &kep_eq, 1e-4) as f32;
            let r = orbit.a * (1.0 - orbit.e * E.cos());

            // True anomaly
            let theta = Rad(
                2.0 * (((1.0 + orbit.e)/(1.0 - orbit.e)).sqrt() * (E * 0.5).tan()).atan()
            );

            // Flight path angle
            let flight_path_angle = (orbit.e*theta.sin()/(1.0 + orbit.e*theta.cos())).atan();
            let gamma = std::f32::consts::PI/2.0 - flight_path_angle;

            // Velocity
            let prim_body_physic = orbit.primary_body.get::<Physics>(&game.world).unwrap();
            let v = (prim_body_physic.mu*((2.0/r) - 1.0/orbit.a)).sqrt();

            // Conversion to cartesian coordinate system
            physics.p = PolarCoo { orbit: &orbit, r, theta }.into();

            let er = (physics.p - prim_body_physic.p).normalize();
            // TODO: test the direction of the velocity vector
            physics.v = er.rotate(gamma, orbit.h) * v;
            physics.has_moved = true;
        }
    }
}

struct PolarCoo<'a> {
    // Orbit reference
    orbit: &'a Orbit,
    // Radius
    r: f32,
    // Theta
    theta: Rad<f32>
}

type CartesianCoo = Vec3<f32>;
use cgmath::Angle;
impl<'a> From<PolarCoo<'a>> for CartesianCoo {
    fn from(c: PolarCoo<'a>) -> CartesianCoo {
        // 4.72 -> 4.74 https://farside.ph.utexas.edu/teaching/celestial/Celestial/node34.html
        let b_omega_c = c.orbit.big_omega.cos();
        let b_omega_s = c.orbit.big_omega.sin();

        let omega_theta_c = (c.theta + c.orbit.omega).cos();
        let omega_theta_s = (c.theta + c.orbit.omega).sin();

        let I_c = c.orbit.i.cos();
        let I_s = c.orbit.i.sin();

        let world = &c.orbit.world;
        let origin = &c.orbit.primary_body.get::<Physics>(world)
            .unwrap()
            .p;

        origin + Vec3::new(
            c.r * (b_omega_s * omega_theta_c + b_omega_c * omega_theta_s * I_c),
            c.r * omega_theta_s * I_s,
            c.r * (b_omega_c * omega_theta_c - b_omega_s * omega_theta_s * I_c)
        )
    }
}

use cgmath::InnerSpace;
use crate::math::Rotation;
impl Orbit {
    pub fn new(
        world: Shared<World>,
        // The entity to which the object is orbiting
        primary_body: Entity,
        // longitude of the ascending node
        big_omega: Rad<f32>,
        // argument of the periapsis
        omega: Rad<f32>,
        // inclinaison
        i: Rad<f32>,
        // semi major axis
        a: f32,
        // time of periapsis passage
        tau: f32,
        // eccentricity
        e: f32,
    ) -> Self {
        let MG = primary_body.get::<Physics>(&world).unwrap().mu;
        // Period of motion
        let T = 2.0 * std::f32::consts::PI * (a*a*a/MG).sqrt();
        // n
        let n = 2.0 * std::f32::consts::PI / T;

        // normal to the elliptical plane
        let h = Vec3::new(
            -i.sin()*big_omega.cos(),
            i.cos(),
            i.sin()*big_omega.sin()
        );

        Orbit { 
            world,
            primary_body,
            big_omega,
            omega,
            i,
            a,
            tau,
            e,
            T,
            n,
            h
        }
    }
}
/*
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
        world: Shared<ecs::World>,
        // Reference body entity
        prim_body: Entity,
        // Position of the orbiting body
        p: &Vec3<f64>,
        // Velocity of the orbiting body
        v: &Vec3<f64>,
        // Mass of the orbiting body
        //mu: f64,
        time: &std::time::Duration,
    ) -> Self {
        let physics_r = prim_body.get::<Physics>(&world).unwrap();
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
                world,
                prim_body,
                nu,
                r,
                gamma,
                data,

                // This will be updated
                //rp: 0.0,
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
                world,
                prim_body,
                nu,
                r,
                gamma,
                data,

                // This will be updated
                //rp: 0.0,
                t0: *time,
                e0
            }
        };

        // Update:
        // - The position vector
        // - The velocity vector
        //orbit.update_physic_data(pr);

        // - The sphere of influence of the body
        //orbit.rp = orbit.r * (mu/MG).pow(0.4);
        orbit
    }

    // At a given zenith angle and position, set the velocity
    pub fn set_velocity(&mut self, v: &Vec3<f64>) {
        // The spacecraft/planet can escape the gravitational attraction of its primary body
        // This occurs when the object is launched at a speed Vesc >= sqrt(2*mu/r)
        let MG = self.prim_body.get::<Physics>(&self.world)
            .unwrap()
            .mu;

        let v = v.magnitude();
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
        self.set_true_anomaly(nu);
    }

    // Update the orbit with regards to the time elapsed
    //
    // Pass as extra parameters:
    // - The origin, i.e. the origin position from which the orbit is computed
    // - The mass of the origin
    pub fn update(&mut self, time: &std::time::Duration, physics: &mut Physics) {
        let MG = self.prim_body.get::<Physics>(&self.world)
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

        self.set_true_anomaly(nu);

        let nu_c = self.nu.cos();
        let nu_s = self.nu.sin();

        let (pos_primary_space, v) = match self.data {
            OrbitData::Elliptical { a, e} => {
                let E_c = (e + nu_c)/(1.0 + e*nu_c);
                let z = a*E_c;
                let x = self.r*nu_s;
                let y = 0.0; // In the equator plane

                let v = (MG*((2.0/self.r) - 1.0/a)).sqrt();

                (Vec3::new(x, y, z), v)
            },
            OrbitData::Circular { r } => {
                let z = r * nu_c;
                let x = r * nu_s;
                let y = 0.0;

                let v = (MG/self.r).sqrt();

                (Vec3::new(x, y, z), v)
            },
            OrbitData::Hyperbolic { a, e, ..} => {
                let z = self.r * nu_c;
                let x = self.r * nu_s;
                let y = 0.0;

                let v = (MG*((2.0/self.r) - 1.0/a)).sqrt();

                (Vec3::new(x, y, z), v)
            }
        };

        // Position
        physics.p = pos_primary_space + self.prim_body.get::<Physics>(&self.world).unwrap().p;
        // Velocity
        let er = pos_primary_space.normalize();
        //physics.v = er.rotate(self.gamma, Vec3::unit_y())*v;
        physics.v = er * v;
        physics.has_moved = true;
    }

    pub fn set_true_anomaly(&mut self, nu: f64) {
        let (MG, origin) = {
            let p = self.prim_body.get::<Physics>(&self.world).unwrap();
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
                //self.v = (MG*((2.0/self.r) - 1.0/a)).sqrt();
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
                //self.v = (MG/self.r).sqrt();
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
        //self.update_physic_data(origin);

        // - The sphere of influence of the body
        //self.rp = self.r * (self.mu/MG).pow(0.4);
    }

    /*fn update_physic_data(&mut self, origin: &Vec3<f64>) {
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
    }*/

    // Given a true anomaly, return the position of the vertex
    fn get_position(&self, nu: f64) -> Vec3<f64> {
        let (MG, origin) = {
            let p = self.prim_body.get::<Physics>(&self.world).unwrap();
            (p.mu, &p.p)
        };

        let nu_c = nu.cos();
        let nu_s = nu.sin();

        let (pos_primary_space, v) = match self.data {
            OrbitData::Elliptical { a, e} => {
                let E_c = (e + nu_c)/(1.0 + e*nu_c);
                let z = a*E_c;
                let x = self.r*nu_s;
                let y = 0.0; // In the equator plane

                let v = (MG*((2.0/self.r) - 1.0/a)).sqrt();

                (Vec3::new(x, y, z), v)
            },
            OrbitData::Circular { r } => {
                let z = r * nu_c;
                let x = r * nu_s;
                let y = 0.0;

                let v = (MG/self.r).sqrt();

                (Vec3::new(x, y, z), v)
            },
            OrbitData::Hyperbolic { a, e, ..} => {
                let z = self.r * nu_c;
                let x = self.r * nu_s;
                let y = 0.0;

                let v = (MG*((2.0/self.r) - 1.0/a)).sqrt();

                (Vec3::new(x, y, z), v)
            }
        };
    }
}

*/
/*use ecs::System;
use crate::world::Game;
use crate::camera::Camera;
pub struct OrbitPlotting;
impl System for OrbitPlotting {
    fn run(&self, game: &mut Game, t: &std::time::Duration) {
        // Loop over the camera to get the current active one
        for (camera, physics) in game.world.query::<(Camera, Physics)>() {
            if camera.active && physics.has_moved {
                // If the active camera has moved
                println!("camera has moved");

                /*for (physics, orbit) in game.world.query::<(Physics, Orbit)>() {
                    // project the orbit to the screen
                    let step = 0.1;
                    let mut nu = 0.0;
                    while nu < 2.0*std::f32::consts::PI {
                        orbit.get_position(nu);

                        thetnua += step;                        
                    }
                }*/
            }
        }
    } 
}*/

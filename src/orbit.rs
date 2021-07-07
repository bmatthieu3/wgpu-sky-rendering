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
#[derive(Debug)]
#[derive(Component)]
pub struct Orbit {
    world: Shared<World>,

    pub prim_body: Entity,
    // orbit data
    data: OrbitData<f64>,
    // time of creation of the orbit
    t0: std::time::Duration,
    // true anomaly (angular distance of the body past the point of periapsis) in radians
    nu: f64,
    // distance to its primary body
    r: f64,
    // zenith angle (angle between position and the velocity vector)
    gamma: f64,

    // radius of the influence sphere
    //rp: f64,

    // Eccentric anomaly E0
    e0: f64,
}

use cgmath::InnerSpace;
use crate::math::Rotation;
impl Orbit {
    // Define a orbit from:
    // - the mass (or product G*M of the star: mu)
    // - the orbit data
    pub fn from_orbital_geometry(
        // World
        world: Shared<ecs::World>,
        // Reference body entity
        prim_body: Entity,
        // Trajectory of the orbiting body around the reference body
        data: OrbitData<f64>,
        time: &std::time::Duration
    ) -> Self {
        let mut orbit = Self {
            world,
            prim_body,
            nu: 0.0,
            r: 0.0,
            gamma: 0.0,
            data,
            //rp: 0.0,
            e0: 0.0,
            t0: *time,
        };

        // Set the object at the periapsis of its orbit
        // periapsis <=> nu = 0.0

        orbit.set_true_anomaly(0.0);

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
}

use ecs::System;
use crate::world::Game;
/*pub struct UpdateOrbitPosition;
impl System for UpdateOrbitPosition {
    fn run(&self, game: &mut Game, t: &std::time::Duration) {
        let world = &mut game.world;

        for orbit in world.query_mut::<Orbit>() {
            orbit.update(t);

            //e.set::<Orbit>(world, orbit);
        }
        println!("run physic system");
    } 
}*/

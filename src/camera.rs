use core_engine::Component;
use crate::{ecs, math::{Mat4, Vec2, Vec3, extract_direction}, physics::Physics, projection::{Gnomonic, Ortho, Projection}, world::Game};

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct CameraData {
    // Matrix computed from the center of screen position on the sky
    pub dir_mat: Mat4<f32>,
    // Origin of the camera,
    pub origin: Vec3<f32>,
    // Dir lookup of the camera
    pub dir: Vec3<f32>,
}

#[derive(Component)]
pub struct Camera {
    // The data that are send to the GPU
    pub data: CameraData,
    // Active boolean
    pub active: bool,
}

impl Camera {
    fn swap_active_camera(&mut self, active_camera: &mut Self) {
        std::mem::swap(&mut self.active, &mut active_camera.active);
    }
}

use cgmath::{Point3, SquareMatrix};
impl Default for CameraData {
    fn default() -> Self {
        Self {
            dir_mat: Mat4::identity(),
            origin: Vec3::new(0.0, 0.0, 0.0),
            dir: Vec3::unit_z(),
        }
    }
}

// This system will update the position of all the camera
//
// It will also upload to the queue the active matrix
pub struct CameraUpdatePositionSystem;
use cgmath::Zero;

use cgmath::InnerSpace;
use ecs::System;
use crate::math::Rotation;
use crate::math::rotation_from_direction;
impl System for CameraUpdatePositionSystem {
    fn run(&self, game: &mut Game, _: &std::time::Instant) {
        for (physic, camera) in game.world.clone().query_mut::<(Physics, Camera)>() {
            // If the physics attached to the camera has moved, change the camera origin
            if physic.has_moved {
                let pos = &physic.p;
                camera.data.origin = Vec3::new(pos.x as f32, pos.y as f32, pos.z as f32);
            }

            // Send the current active camera to the bound shader
            if camera.active {
                game.skybox_camera_uniform.write(&game.queue, 0, &camera.data);

                let po = Point3::new(
                    camera.data.origin.x,
                    camera.data.origin.y,
                    camera.data.origin.z
                );
                //let dir = game.spacecraft.get::<Physics>(&game.world.clone()).unwrap().v.normalize();
                let pd = po + camera.data.dir;
                game.camera_uniform.write(&game.queue, 0, &(crate::render::OPENGL_TO_WGPU_MATRIX * Mat4::look_at(po, Point3::new(0.0, 0.0, 0.0), Vec3::unit_y())));
            }
        }
    }
}
pub struct CameraSpacecraftSystem;
impl System for CameraSpacecraftSystem {
    fn run(&self, game: &mut Game, _: &std::time::Instant) {
        let spacecraft = &mut game.spacecraft;
        let dir = &spacecraft.get::<Physics>(&game.world).unwrap().v;

        // When the spacecraft moves dynamically (orbit or escaping)
        let spacecraft_dir = Vec3::new(
            dir.x as f32,
            dir.y as f32,
            dir.z as f32,
        ).normalize();
        let mut world = game.world.clone();
        let camera = spacecraft.get_mut::<Camera>(&mut world).unwrap();

        // retrieve the last mouse position from the game state
        let input = &game.input;
        let size = &game.size;

        let (pos_ss_x, pos_ss_y) = input.mouse;

        camera.data.dir = Vec3::new(0.0, 0.0, 0.0);

        let mut rot_along_x = Mat4::identity();
        let mut rot_along_y = Mat4::identity();

        // Rotation along the x screen axis
        let pos_cs = Vec2::new(
            2.0*(pos_ss_x/(size.width as f32)) - 1.0, 
            0.0
        );

        if let Some(pos_ws) = Gnomonic::clip_to_world_space(&pos_cs) {
            let pos_ws = pos_ws
                .truncate()
                .normalize();
            // compute the angle between the center of projection and the cursor position
            // this defines the offset angle of view
            let off_theta = Vec3::unit_z()
                .angle(pos_ws).0 * 2.0;
            let off_theta = if pos_cs.x > 0.0 {
                -off_theta
            } else {
                off_theta
            };

            let dir = Vec3::unit_z().rotate(0.1*off_theta, Vec3::unit_y());
            rot_along_x = rotation_from_direction(&dir, &Vec3::new(0.0, 1.0, 0.0));
        }

        // Rotation along the y screen axis
        let pos_cs = Vec2::new(
            0.0, 
            -2.0*(pos_ss_y/(size.height as f32)) + 1.0
        );

        if let Some(pos_ws) = Gnomonic::clip_to_world_space(&pos_cs) {
            let pos_ws = pos_ws
                .truncate()
                .normalize();
            // compute the angle between the center of projection and the cursor position
            // this defines the offset angle of view
            let off_theta = Vec3::unit_z()
                .angle(pos_ws).0;
            let off_theta = if pos_cs.y > 0.0 {
                off_theta
            } else {
                -off_theta
            };

            let dir = Vec3::unit_z().rotate(0.1*off_theta, Vec3::unit_x());
            rot_along_y = rotation_from_direction(&dir, &Vec3::new(0.0, 1.0, 0.0));
        }

        //camera.data.dir = Vec3::unit_z();
        camera.data.dir_mat = rotation_from_direction(&spacecraft_dir, &Vec3::new(0.0, 1.0, 0.0)) * rot_along_x * rot_along_y;
        camera.data.dir = extract_direction(&camera.data.dir_mat).normalize();
    }
}
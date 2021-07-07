use crate::math::{Mat4, Vec2};
use winit::{dpi::PhysicalPosition, event::{
        Event,
        ElementState,
        WindowEvent,
        VirtualKeyCode, KeyboardInput
    }, event_loop::ControlFlow};

use crate::world::Game;

pub type KeyId = VirtualKeyCode;

pub struct KeyParam {
    // instant of when the key has been first pressed
    // None when the key is released
    time_pressed: Option<std::time::Instant>,
    // aa flag telling if the key is pressed
    pressed: bool,
    // a flag telling if it is the first frame that
    // the key is pressed
    // At the next frames, it will be set to false
    triggered: bool,
}

impl Default for KeyParam {
    // By default, all is set to 0
    fn default() -> Self {
        Self {
            time_pressed: None,
            pressed: false,
            triggered: false,
        }
    }
}

use std::collections::HashMap;
pub struct InputGameState {
    // Last position of the mouse
    pub mouse: (f32, f32),
    // A set of keys to check for
    pub keys: HashMap<KeyId, KeyParam>,
}

use crate::physics::Physics;
use crate::camera::Camera;

use cgmath::InnerSpace;
use crate::Gnomonic;
use crate::projection::Projection;
use crate::math::*;
use std::time;
impl InputGameState {
    pub fn new() -> Self {
        let keys = [
                KeyId::Escape,
                KeyId::Up,
                KeyId::Down
            ]
            .iter()
            .map(|&kcode| {
                (kcode, KeyParam::default())
            })
            .collect();

        /*actions.insert(
            Input::Mouse,
            Data::Mouse {
                action: |game: &mut Game, pos| {
                    let w = game.size.width as f32;
                    let h = game.size.height as f32;
                    let pos_cs = Vec2::new(
                        2.0*(pos.0/w) - 1.0, 
                        0.0
                    );
                    if let Some(pos_ws) = Gnomonic::clip_to_world_space(&pos_cs) {
                        let spacecraft_dir = if let Physics::Orbit(orbit) = game.spacecraft.get::<Physics>(&game.world)
                            .unwrap() {
                                orbit.d
                            } else {
                                unreachable!();
                            };
                        let spacecraft_dir = Vec3::new(
                            spacecraft_dir.x as f32,
                            spacecraft_dir.y as f32,
                            spacecraft_dir.z as f32,
                        );
                        let camera = game.spacecraft.get_mut::<Camera>(&mut game.world).unwrap();

                        let pos_ws = pos_ws.truncate();
                        camera.dir = Vec3::new(
                            pos_ws.x,
                            pos_ws.y,
                            pos_ws.z
                        ).normalize();
                        // compute the angle between the center of projection and the cursor position
                        // this defines the offset angle of view
                        let off_theta = Vec3::unit_z()
                            .angle(camera.dir);
                        let off_theta = if pos_cs.x > 0.0 {
                            off_theta.0
                        } else {
                            -off_theta.0
                        };

                        camera.dir = spacecraft_dir.rotate(off_theta, Vec3::unit_y());
                        camera.dir_mat = rotation_from_direction(&camera.dir, &Vec3::new(0.0, 1.0, 0.0));
                        //camera.dir = Vec3::new(d.x as f32, d.y as f32, d.z as f32);

                        //camera.dir_mat = rotation_from_direction(&pos_ws, &Vec3::new(0.0, 1.0, 0.0));
                        //game.rot_mat_uniform.write(&game.queue, &rot);
                    }
                }
            }
        );*/

        let mouse = (0.0, 0.0);
        Self {
            mouse,
            keys,
        }
    }

    pub fn register_inputs(&mut self, event: &WindowEvent) {
        let input = match event {
            WindowEvent::KeyboardInput { input, .. } => match input {
                KeyboardInput {
                    state: ElementState::Pressed,
                    virtual_keycode,
                    ..
                } => {
                    if let Some(k) = self.keys.get_mut(virtual_keycode.as_ref().unwrap()) {
                        if !k.pressed {
                            k.pressed = true;
                            k.triggered = true;
                            k.time_pressed = Some(time::Instant::now());
                        } else {
                            k.triggered = false;
                        }
                    }
                },
                KeyboardInput {
                    state: ElementState::Released,
                    virtual_keycode,
                    ..
                } => {
                    if let Some(k) = self.keys.get_mut(virtual_keycode.as_ref().unwrap()) {
                        if k.pressed {
                            k.pressed = false;
                            k.triggered = false;
                            k.time_pressed = None;
                        }
                    }
                },
                _ => ()
            },
            WindowEvent::CursorMoved { position, .. } => {
                self.mouse = (position.x as f32, position.y as f32);
            },
            _ => (),
        };
    }

    pub fn is_key_pressed(&self, key: &KeyId) -> bool {
        if let Some(k) = self.keys.get(key) {
            k.pressed
        } else {
            false
        }
    }
}



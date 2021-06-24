use crate::math::{Mat4, Vec2};
use winit::{dpi::PhysicalPosition, event::{
        Event,
        ElementState,
        WindowEvent,
        VirtualKeyCode, KeyboardInput
    }, event_loop::ControlFlow};

use crate::world::World;

enum Data {
    Mouse {
        action: fn(&mut World, (f32, f32)) -> (),
    },
    Key {
        key: VirtualKeyCode,
        action: fn(&mut World, &mut ControlFlow) -> (),
    }
}

#[derive(PartialEq, Eq, Hash)]
enum Input {
    Mouse,
    Key(VirtualKeyCode)
}

use std::collections::HashMap;
pub struct CurrentInputFrame {
    mouse_pos: Option<Vec2<f64>>,
    actions: HashMap<Input, Data>,
}

use cgmath::InnerSpace;
use crate::Gnomonic;
use crate::projection::Projection;
use crate::math::*;
impl CurrentInputFrame {
    pub fn new(state: &mut World) -> Self {
        let mut actions = HashMap::new();
        actions.insert(
            Input::Key(VirtualKeyCode::Escape),
            Data::Key {
                key: VirtualKeyCode::Escape,
                action: |_: &mut World, control_flow: &mut ControlFlow| {
                    *control_flow = ControlFlow::Exit;
                    ()
                }
            }
        );

        actions.insert(
            Input::Key(VirtualKeyCode::Left),
            Data::Key {
                key: VirtualKeyCode::Left,
                action: |world: &mut World, _: &mut ControlFlow| {
                    world.id_proj += 1;
                    world.id_proj %= crate::world::NUM_PROJECTIONS;

                    world.set_projection();
                }
            }
        );

        actions.insert(
            Input::Mouse,
            Data::Mouse {
                action: |world: &mut World, pos| {
                    let w = world.size.width as f32;
                    let h = world.size.height as f32;
                    let pos_cs = Vec2::new(
                        2.0*(pos.0/w) - 1.0, 
                        -2.0*(pos.1/h) + 1.0
                    );
                    if let Some(pos_ws) = Gnomonic::clip_to_world_space(&pos_cs) {
                        let pos_ws = pos_ws.truncate().normalize();

                        let elapsed = world.clock.elapsed().as_secs_f32();

                        let rot = rotation_from_direction(&pos_ws, &Vec3::new(0.0, 1.0, 0.0));
                        let rot: &[[f32; 4]; 4] = rot.as_ref();
                        //dbg!(rot);
                        world.queue
                            .write_buffer(
                                &world.rot_mat_buf, 
                                0, 
                                bytemuck::bytes_of(rot)
                            );
                    }
                }
            }
        );

        CurrentInputFrame {
            mouse_pos: None,
            actions,
        }
    }

    pub fn register_frame_events(&self, world: &mut World, event: &WindowEvent, control_flow: &mut ControlFlow) {
        let mut mouse_pos = (0.0, 0.0);
        let input = match event {
            WindowEvent::KeyboardInput { input, .. } => match input {
                KeyboardInput {
                    state: ElementState::Pressed,
                    virtual_keycode,
                    ..
                } => {
                    Some(Input::Key(virtual_keycode.unwrap()))
                },
                _ => {
                    None
                }
            },
            WindowEvent::CursorMoved { position, .. } => {
                mouse_pos = (position.x as f32, position.y as f32);
                Some(Input::Mouse)
            },
            _ => None,
        };

        if let Some(input) = input {
            if let Some(data) = self.actions.get(&input) {
                match data {
                    Data::Key { action, .. } => action(world, control_flow),
                    Data::Mouse { 
                        action
                    } => action(world, mouse_pos),
                }
            }
        }
    }
}



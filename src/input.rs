use crate::math::{Mat4, Vec2};
use winit::{dpi::PhysicalPosition, event::{
        Event,
        ElementState,
        WindowEvent,
        VirtualKeyCode, KeyboardInput
    }, event_loop::ControlFlow};

use crate::world::Game;

enum Data {
    Mouse {
        action: fn(&mut Game, (f32, f32)) -> (),
    },
    Key {
        key: VirtualKeyCode,
        action: fn(&mut Game, &mut ControlFlow) -> (),
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
    pub fn new(state: &mut Game) -> Self {
        let mut actions = HashMap::new();
        actions.insert(
            Input::Key(VirtualKeyCode::Escape),
            Data::Key {
                key: VirtualKeyCode::Escape,
                action: |_: &mut Game, control_flow: &mut ControlFlow| {
                    *control_flow = ControlFlow::Exit;
                    ()
                }
            }
        );

        actions.insert(
            Input::Key(VirtualKeyCode::Left),
            Data::Key {
                key: VirtualKeyCode::Left,
                action: |world: &mut Game, _: &mut ControlFlow| {
                    todo!();
                }
            }
        );

        actions.insert(
            Input::Mouse,
            Data::Mouse {
                action: |world: &mut Game, pos| {
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
                        world.rot_mat_uniform.write(&world.queue, &rot);
                    }
                }
            }
        );

        CurrentInputFrame {
            mouse_pos: None,
            actions,
        }
    }

    pub fn register_frame_events(&self, world: &mut Game, event: &WindowEvent, control_flow: &mut ControlFlow) {
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



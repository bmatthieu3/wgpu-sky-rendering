use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

use ::core_engine;

mod texture;
mod world;
mod input;
mod vertex;
mod angle;
mod math;
mod projection;
mod triangulation;
mod uniform;
mod render;
pub mod ecs;

use crate::world::Game;
use crate::projection::*;
use crate::input::CurrentInputFrame;


mod orbit;
use ecs::SystemManager;
use orbit::UpdatePhysicsSystem;
fn main() {
    env_logger::init();
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("allsky projections")
        .build(&event_loop).unwrap();

    use futures::executor::block_on;
    // Since main can't be async, we're going to need to block
    let mut game = block_on(Game::new(&window));
    let mut systems = SystemManager::new();
    systems.register_system(UpdatePhysicsSystem);
    let mut input = CurrentInputFrame::new(&mut game);
    #[cfg(target_arch = "wasm32")]
    {
        use winit::platform::web::WindowExtWebSys;

        let canvas = window.canvas();

        let window = web_sys::window().unwrap();
        let document = window.document().unwrap();
        let body = document.body().unwrap();

        body.append_child(&canvas)
            .expect("Append canvas to HTML body");
    }

    let mut count: i32 = 5;

    event_loop.run(move |event, _, control_flow| {
        match event {
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == window.id() => {
                input.register_frame_events(&mut game, event, control_flow);
                if !game.input(event) {
                    match event {
                        WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                        WindowEvent::Resized(physical_size) => match count {
                            0 => game.resize::<Aitoff>(*physical_size),
                            1 => game.resize::<Ortho>(*physical_size),
                            2 => game.resize::<Mollweide>(*physical_size),
                            3 => game.resize::<Mercator>(*physical_size),
                            4 => game.resize::<AzimuthalEquidistant>(*physical_size),
                            5 => game.resize::<Gnomonic>(*physical_size),
                            _ => unimplemented!(),
                        },
                        WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                            // new_inner_size is &mut so w have to dereference it twice
                            match count {
                                0 => game.resize::<Aitoff>(**new_inner_size),
                                1 => game.resize::<Ortho>(**new_inner_size),
                                2 => game.resize::<Mollweide>(**new_inner_size),
                                3 => game.resize::<Mercator>(**new_inner_size),
                                4 => game.resize::<AzimuthalEquidistant>(**new_inner_size),
                                5 => game.resize::<Gnomonic>(**new_inner_size),
                                _ => unimplemented!(),
                            }
                        }
                        _ => {}
                    }
                }
            }
            Event::RedrawRequested(_) => {
                game.update(&mut systems);
                match game.render() {
                    Ok(_) => {}
                    // Recreate the swap_chain if lost
                    Err(wgpu::SwapChainError::Lost) => match count {
                        0 => game.resize::<Aitoff>(game.size),
                        1 => game.resize::<Ortho>(game.size),
                        2 => game.resize::<Mollweide>(game.size),
                        3 => game.resize::<Mercator>(game.size),
                        4 => game.resize::<AzimuthalEquidistant>(game.size),
                        5 => game.resize::<Gnomonic>(game.size),
                        _ => unimplemented!(),
                    },
                    // The system is out of memory, we should probably quit
                    Err(wgpu::SwapChainError::OutOfMemory) => *control_flow = ControlFlow::Exit,
                    // All other errors (Outdated, Timeout) should be resolved by the next frame
                    Err(e) => eprintln!("{:?}", e),
                }
            }
            Event::MainEventsCleared => {
                // RedrawRequested will only trigger once, unless we manually
                // request it.
                window.request_redraw();
            }
            _ => {}
        }
    });
}

#[cfg(target_arch = "wasm32")]
mod wasm {
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen(start)]
    pub fn run() {
        console_log::init_with_level(log::Level::Debug);

        super::main();
    }
}
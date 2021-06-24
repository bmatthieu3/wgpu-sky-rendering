use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

mod texture;
mod world;
mod input;
mod vertex;
mod angle;
mod math;
mod projection;
mod triangulation;
mod ecs;

use crate::world::World;
use crate::projection::*;
use crate::input::CurrentInputFrame;
fn main() {
    env_logger::init();
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("allsky projections")
        .build(&event_loop).unwrap();

    use futures::executor::block_on;
    // Since main can't be async, we're going to need to block
    let mut world = block_on(World::new(&window));
    let mut input = CurrentInputFrame::new(&mut world);
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
                input.register_frame_events(&mut world, event, control_flow);
                if !world.input(event) {
                    match event {
                        WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                        WindowEvent::Resized(physical_size) => match count {
                            0 => world.resize::<Aitoff>(*physical_size),
                            1 => world.resize::<Ortho>(*physical_size),
                            2 => world.resize::<Mollweide>(*physical_size),
                            3 => world.resize::<Mercator>(*physical_size),
                            4 => world.resize::<AzimuthalEquidistant>(*physical_size),
                            5 => world.resize::<Gnomonic>(*physical_size),
                            _ => unimplemented!(),
                        },
                        WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                            // new_inner_size is &mut so w have to dereference it twice
                            match count {
                                0 => world.resize::<Aitoff>(**new_inner_size),
                                1 => world.resize::<Ortho>(**new_inner_size),
                                2 => world.resize::<Mollweide>(**new_inner_size),
                                3 => world.resize::<Mercator>(**new_inner_size),
                                4 => world.resize::<AzimuthalEquidistant>(**new_inner_size),
                                5 => world.resize::<Gnomonic>(**new_inner_size),
                                _ => unimplemented!(),
                            }
                        }
                        _ => {}
                    }
                }
            }
            Event::RedrawRequested(_) => {
                world.update();
                match world.render() {
                    Ok(_) => {}
                    // Recreate the swap_chain if lost
                    Err(wgpu::SwapChainError::Lost) => match count {
                        0 => world.resize::<Aitoff>(world.size),
                        1 => world.resize::<Ortho>(world.size),
                        2 => world.resize::<Mollweide>(world.size),
                        3 => world.resize::<Mercator>(world.size),
                        4 => world.resize::<AzimuthalEquidistant>(world.size),
                        5 => world.resize::<Gnomonic>(world.size),
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

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
mod camera;
mod orbit;
mod physics;
mod shared;
mod pipelines;
mod resources;

use crate::world::Game;
use crate::projection::*;

use ecs::SystemManager;
use physics::UpdatePhysicsSystem;
use render::RenderingSystem;
use camera::{
    CameraUpdatePositionSystem,
    CameraSpacecraftSystem,
};

use orbit::UpdateInOrbitObjectsSystem;

use physics::SpacecraftCommandSystem;
use input::KeyId;
use winit::window::Fullscreen;
fn main() {
    env_logger::init();
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("allsky projections")
        .with_inner_size(winit::dpi::Size::Physical(winit::dpi::PhysicalSize::<u32>::new(1024, 768)))
        //.with_fullscreen(Some(Fullscreen::Borderless(None)))
        .build(&event_loop).unwrap();

    use futures::executor::block_on;
    // Since main can't be async, we're going to need to block
    let mut game = block_on(Game::new(&window));
    let mut systems = SystemManager::new();
    systems.register_system(SpacecraftCommandSystem);

    systems.register_system(UpdatePhysicsSystem);
    systems.register_system(UpdateInOrbitObjectsSystem);

    systems.register_system(CameraSpacecraftSystem);
    systems.register_system(CameraUpdatePositionSystem);

    systems.register_system(RenderingSystem);

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

    event_loop.run(move |event, _, control_flow| {
        match event {
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == window.id() => {
                game.register_inputs(event);
                // Check for the escape key pressed
                let inputs = &game.input;
                if inputs.is_key_pressed(&KeyId::Escape) {
                    *control_flow = ControlFlow::Exit;
                }
                // Check for other type event i.e. CloseRequested, Resized...
                match event {
                    WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                    WindowEvent::Resized(physical_size) => {
                        game.resize::<Gnomonic>(*physical_size);
                    },
                    WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                        game.resize::<Gnomonic>(**new_inner_size);
                    }
                    _ => {}
                }
            }
            Event::RedrawRequested(_) => {
                game.update(&mut systems);
                match game.render() {
                    Ok(_) => {}
                    // Recreate the swap_chain if lost
                    Err(wgpu::SwapChainError::Lost) => {
                        game.resize::<Gnomonic>(game.size);
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
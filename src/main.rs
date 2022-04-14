use anyhow::Result;

use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};
use std::{time::{
    Duration,
    SystemTime,
}, alloc::System};

mod renderer;
use renderer::Renderer;

fn main() -> Result<()> {
    pollster::block_on(run())?;
    Ok(())
}

async fn run() -> Result<()> {
    env_logger::init();
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("Interactive Fractal Viewer")
        .build(&event_loop)?;
    
    let mut renderer = Renderer::new(
        &window,
        None,
        None
    ).await?;
   
    let mut last_time: SystemTime = SystemTime::now();
    let mut dt: Duration = last_time.elapsed().unwrap();

    event_loop.run(move |event, _, control_flow|
        match event {
            Event::WindowEvent { ref event, window_id, } if window_id == window.id() => {
                if !renderer.input(&window, event) {
                    match event {
                        WindowEvent::CloseRequested | WindowEvent::KeyboardInput {
                            input:
                                KeyboardInput {
                                    state: ElementState::Pressed,
                                    virtual_keycode: Some(VirtualKeyCode::Escape),
                                    ..
                                },
                            ..
                        } => *control_flow = ControlFlow::Exit,
                        WindowEvent::Resized(physical_size) => {
                            renderer.resize(*physical_size);
                        }
                        WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                            // new_inner_size is &&mut so we have to dereference it twice
                            renderer.resize(**new_inner_size);
                        }
                        _ => {}
                    }
                }
            },
            Event::RedrawRequested(window_id) if window_id == window.id() => {
                dt = last_time.elapsed().unwrap();
                last_time = SystemTime::now();
                renderer.update(&dt);
                match renderer.render() {
                    Ok(_) => {}
                    // The system is out of memory, we should probably quit
                    Err(wgpu::SurfaceError::OutOfMemory) => *control_flow = ControlFlow::Exit,
                    // All other errors (Outdated, Timeout) should be resolved by the next frame
                    Err(e) => eprintln!("{:?}", e),
                }
            },
            Event::MainEventsCleared => {
                // RedrawRequested will only trigger once, unless we manually
                // request it.
                window.request_redraw();
            },
            _ => {}
        }
    );
}
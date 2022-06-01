#![feature(optimize_attribute)]

pub mod data;
pub mod gfx;

use gfx::{debugdraw::RenderTest, do_frame};
use winit::{event::*, event_loop::ControlFlow};

pub struct WindowState {
    pub window: Option<winit::window::Window>,
    pub event_loop: Option<winit::event_loop::EventLoop<()>>,
}

fn handle_result<T>(result: Result<T, Box<dyn std::error::Error>>) {
    match result {
        Err(e) => {
            #[cfg(target_family = "wasm")]
            {
                use web_sys::console;
                unsafe {
                    console::error_1(&format!("Error: {}", e).into());
                    if e.source().is_some() {
                        console::error_1(&format!("Caused by: {}", e.source().unwrap()).into());
                    }
                }
            }

            #[cfg(not(target_family = "wasm"))]
            {
                println!("Error: {}", e);
                if e.source().is_some() {
                    println!("Caused by: {}", e.source().unwrap());
                }
            }

            panic!()
        }
        Ok(result) => result,
    };
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let (mut gfx_state, window_state) = gfx::init().await?;

    let window = window_state.window.unwrap();
    let event_loop = window_state.event_loop.unwrap();

    let mut render_test = RenderTest::default();
    render_test.prep(&gfx_state).await?;

    event_loop.run(move |event, _, control_flow| {
        handle_result(|| -> Result<(), Box<dyn std::error::Error>> {
            match event {
                Event::MainEventsCleared => {
                    window.request_redraw();
                }
                Event::RedrawRequested(_) => do_frame(&mut gfx_state, |state, encoder| {
                    render_test.frame(state, encoder)
                })?,
                Event::WindowEvent {
                    ref event,
                    window_id,
                } if window_id == window.id() => match event {
                    WindowEvent::CloseRequested
                    | WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state: ElementState::Pressed,
                                virtual_keycode: Some(VirtualKeyCode::Escape),
                                ..
                            },
                        ..
                    } => *control_flow = ControlFlow::Exit,
                    _ => {}
                },
                _ => {}
            }
            Ok(())
        }());
    });
}

pub async fn actual_main() {
    handle_result(run().await)
}

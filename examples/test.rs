//! A demonstration of using `winit` to provide events and `vulkano` to draw the UI.
use rsmenu::renderer::Renderer;
use rsmenu::support::convert_event;

pub const WIN_W: u32 = 600;
pub const WIN_H: u32 = 420;

fn main() {
    let mut events_loop = winit::EventsLoop::new();

    let mut renderer = Renderer::new(&events_loop, "RSMenu", WIN_W, WIN_H);

    // Demonstration app state that we'll control with our conrod GUI.
    let mut app = rsmenu::ui::MenuApp::new(WIN_W, WIN_H);

    'main: loop {
        if let Some(primitives) = app.draw_if_changed() {
            renderer.render(primitives)
        }

        let mut should_quit = false;

        events_loop.poll_events(|event| {
            if let Some(event) = convert_event(event.clone(), &renderer.window) {
                app.handle_event(event);
            }

            // Close window if the escape key or the exit button is pressed
            match event {
                winit::Event::WindowEvent {
                    event:
                        winit::WindowEvent::KeyboardInput {
                            input:
                                winit::KeyboardInput {
                                    virtual_keycode: Some(winit::VirtualKeyCode::Escape),
                                    ..
                                },
                            ..
                        },
                    ..
                }
                | winit::Event::WindowEvent {
                    event: winit::WindowEvent::CloseRequested,
                    ..
                } => should_quit = true,
                _ => {}
            }
        });
        if should_quit {
            break 'main;
        }

        // Update widgets if any event has happened
        app.handle_global_input();
    }
}

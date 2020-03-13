use rsmenu::{Event, MenuApp, Renderer};
use winit::EventsLoop;

pub const WIN_W: u32 = 600;
pub const WIN_H: u32 = 420;

fn main() {
    let events_loop = EventsLoop::new();

    let mut renderer = Renderer::new(&events_loop, "RSMenu", WIN_W, WIN_H);

    let mut app = MenuApp::new(WIN_W, WIN_H, events_loop);

    loop {
        match app.main_loop(&mut renderer) {
            Event::Search(search) => {
                app.set_items(vec![format!("foo{}", search), format!("bar{}", search)])
            }
            Event::Continue => {}
            Event::Exit => break,
        }
    }
}

use rufi::{MenuApp, Renderer};
use tokio::time::Duration;
use winit::EventsLoop;

pub const WIN_W: u32 = 600;
pub const WIN_H: u32 = 420;

#[tokio::main]
async fn main() {
    let events_loop = EventsLoop::new();

    let renderer = Renderer::new(&events_loop, "Rufi Test", WIN_W, WIN_H);

    let app = MenuApp::new(WIN_W, WIN_H, events_loop);

    app.main_loop(renderer, |query| async move {
        tokio::time::delay_for(Duration::from_millis(100)).await; // debounce
        vec![
            format!("foo{}", query),
            format!("bar{}", query),
            format!("bar{}", query),
            format!("bar{}", query),
            format!("bar{}", query),
            format!("bar{}", query),
            format!("bar{}", query),
            format!("bar{}", query),
            format!("bar{}", query),
            format!("bar{}", query),
            format!("bar{}", query),
            format!("bar{}", query),
            format!("bar{}", query),
            format!("bar{}", query),
            format!("foo{}", query),
            format!("bar{}", query),
            format!("bar{}", query),
            format!("bar{}", query),
            format!("bar{}", query),
            format!("bar{}", query),
            format!("bar{}", query),
            format!("bar{}", query),
            format!("bar{}", query),
            format!("bar{}", query),
            format!("bar{}", query),
            format!("bar{}", query),
            format!("bar{}", query),
            format!("bar{}", query),
        ]
    })
    .await;
}

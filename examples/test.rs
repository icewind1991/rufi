use rufi::{MenuApp, Renderer};
use tokio::time::Duration;
use winit::EventsLoop;

pub const WIN_W: u32 = 600;

#[tokio::main]
async fn main() {
    let events_loop = EventsLoop::new();

    let renderer = Renderer::new(&events_loop, "Rufi Test", WIN_W);

    let app = MenuApp::new(WIN_W, events_loop);

    app.main_loop(renderer, |query| async move {
        tokio::time::delay_for(Duration::from_millis(100)).await; // debounce
        let mut acc = vec![];
        let mut result: Vec<String> = vec![];
        for char in query.chars() {
            acc.push(char);
            result.push(acc.clone().into_iter().collect());
        }

        result
    })
    .await;
}

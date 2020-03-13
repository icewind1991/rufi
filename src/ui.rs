use crate::renderer::Renderer;
use crate::support::convert_event;
use conrod_core::{widget_ids, Borderable, Ui};
use futures_util::future::{select, Either};
use futures_util::pin_mut;
use std::cmp::min;
use std::future::Future;
use tokio::macros::support::Pin;
use tokio::time::{self, Duration};

pub struct AppState {
    items: Vec<String>,
    selected: usize,
    search: String,
}

impl AppState {
    pub fn set_search(&mut self, search: String) {
        self.search = search
    }
}

#[derive(Debug, Clone)]
pub enum Event {
    Search(String),
    Continue,
    Exit,
}

/// A demonstration of some application state we want to control with a conrod GUI.
pub struct MenuApp<SearchFuture: Future<Output = Vec<String>>> {
    state: AppState,
    ids: Ids,
    ui: Ui,
    events_loop: winit::EventsLoop,
    state_updated: bool,
    search_future: Option<Pin<Box<SearchFuture>>>,
}

impl<SearchFuture: Future<Output = Vec<String>>> MenuApp<SearchFuture> {
    /// Simple constructor for the `DemoApp`.
    pub fn new(width: u32, height: u32, events_loop: winit::EventsLoop) -> Self {
        // Create Ui and Ids of widgets to instantiate
        let mut ui = conrod_core::UiBuilder::new([width as f64, height as f64])
            .theme(default_theme())
            .build();
        let ids = Ids::new(ui.widget_id_generator());

        // Load font from file
        let assets = find_folder::Search::KidsThenParents(3, 5)
            .for_folder("assets")
            .unwrap();
        let font_path = assets.join("fonts/NotoSans/NotoSans-Regular.ttf");
        ui.fonts.insert_from_file(font_path).unwrap();

        ui.keyboard_capture(ids.input);

        MenuApp {
            state: AppState {
                items: vec!["Foo".to_string(), "Bar".to_string()],
                selected: 0,
                search: String::from(""),
            },
            ids,
            ui,
            events_loop,
            state_updated: false,
            search_future: None,
        }
    }

    pub fn set_items(&mut self, items: Vec<String>) {
        self.state.items = items;
        self.state.selected = min(self.state.selected, self.state.items.len());
        self.state_updated = true;
    }

    pub async fn main_loop<Search>(mut self, mut renderer: Renderer, search: Search) -> ()
    where
        Search: Fn(String) -> SearchFuture,
    {
        let ui = &mut self.ui;

        let mut should_quit = false;

        let mut vsync = time::interval(Duration::from_millis(1000 / 60));

        loop {
            if let Some(primitives) = ui.draw_if_changed() {
                renderer.render(primitives)
            }

            self.events_loop.poll_events(|event| {
                if let Some(event) = convert_event(event.clone(), &renderer.window) {
                    ui.handle_event(event);
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
                return;
            } else {
                // Update widgets if any event has happened
                if ui.global_input().events().next().is_some() || self.state_updated {
                    let mut ui = ui.set_widgets();
                    self.state_updated = false;
                    if let Event::Search(query) = gui(&mut ui, &self.ids, &mut self.state) {
                        self.search_future = Some(Box::pin(search(query)));
                    }
                }
            }

            let tick = vsync.tick();
            pin_mut!(tick);

            match self.search_future.take() {
                Some(search_fut) => {
                    match select(tick, search_fut).await {
                        Either::Left((_, search_fut)) => self.search_future = Some(search_fut), // vsync before search completion
                        Either::Right((search_result, _)) => {
                            // search complete before vsync
                            self.state.items = search_result;
                            self.state.selected = 0;
                            self.state_updated = true;
                        }
                    }
                }
                None => {
                    tick.await;
                }
            }
        }
    }
}

/// A set of reasonable stylistic defaults that works for the `gui` below.
pub fn default_theme() -> conrod_core::Theme {
    use conrod_core::position::{Align, Direction, Padding, Position, Relative};
    conrod_core::Theme {
        name: "Default Theme".to_string(),
        padding: Padding::none(),
        x_position: Position::Relative(Relative::Align(Align::Start), None),
        y_position: Position::Relative(Relative::Direction(Direction::Backwards, 20.0), None),
        background_color: conrod_core::color::DARK_CHARCOAL,
        shape_color: conrod_core::color::CHARCOAL,
        border_color: conrod_core::color::BLACK,
        border_width: 0.0,
        label_color: conrod_core::color::WHITE,
        font_id: None,
        font_size_large: 26,
        font_size_medium: 18,
        font_size_small: 12,
        widget_styling: conrod_core::theme::StyleMap::default(),
        mouse_drag_threshold: 0.0,
        double_click_threshold: std::time::Duration::from_millis(500),
    }
}

// Generate a unique `WidgetId` for each widget.
widget_ids! {
    pub struct Ids {
        // The input field
        input,
        // The scrollable canvas with result
        canvas,
        // the list of results
        items,
    }
}

/// Instantiate a GUI demonstrating every widget available in conrod.
pub fn gui(ui: &mut conrod_core::UiCell, ids: &Ids, app: &mut AppState) -> Event {
    use conrod_core::{widget, Colorable, Labelable, Positionable, Sizeable, Widget};

    const MARGIN: conrod_core::Scalar = 2.0;
    const SUBTITLE_SIZE: conrod_core::FontSize = 32;
    widget::Canvas::new()
        .pad(MARGIN)
        .scroll_kids_vertically()
        .set(ids.canvas, ui);

    let search = widget::TextEdit::new(&app.search)
        .font_size(SUBTITLE_SIZE)
        .mid_top_of(ids.canvas)
        .h(38.0)
        .set(ids.input, ui);

    let (mut events, scrollbar) = widget::ListSelect::single(app.items.len())
        .flow_down()
        .item_size(30.0)
        .scrollbar_next_to()
        .w_h(400.0, 230.0)
        .align_middle_x_of(ids.canvas)
        .set(ids.items, ui);

    // Handle the `ListSelect`s events.
    while let Some(event) = events.next(ui, |i| i == app.selected) {
        use conrod_core::widget::list_select::Event;
        match event {
            // For the `Item` events we instantiate the `List`'s items.
            Event::Item(item) => {
                let label = &app.items[item.i];
                let color = match item.i == app.selected {
                    true => ui.theme.label_color,
                    false => ui.theme.background_color,
                };
                let button = widget::Button::new()
                    .border(0.0)
                    .color(color)
                    .label(label)
                    .label_font_size(20);
                item.set(button, ui);
            }

            // The selection has changed.
            Event::Selection(selection) => {
                app.selected = selection;
            }

            // The remaining events indicate interactions with the `ListSelect` widget.
            _event => {}
        }
    }

    // Instantiate the scrollbar for the list.
    if let Some(s) = scrollbar {
        s.set(ui);
    }

    match search {
        Some(search) => {
            app.set_search(search.clone());
            Event::Search(search)
        }
        None => Event::Continue,
    }
}

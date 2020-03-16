use crate::renderer::Renderer;
use crate::support::convert_event;
use conrod_core::position::{Place, Relative};
use conrod_core::text::FontCollection;
use conrod_core::{widget_ids, Borderable, Ui};
use futures_util::future::{select, Either};
use futures_util::pin_mut;
use std::cmp::min;
use std::fmt::Display;
use std::future::Future;
use tokio::time::{self, Duration};
use winit::ElementState;

pub struct AppState<Item: Display> {
    items: Vec<Item>,
    selected: usize,
    search: String,
}

impl<Item: Display> AppState<Item> {
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
pub struct MenuApp<Item: Display> {
    state: AppState<Item>,
    ids: Ids,
    ui: Ui,
    events_loop: winit::EventsLoop,
}

impl<Item: Display> MenuApp<Item> {
    /// Simple constructor for the `DemoApp`.
    pub fn new(width: u32, height: u32, events_loop: winit::EventsLoop) -> Self {
        // Create Ui and Ids of widgets to instantiate
        let mut ui = conrod_core::UiBuilder::new([width as f64, height as f64])
            .theme(default_theme())
            .build();
        let ids = Ids::new(ui.widget_id_generator());

        let font_collection = FontCollection::from_bytes(
            include_bytes!("../assets/fonts/NotoSans/NotoSans-Regular.ttf").to_vec(),
        )
        .unwrap();
        ui.fonts.insert(font_collection.into_font().unwrap());

        ui.keyboard_capture(ids.input);

        MenuApp {
            state: AppState {
                items: vec![],
                selected: 0,
                search: String::from(""),
            },
            ids,
            ui,
            events_loop,
        }
    }

    pub fn set_items(&mut self, items: Vec<Item>) {
        self.state.items = items;
        self.state.selected = 0;
    }

    pub async fn main_loop<Search, SearchFuture>(
        self,
        mut renderer: Renderer,
        search: Search,
    ) -> Option<Item>
    where
        Search: Fn(String) -> SearchFuture,
        SearchFuture: Future<Output = Vec<Item>>,
    {
        let mut should_quit = false;
        let mut result = None;

        let mut vsync = time::interval(Duration::from_millis(1000 / 60));

        let MenuApp {
            mut state,
            ids,
            mut ui,
            mut events_loop,
        } = self;

        let mut state_updated = false;

        let mut search_future = None;

        loop {
            if let Some(primitives) = ui.draw_if_changed() {
                renderer.render(primitives)
            }

            events_loop.poll_events(|event| {
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
                                        virtual_keycode,
                                        state: ElementState::Pressed,
                                        ..
                                    },
                                ..
                            },
                        ..
                    } => match virtual_keycode {
                        Some(winit::VirtualKeyCode::Return) => {
                            result = if state.items.len() > state.selected {
                                Some(state.items.remove(state.selected))
                            } else {
                                None
                            };
                            should_quit = true;
                        }
                        Some(winit::VirtualKeyCode::Escape) => should_quit = true,
                        Some(winit::VirtualKeyCode::Up) => {
                            state.selected = state.selected.saturating_sub(1);
                            state_updated = true;
                        }
                        Some(winit::VirtualKeyCode::Down) => {
                            state.selected =
                                min(state.selected + 1, state.items.len().saturating_sub(1));
                            state_updated = true;
                        }
                        _ => {}
                    },
                    winit::Event::WindowEvent {
                        event: winit::WindowEvent::CloseRequested,
                        ..
                    } => should_quit = true,
                    winit::Event::WindowEvent {
                        event: winit::WindowEvent::Focused(focused),
                        ..
                    } => should_quit = !focused,
                    _ => {}
                }
            });
            if should_quit {
                return result;
            } else {
                // Update widgets if any event has happened
                if ui.global_input().events().next().is_some() || state_updated {
                    let mut ui = ui.set_widgets();
                    state_updated = false;
                    let (height, event) = gui(&mut ui, &ids, &mut state);
                    if let Event::Search(query) = event {
                        search_future = Some(Box::pin(search(query)));
                    }
                }
            }

            let tick = vsync.tick();
            pin_mut!(tick);

            match search_future.take() {
                Some(search_fut) => {
                    match select(tick, search_fut).await {
                        Either::Left((_, search_fut)) => search_future = Some(search_fut), // vsync before search completion
                        Either::Right((search_result, _)) => {
                            // search complete before vsync
                            state.items = search_result;
                            state.selected = 0;
                            state_updated = true;
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
    use conrod_core::position::{Align, Direction, Padding, Position};
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
pub fn gui<Item: Display>(
    ui: &mut conrod_core::UiCell,
    ids: &Ids,
    app: &mut AppState<Item>,
) -> (u32, Event) {
    use conrod_core::{widget, Colorable, Labelable, Positionable, Sizeable, Widget};

    const MARGIN: conrod_core::Scalar = 2.0;
    const SUBTITLE_SIZE: conrod_core::FontSize = 16;
    widget::Canvas::new()
        .pad(MARGIN)
        .scroll_kids_vertically()
        .border(1.0)
        .border_color(ui.theme.label_color)
        .set(ids.canvas, ui);

    let search = widget::TextEdit::new(&app.search)
        .font_size(SUBTITLE_SIZE)
        .w_of(ids.canvas)
        .mid_top()
        .h(SUBTITLE_SIZE as f64 + 1.0)
        .set(ids.input, ui);

    let (mut events, scrollbar) = widget::ListSelect::single(app.items.len())
        .flow_down()
        .item_size(SUBTITLE_SIZE as f64 + 2.0)
        .scrollbar_next_to()
        .h(360.0)
        .mid_bottom()
        .kid_area_w_of(ids.canvas)
        .set(ids.items, ui);

    let height = SUBTITLE_SIZE * (app.items.len() as u32 + 1);

    // Handle the `ListSelect`s events.
    while let Some(event) = events.next(ui, |i| i == app.selected) {
        use conrod_core::widget::list_select::Event;
        match event {
            // For the `Item` events we instantiate the `List`'s items.
            Event::Item(item) => {
                let label = app.items[item.i].to_string();
                let color = match item.i == app.selected {
                    true => ui.theme.shape_color,
                    false => ui.theme.background_color,
                };
                let button = widget::Button::new()
                    .border(0.0)
                    .color(color)
                    .label(&label)
                    .left_justify_label()
                    .label_x(Relative::Place(Place::Start(None)))
                    .label_font_size(SUBTITLE_SIZE);
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

    (
        height,
        match search {
            Some(search) => {
                app.set_search(search.clone());
                Event::Search(search)
            }
            None => Event::Continue,
        },
    )
}

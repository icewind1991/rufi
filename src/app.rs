// use crate::renderer::Renderer;
// use crate::support::convert_event;
use conrod_core::position::{Place, Relative};
use conrod_core::text::FontCollection;
use conrod_core::{widget_ids, Borderable, Sizeable, Ui};
use std::cmp::min;
use std::fmt::Display;

use crate::renderer::Renderer;
use crate::window::convert_event;
use std::sync::mpsc::channel;
use std::time::Duration;
use winit::dpi::{LogicalSize, PhysicalPosition};
use winit::event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent};
use winit::{
    event_loop::{ControlFlow, EventLoop},
    platform::desktop::EventLoopExtDesktop,
};

pub const INITIAL_HEIGHT: u32 = 26;

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
pub enum AppEvent {
    Search(String),
    Continue,
    Exit,
}

/// A demonstration of some application state we want to control with a conrod GUI.
pub struct MenuApp<Item: Display + Send + 'static> {
    state: AppState<Item>,
    ids: Ids,
    ui: Ui,
    title: String,
}

impl<Item: Display + Send + 'static> MenuApp<Item> {
    /// Simple constructor for the `DemoApp`.
    pub fn new(width: u32, title: &str) -> Self {
        // Create Ui and Ids of widgets to instantiate
        let mut ui = conrod_core::UiBuilder::new([width as f64, INITIAL_HEIGHT as f64])
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
            title: title.to_string(),
        }
    }

    pub fn set_items(&mut self, items: Vec<Item>) {
        self.state.items = items;
        self.state.selected = 0;
    }

    pub fn main_loop<Search>(self, search: Search) -> Option<Item>
    where
        Search: Fn(String) -> Vec<Item> + Send + 'static,
    {
        let MenuApp {
            mut state,
            ids,
            mut ui,
            title,
        } = self;

        let mut event_loop = EventLoop::new();

        let window = winit::window::WindowBuilder::new()
            .with_title(&title)
            .with_inner_size(winit::dpi::LogicalSize {
                width: ui.win_w,
                height: ui.win_h,
            })
            .with_visible(false)
            .build(&event_loop)
            .unwrap();
        let mut renderer = Renderer::new(&window);
        let mut visible = false;

        let image_map = conrod_core::image::Map::new();

        let mut result = None;

        let (query_tx, query_rx) = channel();
        let (items_tx, items_rx) = channel();

        let event_proxy = event_loop.create_proxy();

        std::thread::spawn(move || {
            // first block for the first query
            while let Ok(mut query) = query_rx.recv() {
                // then wait until there is no new query set for some duration
                while let Ok(new_query) = query_rx.recv_timeout(Duration::from_millis(100)) {
                    query = new_query;
                }

                if let Err(_) = items_tx.send(search(query)) {
                    break;
                }

                // wakeup the event loop
                let _ = event_proxy.send_event(());
            }
        });

        let mut state_updated = false;

        event_loop.run_return(|event, _, control_flow| {
            if let Some(event) = convert_event(&event, &window) {
                ui.handle_event(event);
            }

            *control_flow = if cfg!(feature = "metal-auto-capture") {
                ControlFlow::Exit
            } else {
                ControlFlow::Wait
            };

            if let Ok(items) = items_rx.try_recv() {
                state.items = items;
                state_updated = true
            };

            match event {
                Event::MainEventsCleared => {
                    // Update widgets if any event has happened
                    if ui.global_input().events().next().is_some() || state_updated {
                        state_updated = false;
                        let mut ui = ui.set_widgets();
                        let (height, event) = gui(&mut ui, &ids, &mut state);
                        if let AppEvent::Search(query) = event {
                            if let Err(e) = query_tx.send(query) {
                                eprintln!("{}", e);
                            }
                        }

                        window.set_inner_size(LogicalSize::new(
                            window.inner_size().to_logical(window.scale_factor()).width,
                            height,
                        ));
                        window.request_redraw();
                    }
                }
                Event::RedrawRequested(_) => {
                    if let Some(primitives) = ui.draw_if_changed() {
                        renderer.render(primitives, &window, &image_map);
                    }

                    if !visible {
                        window.set_visible(true);
                        let monitor = window.primary_monitor();
                        let (x_pos, y_pos): (i32, i32) = monitor.position().into();
                        let (x_size, y_size): (u32, u32) = monitor.size().into();

                        let size = window.inner_size();
                        window.set_outer_position(PhysicalPosition::new(
                            x_pos + (x_size - size.width) as i32 / 2,
                            y_pos + (y_size - size.height) as i32 / 3,
                        ));
                        window.set_decorations(false);
                        visible = true;
                    }
                }
                Event::WindowEvent { event, .. } => match event {
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                virtual_keycode,
                                state: ElementState::Pressed,
                                ..
                            },
                        ..
                    } => match virtual_keycode {
                        Some(VirtualKeyCode::Return) => {
                            result = if state.items.len() > state.selected {
                                Some(state.items.remove(state.selected))
                            } else {
                                None
                            };
                            *control_flow = ControlFlow::Exit;
                        }
                        Some(VirtualKeyCode::Escape) => *control_flow = ControlFlow::Exit,
                        Some(VirtualKeyCode::Up) => {
                            state.selected = state.selected.saturating_sub(1);
                            state_updated = true;
                        }
                        Some(VirtualKeyCode::Down) => {
                            state.selected =
                                min(state.selected + 1, state.items.len().saturating_sub(1));
                            state_updated = true;
                        }
                        _ => {}
                    },
                    WindowEvent::CloseRequested | WindowEvent::Destroyed => {
                        *control_flow = ControlFlow::Exit
                    }
                    WindowEvent::Focused(focused) => {
                        if !focused {
                            *control_flow = ControlFlow::Exit;
                        }
                    }
                    WindowEvent::Resized(new_size) => {
                        renderer.resize(new_size);
                    }
                    WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                        renderer.resize(*new_inner_size);
                    }
                    _ => {}
                },
                _ => {}
            }
        });

        window.set_visible(false);

        result
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
) -> (u32, AppEvent) {
    use conrod_core::{widget, Colorable, Labelable, Positionable, Widget};

    const MARGIN: conrod_core::Scalar = 2.0;
    const SUBTITLE_SIZE: conrod_core::FontSize = 16;

    let item_size = SUBTITLE_SIZE + 2;
    let height = item_size * (app.items.len() as u32 + 1) + 4;

    widget::Canvas::new()
        .pad(MARGIN)
        .scroll_kids_vertically()
        .border(1.0)
        .border_color(ui.theme.label_color)
        .h(height as f64)
        .set(ids.canvas, ui);

    let search = widget::TextEdit::new(&app.search)
        .font_size(SUBTITLE_SIZE)
        .kid_area_w_of(ids.canvas)
        .mid_top()
        .h(SUBTITLE_SIZE as f64 + 1.0)
        .set(ids.input, ui);

    let (mut events, scrollbar) = widget::ListSelect::single(app.items.len())
        .flow_down()
        .item_size(SUBTITLE_SIZE as f64 + 2.0)
        .scrollbar_next_to()
        .h(item_size as f64 * (app.items.len() as f64))
        .mid_bottom()
        .kid_area_w_of(ids.canvas)
        .set(ids.items, ui);

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
                AppEvent::Search(search)
            }
            None => AppEvent::Continue,
        },
    )
}

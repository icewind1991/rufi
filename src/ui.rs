use conrod_core::widget::text_box::Event as TextBoxEvent;
use conrod_core::{widget_ids, Borderable, Ui};

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

/// A demonstration of some application state we want to control with a conrod GUI.
pub struct MenuApp {
    state: AppState,
    ids: Ids,
    ui: Ui,
}

impl MenuApp {
    /// Simple constructor for the `DemoApp`.
    pub fn new(width: u32, height: u32) -> Self {
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

        MenuApp {
            state: AppState {
                items: vec!["Foo".to_string(), "Bar".to_string()],
                selected: 0,
                search: String::from("Search..."),
            },
            ids,
            ui,
        }
    }

    pub fn draw_if_changed(&self) -> Option<conrod_core::render::Primitives> {
        self.ui.draw_if_changed()
    }

    pub fn handle_event(&mut self, event: conrod_core::event::Input) {
        self.ui.handle_event(event);
    }

    pub fn handle_global_input(&mut self) {
        if self.ui.global_input().events().next().is_some() {
            let mut ui = self.ui.set_widgets();
            gui(&mut ui, &self.ids, &mut self.state);
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
pub fn gui(ui: &mut conrod_core::UiCell, ids: &Ids, app: &mut AppState) {
    use conrod_core::{widget, Colorable, Labelable, Positionable, Sizeable, Widget};

    const MARGIN: conrod_core::Scalar = 30.0;
    const SUBTITLE_SIZE: conrod_core::FontSize = 32;
    widget::Canvas::new()
        .pad(MARGIN)
        .scroll_kids_vertically()
        .set(ids.canvas, ui);

    for event in widget::TextBox::new(&app.search)
        .font_size(SUBTITLE_SIZE)
        .mid_top_of(ids.canvas)
        .set(ids.input, ui)
    {
        match event {
            TextBoxEvent::Update(text) => app.set_search(text),
            _ => {}
        }
    }

    let (mut events, scrollbar) = widget::ListSelect::single(app.items.len())
        .flow_down()
        .item_size(30.0)
        .scrollbar_next_to()
        .w_h(400.0, 230.0)
        .mid_bottom_of(ids.canvas)
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
}

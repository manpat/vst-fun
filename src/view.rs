use conrod::backend::glium::{glium, glium::glutin, Renderer as GliumRenderer};
use vst::editor::Editor;

use conrod::widget_ids;
use conrod::color::hsl;
use conrod::utils::degrees;

use log::info;

use crate::model::Model;

use std::sync::Once;
static LOGGER_INIT: Once = Once::new();

const WINDOW_SIZE: (u32, u32) = (440, 260);

pub struct View {
    ui: conrod::Ui,
    ids: Ids,

    event_loop: glutin::EventsLoop,
    display: glium::Display,
    renderer: GliumRenderer,

    image_map: conrod::image::Map<glium::texture::Texture2d>,

    visible: bool,

    model: Model,
    view_state: ViewState,
}

impl View {
    pub fn new(init_state: Model) -> View {
        LOGGER_INIT.call_once(|| {
            use simplelog::*;
            use std::fs::File;
            use std::fs;

            std::env::set_var("RUST_BACKTRACE", "1");

            let log_dir = dirs::data_dir().unwrap().join("_manpat");

            fs::create_dir_all(&log_dir).unwrap();

            if let Ok(file) = File::create(log_dir.join("vst-gui.log")) {
                WriteLogger::init(
                    LevelFilter::Info,
                    Config::default(),
                    file
                ).unwrap();
            }

            info!("Logging enabled");
        });

        let event_loop = glutin::EventsLoop::new();
        let window = glutin::WindowBuilder::new()
            .with_dimensions(WINDOW_SIZE.into())
            .with_always_on_top(true)
            .with_resizable(false)
            .with_visibility(false)
            .with_title("WOMP");

        let context = glutin::ContextBuilder::new();
        let display = glium::Display::new(window, context, &event_loop).unwrap();

        let renderer = conrod::backend::glium::Renderer::new(&display).unwrap();

        let image_map = conrod::image::Map::new();
        let mut ui = conrod::UiBuilder::new([WINDOW_SIZE.0 as f64, WINDOW_SIZE.1 as f64])
            .theme(theme())
            .build();

        const FONT_BYTES: &'static [u8] = include_bytes!("../assets/Quirk.ttf");
        let font = conrod::text::Font::from_bytes(FONT_BYTES).unwrap();
        ui.fonts.insert(font);

        let ids = Ids::new(ui.widget_id_generator());

        View {
            ui,
            ids,

            event_loop,
            display,
            renderer,
            image_map,

            visible: false,

            model: init_state,
            view_state: ViewState::new(),
        }
    }

    pub fn model(&self) -> Model { self.model }
}


use std::ffi::c_void;

impl Editor for View {
    fn size(&self) -> (i32, i32) { (200, 100) }
    fn position(&self) -> (i32, i32) { (0, 0) }

    fn open(&mut self, _: *mut c_void) {
        self.display.gl_window().show();
        self.display.gl_window().set_always_on_top(true);
        self.visible = true;
    }

    fn close(&mut self) {
        self.display.gl_window().hide();
        self.visible = false;
    }

    fn is_open(&mut self) -> bool { self.visible }

    fn idle(&mut self) {
        let mut events = Vec::new();

        self.event_loop.poll_events(|e| events.push(e));

        for event in events {
            use glutin::{Event, WindowEvent};

            if let Some(event) = conrod::backend::winit::convert_event(event.clone(), &self.display) {
                self.ui.handle_event(event);
            }

            match event {
                Event::WindowEvent { event, .. } => match event {
                    WindowEvent::CloseRequested => self.close(),
                    _ => {}
                }

                _ => {}
            }
        }

        set_widgets(&mut self.ui.set_widgets(), &self.ids, &mut self.view_state, &mut self.model);

        // Render the `Ui` and then display it on the screen.
        if let Some(primitives) = self.ui.draw_if_changed() {
            use glium::Surface;

            let mut target = self.display.draw();
            target.clear_color(0.03, 0.03, 0.03, 1.0);
            self.renderer.fill(&self.display, primitives, &self.image_map);
            self.renderer.draw(&self.display, &mut target, &self.image_map).unwrap();
            target.finish().unwrap();
        }
    }
}


fn theme() -> conrod::Theme {
    use conrod::*;
    use conrod::position::{Padding, Direction, Position, Relative};

    Theme {
        border_width: 0.0,

        font_size_large: 20,
        font_size_medium: 12,
        font_size_small: 8,

        shape_color: hsl(degrees(80.0), 0.3, 0.7),
        label_color: hsl(degrees(0.0), 0.6, 0.6),

        y_position: Position::Relative(Relative::Direction(Direction::Backwards, 10.0), None),

        padding: Padding {
            x: Range::new(5.0, 5.0),
            y: Range::new(5.0, 5.0),
        },

        .. Theme::default()
    }
}




struct ViewState {
    about: bool,
}

impl ViewState {
    fn new() -> Self {
        ViewState { about: false, }
    }
}


fn set_widgets(ui: &mut conrod::UiCell, ids: &Ids, ctx: &mut ViewState, model: &mut Model) {
    use conrod::*;
    use conrod::widget::*;
    use conrod::position::*;

    let canvas_split = 2.0;
    let canvas_offset = -1.0;

    let canvas_color = hsl(degrees(160.0), 0.3, 0.6);

    let canvas_width = WINDOW_SIZE.0 as f64 - 20.0;
    let canvas_height = WINDOW_SIZE.1 as f64 - 20.0;

    Canvas::new()
        .color(canvas_color.complement())
        .w_h(canvas_width, canvas_height)
        .x_y(canvas_offset-canvas_split, canvas_offset-canvas_split)
        .set(ids.canvas_bg, ui);

    Canvas::new()
        .color(canvas_color)
        .w_h(canvas_width, canvas_height)
        .x_y(canvas_offset+canvas_split, canvas_offset+canvas_split)
        .set(ids.canvas, ui);

    let about_button = Button::new()
        .top_right_of(ids.canvas)
        .label("?")
        .w_h(20.0, 20.0)
        .set(ids.about_button, ui);

    if about_button.was_clicked() {
        ctx.about = !ctx.about;
    }


    let cutoff_dip = Slider::new(model.cutoff_dip, 25.0, 900.0)
        .w_h(200.0, 20.0)
        .skew(1.3)
        .label("cutoff dip")
        .top_left_of(ids.canvas)
        .set(ids.cutoff_dip_slider, ui);

    if let Some(v) = cutoff_dip {
        model.cutoff_dip = v;
    }



    let wonk_amt_a = 1.0;
    let wonk_amt_b = 1.0;

    let wonk_xy = XYPad::new(
            model.wonk.0, -wonk_amt_a, wonk_amt_a,
            model.wonk.1, -wonk_amt_b, wonk_amt_b)
        .w_h(200.0, 200.0)
        .label("wonk")
        .set(ids.wonk_xypad, ui);

    if let Some(wonk) = wonk_xy {
        model.wonk = wonk;
    }


    let lfo_xy = XYPad::new(model.lfo_depth, 0.0, 20.0,  model.filter_lfo_depth, 0.0, 100.0)
        .w_h(200.0, 200.0)
        .label("lfo")
        .right_from(ids.wonk_xypad, 10.0)
        .set(ids.lfo_xypad, ui);

    if let Some((x, y)) = lfo_xy {
        model.lfo_depth = x;
        model.filter_lfo_depth = y;
    }



    if ctx.about {
        let about_color = hsl(degrees(280.0), 0.2, 0.7);
        let about_color_bg = canvas_color.complement();

        Canvas::new()
            .title_bar("about womp")
            .title_bar_color(about_color.complement())
            .color(about_color)
            .floating(true)
            .w_h(300.0, 180.0)
            .set(ids.about_panel, ui);

        let relative_offset = Relative::Scalar(-canvas_split*2.0);

        Canvas::new()
            .color(about_color_bg)
            .depth(-1.0)
            .w_h(300.0, 180.0)
            .x_y_position_relative_to(ids.about_panel, relative_offset, relative_offset)
            .set(ids.about_panel_bg, ui);

        const PKG_VERSION: &'static str = env!("CARGO_PKG_VERSION");

        Text::new(&format!("a vst of _manpat\nversion {}\nfont is Quirk by eeve somepx", PKG_VERSION))
            .top_left_of(ids.about_panel)
            .line_spacing(20.0)
            .set(ids.about_text, ui);
    }
}


widget_ids! {
    struct Ids {
        canvas_bg,
        canvas,

        cutoff_dip_slider,
        lfo_xypad,
        wonk_xypad,

        about_button,
        about_panel_bg,
        about_panel,
        about_text,
    }
}
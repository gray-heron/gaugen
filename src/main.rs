mod basic_components;
mod frontend;
mod gaugen;
mod geometry_components;
mod rtps;

extern crate gl;
extern crate glutin;
extern crate nalgebra as na;
extern crate nanovg;
extern crate rand;

use gilrs::{Axis, Button, Event, Gilrs};
use glutin::GlContext;
use nanovg::{
    Alignment, Clip, Color, Context, Direction, Font, Frame, Gradient, Image, ImagePattern,
    Intersect, LineCap, LineJoin, PathOptions, Scissor, Solidity, StrokeOptions, TextOptions,
    Transform, Winding,
};
use nalgebra::Vector2;

use rand::Rng;
use std::time::Instant;

const INIT_WINDOW_SIZE: (u32, u32) = (800, 800);

const ICON_SEARCH: &str = "\u{1F50D}";
const ICON_CIRCLED_CROSS: &str = "\u{2716}";
const ICON_CHEVRON_RIGHT: &str = "\u{E75E}";
const ICON_CHECK: &str = "\u{2713}";
const ICON_LOGIN: &str = "\u{E740}";
const ICON_TRASH: &str = "\u{E729}";

const GRAPH_HISTORY_COUNT: usize = 100;

fn clamp(value: f32, min: f32, max: f32) -> f32 {
    if value < min {
        min
    } else if value > max {
        max
    } else {
        value
    }
}

struct DemoData<'a> {
    fonts: DemoFonts<'a>,
    images: Vec<Image<'a>>,
}

struct DemoFonts<'a> {
    icons: Font<'a>,
    sans: Font<'a>,
    sans_bold: Font<'a>,
}

fn main() {
    let mut gilrs = Gilrs::new().unwrap();

    // Iterate over all connected gamepads
    for (_id, gamepad) in gilrs.gamepads() {
        println!("{} is {:?}", gamepad.name(), gamepad.power_info());
    }

    let mut events_loop = glutin::EventsLoop::new();
    let window = glutin::WindowBuilder::new()
        .with_title("NanoVG UI")
        .with_dimensions(INIT_WINDOW_SIZE.0, INIT_WINDOW_SIZE.1);
    let context = glutin::ContextBuilder::new()
        .with_vsync(false)
        .with_multisampling(4)
        .with_srgb(true);
    let gl_window = glutin::GlWindow::new(window, context, &events_loop).unwrap();

    unsafe {
        gl_window.make_current().unwrap();
        gl::load_with(|symbol| gl_window.get_proc_address(symbol) as *const _);
    }

    let context = nanovg::ContextBuilder::new()
        .stencil_strokes()
        .build()
        .expect("Initialization of NanoVG failed!");

    let start_time = Instant::now();
    let mut running = true;

    let mut mx = 0.0f32;
    let mut my = 0.0f32;

    let demo_data = load_demo_data(&context);

    let mut fps_graph = PerformanceGraph::new(GraphRenderStyle::Fps, "Frame Time");
    let mut cpu_graph = PerformanceGraph::new(GraphRenderStyle::Ms, "CPU Time");
    let mut rng_graph = PerformanceGraph::new(GraphRenderStyle::Percent, "Random");

    let mut percent = 0.0f32;
    let mut rng = rand::thread_rng();
    let mut prev_time = 0.0;

    let mut active_gamepad = None;

    let mut gui_manager = gaugen::Manager::new();
    basic_components::register_basic_components(&mut gui_manager);
    geometry_components::register_geometry_components(&mut gui_manager);

    let resources = frontend::Resources {
        palette: Box::new(frontend::DarkPalette {}),
        font: demo_data.fonts.sans,
    };

    while running {
        let elapsed = get_elapsed(&start_time);
        let delta_time = elapsed - prev_time;
        prev_time = elapsed;

        events_loop.poll_events(|event| match event {
            glutin::Event::WindowEvent { event, .. } => match event {
                glutin::WindowEvent::Closed => running = false,
                glutin::WindowEvent::Resized(w, h) => gl_window.resize(w, h),
                glutin::WindowEvent::CursorMoved { position, .. } => {
                    mx = position.0 as f32;
                    my = position.1 as f32;
                }
                _ => {}
            },
            _ => {}
        });

        let (width, height) = gl_window.get_inner_size().unwrap();
        let (width, height) = (width as i32, height as i32);

        unsafe {
            gl::Viewport(0, 0, width, height);
            gl::ClearColor(0.0, 0.0, 0.0, 1.0);
            gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT | gl::STENCIL_BUFFER_BIT);
        }

        let (width, height) = (width as f32, height as f32);

        while let Some(Event { id, event, time }) = gilrs.next_event() {
            println!("{:?} New event from {}: {:?}", time, id, event);
            active_gamepad = Some(id);
        }

        context.frame((width, height), gl_window.hidpi_factor(), |frame| {
            let mut ctx = frontend::PresentationContext {
                frame: &frame,
                time: elapsed,
                resources: &resources,
            };

            let zone = gaugen::DrawZone::from_rect(
                Vector2::new(0.0, 0.0),
                Vector2::new(width, height),
            );

            match gui_manager.make_screen("screen.json") {
                Some((ref mut screen, ref geometry)) => {
                    match geometry.aspect {
                        Some(aspect) => {
                            let corrected_zone = gaugen::DrawZone {
                                m: zone.m,
                                size: match aspect > zone.aspect(){
                                    true => Vector2::new(zone.size.x, 1.0 / aspect * zone.size.x),
                                    false => Vector2::new(aspect * zone.size.y, zone.size.y),
                                }
                            };
                            screen.draw(&mut ctx, corrected_zone);
                        },
                        None => screen.draw(&mut ctx, zone)
                    }
                }
                None => {}
            };

            /*
            fps_graph.draw(&frame, demo_data.fonts.sans, 5.0, 5.0);
            cpu_graph.draw(&frame, demo_data.fonts.sans, 5.0 + 200.0 + 5.0, 5.0);
            rng_graph.draw(
                &frame,
                demo_data.fonts.sans,
                5.0 + 200.0 + 5.0 + 200.0 + 5.0,
                5.0,
            );
            */
        });

        fps_graph.update(delta_time);

        percent = if rng.gen() {
            percent + 1.0
        } else {
            percent - 1.0
        };
        percent = clamp(percent, 0.0, 100.0);
        rng_graph.update(percent);

        let cpu_time = get_elapsed(&start_time) - elapsed;
        cpu_graph.update(cpu_time);

        gl_window.swap_buffers().unwrap();
    }

    println!("Average Frame Time: {:.2} ms", fps_graph.average() * 1000.0);
    println!("          CPU Time: {:.2} ms", cpu_graph.average() * 1000.0);
    println!("       RNG Percent: {:.2}%  ", rng_graph.average());
}

fn load_demo_data(context: &Context) -> DemoData {
    let demo_fonts = DemoFonts {
        icons: Font::from_file(context, "Entypo", "resources/entypo.ttf")
            .expect("Failed to load font 'entypo.ttf'"),

        sans: Font::from_file(context, "Roboto-Regular", "resources/Roboto-Regular.ttf")
            .expect("Failed to load font 'Roboto-Regular.ttf'"),

        sans_bold: Font::from_file(context, "Roboto-Bold", "resources/Roboto-Bold.ttf")
            .expect("Failed to load font 'Roboto-Bold.ttf'"),
    };

    let emoji = Font::from_file(context, "NotoEmoji", "resources/NotoEmoji-Regular.ttf")
        .expect("Failed to load font 'NotoEmoji-Regular.ttf'");

    let mut images = Vec::new();
    for i in 0..12 {
        let file_name = format!("resources/images/image{}.jpg", i + 1);
        let image = Image::new(context)
            .build_from_file(&file_name)
            .expect(&format!("Failed to load image {}", &file_name));
        images.push(image);
    }

    demo_fonts.sans.add_fallback(emoji);
    demo_fonts.sans_bold.add_fallback(emoji);

    DemoData {
        fonts: demo_fonts,
        images: images,
    }
}

fn get_elapsed(instant: &Instant) -> f32 {
    let elapsed = instant.elapsed();
    let elapsed = elapsed.as_secs() as f64 + elapsed.subsec_nanos() as f64 * 1e-9;
    elapsed as f32
}

/*
fn render_demo(
    frame: &Frame,
    gamepad: Option<gilrs::Gamepad>,
    mx: f32,
    my: f32,
    width: f32,
    height: f32,
    t: f32,
    data: &DemoData,
) {
    let ctx = PresentationContext {
        frame: frame,
        time: t,
        fonts: &data.fonts,
    };
    let ri = RotationalIndicator {
        precision: 2,
        unit: "V".to_string(),
        caption: "VOLTAGE".to_string(),
        value_min: 4.0,
        value_ranges: vec![
            (6.0, Status::Error),
            (6.6, Status::Warning),
            (8.4, Status::Ok),
            (10.0, Status::Error),
        ],
    };

    let mut ssi = SpatialSituationIndicator {
        projection_zoom: 2.1,
        o: UnitQuaternion::identity(),
    };

    match gamepad {
        Some(gp) => {
            ssi.o = UnitQuaternion::from_euler_angles(
                (gp.value(Axis::LeftStickX) * 90.0).rad(),
                (gp.value(Axis::LeftStickY) * 90.0).rad(),
                (gp.value(Axis::RightStickX) * 90.0).rad(),
            );
        }
        _ => {}
    }

    let zone = DrawZone {
        width: width / 2.0,
        height: height / 2.0,
        mx: width / 2.0,
        my: height / 2.0,
    };

    ssi.draw(&ctx, &zone, frame);

    //ri.Draw(&ctx, &data.fonts, t / 2.0 + 3.0, frame, width, height);
}
*/
enum GraphRenderStyle {
    Fps,
    Ms,
    Percent,
}

struct PerformanceGraph {
    style: GraphRenderStyle,
    name: String,
    values: [f32; GRAPH_HISTORY_COUNT],
    head: usize,
}

impl PerformanceGraph {
    fn new(style: GraphRenderStyle, name: &str) -> PerformanceGraph {
        PerformanceGraph {
            style,
            name: String::from(name),
            values: [0.0; GRAPH_HISTORY_COUNT],
            head: 0,
        }
    }

    fn update(&mut self, frame_time: f32) {
        self.head = (self.head + 1) % GRAPH_HISTORY_COUNT;
        self.values[self.head] = frame_time;
    }

    fn draw(&self, frame: &Frame, font: Font, x: f32, y: f32) {
        let w = 200.0;
        let h = 35.0;
        let average = self.average();

        frame.path(
            |path| {
                path.rect((x, y), (w, h));
                path.fill(Color::from_rgba(0, 0, 0, 128), Default::default());
            },
            Default::default(),
        );

        frame.path(
            |path| {
                path.move_to((x, y + h));
                match self.style {
                    GraphRenderStyle::Fps => {
                        for i in 0..self.values.len() {
                            let v =
                                1.0 / (0.00001 + self.values[(self.head + i) % self.values.len()]);
                            let v = clamp(v, 0.0, 80.0);
                            let vx = x + (i as f32 / (self.values.len() - 1) as f32) * w;
                            let vy = y + h - ((v / 80.0) * h);
                            path.line_to((vx, vy));
                        }
                    }
                    GraphRenderStyle::Ms => {
                        for i in 0..self.values.len() {
                            let v = self.values[(self.head + i) % self.values.len()] * 1000.0;
                            let v = clamp(v, 0.0, 20.0);
                            let vx = x + (i as f32 / (self.values.len() - 1) as f32) * w;
                            let vy = y + h - ((v / 20.0) * h);
                            path.line_to((vx, vy));
                        }
                    }
                    GraphRenderStyle::Percent => {
                        for i in 0..self.values.len() {
                            let v = self.values[(self.head + i) % self.values.len()] * 1.0;
                            let v = clamp(v, 0.0, 100.0);
                            let vx = x + (i as f32 / (self.values.len() - 1) as f32) * w;
                            let vy = y + h - ((v / 100.0) * h);
                            path.line_to((vx, vy));
                        }
                    }
                }

                path.line_to((x + w, y + h));

                path.fill(Color::from_rgba(255, 192, 0, 128), Default::default());
            },
            Default::default(),
        );

        frame.text(
            font,
            (x + 3.0, y + 1.0),
            &self.name,
            TextOptions {
                color: Color::from_rgba(240, 240, 240, 192),
                align: Alignment::new().left().top(),
                size: 14.0,
                ..Default::default()
            },
        );

        match self.style {
            GraphRenderStyle::Fps => {
                frame.text(
                    font,
                    (x + w - 3.0, y + 1.0),
                    format!("{:.2} FPS", 1.0 / average),
                    TextOptions {
                        size: 18.0,
                        color: Color::from_rgba(240, 240, 240, 255),
                        align: Alignment::new().right().top(),
                        ..Default::default()
                    },
                );

                frame.text(
                    font,
                    (x + w - 3.0, y + h - 1.0),
                    format!("{:.2} ms", average * 1000.0),
                    TextOptions {
                        size: 15.0,
                        color: Color::from_rgba(240, 240, 240, 160),
                        align: Alignment::new().right().bottom(),
                        ..Default::default()
                    },
                );
            }
            GraphRenderStyle::Ms => {
                frame.text(
                    font,
                    (x + w - 3.0, y + 1.0),
                    format!("{:.2} ms", average * 1000.0),
                    TextOptions {
                        size: 18.0,
                        color: Color::from_rgba(240, 240, 240, 255),
                        align: Alignment::new().right().top(),
                        ..Default::default()
                    },
                );
            }
            GraphRenderStyle::Percent => frame.text(
                font,
                (x + w - 3.0, y + 1.0),
                format!("{:.1} %", average * 1.0),
                TextOptions {
                    size: 18.0,
                    color: Color::from_rgba(240, 240, 240, 255),
                    align: Alignment::new().right().top(),
                    ..Default::default()
                },
            ),
        }
    }

    fn average(&self) -> f32 {
        self.values.iter().sum::<f32>() / self.values.len() as f32
    }
}

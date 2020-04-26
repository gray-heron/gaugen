mod rtps;
mod gaugen;
mod indicators;
mod geometry_components;

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
use rand::Rng;
use std::f32::consts;
use std::time::Instant;

use na::{Quaternion, UnitQuaternion, Vector2, Vector3};

const INIT_WINDOW_SIZE: (u32, u32) = (800, 800);

const ICON_SEARCH: &str = "\u{1F50D}";
const ICON_CIRCLED_CROSS: &str = "\u{2716}";
const ICON_CHEVRON_RIGHT: &str = "\u{E75E}";
const ICON_CHECK: &str = "\u{2713}";
const ICON_LOGIN: &str = "\u{E740}";
const ICON_TRASH: &str = "\u{E729}";

const GRAPH_HISTORY_COUNT: usize = 100;

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
            render_demo(
                &frame,
                match active_gamepad {
                    Some(id) => Some(gilrs.gamepad(id)),
                    None => None,
                },
                mx,
                my,
                width as f32,
                height as f32,
                elapsed,
                &demo_data,
            );

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

fn clamp(value: f32, min: f32, max: f32) -> f32 {
    if value < min {
        min
    } else if value > max {
        max
    } else {
        value
    }
}

const CGA: f32 = consts::PI / 8.0;



fn StatusToColor(s: Status) -> Color {
    match s {
        Status::Ok => Color::from_rgba(0, 160, 0, 255),
        Status::Warning => Color::from_rgba(250, 120, 0, 255),
        Status::Error => Color::from_rgba(200, 0, 0, 255),
    }
}

fn StatusToColorBg(s: Status) -> Color {
    match s {
        Status::Ok => Color::from_rgba(30, 30, 40, 255),
        Status::Warning => Color::from_rgba(0xbe, 0x55, 0, 255),
        Status::Error => Color::from_rgba(200 / 2, 0, 0, 255),
    }
}

fn FormatFloat(v: f32, decimals: u32) -> String {
    let mut ret: String = "".to_string();
    let mut countdown = false;
    let mut counter = decimals;
    let s = v.to_string();

    for c in s.chars() {
        if countdown {
            if counter == 0 {
                break;
            }

            counter -= 1;
        } else if c == '.' {
            countdown = true;
        }

        ret.push(c);
    }

    for _ in 0..counter {
        ret.push('0');
    }

    ret
}

struct RotationalIndicator {
    precision: u32,
    unit: String,
    caption: String,
    value_min: f32,
    value_ranges: Vec<(f32, Status)>,
}

impl RotationalIndicator {
    fn draw(&self, ctx: &PresentationContext, value: f32, frame: &Frame, width: f32, height: f32) {
        let base_radius = (if width > height { height } else { width }) / 2.5;
        let base_thickness = base_radius / 10.0;

        let value_max = self.value_ranges[self.value_ranges.len() - 1].0;

        let normalize = |value: f32| {
            if value < self.value_min {
                0.0
            } else if value > value_max {
                1.0
            } else {
                (value - self.value_min) / (value_max - self.value_min)
            }
        };

        let smartarc = |p0: f32,
                        p1: f32,
                        radius: f32,
                        stroke_width: f32,
                        stroke_color: Color,
                        fill_color: Color| {
            let arcstart = p0 * (consts::PI + CGA * 2.0);
            let arcend = (1.0 - p1) * (consts::PI + CGA * 2.0);

            frame.path(
                |path| {
                    path.arc(
                        (width / 2.0, height / 2.0 - base_radius * 0.0),
                        radius,
                        0.0 + CGA - arcend,
                        consts::PI - CGA + arcstart,
                        Winding::Direction(Direction::CounterClockwise),
                    );
                    path.fill(fill_color, Default::default());
                    path.stroke(
                        stroke_color,
                        StrokeOptions {
                            width: stroke_width,
                            ..Default::default()
                        },
                    );
                },
                Default::default(),
            );
        };

        let mut last_range_end = normalize(self.value_min);

        let mut value_status = Status::Error;

        let nvalue = normalize(value);

        for range_end in &self.value_ranges {
            let current_range_end = normalize(range_end.0);
            smartarc(
                last_range_end,
                current_range_end,
                base_radius * 1.13,
                base_thickness,
                StatusToColor(range_end.1),
                Color::from_rgba(0, 0, 0, 0),
            );

            if nvalue > last_range_end && nvalue <= current_range_end {
                value_status = range_end.1;
            }

            last_range_end = current_range_end;
        }

        smartarc(
            0.0,
            1.0,
            base_radius * 1.09,
            0.0,
            Color::from_rgba(160, 160, 160, 255),
            StatusToColorBg(value_status),
        );

        if nvalue > 0.0 {
            smartarc(
                0.0,
                nvalue,
                base_radius,
                base_thickness * 1.75,
                Color::from_rgba(255, 255, 255, 255),
                Color::from_rgba(0, 0, 0, 0),
            );
        }

        let text_opts_caption = TextOptions {
            color: Color::from_rgba(180, 180, 180, 255),
            size: base_radius / 2.0,
            align: Alignment::new().center().middle(),
            line_height: base_radius / 3.0,
            line_max_width: width,
            ..Default::default()
        };

        let text_opts_value = TextOptions {
            color: Color::from_rgba(255, 255, 255, 255),
            size: base_radius / 1.55,
            align: Alignment::new().center().middle(),
            line_height: base_radius / 2.5,
            line_max_width: width,
            ..Default::default()
        };

        if value_status != Status::Error || ctx.time * 2.0 - ((ctx.time * 2.0) as i32 as f32) < 0.66
        {
            frame.text_box(
                ctx.fonts.sans,
                (0.0, height / 2.0 + base_radius / 1.5),
                &self.caption,
                text_opts_caption,
            );
        }
        frame.text_box(
            ctx.fonts.sans,
            (0.0, height / 2.0 - base_radius / 10.0),
            FormatFloat(value, self.precision) + &self.unit,
            text_opts_value,
        );
    }
}

struct SpatialSituationIndicator {
    projection_zoom: f32,
    o: UnitQuaternion<f32>,
}

trait DegreeTrigonometry {
    fn rad(&self) -> f32;
    fn deg(&self) -> f32;
}

impl DegreeTrigonometry for f32 {
    fn rad(&self) -> f32 {
        self / 180.0 * consts::PI
    }

    fn deg(&self) -> f32 {
        self / consts::PI * 180.0
    }
}

impl SpatialSituationIndicator {
    fn projection(&self, p: Vector2<f32>) -> Option<Vector2<f32>> {
        let v3 = UnitQuaternion::from_euler_angles(0.0, p.y.rad(), p.x.rad())
            * Vector3::new(1.0, 0.0, 0.0);
        let out = self.o.inverse() * v3;

        match out.x > 0.0 {
            true => Some(Vector2::new(
                out.y * self.projection_zoom,
                out.z * self.projection_zoom,
            )),
            _ => None,
        }
    }

    pub fn draw_line<F>(
        &self,
        frame: &Frame,
        zone: &DrawZone,
        p1: Vector2<f32>,
        p2: Vector2<f32>,
        path_style: F,
    ) where
        F: Fn(&nanovg::Path),
    {
        match (self.projection(p1), self.projection(p2)) {
            (Some(tp1), Some(tp2)) => {
                frame.path(
                    |path| {
                        let from = (tp1.x * zone.width + zone.mx, tp1.y * zone.height + zone.my);
                        let to = (tp2.x * zone.width + zone.mx, tp2.y * zone.height + zone.my);
                        path.move_to(from);
                        path.line_to(to);
                        path_style(&path);
                    },
                    Default::default(),
                );
            }
            _ => {}
        }
    }

    pub fn draw_text(
        &self,
        ctx: &PresentationContext,
        zone: &DrawZone,
        t: String,
        p: Vector2<f32>,
    ) {
        let linelen = zone.height / 2.0;
        let text_opts_value = TextOptions {
            color: Color::from_rgba(255, 255, 255, 255),
            size: zone.height / 15.0,
            align: Alignment::new().center().middle(),
            line_height: zone.height / 15.0,
            line_max_width: linelen,
            ..Default::default()
        };

        match self.projection(p) {
            Some(tp) => ctx.frame.text_box(
                ctx.fonts.sans,
                (tp.x * zone.width + zone.mx - linelen / 2.0, tp.y * zone.height + zone.my),
                t,
                text_opts_value,
            ),
            _ => {}
        }
    }

    pub fn draw_ffd(
        &self,
        ctx: &PresentationContext,
        zone: &DrawZone
    ) {
        let unit = zone.height / 20.0;
        ctx.frame.path(
            |path| {
                path.move_to((zone.mx - 2.0 * unit, zone.my));
                path.line_to((zone.mx - 0.66 * unit, zone.my));
                path.line_to((zone.mx, zone.my + unit));
                path.line_to((zone.mx + 0.66 * unit, zone.my));
                path.line_to((zone.mx + 2.0 * unit, zone.my));

                path.circle((zone.mx, zone.my), 1.0);
    
                path.stroke(
                    Color::from_rgba(0xff, 0xff, 0x20, 0xa2),
                    StrokeOptions {
                        width: 3.0,
                        ..Default::default()
                    },
                );
            },
            Default::default(),
        );

    }

    pub fn draw(
        &self,
        ctx: &PresentationContext,
        zone: &DrawZone,
        frame: &Frame,
    ) {
        let orientation = self.o.euler_angles();

        //draw vertical ladder
        for i in -22..22 {
            let h = (i * 5) as f32;
            let p1 = Vector2::new(5.0 + orientation.2.deg(), h);
            let p2 = Vector2::new(-5.0 + orientation.2.deg(), h);
            let p3 = Vector2::new(6.3 + orientation.2.deg(), h);

            self.draw_line(frame, zone, p1, p2, |path| {
                path.stroke(
                    Color::from_rgba(0x80, 0x80, 0x80, 0xff),
                    StrokeOptions {
                        width: 1.5,
                        ..Default::default()
                    },
                );
            });

            if i != 0 && i * 5 <= 90 {
                self.draw_text(ctx, zone, (i * 5).to_string(), p3)
            }
        }

        let spacing = match orientation.1.deg().abs() > 60.0 {
            true => 30,
            _ => 10,
        };

        let ladder_height = match orientation.1.deg().abs() < 75.0 {
            true => orientation.1.deg(),
            _ => orientation.1.deg().signum() * 75.0,
        };

        //draw horizontal ladder
        for i in 0 / spacing..360 / spacing {
            let y = (i * spacing) as f32;
            let p1 = Vector2::new(y, 3.0 + ladder_height);
            let p2 = Vector2::new(y, -3.0 + ladder_height);
            let p3 = Vector2::new(y, 0.0 + ladder_height);

            self.draw_line(frame, zone, p1, p2, |path| {
                path.stroke(
                    Color::from_rgba(0x80, 0x80, 0x80, 0xff),
                    StrokeOptions {
                        width: 1.5,
                        ..Default::default()
                    },
                );
            });

            self.draw_text(ctx, zone, (i * spacing).to_string(), p3)
        }

        self.draw_ffd(ctx, zone);
    }
}

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

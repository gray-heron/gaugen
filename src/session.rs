use crate::frontend;
use crate::gaugen;

extern crate gl;
extern crate glutin;
extern crate nalgebra as na;
extern crate nanovg;
extern crate rand;

use glutin::GlContext;
use nalgebra::Vector2;
use std::time::Instant;

const INIT_WINDOW_SIZE: (u32, u32) = (800, 800);


fn get_elapsed_time(instant: &Instant) -> f32 {
    let elapsed = instant.elapsed();
    let elapsed = elapsed.as_secs() as f64 + elapsed.subsec_nanos() as f64 * 1e-9;
    elapsed as f32
}

pub struct SessionBuilder {
    manager: gaugen::Manager,
}

pub struct Session<'a> {
    context: &'a nanovg::Context,
    font: nanovg::Font<'a>,
    manager: gaugen::Manager,
    default_screen: Screen,
    start_time: Instant
}

pub struct Screen {
    gl_window: glutin::GlWindow,
    events_loop: glutin::EventsLoop
}

// multi-screen capability waits for glutin support for multi-window with one context
pub enum __TargetScreen<'a> {
    Custom(&'a mut Screen),
    Default,
}

impl SessionBuilder {
    pub fn new() -> SessionBuilder {
        SessionBuilder {
            manager: gaugen::Manager::new(),
        }
    }

    pub fn register_components<F: Fn(&mut gaugen::Manager)>(mut self, components: F) -> Self {
        components(&mut self.manager);
        self
    }

    fn make_screen() -> Screen {
        let events_loop = glutin::EventsLoop::new();
        let window = glutin::WindowBuilder::new()
            .with_title("NanoVG UI")
            .with_dimensions(INIT_WINDOW_SIZE.0, INIT_WINDOW_SIZE.1);
        let context = glutin::ContextBuilder::new()
            .with_vsync(false)
            .with_multisampling(4)
            .with_srgb(true);
        let gl_window = glutin::GlWindow::new(window, context, &events_loop).unwrap();

        Screen {
            gl_window: gl_window,
            events_loop: events_loop,
        }
    }

    pub fn init<F: Fn(&mut Session)>(self, handler: F) {
        let default_screen = SessionBuilder::make_screen();

        unsafe {
            default_screen.gl_window.make_current().unwrap();
            gl::load_with(|symbol| default_screen.gl_window.get_proc_address(symbol) as *const _);
        }

        let context = nanovg::ContextBuilder::new()
            .stencil_strokes()
            .build()
            .expect("Initialization of NanoVG failed!");

        let font =
            nanovg::Font::from_file(&context, "Roboto-Regular", "resources/Roboto-Regular.ttf")
                .expect("Failed to load font 'Roboto-Regular.ttf'");

        let mut session = Session {
            context: &context,
            font: font,
            manager: self.manager,
            default_screen: default_screen,
            start_time: Instant::now()
        };

        handler(&mut session);
    }
}

impl Session<'_> {
    pub fn draw(
        &mut self,
        //screen: TargetScreen,
        view: &mut gaugen::View,
        palette: &dyn frontend::Palette,
        hooks: &gaugen::Hooks,
    ) -> bool {
        /*
        see __TargetScreen
        let screen = match screen {
            TargetScreen::Custom(screen) => screen,
            _ => &mut self.default_screen,
        };
        */
        let screen = &mut self.default_screen;

        unsafe {
            screen.gl_window.make_current().unwrap();
            gl::load_with(|symbol| screen.gl_window.get_proc_address(symbol) as *const _);
        }

        let mut mx = 0.0f32;
        let mut my = 0.0f32;
        let mut quit = false;

        let __window = &mut screen.gl_window;

        screen.events_loop.poll_events(|event| match event {
            glutin::Event::WindowEvent { event, .. } => match event {
                glutin::WindowEvent::Closed => quit = true,
                glutin::WindowEvent::Resized(w, h) => __window.resize(w, h),
                glutin::WindowEvent::CursorMoved { position, .. } => {
                    mx = position.0 as f32;
                    my = position.1 as f32;
                }
                _ => {}
            },
            _ => {}
        });

        if quit {
            return false;
        }

        let (width, height) = screen.gl_window.get_inner_size().unwrap();
        let (width, height) = (width as i32, height as i32);

        unsafe {
            gl::Viewport(0, 0, width, height);
            gl::ClearColor(0.0, 0.0, 0.0, 1.0);
            gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT | gl::STENCIL_BUFFER_BIT);
        }

        let (width, height, dpi) = (width as f32, height as f32, screen.gl_window.hidpi_factor());

        let __font = self.font; //so no "self" is not used in closure
        let __time = &self.start_time;

        self.context.frame((width, height), dpi, |frame| {
            let mut ctx = frontend::PresentationContext {
                frame: &frame,
                time: get_elapsed_time(__time),
                resources: &frontend::Resources {
                    palette: palette,
                    font: __font,
                },
            };

            let zone =
                gaugen::DrawZone::from_rect(Vector2::new(0.0, 0.0), Vector2::new(width, height))
                    .constraint_to_aspect(view.1.aspect);

            view.0.draw(&mut ctx, zone, hooks);
        });

        screen.gl_window.swap_buffers().unwrap();

        true
    }

    pub fn new_view(&self, path_to_json: &str) -> Option<gaugen::View> {
        let mut ret = None; //fixme
        let (width, height) = self.default_screen.gl_window.get_inner_size().unwrap();

        self.context.frame(
            (width as f32, height as f32),
            self.default_screen.gl_window.hidpi_factor(),
            |frame| {
                ret = Some(self.manager.make_screen(
                    &frontend::PresentationContext {
                        frame: &frame,
                        time: 0.0,
                        resources: &frontend::Resources {
                            palette: &frontend::DarkPalette {},
                            font: self.font,
                        },
                    },
                    path_to_json,
                ))
            },
        );

        ret.unwrap()
    }

    //see __TargetSreen
    fn __new_screen(&mut self) -> Screen {
        SessionBuilder::make_screen()
    }
}

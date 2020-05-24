use crate::frontend;
use crate::gaugen;

extern crate gl;
extern crate glutin;
extern crate nalgebra as na;
extern crate nanovg;
extern crate rand;

use glutin::GlContext;
use nalgebra::Vector2;

const INIT_WINDOW_SIZE: (u32, u32) = (800, 800);

pub struct SessionBuilder {
    manager: gaugen::Manager,
}

pub struct Session<'a> {
    context: &'a nanovg::Context,
    events_loop: glutin::EventsLoop,
    gl_window: glutin::GlWindow,
    font: nanovg::Font<'a>,
    manager: gaugen::Manager,
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

    pub fn init<F: Fn(&mut Session)>(mut self, handler: F) {
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

        let font =
        nanovg::Font::from_file(&context, "Roboto-Regular", "resources/Roboto-Regular.ttf")
            .expect("Failed to load font 'Roboto-Regular.ttf'");

        let mut session = Session {
            context: &context,
            events_loop: events_loop,
            gl_window: gl_window,
            font: font,
            manager: self.manager,
        };

        handler(&mut session);
    }
}

impl Session<'_> {
    pub fn draw(
        &mut self,
        view: &mut gaugen::View,
        palette: &dyn frontend::Palette,
        hooks: &gaugen::Hooks,
        dt: f32,
    ) -> bool {
        let mut mx = 0.0f32;
        let mut my = 0.0f32;
        let mut quit = false;

        let __window  = &mut self.gl_window;

        self.events_loop.poll_events(|event| match event {
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

        let (width, height) = self.gl_window.get_inner_size().unwrap();
        let (width, height) = (width as i32, height as i32);

        unsafe {
            gl::Viewport(0, 0, width, height);
            gl::ClearColor(0.0, 0.0, 0.0, 1.0);
            gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT | gl::STENCIL_BUFFER_BIT);
        }

        let (width, height) = (width as f32, height as f32);

        self.context
            .frame((width, height), self.gl_window.hidpi_factor(), |frame| {
                let mut ctx = frontend::PresentationContext {
                    frame: &frame,
                    time: dt,
                    resources: &frontend::Resources {
                        palette: palette,
                        font: self.font,
                    },
                };

                let zone = gaugen::DrawZone::from_rect(
                    Vector2::new(0.0, 0.0),
                    Vector2::new(width, height),
                );

                match view.1.aspect {
                    Some(aspect) => {
                        let corrected_zone = gaugen::DrawZone {
                            m: zone.m,
                            size: match aspect > zone.aspect() {
                                true => Vector2::new(zone.size.x, 1.0 / aspect * zone.size.x),
                                false => Vector2::new(aspect * zone.size.y, zone.size.y),
                            },
                        };
                        view.0.draw(&mut ctx, corrected_zone, hooks);
                    }
                    None => view.0.draw(&mut ctx, zone, hooks),
                }
            });

        self.gl_window.swap_buffers().unwrap();

        true
    }

    pub fn new_view(&self, path_to_json: &str) -> Option<gaugen::View> {
        let mut ret = None; //fixme
        let (width, height) = self.gl_window.get_inner_size().unwrap();

        self.context.frame(
            (width as f32, height as f32),
            self.gl_window.hidpi_factor(),
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
}
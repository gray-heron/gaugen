use crate::frontend;
use crate::*;

use glutin::GlContext;
use nalgebra::Vector2;
use nanovg::*;
use std::cell::RefCell;
use std::time::Instant;

const INIT_WINDOW_SIZE: (u32, u32) = (800, 800);

fn get_elapsed_time(instant: &Instant) -> f32 {
    let elapsed = instant.elapsed();
    let elapsed = elapsed.as_secs() as f64 + elapsed.subsec_nanos() as f64 * 1e-9;
    elapsed as f32
}

pub struct SessionBuilder {
    manager: Manager,
}

pub struct Session<'a> {
    context: &'a nanovg::Context,
    font: nanovg::Font<'a>,
    manager: Manager,
    default_screen: Screen,
    start_time: Instant,
}

pub struct Screen {
    gl_window: glutin::GlWindow,
    events_loop: glutin::EventsLoop,
}

// multi-screen capability waits for glutin support for multi-window with one context
pub enum __TargetScreen<'a> {
    Custom(&'a mut Screen),
    Default,
}

impl SessionBuilder {
    pub fn new() -> SessionBuilder {
        SessionBuilder {
            manager: Manager::new(),
        }
    }

    pub fn register_components<F: Fn(&mut Manager)>(mut self, components: F) -> Self {
        components(&mut self.manager);
        self
    }

    fn make_screen() -> Screen {
        let events_loop = glutin::EventsLoop::new();
        let window = glutin::WindowBuilder::new()
            .with_title("Gaugen Demo")
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
            start_time: Instant::now(),
        };

        handler(&mut session);
    }
}

impl Session<'_> {
    pub fn draw(
        &mut self,
        //screen: TargetScreen,
        view: &mut View,
        palette: &dyn frontend::Palette,
        hooks: &Hooks,
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

        let mut m_x = 0.0f32;
        let mut m_y = 0.0f32;
        let mut m_pressed = false;
        let mut quit = false;

        let __window = &mut screen.gl_window;

        screen.events_loop.poll_events(|event| match event {
            glutin::Event::WindowEvent { event, .. } => match event {
                glutin::WindowEvent::Closed => quit = true,
                glutin::WindowEvent::Resized(w, h) => __window.resize(w, h),
                glutin::WindowEvent::CursorMoved { position, .. } => {
                    m_x = position.0 as f32;
                    m_y = position.1 as f32;
                }
                glutin::WindowEvent::MouseInput { button, .. } => match button {
                    glutin::MouseButton::Left => m_pressed = true,
                    _ => {}
                },
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

        let __font = self.font; //so no "self" is not used in the closure
        let __time = &self.start_time;

        self.context.frame((width, height), dpi, |frame| {
            let res = frontend::Resources {
                palette: palette,
                font: __font,
            };

            let mut ctx = frontend::PresentationContext {
                frame: frame,
                time: 0.0, //get_elapsed_time(__time),
                resources: res,
                shell_stack: vec![DrawZone::new_empty()],
            };

            let zone = DrawZone::from_rect(Vector2::new(0.0, 0.0), Vector2::new(width, height));

            view.with_mut(|fields| {
                for layer in fields.layers {
                    layer.components[0].borrow_mut().draw(&mut ctx, zone, hooks);
                }
            });
        });

        screen.gl_window.swap_buffers().unwrap();

        if m_pressed {
            view.with_mut(|fields| {
                for layer in fields.layers {
                    //fixme: don't traverse all the components, somehow register which components listen to what
                    for component in &layer.components {
                        let mut component = component.borrow_mut();
                        let drawn_location = component.get_drawn_location().clone();

                        if (drawn_location.m.x - m_x).abs() <= drawn_location.size.x / 2.0
                            && (drawn_location.m.y - m_y).abs() <= drawn_location.size.y / 2.0
                        {
                            component.handle_event(&drawn_location, &Event::MouseClick(m_x, m_y));
                        }
                    }
                }
            });
        }

        true
    }

    pub fn new_view(&self, path_to_json: &str) -> Option<View> {
        let mut ret = None; //fixme

        let (width, height) = self.default_screen.gl_window.get_inner_size().unwrap();

        self.context.frame(
            (width as f32, height as f32),
            self.default_screen.gl_window.hidpi_factor(),
            |frame| {
                let res = frontend::Resources {
                    palette: &frontend::DarkPalette {},
                    font: self.font,
                };

                let mut ctx = frontend::PresentationContext {
                    frame: frame,
                    time: 0.0,
                    resources: res,
                    shell_stack: vec![DrawZone::new_empty()],
                };

                ret = Some(self.manager.make_screen(&mut ctx, path_to_json))
            },
        );

        ret.unwrap()
    }

    pub fn instantiate_component(&self, path_to_json: &str) -> Option<View> {
        let mut ret = None; //fixme

        let (width, height) = self.default_screen.gl_window.get_inner_size().unwrap();

        self.context.frame(
            (width as f32, height as f32),
            self.default_screen.gl_window.hidpi_factor(),
            |frame| {
                let res = frontend::Resources {
                    palette: &frontend::DarkPalette {},
                    font: self.font,
                };

                let mut ctx = frontend::PresentationContext {
                    frame: frame,
                    time: 0.0,
                    resources: res,
                    shell_stack: vec![DrawZone::new_empty()],
                };

                ret = Some(self.manager.make_screen(&mut ctx, path_to_json))
            },
        );

        ret.unwrap()
    }

    //see __TargetSreen
    fn __new_screen(&mut self) -> Screen {
        SessionBuilder::make_screen()
    }
}

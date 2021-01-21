use filetime::FileTime;
use std::cell::RefCell;
use std::cmp;
use std::fs;
use std::rc::Rc;

extern crate gaugen;
use gaugen::AbstractWrappedComponent;
use gaugen::WrappedComponent;

static SEGMENT_LENGTH: f32 = 15.0;

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct SelectionInstance {
    id: usize,
    selection_enabled: bool,
}

pub struct Selection<'a> {
    on_selection: Box<dyn Fn(usize, glutin::MouseButton) + 'a>,
}

impl<'a> Selection<'a> {
    pub fn dotted_line(
        ctx: &mut gaugen::frontend::PresentationContext,
        start: nalgebra::Vector2<f32>,
        stop: nalgebra::Vector2<f32>,
    ) {
        let segments_n = (((start - stop).norm() / SEGMENT_LENGTH) as u32) / 2 * 2;
        let new_length = (start - stop).norm() / (segments_n as f32);
        let segment = (stop - start).normalize() * new_length;
        let shift = ctx.time * 2.0 % 2.0;

        ctx.frame.path(
            |path| {
                for i in 0..segments_n / 2 {
                    let sstart = start + ((i as f32 * 2.0 + shift) * segment);
                    let sstop = match i == (segments_n as u32 / 2) - 1 && shift > 1.0 {
                        false => start + ((i as f32 * 2.0 + 1.0 + shift) * segment),
                        true => stop,
                    };

                    path.move_to((sstart.x, sstart.y));
                    path.line_to((sstop.x, sstop.y));
                }

                if shift > 1.0 {
                    let sstart = start;
                    let sstop = start + (shift - 1.0) * segment;

                    path.move_to((sstart.x, sstart.y));
                    path.line_to((sstop.x, sstop.y));
                }

                path.stroke(
                    nanovg::Color::from_rgba(0xff, 0xff, 0x20, 0xa2),
                    nanovg::StrokeOptions {
                        width: 3.0,
                        ..Default::default()
                    },
                );
            },
            Default::default(),
        );
    }
}

impl<'a> gaugen::Component<SelectionInstance, ()> for Selection<'a> {
    fn get_default_data(&self) -> Option<SelectionInstance> {
        None
    }

    fn max_children(&self) -> Option<u32> {
        Some(0)
    }

    fn get_name(&self) -> &'static str {
        "Selection"
    }

    fn init_instance(
        &self,
        __ctx: &mut gaugen::frontend::PresentationContext,
        __data: &SelectionInstance,
    ) {
    }

    fn draw(
        &self,
        ctx: &mut gaugen::frontend::PresentationContext,
        zone: nanovg::DrawZone,
        __children: &mut [gaugen::DrawChild],
        __internal_data: &mut (),
        data: &SelectionInstance,
    ) {
        if !data.selection_enabled {
            ctx.frame
                .mark_as_used((zone.bottom_right().x, zone.bottom_right().y));
            ctx.frame
                .mark_as_used((zone.top_left().x, zone.top_left().y));
            return;
        }

        Self::dotted_line(
            ctx,
            zone.bottom_right(),
            nalgebra::Vector2::new(zone.left(), zone.bottom()),
        );
        Self::dotted_line(
            ctx,
            nalgebra::Vector2::new(zone.left(), zone.bottom()),
            zone.top_left(),
        );
        Self::dotted_line(
            ctx,
            zone.top_left(),
            nalgebra::Vector2::new(zone.right(), zone.top()),
        );
        Self::dotted_line(
            ctx,
            nalgebra::Vector2::new(zone.right(), zone.top()),
            zone.bottom_right(),
        );
    }

    fn handle_event(
        &self,
        _drawn_location: &nanovg::DrawZone,
        event: &gaugen::Event,
        __internal_data: &mut (),
        data: &mut SelectionInstance,
    ) {
        match event {
            gaugen::Event::MouseClick(_, _, button) => {
                self.on_selection.as_ref()(data.id, *button);
            }
            _ => {}
        }
    }
}

fn main() {
    let view_path = "resources/playground.json";
    let floating_area: Rc<
        Box<dyn gaugen::Component<gaugen::geometry_components::FloatingAreaInstance, ()>>,
    > = Rc::new(Box::new(gaugen::geometry_components::FloatingArea {}));

    let absolute_coordinate = gaugen::geometry_components::CoordinateUnit::Pixels;
    let origin_middle = gaugen::geometry_components::CoordinateOrigin::FromMiddle;

    gaugen::session::SessionBuilder::new()
        .register_components(gaugen::basic_components::components())
        .register_components(gaugen::geometry_components::components())
        .init(|session: &mut gaugen::session::Session| {
            let get_modtime = || {
                let metadata = fs::metadata(view_path).unwrap();
                let mtime = FileTime::from_last_modification_time(&metadata);
                mtime.seconds()
            };

            let mut exit = false;
            let mut recursion_level: i32 = 0;

            while !exit {
                let view = session.new_view(view_path);
                let last_modtime = get_modtime();

                let mut view = match view {
                    Some(view) => view,
                    _ => continue,
                };

                view.into_inner(|mut inner_view| {
                    let floating_area = session.instantiate_component(
                        &mut inner_view,
                        1,
                        None,
                        floating_area.clone(),
                    );

                    let selections: RefCell<Vec<&WrappedComponent<SelectionInstance, ()>>> =
                        RefCell::new(Vec::new());

                    let clicks: Rc<RefCell<(Vec<usize>, glutin::MouseButton)>> =
                        Rc::new(RefCell::new((Vec::new(), glutin::MouseButton::Left))); // fixme:
                    let mut previous_clicks: Vec<usize> = Vec::new();

                    let __clicks = clicks.clone();

                    let selection_component_type: Box<
                        dyn gaugen::Component<SelectionInstance, ()> + '_,
                    > = Box::new(Selection {
                        on_selection: Box::new(move |id: usize, button: glutin::MouseButton| {
                            let mut c = __clicks.borrow_mut();
                            c.0.push(id);
                            c.1 = button;
                        }),
                    });

                    let selection_component_type = Rc::new(selection_component_type);

                    loop {
                        let mut selections = selections.borrow_mut();
                        let modtime = get_modtime();

                        if last_modtime != modtime {
                            break;
                        }

                        let hooks = gaugen::Hooks::new();

                        while selections.len() < inner_view.get_layer(0).components.len() {
                            let selection = session.instantiate_component(
                                &mut inner_view,
                                1,
                                Some(SelectionInstance {
                                    id: selections.len(),
                                    selection_enabled: false,
                                }),
                                selection_component_type.clone(),
                            );

                            selections.push(selection);
                            floating_area.borrow_mut().add_child(selection);
                            floating_area
                                .storage
                                .borrow_mut()
                                .public_data
                                .positions
                                .push((absolute_coordinate, origin_middle, 0.0, 0.0, 0.0, 0.0));
                        }

                        let components_no = inner_view.get_layer(0).components.len();

                        {
                            let floating_positions =
                                &mut floating_area.storage.borrow_mut().public_data.positions;

                            for i in 0..components_no {
                                let drawn_zone = inner_view.get_layer(0).components[i]
                                    .borrow()
                                    .get_drawn_location()
                                    .clone();

                                floating_positions[i] = (
                                    absolute_coordinate,
                                    origin_middle,
                                    drawn_zone.m.x,
                                    drawn_zone.m.y,
                                    drawn_zone.size.x,
                                    drawn_zone.size.y,
                                );
                            }
                        }

                        if clicks.borrow().0.len() != 0 {
                            let button = clicks.borrow().1;

                            if *clicks.borrow().0 == previous_clicks {
                                match button {
                                    glutin::MouseButton::Right => {
                                        recursion_level = recursion_level - 1
                                    }
                                    glutin::MouseButton::Left => {
                                        recursion_level = recursion_level + 1
                                    }
                                    _ => {}
                                }
                            } else {
                                previous_clicks = clicks.borrow().0.clone();
                            }

                            recursion_level = cmp::max(recursion_level, 0);
                            recursion_level =
                                cmp::min(recursion_level, (clicks.borrow().0.len() - 1) as i32);

                            let selected_id = clicks.borrow().0[recursion_level as usize];

                            for i in 0..components_no {
                                selections[i]
                                    .storage
                                    .borrow_mut()
                                    .public_data
                                    .selection_enabled = i == selected_id;
                            }
                        }

                        drop(selections);
                        clicks.borrow_mut().0.clear();

                        if !session.draw_inner(
                            &inner_view,
                            &gaugen::frontend::DarkPalette {},
                            &hooks,
                        ) {
                            exit = true;
                            break; // handle window being closed, etc.
                        }
                    }
                });
            }
        });
}

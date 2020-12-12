use crate::frontend::*;
use crate::*;
use nalgebra as na;

use nanovg::{Alignment, Color, Direction, Frame, StrokeOptions, TextOptions, Winding};

use na::{UnitQuaternion, Vector2, Vector3};
use std::f32::consts;

const CGA: f32 = consts::PI / 8.0;

fn format_float(v: f32, decimals: u32) -> String {
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

            if decimals == 0 {
                break;
            }
        }

        ret.push(c);
    }

    if counter == decimals && decimals != 0 {
        ret.push('.');
    }

    for _ in 0..counter {
        ret.push('0');
    }

    ret
}

// =========================== ROTATIONAL INDICATOR ===========================

pub struct RotationalIndicator {}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct RotationalIndicatorData {
    pub precision: u32,
    pub unit: String,
    pub caption: String,
    pub value: f32,
    pub value_min: f32,
    pub value_ranges: Vec<(f32, Status)>,
}

impl Component<RotationalIndicatorData, ()> for RotationalIndicator {
    fn draw(
        &self,
        ctx: &mut PresentationContext,
        zone: DrawZone,
        __children: &mut [DrawChild],
        __internal_data: &mut (),
        data: &RotationalIndicatorData,
    ) {
        let ref palette = ctx.resources.palette;
        let ref mut frame = ctx.frame;

        let zone = zone.constraint_to_aspect(Some(1.15));
        let base_radius = zone.size.x / 2.4;
        let base_thickness = base_radius / 10.0;
        let ymo = base_radius / -5.5; //y middle offset

        let value_max = data.value_ranges[data.value_ranges.len() - 1].0;

        let normalize = |value: f32| {
            if value < data.value_min {
                0.001
            } else if value > value_max {
                1.0
            } else {
                (value - data.value_min) / (value_max - data.value_min)
            }
        };

        let mut smartarc = |p0: f32,
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
                        (zone.m.x, zone.m.y - ymo),
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

        let mut last_range_end = normalize(data.value_min);

        let mut value_status = Status::Error;

        let nvalue = normalize(data.value);

        for range_end in &data.value_ranges {
            let current_range_end = normalize(range_end.0);
            smartarc(
                last_range_end,
                current_range_end,
                base_radius * 1.13,
                base_thickness,
                palette.status_to_color(range_end.1),
                Color::from_rgba(0, 0, 0, 0),
            );

            if nvalue >= last_range_end && nvalue < current_range_end {
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
            palette.status_to_color_bg(Status::Ok),
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
            line_max_width: zone.size.x,
            ..Default::default()
        };

        let text_opts_value = TextOptions {
            color: ctx.resources.palette.status_to_color_font(value_status),
            size: base_radius / 1.55,
            align: Alignment::new().center().middle(),
            line_height: base_radius / 2.5,
            line_max_width: zone.size.x,
            ..Default::default()
        };

        if value_status != Status::Error || ctx.time * 2.0 - ((ctx.time * 2.0) as i32 as f32) < 0.66
        {
            ctx.frame.text_box(
                ctx.resources.font,
                (zone.left(), zone.m.y + base_radius / 1.5 - ymo),
                &data.caption,
                text_opts_caption,
            );
        }
        ctx.frame.text_box(
            ctx.resources.font,
            (zone.left(), zone.m.y - base_radius / 10.0 - ymo),
            format_float(data.value, data.precision) + &data.unit,
            text_opts_value,
        );
    }

    fn init_instance(&self, __ctx: &mut PresentationContext, __data: &RotationalIndicatorData) {}

    fn get_default_data(&self) -> Option<RotationalIndicatorData> {
        Some(RotationalIndicatorData {
            precision: 1,
            unit: "".to_string(),
            caption: "".to_string(),
            value: 50.0,
            value_min: 0.0,
            value_ranges: vec![(100.0, Status::Ok)],
        })
    }

    fn max_children(&self) -> Option<u32> {
        Some(0)
    }

    fn get_name(&self) -> &'static str {
        "RotationalIndicator"
    }
}

// =========================== TEXT FIELD ===========================

pub struct TextField {}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct TextFieldData {
    pub text: String,
    pub front_color: SerializableColor,
    pub back_color: SerializableColor,
}

impl Component<TextFieldData, f32> for TextField {
    fn draw(
        &self,
        ctx: &mut PresentationContext,
        zone: DrawZone,
        __children: &mut [DrawChild],
        aspect: &mut f32,
        data: &TextFieldData,
    ) {
        let zone = zone.constraint_to_aspect(Some(*aspect));

        ctx.frame.path(
            |path| {
                path.rect((zone.left(), zone.top()), (zone.size.x, zone.size.y));
                path.fill(data.back_color.color, Default::default());
            },
            Default::default(),
        );
        let text_opts = TextOptions {
            color: data.front_color.color,
            size: zone.size.y * 1.0,
            align: Alignment::new().center().middle(),
            line_height: zone.size.y * 1.0,
            line_max_width: zone.size.x * 1.0,
            ..Default::default()
        };

        ctx.frame.text_box(
            ctx.resources.font,
            (zone.left(), zone.m.y),
            &data.text,
            text_opts,
        );
    }

    fn init_instance(&self, ctx: &mut PresentationContext, data: &TextFieldData) -> f32 {
        let bounds = ctx.frame.text_box_bounds(
            ctx.resources.font,
            (0.0, 0.0),
            data.text.as_str(),
            nanovg::TextOptions::default(),
        );

        let w = bounds.max_x - bounds.min_x;
        let h = bounds.max_y - bounds.min_y;

        w / h
    }

    fn get_default_data(&self) -> Option<TextFieldData> {
        Some(TextFieldData {
            text: "<Placeholder>".to_string(),
            front_color: SerializableColor {
                color: Color::from_rgb(0x80, 0x80, 0x80),
            },
            back_color: SerializableColor {
                color: Color::from_rgb(0x0, 0x0, 0x60),
            },
        })
    }

    fn max_children(&self) -> Option<u32> {
        Some(0)
    }

    fn get_name(&self) -> &'static str {
        "TextField"
    }
}

// =========================== SPATIAL SITUATION INDICATOR ===========================

struct SpatialSituationIndicator {}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
struct SpatialSituationIndicatorData {
    projection_zoom: f32,
    yaw: f32,
    pitch: f32,
    roll: f32,
}

trait DegreeRadConversions {
    fn rad(&self) -> f32;
    fn deg(&self) -> f32;
}

impl DegreeRadConversions for f32 {
    fn rad(&self) -> f32 {
        self / 180.0 * consts::PI
    }

    fn deg(&self) -> f32 {
        self / consts::PI * 180.0
    }
}

impl SpatialSituationIndicator {
    fn projection(
        &self,
        p: Vector2<f32>,
        o: &nalgebra::UnitQuaternion<f32>,
        zoom: f32,
        overhead: f32,
    ) -> Option<Vector2<f32>> {
        let v3 = UnitQuaternion::from_euler_angles(0.0, p.y.rad(), p.x.rad())
            * Vector3::new(1.0, 0.0, 0.0);
        let out = o.inverse() * v3;

        let mag = Vector2::new(out.y, out.z).magnitude();

        match out.x > 0.0 && mag <= overhead * 0.9 / zoom {
            true => Some(Vector2::new(out.y * zoom / 2.0, out.z * zoom / 2.0)),
            _ => None,
        }
    }

    pub fn draw_line<F>(
        &self,
        frame: &mut Frame,
        zone: &DrawZone,
        o: &nalgebra::UnitQuaternion<f32>,
        zoom: f32,
        p1: Vector2<f32>,
        p2: Vector2<f32>,
        path_style: F,
    ) where
        F: Fn(&mut nanovg::Path),
    {
        match (
            self.projection(p1, o, zoom, 0.97),
            self.projection(p2, o, zoom, 0.96),
        ) {
            (Some(tp1), Some(tp2)) => {
                frame.path(
                    |path| {
                        let from = (
                            tp1.x * zone.size.x + zone.m.x,
                            tp1.y * zone.size.y + zone.m.y,
                        );
                        let to = (
                            tp2.x * zone.size.x + zone.m.x,
                            tp2.y * zone.size.y + zone.m.y,
                        );
                        path.move_to(from);
                        path.line_to(to);
                        path_style(path);
                    },
                    Default::default(),
                );
            }
            _ => {}
        }
    }

    pub fn draw_text(
        &self,
        ctx: &mut PresentationContext,
        zone: &DrawZone,
        o: &nalgebra::UnitQuaternion<f32>,
        zoom: f32,
        t: String,
        p: Vector2<f32>,
    ) {
        let linelen = zone.size.y / 2.5;
        let text_opts_value = TextOptions {
            color: Color::from_rgba(255, 255, 255, 255),
            size: zone.size.y / 20.0,
            align: Alignment::new().center().middle(),
            line_height: zone.size.y / 20.0,
            line_max_width: linelen,
            ..Default::default()
        };

        match self.projection(p, o, zoom, 0.85) {
            Some(tp) => ctx.frame.text_box(
                ctx.resources.font,
                (
                    tp.x * zone.size.x + zone.m.x - linelen / 2.0,
                    tp.y * zone.size.y + zone.m.y,
                ),
                t,
                text_opts_value,
            ),
            _ => {}
        }
    }

    pub fn draw_ffd(&self, ctx: &mut PresentationContext, zone: &DrawZone) {
        let unit = zone.size.y / 20.0;
        ctx.frame.path(
            |path| {
                path.move_to((zone.m.x - 2.0 * unit, zone.m.y));
                path.line_to((zone.m.x - 0.66 * unit, zone.m.y));
                path.line_to((zone.m.x, zone.m.y + unit));
                path.line_to((zone.m.x + 0.66 * unit, zone.m.y));
                path.line_to((zone.m.x + 2.0 * unit, zone.m.y));

                path.circle((zone.m.x, zone.m.y), 1.0);
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
}

impl Component<SpatialSituationIndicatorData, ()> for SpatialSituationIndicator {
    fn max_children(&self) -> Option<u32> {
        Some(0)
    }

    fn get_name(&self) -> &'static str {
        "SpatialSituationIndicator"
    }

    fn get_default_data(&self) -> Option<SpatialSituationIndicatorData> {
        Some(SpatialSituationIndicatorData {
            projection_zoom: 1.5,
            yaw: 0.0,
            pitch: 0.0,
            roll: 0.0,
        })
    }
    fn init_instance(
        &self,
        __ctx: &mut PresentationContext,
        __data: &SpatialSituationIndicatorData,
    ) {
    }
    fn draw(
        &self,
        ctx: &mut PresentationContext,
        zone: DrawZone,
        __children: &mut [DrawChild],
        __internal_data: &mut (),
        public_data: &SpatialSituationIndicatorData,
    ) {
        let orientation = Vector3::new(public_data.roll, public_data.pitch, public_data.yaw);

        let orientation_quat =
            UnitQuaternion::from_euler_angles(public_data.roll, public_data.pitch, public_data.yaw);

        ctx.frame.path(
            |path| {
                path.circle((zone.m.x, zone.m.y), 1.0 * zone.size.x / 2.0);
                path.circle((zone.m.x, zone.m.y), 0.9 * zone.size.x / 2.0);
                path.stroke(
                    Color::from_rgba(0xa0, 0xa0, 0xa0, 0xa0),
                    StrokeOptions {
                        width: 3.0,
                        ..Default::default()
                    },
                );
            },
            Default::default(),
        );

        //draw vertical ladder
        for i in -22..22 {
            let h = (i * 5) as f32;
            let width = match i == 0 {
                true => 25.0 * (public_data.pitch * 3.0).cos(),
                false => 5.0,
            };
            let p1 = Vector2::new(width + orientation.z.deg(), h);
            let p2 = Vector2::new(-width + orientation.z.deg(), h);
            let p3 = Vector2::new(7.0 + orientation.z.deg(), h);

            self.draw_line(
                &mut ctx.frame,
                &zone,
                &orientation_quat,
                public_data.projection_zoom,
                p1,
                p2,
                |path| {
                    path.stroke(
                        Color::from_rgba(0x50, 0x50, 0x50, 0xff),
                        StrokeOptions {
                            width: 1.5,
                            ..Default::default()
                        },
                    );
                },
            );

            if i != 0 && i * 5 <= 90 {
                self.draw_text(
                    ctx,
                    &zone,
                    &orientation_quat,
                    public_data.projection_zoom,
                    (i * 5).to_string(),
                    p3,
                )
            }
        }

        let spacing = match orientation.y.deg().abs() > 60.0 {
            true => 30,
            _ => 10,
        };

        let ladder_height = match orientation.y.deg().abs() < 75.0 {
            true => orientation.y.deg(),
            _ => orientation.y.deg().signum() * 75.0,
        };

        //draw horizontal ladder
        for i in 0 / spacing..360 / spacing {
            let y = (i * spacing) as f32;
            let p1 = Vector2::new(y, 2.0 + ladder_height);
            let p2 = Vector2::new(y, -2.0 + ladder_height);
            let p3 = Vector2::new(y, -4.0 + ladder_height);

            self.draw_line(
                &mut ctx.frame,
                &zone,
                &orientation_quat,
                public_data.projection_zoom,
                p1,
                p2,
                |path| {
                    path.stroke(
                        Color::from_rgba(0x50, 0x50, 0x50, 0xff),
                        StrokeOptions {
                            width: 1.5,
                            ..Default::default()
                        },
                    );
                },
            );

            self.draw_text(
                ctx,
                &zone,
                &orientation_quat,
                public_data.projection_zoom,
                (i * spacing).to_string(),
                p3,
            );
        }

        self.draw_ffd(ctx, &zone);
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
enum GeometryTestType {
    Line,
    Circle,
    Arc(f32, f32),
}

struct GeometryTester;

impl Component<GeometryTestType, ()> for GeometryTester {
    fn max_children(&self) -> Option<u32> {
        Some(0)
    }

    fn get_name(&self) -> &'static str {
        "GeometryTester"
    }

    fn get_default_data(&self) -> Option<GeometryTestType> {
        Some(GeometryTestType::Line)
    }

    fn init_instance(&self, __ctx: &mut PresentationContext, __data: &GeometryTestType) {}

    fn draw(
        &self,
        ctx: &mut PresentationContext,
        zone: DrawZone,
        __children: &mut [DrawChild],
        __internal_data: &mut (),
        public_data: &GeometryTestType,
    ) {
        match public_data {
            GeometryTestType::Line => ctx.frame.path(
                |path| {
                    path.move_to((zone.m.x, zone.m.y));
                    path.line_to((zone.m.x + zone.size.x / 4.0, zone.m.y - zone.size.y / 6.0));
                    path.line_to((zone.m.x - zone.size.x / 4.0, zone.m.y - zone.size.y / 3.0));
                    path.line_to((zone.m.x, zone.m.y + zone.size.y / 3.0));

                    path.stroke(
                        Color::from_rgba(0xa0, 0xa0, 0xa0, 0xa0),
                        StrokeOptions {
                            width: 3.0,
                            ..Default::default()
                        },
                    );
                },
                Default::default(),
            ),
            GeometryTestType::Circle => ctx.frame.path(
                |path| {
                    path.circle(
                        (zone.m.x - zone.size.x / 4.0, zone.m.y - zone.size.y / 3.0),
                        zone.size.x / 5.0,
                    );

                    path.stroke(
                        Color::from_rgba(0xa0, 0xa0, 0xa0, 0xa0),
                        StrokeOptions {
                            width: 3.0,
                            ..Default::default()
                        },
                    );
                },
                Default::default(),
            ),
            GeometryTestType::Arc(begin, end) =>  ctx.frame.path(|path| {
                path.arc(
                    (zone.m.x, zone.m.y),
                    zone.size.x / 3.0,
                    *begin,
                    *end,
                    Winding::Direction(Direction::CounterClockwise),
                );

                path.stroke(
                    Color::from_rgba(0xa0, 0xa0, 0xa0, 0xa0),
                    StrokeOptions {
                        width: 3.0,
                        ..Default::default()
                    },
                );
            },
            Default::default()),
        };
    }
}

pub fn components() -> impl Fn(&mut Manager) {
    |manager: &mut Manager| {
        let rt = Box::new(RotationalIndicator {});
        let ssi = Box::new(SpatialSituationIndicator {});
        let textfield = Box::new(TextField {});
        let geometry_tester = Box::new(GeometryTester {});

        manager.register_component_type(rt);
        manager.register_component_type(textfield);
        manager.register_component_type(ssi);
        manager.register_component_type(geometry_tester);
    }
}

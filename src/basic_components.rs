use crate::frontend::*;
use crate::gaugen::*;

use nanovg::{
    Alignment, Clip, Color, Context, Direction, Font, Frame, Gradient, Image, ImagePattern,
    Intersect, LineCap, LineJoin, PathOptions, Scissor, Solidity, StrokeOptions, TextOptions,
    Transform, Winding,
};

use na::{UnitQuaternion, Vector2, Vector3};
use std::f32::consts;

const CGA: f32 = consts::PI / 8.0;

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

    if counter == decimals {
        ret.push('.');
    }

    for _ in 0..counter {
        ret.push('0');
    }

    ret
}

// =========================== ROTATIONAL INDICATOR ===========================

pub struct RotationalIndicator {}

#[derive(serde::Serialize, serde::Deserialize)]
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
        ctx: &PresentationContext,
        zone: DrawZone,
        __children: &mut [Box<dyn FnMut(DrawZone) + '_>],
        __internal_data: &mut (),
        data: &RotationalIndicatorData,
    ) {
        let base_radius = zone.size.x / 2.4;
        let base_thickness = base_radius / 10.0;
        let ymo = base_radius / -5.5; //y middle offset

        let value_max = data.value_ranges[data.value_ranges.len() - 1].0;

        let normalize = |value: f32| {
            if value < data.value_min {
                0.0
            } else if value > value_max {
                1.0
            } else {
                (value - data.value_min) / (value_max - data.value_min)
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

            ctx.frame.path(
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
                ctx.resources.palette.as_ref().StatusToColor(range_end.1),
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
            ctx.resources.palette.as_ref().StatusToColorBg(Status::Ok),
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
            color: ctx.resources.palette.as_ref().StatusToColorFont(value_status),
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
            FormatFloat(data.value, data.precision) + &data.unit,
            text_opts_value,
        );
    }

    fn init_instance(
        &self,
        __ctx: &PresentationContext,
        __data: &RotationalIndicatorData,
        __sizes: &[ControlGeometry],
    ) -> AfterInit<()> {
        AfterInit {
            aspect: Some(1.15),
            internal_data: (),
        }
    }

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

#[derive(serde::Deserialize)]
pub struct TextFieldData {
    pub text: String,
    pub front_color: SerializableColor,
    pub back_color: SerializableColor,
}

impl Component<TextFieldData, ()> for TextField {
    fn draw(
        &self,
        ctx: &PresentationContext,
        zone: DrawZone,
        __children: &mut [Box<dyn FnMut(DrawZone) + '_>],
        __internal_data: &mut (),
        data: &TextFieldData,
    ) {
        ctx.frame.path(
            |path| {
                path.rect(
                    (zone.left(), zone.bottom()),
                    (zone.size.x, zone.size.y)
                );
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

    fn init_instance(
        &self,
        ctx: &PresentationContext,
        data: &TextFieldData,
        __sizes: &[ControlGeometry],
    ) -> AfterInit<()> {
        let bounds = ctx.frame.text_box_bounds(
            ctx.resources.font,
            (0.0, 0.0),
            data.text.as_str(),
            nanovg::TextOptions::default(),
        );

        let w = bounds.max_x - bounds.min_x;
        let h = bounds.max_y - bounds.min_y;

        AfterInit {
            aspect: Some(w / h),
            internal_data: (),
        }
    }

    fn get_default_data(&self) -> Option<TextFieldData> {
        Some(TextFieldData {
            text: "<Placeholder>".to_string(),
            front_color: SerializableColor{
                color: Color::from_rgb(0x80, 0x80, 0x80)
            },
            back_color: SerializableColor{
                color: Color::from_rgb(0x0, 0x0, 0x60)
            }
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
/*
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
*/

pub fn register_basic_components(manager: &mut Manager) {
    let rt = Box::new(RotationalIndicator {});
    let textfield = Box::new(TextField{});

    manager.register_component_type(rt);
    manager.register_component_type(textfield);
}

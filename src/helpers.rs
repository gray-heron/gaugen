use crate::frontend;
use nalgebra;
use nanovg;

static SEGMENT_LENGTH: f32 = 15.0;

pub fn dotted_line(
    ctx: &mut frontend::PresentationContext,
    start: nalgebra::Vector2<f32>,
    stop: nalgebra::Vector2<f32>,
) {
    let segments_n = (((start - stop).norm() / SEGMENT_LENGTH) as u32) / 2 * 2;
    let new_length = (start - stop).norm() / (segments_n as f32);
    let segment = (stop - start).normalize() * new_length;
    let shift = ctx.time * 2.0 % 2.0;

    ctx.frame.path(|path|{    
        for i in 0..segments_n / 2 {
            let sstart = start + ((i as f32 * 2.0 + shift) * segment);
            let sstop = match i == (segments_n as u32 / 2)-1 && shift > 1.0{
                false => start + ((i as f32 * 2.0 + 1.0 + shift) * segment),
                true => stop,
            };

            path.move_to((sstart.x, sstart.y));
            path.line_to((sstop.x, sstop.y));
        };

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
    }, Default::default());
}

pub fn dotted_zone(
    ctx: &mut frontend::PresentationContext,
    zone: &nanovg::DrawZone
) {
    dotted_line(ctx, zone.bottom_right(), nalgebra::Vector2::new(zone.left(), zone.bottom()));
    dotted_line(ctx, nalgebra::Vector2::new(zone.left(), zone.bottom()), zone.top_left());
    dotted_line(ctx, zone.top_left(), nalgebra::Vector2::new(zone.right(), zone.top()));
    dotted_line(ctx, nalgebra::Vector2::new(zone.right(), zone.top()), zone.bottom_right());
}
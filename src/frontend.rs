use nanovg::{Color, Font};

#[derive(serde::Serialize, serde::Deserialize, PartialEq, Copy, Clone)]
pub enum Status {
    Ok,
    Warning,
    Error,
}

pub trait Palette {
    fn StatusToColor(&self, s: Status) -> Color;
    fn StatusToColorBg(&self, s: Status) -> Color;
}

pub struct DarkPalette {}

impl Palette for DarkPalette {
    fn StatusToColor(&self, s: Status) -> Color {
        match s {
            Status::Ok => Color::from_rgba(0, 160, 0, 255),
            Status::Warning => Color::from_rgba(250, 120, 0, 255),
            Status::Error => Color::from_rgba(200, 0, 0, 255),
        }
    }

    fn StatusToColorBg(&self, s: Status) -> Color {
        match s {
            Status::Ok => Color::from_rgba(30, 30, 40, 255),
            Status::Warning => Color::from_rgba(0xbe, 0x55, 0, 255),
            Status::Error => Color::from_rgba(200 / 2, 0, 0, 255),
        }
    }
}

pub struct Resources<'a> {
    pub palette: Box<dyn Palette>,
    pub font: Font<'a>,
}

pub struct PresentationContext<'a> {
    pub frame: &'a nanovg::Frame<'a>,
    pub time: f32,
    pub resources: &'a Resources<'a>,
}
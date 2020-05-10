use nanovg::{Color, Font};
use std::fmt;

#[derive(serde::Serialize, serde::Deserialize, PartialEq, Copy, Clone)]
pub enum Status {
    Ok,
    Warning,
    Error,
}

#[derive(Clone)]
pub struct SerializableColor {
    pub color: Color,
}

struct SerializableColorVisitor;

impl<'de> serde::de::Visitor<'de> for SerializableColorVisitor {
    type Value = SerializableColor;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("String encoding a color in AARRGGBB format")
    }

    fn visit_borrowed_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        match u32::from_str_radix(value, 16) {
            Ok(color) => Ok(SerializableColor {
                color: Color::from_rgba(
                    ((color >> 16) & 0xff) as u8,
                    ((color >> 8) & 0xff) as u8,
                    ((color >> 0) & 0xff) as u8,
                    ((color >> 24) & 0xff) as u8,
                ),
            }),
            Err(_) => Err(E::custom(format!("Color is in incorrect format"))),
        }
    }


    fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        match u32::from_str_radix(value.as_str(), 16) {
            Ok(color) => Ok(SerializableColor {
                color: Color::from_rgba(
                    ((color >> 16) & 0xff) as u8,
                    ((color >> 8) & 0xff) as u8,
                    ((color >> 0) & 0xff) as u8,
                    ((color >> 24) & 0xff) as u8,
                ),
            }),
            Err(_) => Err(E::custom(format!("Color is in incorrect format"))),
        }
    }
}

impl<'de> serde::Deserialize<'de> for SerializableColor {
    fn deserialize<D>(deserializer: D) -> Result<SerializableColor, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_string(SerializableColorVisitor)
    }
}

impl serde::Serialize for SerializableColor {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        let a = (self.color.alpha() * 255.0) as u8;
        let r = (self.color.red() * 255.0) as u8;
        let g = (self.color.green() * 255.0) as u8;
        let b = (self.color.blue() * 255.0) as u8;

        serializer.serialize_str(format!("{:02x}{:02x}{:02x}{:02x}", a, r, g, b).as_str())
    }
}


pub trait Palette {
    fn status_to_color(&self, s: Status) -> Color;
    fn status_to_color_font(&self, s: Status) -> Color;
    fn status_to_color_bg(&self, s: Status) -> Color;
}

pub struct DarkPalette {}

impl Palette for DarkPalette {
    fn status_to_color(&self, s: Status) -> Color {
        match s {
            Status::Ok => Color::from_rgba(0, 160, 0, 255),
            Status::Warning => Color::from_rgba(250, 120, 0, 255),
            Status::Error => Color::from_rgba(200, 0, 0, 255),
        }
    }

    fn status_to_color_font(&self, s: Status) -> Color {
        match s {
            Status::Ok => Color::from_rgba(255, 255, 255, 255),
            Status::Warning => Color::from_rgba(240, 180, 0, 255),
            Status::Error => Color::from_rgba(255, 50, 50, 255),
        }
    }

    fn status_to_color_bg(&self, s: Status) -> Color {
        match s {
            Status::Ok => Color::from_rgba(30, 30, 40, 255),
            Status::Warning => Color::from_rgba(30, 30, 40, 255),
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

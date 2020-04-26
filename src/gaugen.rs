use nalgebra::Vector2;
use nanovg::Frame;
use serde;
use serde_json;
use std::collections::HashMap;
//RunTime Parametric Structures

#[derive(Copy, Clone, PartialEq)]
enum Status {
    Ok,
    Warning,
    Error,
}

pub struct DrawZone {
    pub m: Vector2<f32>,
    pub size: Vector2<f32>
}

impl DrawZone{
    pub fn left(&self) -> f32 {
        self.m.x - self.size.x / 2.0
    }

    pub fn right(&self) -> f32 {
        self.m.x + self.size.x / 2.0
    }

    pub fn top(&self) -> f32 {
        self.m.y + self.size.y / 2.0
    }

    pub fn bottom(&self) -> f32 {
        self.m.y - self.size.y / 2.0
    }

    pub fn from_rect(bottom_left: Vector2<f32>, top_right: Vector2<f32>) -> DrawZone{
        DrawZone {
            m: (bottom_left + top_right) / 2.0,
            size: top_right - bottom_left
        }
    }
}

pub struct Resources {}

pub struct PresentationContext<'a> {
    frame: &'a nanovg::Frame<'a>,
    time: f32,
    fonts: &'a Resources,
}

//pub trait Deserializable {
//    fn from_json(json_str: &serde_json::Value) -> Self;
//}

pub struct ControlGeometry {
    aspect: Option<f32>,
    size_preference: Option<f32>,
}

pub trait Component<TComponentData>
where
    TComponentData: serde::de::DeserializeOwned,
{
    fn max_children(&self) -> Option<u32>;
    fn get_name(&self) -> &'static str;

    fn get_size(&self) -> ControlGeometry;

    fn draw(
        &self,
        ctx: &mut PresentationContext,
        zone: DrawZone,
        children: &[(ControlGeometry, Box<dyn FnMut(DrawZone) + '_>)],
        data: &TComponentData,
    );
}

struct ControlInstance {
    draw: Box<
        dyn Fn(
            &mut PresentationContext,
            DrawZone,
            &[(ControlGeometry, Box<dyn FnMut(DrawZone) + '_>)],
        ),
    >,
    get_size: Box<dyn Fn() -> ControlGeometry>,
}

struct TreeComponent {
    children: Vec<TreeComponent>,
    control: ControlInstance,
}

impl TreeComponent {
    fn draw(&self, ctx: &mut PresentationContext, zone: DrawZone) {
        let sizes_n_draws: Vec<(ControlGeometry, Box<dyn FnMut(DrawZone)>)> = Vec::new();
        for child in &self.children {
            let b = Box::new(|z: DrawZone| child.draw(ctx, z));
            sizes_n_draws.push((child.control.get_size.as_ref()(), b));
        }
        self.control.draw.as_ref()(ctx, zone, &sizes_n_draws[..]);
    }
}

struct Manager {
    controls_types:
        HashMap<&'static str, Box<dyn Fn(serde_json::Value) -> Option<ControlInstance>>>,
}

impl Manager {
    pub fn register_component_type<TComponentData>(
        &self,
        component: &'static dyn Component<TComponentData>,
    ) where
        TComponentData: serde::de::DeserializeOwned,
    {
        let mk_instance = Box::new(|json: serde_json::Value| -> Option<ControlInstance> {
            let maybe_data = &TComponentData::deserialize(&json);

            match maybe_data {
                Ok(data) => {
                    Some(
                        ControlInstance {
                            draw:
                                Box::new(
                                    |ctx: &mut PresentationContext,
                                     zone: DrawZone,
                                     children: &[(
                                        ControlGeometry,
                                        Box<dyn FnMut(DrawZone)>,
                                    )]| {
                                        component.draw(ctx, zone, children, data);
                                    },
                                ),
                            get_size: Box::new(|| component.get_size()),
                        },
                    )
                }
                Err(_) => None,
            }
        });

        self.controls_types
            .insert(component.get_name(), mk_instance);
    }

    pub fn make_screen(&self, json: &String) -> Option<TreeComponent> {
        let data: serde_json::Value = serde_json::from_str(json).unwrap();

        self.build_tree(&data)
    }

    fn build_tree(&self, v: &serde_json::Value) -> Option<TreeComponent> {
        let make_control = self.controls_types[v["type"].as_str().unwrap()];
        let component_type = v["type"].as_str().unwrap();

        let mut children: Vec<TreeComponent> = Vec::new();

        for child in v["children"].as_array().unwrap() {
            children.push(self.build_tree(child).unwrap());
        }

        match make_control(v["data"]) {
            Some(control) => Some(TreeComponent {
                children: children,
                control: control,
            }),
            None => None,
        }
    }

    fn draw(&self, ctx: &mut PresentationContext, zone: DrawZone) {}
}

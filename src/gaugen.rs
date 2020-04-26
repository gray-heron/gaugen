use crate::frontend;

use nalgebra::Vector2;
use nanovg::Frame;
use serde;
use serde_json;
use std::collections::HashMap;
use std::fs;
use std::rc;

//RunTime Parametric Structures


pub struct DrawZone {
    pub m: Vector2<f32>,
    pub size: Vector2<f32>,
}

impl DrawZone {
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

    pub fn from_rect(bottom_left: Vector2<f32>, top_right: Vector2<f32>) -> DrawZone {
        DrawZone {
            m: (bottom_left + top_right) / 2.0,
            size: top_right - bottom_left,
        }
    }
}

//pub trait Deserializable {
//    fn from_json(json_str: &serde_json::Value) -> Self;
//}

pub struct ControlGeometry {
    pub aspect: Option<f32>,
    pub size_preference: Option<f32>,
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
        ctx: &frontend::PresentationContext,
        zone: DrawZone,
        children: &[(ControlGeometry, Box<dyn Fn(DrawZone) + '_>)],
        data: &TComponentData,
    );
}

struct ControlInstance {
    draw: Box<
        dyn Fn(
            &frontend::PresentationContext,
            DrawZone,
            &[(ControlGeometry, Box<dyn Fn(DrawZone) + '_>)],
        ),
    >,
    get_size: Box<dyn Fn() -> ControlGeometry>,
}

pub struct TreeComponent {
    children: Vec<TreeComponent>,
    control: ControlInstance,
    name: Option<String>,
}

impl TreeComponent {
    pub fn draw(&self, ctx: &frontend::PresentationContext, zone: DrawZone) {
        let mut sizes_n_draws: Vec<(ControlGeometry, Box<dyn Fn(DrawZone)>)> = Vec::new();
        for child in &self.children {
            let b = Box::new(move |z: DrawZone|{
                child.draw(ctx, z)
            });
            sizes_n_draws.push((child.control.get_size.as_ref()(), b));
        }
        self.control.draw.as_ref()(ctx, zone, &sizes_n_draws[..]);
    }
}

pub struct Manager {
    controls_types:
        HashMap<&'static str, Box<dyn Fn(&serde_json::Value) -> Option<ControlInstance>>>
}

impl Manager {
    pub fn register_component_type<TComponentData>(
        &mut self,
        component: Box<dyn Component<TComponentData>>,
    ) where
        TComponentData: serde::de::DeserializeOwned + 'static,
    {
        let stored_component = rc::Rc::new(component);
        let __stored_component = rc::Rc::clone(&stored_component);

        let mk_instance = Box::new(move |json: &serde_json::Value| -> Option<ControlInstance> {
            let __stored_component1 = rc::Rc::clone(&__stored_component);
            let __stored_component2 = rc::Rc::clone(&__stored_component);

            let maybe_data = TComponentData::deserialize(json);

            match maybe_data {
                Ok(data) => {
                    Some(
                        ControlInstance {
                            draw:
                                Box::new(
                                    move |ctx: &frontend::PresentationContext,
                                     zone: DrawZone,
                                     children: &[(
                                        ControlGeometry,
                                        Box<dyn Fn(DrawZone)>,
                                    )]| {
                                        __stored_component1.as_ref().draw(ctx, zone, children, &data);
                                    },
                                ),
                            get_size: Box::new(move || __stored_component2.as_ref().get_size()),
                        },
                    )
                }
                Err(_) => None,
            }
        });

        self.controls_types
            .insert(stored_component.as_ref().get_name(), mk_instance);
    }

    pub fn make_screen(&self, path_to_json: &str) -> Option<TreeComponent> {
        let json = fs::read_to_string(path_to_json).unwrap();
        let data: serde_json::Value = serde_json::from_str(&json).unwrap();

        self.build_tree(&data)
    }

    pub fn build_tree(&self, v: &serde_json::Value) -> Option<TreeComponent> {
        let make_control = &self.controls_types[v["type"].as_str().unwrap()];

        let mut children: Vec<TreeComponent> = Vec::new();

        match v["children"].as_array() {
            Some(json_children) => 
                for child in json_children {
                    children.push(self.build_tree(child).unwrap());
                },
            None => {}
        }

        match make_control(&v["data"]) {
            Some(control) => Some(TreeComponent {
                children: children,
                control: control,
                name: match v["name"].as_str() {
                    Some(s) => Some(s.to_string()),
                    None => None,
                },
            }),
            None => None,
        }
    }

    //pub fn draw(&self, ctx: &frontend::PresentationContext, zone: DrawZone) {}

    pub fn new() -> Manager{
        Manager{
            controls_types: HashMap::new()
        }
    }
}

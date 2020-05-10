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

    pub fn bottom_left(&self) -> Vector2<f32> {
        Vector2::new(self.left(), self.bottom())
    }

    pub fn top_right(&self) -> Vector2<f32> {
        Vector2::new(self.right(), self.top())
    }

    pub fn from_rect(bottom_left: Vector2<f32>, top_right: Vector2<f32>) -> DrawZone {
        DrawZone {
            m: (bottom_left + top_right) / 2.0,
            size: top_right - bottom_left,
        }
    }

    pub fn aspect(&self) -> f32 {
        self.size.x / self.size.y
    }
}

//pub trait Deserializable {
//    fn from_json(json_str: &serde_json::Value) -> Self;
//}

pub struct ControlGeometry {
    pub aspect: Option<f32>,
    pub size_preference: f32,
}

pub struct AfterInit<TPrivateData> {
    pub aspect: Option<f32>,
    pub internal_data: TPrivateData,
}

pub trait Component<TComponentPublicInstanceData, TComponentInternalInstanceData>
where
    TComponentPublicInstanceData: serde::de::DeserializeOwned,
{
    fn max_children(&self) -> Option<u32>;
    fn get_name(&self) -> &'static str;
    fn get_default_data(&self) -> Option<TComponentPublicInstanceData>;
    fn init_instance(
        &self,
        ctx: &frontend::PresentationContext,
        data: &TComponentPublicInstanceData,
        sizes: &[ControlGeometry],
    ) -> AfterInit<TComponentInternalInstanceData>;

    fn draw(
        &self,
        ctx: &frontend::PresentationContext,
        zone: DrawZone,
        children: &mut [Box<dyn FnMut(DrawZone) + '_>],
        internal_data: &mut TComponentInternalInstanceData,
        public_data: &TComponentPublicInstanceData,
    );
}

type WrappedInit = Box<
    dyn Fn(
        &frontend::PresentationContext,
        &serde_json::Value,
        &[ControlGeometry],
    ) -> Option<(WrappedDraw, ControlGeometry)>,
>;

type WrappedDraw = Box<
    dyn FnMut(
        &frontend::PresentationContext,
        DrawZone,
        &mut [Box<dyn FnMut(DrawZone) + '_>],
        &serde_json::Map<String, serde_json::Value>,
    ),
>;
type Hooks = HashMap<String, serde_json::Map<String, serde_json::Value>>;

pub struct TreeComponent {
    children: Vec<TreeComponent>,
    draw: WrappedDraw,
    name: Option<String>,
}

impl TreeComponent {
    pub fn draw(&mut self, ctx: &frontend::PresentationContext, zone: DrawZone, hooks: &Hooks) {
        let mut draws: Vec<Box<dyn FnMut(DrawZone)>> = Vec::new();
        for child in &mut self.children {
            let b = Box::new(move |z: DrawZone| child.draw(ctx, z, hooks));
            draws.push(b);
        }

        let no_hooks = serde_json::Map::new();

        let my_hooks = match &self.name {
            Some(name) => match hooks.get(name) {
                Some(hooks) => hooks,
                None => &no_hooks,
            },
            None => &no_hooks,
        };

        self.draw.as_mut()(ctx, zone, &mut draws[..], my_hooks);
    }
}

pub struct Manager {
    controls_types: HashMap<&'static str, WrappedInit>,
}

impl Manager {
    fn join_hooks<T>(value: &T, hooks: &serde_json::Map<String, serde_json::Value>) -> T
    where
        T: serde::ser::Serialize + serde::de::DeserializeOwned + Clone + 'static,
    {
        let mut serialized = match serde_json::to_value(value) {
            Ok(serialized) => serialized,
            _ => return value.clone(),
        };

        for hook in hooks {
            serialized[hook.0] = hook.1.clone();
        }

        match serde_json::from_value(serialized) {
            Ok(object) => object,
            Err(er) => {
                println!("Error while applying hook: {}", er);
                value.clone()
            }
        }
    }

    fn mk_init<T1, T2>(
        ctx: &frontend::PresentationContext,
        component_type: std::rc::Rc<Box<dyn Component<T1, T2>>>, // fixme Rc<Box> => Rc
        children: &[ControlGeometry],
        public_data: T1,
        size_preference: f32,
    ) -> (WrappedDraw, ControlGeometry)
    where
        T1: serde::ser::Serialize + serde::de::DeserializeOwned + Clone + 'static,
        T2: 'static,
    {
        let after_init =
            component_type
                .as_ref()
                .as_ref()
                .init_instance(ctx, &public_data, children);
        let mut internal_data = after_init.internal_data;

        match component_type.max_children() {
            Some(max) => assert!(children.len() <= (max as usize)),
            None => {}
        }

        (
            Box::new(
                move |ctx: &frontend::PresentationContext,
                      zone: DrawZone,
                      children: &mut [Box<dyn FnMut(DrawZone) + '_>],
                      my_hooks: &serde_json::Map<String, serde_json::Value>| {
                    if my_hooks.len() == 0 {
                        component_type.as_ref().as_ref().draw(
                            ctx,
                            zone,
                            children,
                            &mut internal_data,
                            &public_data,
                        );
                    } else {
                        let merged_data = Manager::join_hooks(&public_data, my_hooks);

                        component_type.as_ref().as_ref().draw(
                            ctx,
                            zone,
                            children,
                            &mut internal_data,
                            &merged_data,
                        );
                    }
                },
            ),
            ControlGeometry {
                aspect: after_init.aspect,
                size_preference: size_preference,
            },
        )
    }

    pub fn register_component_type<TComponentData, TPrivateComponentData>(
        &mut self,
        component: Box<dyn Component<TComponentData, TPrivateComponentData>>,
    ) where
        TComponentData: serde::ser::Serialize + serde::de::DeserializeOwned + Clone + 'static,
        TPrivateComponentData: 'static,
    {
        let stored_component = rc::Rc::new(component);
        let __stored_component = rc::Rc::clone(&stored_component);

        let mk_wrapped_init = Box::new(
            move |ctx: &frontend::PresentationContext,
                  json: &serde_json::Value,
                  children: &[ControlGeometry]|
                  -> Option<(WrappedDraw, ControlGeometry)> {

                let __stored_component2 = rc::Rc::clone(&__stored_component);

                let data = match TComponentData::deserialize(json) {
                    Ok(data) => data,
                    Err(_) => {
                        let default_data = __stored_component.as_ref().get_default_data()?;
                        match json.as_object() {
                            Some(hooks) => Manager::join_hooks(&default_data, hooks),
                            None => default_data
                        }                        
                    }
                };

                Some(Manager::mk_init(
                    ctx,
                    __stored_component2,
                    children,
                    data,
                    1.0,
                ))
            },
        );

        self.controls_types
            .insert(stored_component.as_ref().get_name(), mk_wrapped_init);
    }

    pub fn make_screen(
        &self,
        ctx: &frontend::PresentationContext,
        path_to_json: &str,
    ) -> Option<(TreeComponent, ControlGeometry)> {
        let json = fs::read_to_string(path_to_json).unwrap();
        let data: serde_json::Value = match serde_json::from_str(&json) {
            Ok(data) => data,
            Err(_) => return None,
        };

        self.build_tree(ctx, &data)
    }

    pub fn build_tree(
        &self,
        ctx: &frontend::PresentationContext,
        v: &serde_json::Value,
    ) -> Option<(TreeComponent, ControlGeometry)> {
        let mk_init = &self.controls_types[v["type"].as_str()?];

        let mut children: Vec<TreeComponent> = Vec::new();
        let mut children_geometries: Vec<ControlGeometry> = Vec::new();

        match v["children"].as_array() {
            Some(json_children) => {
                for json_child in json_children {
                    let child_n_geometry = self.build_tree(ctx, json_child)?;
                    children.push(child_n_geometry.0);
                    children_geometries.push(child_n_geometry.1);
                }
            }
            None => {}
        }

        match mk_init(ctx, &v["data"], &children_geometries) {
            Some((wrapped_draw, geometry)) => Some((
                TreeComponent {
                    children: children,
                    draw: wrapped_draw,
                    name: match v["name"].as_str() {
                        Some(s) => Some(s.to_string()),
                        None => None,
                    },
                },
                geometry,
            )),
            None => None,
        }
    }

    //pub fn draw(&self, ctx: &frontend::PresentationContext, zone: DrawZone) {}

    pub fn new() -> Manager {
        Manager {
            controls_types: HashMap::new(),
        }
    }
}

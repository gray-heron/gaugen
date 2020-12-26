pub mod basic_components;
pub mod frontend;
pub mod geometry_components;
pub mod session;
mod helpers;

use nalgebra::Vector2;
use serde;
use serde_json;
use std::collections::HashMap;
use std::fs;
use std::rc;
use nanovg::{DrawZone, Color, StrokeOptions};
use std::cell::RefCell;

//RunTime Parametric Structures

pub struct ControlGeometry {
    pub aspect: Option<f32>,
    pub size_preference: f32,
}

type DrawChild<'a> = Box<dyn FnMut(&mut frontend::PresentationContext, DrawZone) -> DrawZone + 'a>;

pub trait Component<TComponentPublicInstanceData, TComponentInternalInstanceData>
where
    TComponentPublicInstanceData: serde::de::DeserializeOwned,
{
    fn max_children(&self) -> Option<u32>; // None = no restrictions
    fn get_name(&self) -> &'static str;
    fn get_default_data(&self) -> Option<TComponentPublicInstanceData>;
    fn init_instance(
        &self,
        ctx: &mut frontend::PresentationContext,
        data: &TComponentPublicInstanceData,
    ) -> TComponentInternalInstanceData;

    fn draw(
        &self,
        ctx: &mut frontend::PresentationContext,
        zone: DrawZone,
        children: &mut [DrawChild],
        internal_data: &mut TComponentInternalInstanceData,
        public_data: &TComponentPublicInstanceData,
    );
}

type WrappedInit = Box<
    dyn Fn(
        &mut frontend::PresentationContext,
        &serde_json::Value,
        usize,
    ) -> Option<WrappedDraw>,
>;

type WrappedDraw = Box<
    dyn FnMut(
        &mut frontend::PresentationContext,
        DrawZone,
        &mut [DrawChild],
        &serde_json::Map<String, serde_json::Value>,
    ),
>;

pub type Hooks = HashMap<String, serde_json::Map<String, serde_json::Value>>;
pub type View = TreeComponent;

pub struct TreeComponent {
    children: Vec<TreeComponent>,
    draw: WrappedDraw,
    name: Option<String>
}

impl TreeComponent {
    fn shell_merge_bottom(stack: &mut Vec<DrawZone>, zone: &DrawZone) -> DrawZone {
        stack.last_mut().unwrap().convex_with_a_zone(zone);
        *stack.last().unwrap()
    }

    pub fn draw(&mut self, ctx: &mut frontend::PresentationContext, zone: DrawZone, hooks: &Hooks, layers: &RefCell<Vec<DrawZone>>) {
        let mut draws: Vec<Box<dyn FnMut(&mut frontend::PresentationContext, DrawZone) -> DrawZone>> =
            Vec::new();

        for child in &mut self.children {

            let b = Box::new(
                move |ctx: &mut frontend::PresentationContext, z: DrawZone| -> DrawZone {
                    TreeComponent::shell_merge_bottom(&mut ctx.shell_stack, &ctx.frame.shell_replace());
                    ctx.shell_stack.push(DrawZone::new_empty());

                    child.draw(ctx, z, hooks, layers);

                    let drawn = TreeComponent::shell_merge_bottom(&mut ctx.shell_stack, &ctx.frame.shell_replace());
                    ctx.shell_stack.pop();
                    ctx.shell_stack.last_mut().unwrap().convex_with_a_zone(&drawn);

                    layers.borrow_mut().push(drawn);
                    
                    drawn
                },
            );
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
        TreeComponent::shell_merge_bottom(&mut ctx.shell_stack, &ctx.frame.shell_replace());
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
        ctx: &mut frontend::PresentationContext,
        component_type: std::rc::Rc<Box<dyn Component<T1, T2>>>, // fixme Rc<Box> => Rc
        children_n: usize,
        public_data: T1,
        size_preference: f32,
    ) -> WrappedDraw
    where
        T1: serde::ser::Serialize + serde::de::DeserializeOwned + Clone + 'static,
        T2: 'static,
    {
        let mut internal_data = component_type
            .as_ref()
            .as_ref()
            .init_instance(ctx, &public_data);

        match component_type.max_children() {
            Some(max) => assert!(children_n <= (max as usize)),
            None => {}
        }

        Box::new(
            move |ctx: &mut frontend::PresentationContext,
                  zone: DrawZone,
                  children: &mut [DrawChild],
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
            move |ctx: &mut frontend::PresentationContext,
                  json: &serde_json::Value,
                  children_n: usize|
                  -> Option<WrappedDraw> {
                
                let __stored_component2 = rc::Rc::clone(&__stored_component);

                let data = match TComponentData::deserialize(json) {
                    Ok(data) => data,
                    Err(_) => {
                        let default_data = __stored_component.as_ref().get_default_data()?;
                        match json.as_object() {
                            Some(hooks) => Manager::join_hooks(&default_data, hooks),
                            None => default_data,
                        }
                    }
                };

                Some(Manager::mk_init(
                    ctx,
                    __stored_component2,
                    children_n,
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
        ctx: &mut frontend::PresentationContext,
        path_to_json: &str,
    ) -> Option<View> {
        let json = fs::read_to_string(path_to_json).unwrap();
        let data: serde_json::Value = match serde_json::from_str(&json) {
            Ok(data) => data,
            Err(_) => return None,
        };

        self.build_tree(ctx, &data)
    }

    pub fn build_tree(
        &self,
        ctx: &mut frontend::PresentationContext,
        v: &serde_json::Value,
    ) -> Option<View> {
        let mk_init = &self.controls_types[v["type"].as_str()?];

        let mut children: Vec<TreeComponent> = Vec::new();

        match v["children"].as_array() {
            Some(json_children) => {
                for json_child in json_children {
                    let child_n_geometry = self.build_tree(ctx, json_child)?;
                    children.push(child_n_geometry);
                }
            }
            None => {}
        }

        match mk_init(ctx, &v["data"], children.len()) {
            Some(wrapped_draw) => Some(
                TreeComponent {
                    children: children,
                    draw: wrapped_draw,
                    name: match v["name"].as_str() {
                        Some(s) => Some(s.to_string()),
                        None => None,
                    },
                }
            ),
            None => None,
        }
    }

    pub fn new() -> Manager {
        Manager {
            controls_types: HashMap::new(),
        }
    }
}

pub fn add_hook<T>(hooks: &mut Hooks, component: &str, property: &str, value: T)
where
    T: serde::ser::Serialize + serde::de::DeserializeOwned + Clone + 'static,
{
    if hooks.contains_key(&component.to_string()) {
        hooks
            .get_mut(&component.to_string())
            .unwrap()
            .insert(property.to_string(), serde_json::json!(value));
    } else {
        let mut properties = serde_json::Map::new();
        properties.insert(property.to_string(), serde_json::json!(value));
        hooks.insert(component.to_string(), properties);
    }
}

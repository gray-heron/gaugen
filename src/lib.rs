pub mod basic_components;
pub mod frontend;
pub mod geometry_components;
mod helpers;
pub mod session;

use nalgebra::Vector2;
use nanovg::{Color, DrawZone, StrokeOptions};
use serde;
use serde_json;
use std::cell::Cell;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fs;
use std::rc;
use typed_arena::Arena;

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

pub type Hooks = HashMap<String, serde_json::Map<String, serde_json::Value>>;

pub struct View<'a> {
    components: Arena<RefCell<Box<dyn AbstractTreeComponent<'a> + 'a>>>,
}

impl<'a> View<'a> {
    pub fn draw(
        &mut self,
        ctx: &mut frontend::PresentationContext,
        zone: DrawZone,
        hooks: &Hooks,
        layers: &RefCell<Vec<DrawZone>>,
    ) {
    }
}

trait AbstractTreeComponent<'a> {
    fn draw(
        &mut self,
        ctx: &mut frontend::PresentationContext,
        zone: DrawZone,
        hooks: &Hooks,
        layers: &RefCell<Vec<DrawZone>>,
    );

    fn add_child(&mut self, child: &'a RefCell<Box<dyn AbstractTreeComponent<'a> + 'a>>);
}

pub struct Manager {
    controls_types: HashMap<&'static str, Box<dyn AbstractComponentFactory>>,
}

struct ComponentInstance<'a, TPublicData, TInternalData>
where
    TPublicData: serde::ser::Serialize + serde::de::DeserializeOwned + Clone + 'static,
    TInternalData: 'static,
{
    pub public_data: TPublicData,
    pub internal_data: TInternalData,
    pub children: Vec<&'a RefCell<Box<dyn AbstractTreeComponent<'a> + 'a>>>,
    pub component_type: std::rc::Rc<Box<dyn Component<TPublicData, TInternalData>>>,
    pub name: Option<String>,
}

fn shell_merge_bottom(stack: &mut Vec<DrawZone>, zone: &DrawZone) -> DrawZone {
    stack.last_mut().unwrap().convex_with_a_zone(zone);
    *stack.last().unwrap()
}

impl<'a, TPublicData, TInternalData> AbstractTreeComponent<'a>
    for ComponentInstance<'a, TPublicData, TInternalData>
where
    TPublicData: serde::ser::Serialize + serde::de::DeserializeOwned + Clone + 'static,
    TInternalData: 'static,
{
    fn draw(
        &mut self,
        ctx: &mut frontend::PresentationContext,
        zone: DrawZone,
        hooks: &Hooks,
        layers: &RefCell<Vec<DrawZone>>,
    ) {
        let mut draws: Vec<
            Box<dyn FnMut(&mut frontend::PresentationContext, DrawZone) -> DrawZone>,
        > = Vec::new();

        for child in &self.children {
            let b = Box::new(
                move |ctx: &mut frontend::PresentationContext, z: DrawZone| -> DrawZone {
                    let mut child = child.borrow_mut();
                    shell_merge_bottom(&mut ctx.shell_stack, &ctx.frame.shell_replace());
                    ctx.shell_stack.push(DrawZone::new_empty());

                    child.draw(ctx, z, hooks, layers);

                    let drawn =
                        shell_merge_bottom(&mut ctx.shell_stack, &ctx.frame.shell_replace());
                    ctx.shell_stack.pop();
                    ctx.shell_stack
                        .last_mut()
                        .unwrap()
                        .convex_with_a_zone(&drawn);

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

        self.component_type.as_ref().draw(
            ctx,
            zone,
            &mut draws[..],
            &mut self.internal_data,
            &self.public_data,
        );
        shell_merge_bottom(&mut ctx.shell_stack, &ctx.frame.shell_replace());
    }

    fn add_child(&mut self, child: &'a RefCell<Box<dyn AbstractTreeComponent<'a> + 'a>>) {
        self.children.push(child);
    }
}

trait AbstractComponentFactory {
    fn make_component<'a>(
        &self,
        ctx: &mut frontend::PresentationContext,
        json: &serde_json::Value,
        children: Vec<&'a RefCell<Box<dyn AbstractTreeComponent<'a> + 'a>>>,
    ) -> Option<Box<dyn AbstractTreeComponent<'a> + 'a>>;
}

struct ConcreteComponentFactory<TPublicData, TInternalData>
where
    TPublicData: serde::ser::Serialize + serde::de::DeserializeOwned + Clone + 'static,
    TInternalData: 'static,
{
    pub component_type: std::rc::Rc<Box<dyn Component<TPublicData, TInternalData>>>,
}

impl<TPublicData, TInternalData> AbstractComponentFactory
    for ConcreteComponentFactory<TPublicData, TInternalData>
where
    TPublicData: serde::ser::Serialize + serde::de::DeserializeOwned + Clone + 'static,
    TInternalData: 'static,
{
    fn make_component<'a>(
        &self,
        ctx: &mut frontend::PresentationContext,
        json: &serde_json::Value,
        children: Vec<&'a RefCell<Box<dyn AbstractTreeComponent<'a> + 'a>>>,
    ) -> Option<Box<dyn AbstractTreeComponent<'a> + 'a>> {
        let public_data = match TPublicData::deserialize(json) {
            Ok(data) => data,
            Err(_) => {
                let default_data = self.component_type.as_ref().get_default_data()?;
                match json.as_object() {
                    Some(hooks) => Manager::join_hooks(&default_data, hooks),
                    None => default_data,
                }
            }
        };

        let private_data = self
            .component_type
            .as_ref()
            .init_instance(ctx, &public_data);

        let name = match json["name"].as_str() {
            Some(s) => Some(s.to_string()),
            None => None,
        };

        Some(Box::new(ComponentInstance {
            public_data: public_data,
            internal_data: private_data,
            children: children,
            component_type: self.component_type.clone(),
            name: name,
        }))
    }
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

    pub fn register_component_type<TComponentData, TPrivateComponentData>(
        &mut self,
        component: Box<dyn Component<TComponentData, TPrivateComponentData>>,
    ) where
        TComponentData: serde::ser::Serialize + serde::de::DeserializeOwned + Clone + 'static,
        TPrivateComponentData: 'static,
    {
        let component = rc::Rc::new(component);

        let component_factory = Box::new(ConcreteComponentFactory {
            component_type: component.clone(),
        });

        self.controls_types
            .insert(component.as_ref().get_name(), component_factory);
    }

    pub fn make_screen<'a>(
        &self,
        ctx: &mut frontend::PresentationContext,
        path_to_json: &str,
    ) -> Option<Box<View>> {
        let json = fs::read_to_string(path_to_json).unwrap();
        let data: serde_json::Value = match serde_json::from_str(&json) {
            Ok(data) => data,
            Err(_) => return None,
        };

        let mut view = Box::new(View {
            components: Arena::new(),
        });
        let result = self.build_tree_components(ctx, &data, &mut view);

        match result {
            Some(_) => Some(view),
            _ => None,
        }
    }

    fn build_tree_components<'a, 'b>(
        &self,
        ctx: &mut frontend::PresentationContext,
        v: &serde_json::Value,
        view: &'b mut Box<View<'a>>,
    ) -> Option<()> {
        // can't type it in recursive manner

        let mut call_stack: Vec<(
            &serde_json::Value,
            Option<&'a RefCell<Box<dyn AbstractTreeComponent + 'a>>>,
        )> = Vec::new();

        let mut root = None;
        call_stack.push((v, None));

        loop {
            match call_stack.pop() {
                Some((json, parent)) => {
                    let factory = &self.controls_types[json["type"].as_str()?];
                    let component = &*view.components.alloc(RefCell::new(factory.make_component(
                        ctx,
                        v,
                        Vec::new(),
                    )?));

                    match parent {
                        Some(parent) => {
                            let r = parent.borrow_mut().add_child(component);
                        }
                        None => root = Some(component),
                    }

                    match json["children"].as_array() {
                        Some(json_children) => {
                            for json_child in json_children {
                                call_stack.push((json_child, Some(component)))
                            }
                        }
                        None => {}
                    }
                }
                None => {
                    break;
                }
            };
        };

        Some(())
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

pub mod basic_components;
pub mod frontend;
pub mod geometry_components;
mod helpers;
pub mod session;

use bumpalo::Bump;
use nanovg::DrawZone;
use ouroboros::self_referencing;
use serde;
use serde_json;
use std::cell::{Ref, RefCell, RefMut};
use std::collections::HashMap;
use std::collections::VecDeque;
use std::fs;
use std::rc;

pub struct ControlGeometry {
    pub aspect: Option<f32>,
    pub size_preference: f32,
}

pub enum Event {
    MouseClick(f32, f32, glutin::MouseButton),
}

pub type DrawChild<'a> = Box<dyn FnMut(&mut frontend::PresentationContext, DrawZone) -> DrawZone + 'a>;

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

    fn handle_event(
        &self,
        _drawn_location: &DrawZone,
        _event: &Event,
        _internal_data: &mut TComponentInternalInstanceData,
        _public_data: &mut TComponentPublicInstanceData,
    ) {
    }
}

pub type Hooks = HashMap<String, serde_json::Map<String, serde_json::Value>>;

pub struct Layer<'a> {
    pub components: Vec<&'a dyn AbstractWrappedComponent<'a>>,
}

#[self_referencing]
pub struct View {
    components: Box<Bump>,
    #[borrows(components)]
    #[not_covariant]
    layers: Vec<Layer<'this>>,
}

impl View{
    pub fn into_inner<F>(&mut self, f: F)
    where F: FnOnce(InnerView) {
        self.with_mut(|fields| {
            f(InnerView{
                fields: fields
            });
        });
    }
}

pub struct InnerView<'a, 'b> {
    fields: ouroboros_impl_view::BorrowedMutFields<'a, 'b>
}

impl<'a, 'b> InnerView<'a, 'b> {
    pub fn get_layer(& self, layer: usize) -> &Layer<'b> {
        &self.fields.layers[layer]
    }
}

pub trait AbstractTreeComponent<'a> {
    fn draw(&mut self, ctx: &mut frontend::PresentationContext, zone: DrawZone, hooks: &Hooks);
    fn add_child(&mut self, child: &'a dyn AbstractWrappedComponent<'a>);
    fn get_drawn_location(&self) -> &DrawZone;
    fn handle_event(&mut self, drawn_location: &DrawZone, event: &Event);
}

pub struct Manager {
    controls_types: HashMap<&'static str, Box<dyn AbstractComponentFactory>>,
}

pub struct ComponentInstance<'a, 'b, TPublicData, TInternalData>
where
    TPublicData: serde::ser::Serialize + serde::de::DeserializeOwned + Clone + 'static,
    TInternalData: 'static,
{
    pub public_data: TPublicData,
    pub internal_data: TInternalData,
    pub children: Vec<&'a dyn AbstractWrappedComponent<'a>>,
    pub component_type: std::rc::Rc<Box<dyn Component<TPublicData, TInternalData> + 'b>>, //fixme
    pub name: Option<String>,
    pub drawn_location: DrawZone,
}

fn shell_merge_bottom(stack: &mut Vec<DrawZone>, zone: &DrawZone) -> DrawZone {
    stack.last_mut().unwrap().convex_with_a_zone(zone);
    *stack.last().unwrap()
}

impl<'a, 'b, TPublicData, TInternalData> AbstractTreeComponent<'a>
    for ComponentInstance<'a, 'b, TPublicData, TInternalData>
where
    TPublicData: serde::ser::Serialize + serde::de::DeserializeOwned + Clone + 'static,
    TInternalData: 'static,
{
    fn draw(&mut self, ctx: &mut frontend::PresentationContext, zone: DrawZone, hooks: &Hooks) {
        let mut draws: Vec<
            Box<dyn FnMut(&mut frontend::PresentationContext, DrawZone) -> DrawZone>,
        > = Vec::new();

        for child in &self.children {
            let b = Box::new(
                move |ctx: &mut frontend::PresentationContext, z: DrawZone| -> DrawZone {
                    let mut child = child.borrow_mut();
                    shell_merge_bottom(&mut ctx.shell_stack, &ctx.frame.shell_replace());
                    ctx.shell_stack.push(DrawZone::new_empty());

                    child.draw(ctx, z, hooks);

                    let drawn =
                        shell_merge_bottom(&mut ctx.shell_stack, &ctx.frame.shell_replace());

                    ctx.shell_stack.pop();
                    ctx.shell_stack
                        .last_mut()
                        .unwrap()
                        .convex_with_a_zone(&drawn);
                    drawn
                },
            );
            draws.push(b);
        }

        self.component_type.as_ref().draw(
            ctx,
            zone,
            &mut draws[..],
            &mut self.internal_data,
            &self.public_data,
        );

        self.drawn_location = shell_merge_bottom(&mut ctx.shell_stack, &ctx.frame.shell_replace());
    }

    fn add_child(&mut self, child: &'a dyn AbstractWrappedComponent<'a>) {
        self.children.push(child);
    }

    fn get_drawn_location(&self) -> &DrawZone {
        &self.drawn_location
    }

    fn handle_event(&mut self, drawn_location: &DrawZone, event: &Event) {
        self.component_type.handle_event(
            drawn_location,
            event,
            &mut self.internal_data,
            &mut self.public_data,
        );
    }
}

pub trait AbstractWrappedComponent<'a> {
    fn borrow(&self) -> Ref<dyn AbstractTreeComponent<'a>>;
    fn borrow_mut(&self) -> RefMut<dyn AbstractTreeComponent<'a>>;
}

pub struct WrappedComponent<'a, 'b, TPublicData, TInternalData>
where
    TPublicData: serde::ser::Serialize + serde::de::DeserializeOwned + Clone + 'static,
    TInternalData: 'static,
{
    pub storage: RefCell<ComponentInstance<'a, 'b, TPublicData, TInternalData>>,
}

impl<'a, 'b, TPublicData, TInternalData> AbstractWrappedComponent<'a>
    for WrappedComponent<'a, 'b, TPublicData, TInternalData>
where
    TPublicData: serde::ser::Serialize + serde::de::DeserializeOwned + Clone + 'static,
    TInternalData: 'static,
{
    fn borrow(&self) -> Ref<dyn AbstractTreeComponent<'a>> {
        self.storage.borrow()
    }

    fn borrow_mut(&self) -> RefMut<dyn AbstractTreeComponent<'a>> {
        self.storage.borrow_mut()
    }
}

trait AbstractComponentFactory {
    fn make_component<'a>(
        &self,
        ctx: &mut frontend::PresentationContext,
        json: &serde_json::Value,
        bump: &'a Bump,
        children: Vec<&'a dyn AbstractWrappedComponent<'a>>,
    ) -> Option<&'a dyn AbstractWrappedComponent<'a>>;
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
        bump: &'a Bump,
        children: Vec<&'a dyn AbstractWrappedComponent<'a>>,
    ) -> Option<&'a dyn AbstractWrappedComponent<'a>> {
        let name = match json["name"].as_str() {
            Some(s) => Some(s.to_string()),
            None => None,
        };
        let public_data = match TPublicData::deserialize(json) {
            Ok(data) => data,
            Err(_err) => {
                let default_data = self.component_type.as_ref().get_default_data()?;
                match json.as_object() {
                    Some(partial) => Manager::join_hooks(&default_data, partial),
                    None => default_data,
                }
            }
        };

        let private_data = self
            .component_type
            .as_ref()
            .init_instance(ctx, &public_data);

        let instance = WrappedComponent {
            storage: RefCell::new(ComponentInstance {
                public_data: public_data,
                internal_data: private_data,
                children: children,
                component_type: self.component_type.clone(),
                name: name,
                drawn_location: DrawZone::new_empty(),
            }),
        };

        Some(bump.alloc(instance))
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

        let view = ViewBuilder {
            components: Box::new(Bump::new()),
            layers_builder: |arena| {
                let mut call_stack: VecDeque<(
                    &serde_json::Value,
                    Option<&'_ dyn AbstractWrappedComponent<'_>>,
                )> = VecDeque::new();

                let mut ret = vec![Layer {
                    components: Vec::new(),
                }];
                let first_layer = &mut ret[0];

                call_stack.push_back((&data, None));

                loop {
                    match call_stack.pop_front() {
                        Some((json, parent)) => {
                            let component_type = match json["type"].as_str() {
                                Some(ct) => ct,
                                _ => return Vec::new(),
                            };

                            let factory = &self.controls_types[component_type];

                            let component =
                                match factory.make_component(ctx, &json["data"], arena, Vec::new())
                                {
                                    Some(c) => c,
                                    _ => return Vec::new(),
                                };

                            match parent {
                                Some(parent) => {
                                    parent.borrow_mut().add_child(component);
                                }
                                None => {}
                            }

                            match json["children"].as_array() {
                                Some(json_children) => {
                                    for json_child in json_children {
                                        call_stack.push_back((json_child, Some(component)))
                                    }
                                }
                                None => {}
                            }

                            first_layer.components.push(component);
                        }
                        None => {
                            break;
                        }
                    };
                }

                ret
            },
        }
        .build();

        if view.with(|fields| fields.layers[0].components.len() > 0) {
            Some(view)
        } else {
            None
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

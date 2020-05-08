use crate::gaugen;
use crate::frontend;

use nalgebra::Vector2;

// =========================== SPACER ===========================

#[derive(serde::Deserialize)]
pub struct SpacerInstance {
    spacing: f32,
}

pub struct Spacer {}

impl gaugen::Component<SpacerInstance, ()> for Spacer {
    fn get_default_data(&self) -> Option<SpacerInstance>{
        Some(SpacerInstance{
            spacing: 1.0
        })
    }

    fn max_children(&self) -> Option<u32> {
        Some(1)
    }

    fn get_name(&self) -> &'static str {
        "Spacer"
    }

    fn init_instance(
        &self,
        __ctx: &frontend::PresentationContext,
        data: &SpacerInstance,
        sizes: &[gaugen::ControlGeometry],
    ) -> gaugen::AfterInit<()>{
        gaugen::AfterInit{
            aspect: sizes[0].aspect,
            internal_data: ()
        }
    }

    fn draw(
        &self,
        ctx: &frontend::PresentationContext,
        zone: gaugen::DrawZone,
        children: &mut [
            Box<dyn FnMut(gaugen::DrawZone) + '_>
        ],
        internal_data: &mut (),
        data: &SpacerInstance,
    ) {
        assert!(children.len() == 1);

        let childzone = gaugen::DrawZone {
            m: zone.m,
            size: zone.size * data.spacing,
        };

        children[0].as_mut()(childzone);
    }
}

// =========================== VERTICAL SPLIT ===========================

#[derive(serde::Deserialize, std::cmp::PartialEq)]
pub enum SplitDirection{
    Horizontal,
    Vertical
}

#[derive(serde::Deserialize, std::cmp::PartialEq)]
pub enum SplitMode{
    EqualArea,
    EqualSide
}

#[derive(serde::Deserialize)]
pub struct SplitInstance {
    spacing: f32,
    direction: SplitDirection,
    mode: SplitMode
}

pub struct Split {
    spacer: Spacer,
}

struct SplitInternalData{
    sizes: Vec<Vector2<f32>>,
    primary_width: f32
}

impl SplitInstance{
    // primary dimension = along split direction
    fn pm<'a>(&self, vector: &'a mut Vector2<f32>) -> &'a mut f32{ 
        if self.direction == SplitDirection::Horizontal{
            &mut vector.x
        } else {
            &mut vector.y
        }
    }

    fn p<'a>(&self, vector: &'a Vector2<f32>) -> &'a f32{ 
        if self.direction == SplitDirection::Horizontal{
            &vector.x
        } else {
            &vector.y
        }
    }

    // secondary dimension
    fn sm<'a>(&self, vector: &'a mut Vector2<f32>) -> &'a mut f32{
        if self.direction == SplitDirection::Horizontal{
            &mut vector.y
        } else {
            &mut vector.x
        }
    }

    fn s<'a>(&self, vector: &'a Vector2<f32>) -> &'a f32{
        if self.direction == SplitDirection::Horizontal{
            &vector.y
        } else {
            &vector.x
        }
    }

    fn aspect_to_primary_to_secondary(&self, aspect: f32) -> f32{
        if self.direction == SplitDirection::Horizontal{
            aspect
        } else {
            1.0 / aspect
        }
    }
}

impl gaugen::Component<SplitInstance, SplitInternalData> for Split {
    fn get_default_data(&self) -> Option<SplitInstance>{
        Some(SplitInstance{
            spacing: 0.9,
            direction: SplitDirection::Horizontal,
            mode: SplitMode::EqualSide
        })
    }

    fn init_instance(
        &self,
        __ctx: &frontend::PresentationContext,
        data: &SplitInstance,
        sizes: &[gaugen::ControlGeometry],
    ) -> gaugen::AfterInit<SplitInternalData>{
        
        if data.mode == SplitMode::EqualSide {
            let mut internal_sizes: Vec<Vector2<f32>> = Vec::new();
            let mut total_size = 0.0;

            for size in sizes {
                let aspect = match size.aspect {
                    Some(aspect) => aspect,
                    None => size.size_preference
                };

                let relative_aspect = data.aspect_to_primary_to_secondary(aspect);

                internal_sizes.push(Vector2::new(relative_aspect, 1.0));
                total_size += relative_aspect;
            }

            gaugen::AfterInit {
                aspect: Some(data.aspect_to_primary_to_secondary(total_size)), 
                internal_data: SplitInternalData{
                    sizes: internal_sizes,
                    primary_width: total_size
                }
            }
        } else {
            panic!();
            /*
            gaugen::AfterInit{
                aspect: sizes[0].aspect,
                internal_data: SplitInternalData{
                    sizes: 
                }
            }
            */
        }
    }

    fn max_children(&self) -> Option<u32> {
        None
    }

    fn get_name(&self) -> &'static str {
        "Split"
    }

    fn draw(
        &self,
        ctx: &frontend::PresentationContext,
        zone: gaugen::DrawZone,
        children: &mut [
            Box<dyn FnMut(gaugen::DrawZone) + '_>
        ],
        internal_data: &mut SplitInternalData,
        data: &SplitInstance,
    ) {
        assert_eq!(children.len(), internal_data.sizes.len());

        if data.mode == SplitMode::EqualSide {
            let space_per_unit = data.p(&zone.size) / internal_data.primary_width;
            let mut primary_cursor = *data.p(&zone.bottom_left());

            for i in 0..children.len() {
                let mut bottom_left = Vector2::new(0.0, 0.0);
                let mut top_right = Vector2::new(0.0, 0.0);

                *data.pm(&mut bottom_left) = primary_cursor;
                *data.sm(&mut bottom_left) = *data.s(&zone.bottom_left());

                *data.pm(&mut top_right) = primary_cursor + internal_data.sizes[i].x * space_per_unit;
                *data.sm(&mut top_right) = *data.s(&zone.top_right());

                let zone = gaugen::DrawZone::from_rect(bottom_left, top_right);

                self.spacer.draw(ctx, zone, &mut children[i..i+1], &mut (), &SpacerInstance{
                    spacing: data.spacing
                });

                primary_cursor += internal_data.sizes[i].x * space_per_unit;
            }
        } else {
            panic!();
            /*
            gaugen::AfterInit{
                aspect: sizes[0].aspect,
                internal_data: SplitInternalData{
                    sizes: 
                }
            }
            */
        }
    }
}

// ===========================

pub fn register_geometry_components(manager: &mut gaugen::Manager) {
    let vs = Box::new(Split { spacer: Spacer{} });
    let spacer = Box::new(Spacer{});

    manager.register_component_type(vs);
    manager.register_component_type(spacer);
}

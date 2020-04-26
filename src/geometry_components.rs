use crate::gaugen;
use crate::frontend;

use nalgebra::Vector2;

// =========================== SPACER ===========================

#[derive(serde::Deserialize)]
pub struct SpacerData {
    spacing: f32,
}

pub struct Spacer {}

impl gaugen::Component<SpacerData> for Spacer {
    fn max_children(&self) -> Option<u32> {
        Some(1)
    }

    fn get_name(&self) -> &'static str {
        "Spacer"
    }

    fn get_size(&self) -> gaugen::ControlGeometry {
        gaugen::ControlGeometry {
            aspect: None,
            size_preference: None,
        }
    }

    fn draw(
        &self,
        ctx: &frontend::PresentationContext,
        zone: gaugen::DrawZone,
        children: &[(
            gaugen::ControlGeometry,
            Box<dyn Fn(gaugen::DrawZone) + '_>,
        )],
        data: &SpacerData,
    ) {
        assert!(children.len() == 1);

        let childzone = gaugen::DrawZone {
            m: zone.m,
            size: zone.size * data.spacing,
        };

        children[0].1.as_ref()(childzone);
    }
}

// =========================== VERTICAL SPLIT ===========================

#[derive(serde::Deserialize)]
pub struct VerticalSplitInstance {
    spacing: f32,
}

pub struct VerticalSplit {
    spacer: Spacer,
}

impl gaugen::Component<VerticalSplitInstance> for VerticalSplit {
    fn max_children(&self) -> Option<u32> {
        None
    }

    fn get_name(&self) -> &'static str {
        "VerticalSplit"
    }

    fn get_size(&self) -> gaugen::ControlGeometry {
        gaugen::ControlGeometry {
            aspect: None,
            size_preference: None,
        }
    }

    fn draw(
        &self,
        ctx: &frontend::PresentationContext,
        zone: gaugen::DrawZone,
        children: &[(
            gaugen::ControlGeometry,
            Box<dyn Fn(gaugen::DrawZone) + '_>,
        )],
        data: &VerticalSplitInstance,
    ) {
        let space_per_child = zone.size.x / (children.len() as f32);
        let mut left = zone.left();

        for i in 0..children.len() {
            let childzone = gaugen::DrawZone::from_rect(
                Vector2::new(left, zone.bottom()),
                Vector2::new(left + space_per_child, zone.top()),
            );

            self.spacer.draw(ctx, childzone, &children[i..i+1], &SpacerData{
                spacing: data.spacing
            });

            left += space_per_child;
        }
    }
}

// ===========================

pub fn register_geometry_components(manager: &mut gaugen::Manager) {
    let vs = Box::new(VerticalSplit { spacer: Spacer{} });
    let spacer = Box::new(Spacer{});

    manager.register_component_type(vs);
    manager.register_component_type(spacer);
}

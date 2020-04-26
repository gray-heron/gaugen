use crate::gaugen;
use nalgebra::Vector2;

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
        ctx: &mut gaugen::PresentationContext,
        zone: gaugen::DrawZone,
        children: &[(
            gaugen::ControlGeometry,
            Box<dyn FnMut(gaugen::DrawZone) + '_>,
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
        ctx: &mut gaugen::PresentationContext,
        zone: gaugen::DrawZone,
        children: &[(
            gaugen::ControlGeometry,
            Box<dyn FnMut(gaugen::DrawZone) + '_>,
        )],
        data: &VerticalSplitInstance,
    ) {
        let space_per_child = zone.size.x / (children.len() as f32);
        let mut left = zone.left();

        for child in children {
            let childzone = gaugen::DrawZone::from_rect(
                Vector2::new(left, zone.bottom()),
                Vector2::new(left + space_per_child, zone.top()),
            );

            self.spacer.draw(ctx, childzone, &[(child.0, child.1)], &SpacerData{
                spacing: data.spacing
            });

            left += space_per_child;
        }
    }
}

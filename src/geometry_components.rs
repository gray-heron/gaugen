use crate::frontend;
use crate::*;

use math::round;
use nalgebra::Vector2;
use std::f32;

// =========================== SPACER ===========================

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct SpacerInstance {
    spacing: f32,
}

pub struct Spacer {}

impl Component<SpacerInstance, ()> for Spacer {
    fn get_default_data(&self) -> Option<SpacerInstance> {
        Some(SpacerInstance { spacing: 1.0 })
    }

    fn max_children(&self) -> Option<u32> {
        Some(1)
    }

    fn get_name(&self) -> &'static str {
        "Spacer"
    }

    fn init_instance(&self, __ctx: &mut frontend::PresentationContext, __data: &SpacerInstance) {}

    fn draw(
        &self,
        ctx: &mut frontend::PresentationContext,
        zone: DrawZone,
        children: &mut [DrawChild],
        __internal_data: &mut (),
        data: &SpacerInstance,
    ) {
        assert!(children.len() == 1);

        let childzone = DrawZone {
            m: zone.m,
            size: zone.size * data.spacing,
            empty: false,
        };

        children[0].as_mut()(ctx, childzone);
    }
}

// =========================== SPLIT ===========================

fn draw_with_spacing(
    ctx: &mut frontend::PresentationContext,
    child: &mut DrawChild,
    zone: &DrawZone,
    spacing: f32,
) -> DrawZone {
    let new_zone = DrawZone {
        m: zone.m,
        size: zone.size * spacing,
        empty: false,
    };

    let mut drawn = child(ctx, new_zone);
    drawn.size *= 1.0 / spacing;

    ctx.frame.mark_as_used((drawn.left(), drawn.bottom()));
    ctx.frame.mark_as_used((drawn.right(), drawn.top()));

    drawn
}

// =========================== SPLIT ===========================

#[derive(serde::Serialize, serde::Deserialize, Clone, std::cmp::PartialEq)]
pub enum SplitDirection {
    Horizontal,
    Vertical,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, std::cmp::PartialEq)]
pub enum SplitMode {
    EqualArea,
    EqualSide,
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct SplitInstance {
    spacing: f32,
    direction: SplitDirection,
    mode: SplitMode,
}

pub struct Split;

struct SplitInternalData {
    aspects: Vec<f32>,
    primary_width: f32,
    primary_aspect: f32
}

impl SplitInstance {
    // primary dimension = perpendicular to the separators
    fn pm<'a>(&self, vector: &'a mut Vector2<f32>) -> &'a mut f32 {
        if self.direction == SplitDirection::Horizontal {
            &mut vector.x
        } else {
            &mut vector.y
        }
    }

    fn p<'a>(&self, vector: &'a Vector2<f32>) -> &'a f32 {
        if self.direction == SplitDirection::Horizontal {
            &vector.x
        } else {
            &vector.y
        }
    }

    // secondary dimension
    fn sm<'a>(&self, vector: &'a mut Vector2<f32>) -> &'a mut f32 {
        if self.direction == SplitDirection::Horizontal {
            &mut vector.y
        } else {
            &mut vector.x
        }
    }

    fn s<'a>(&self, vector: &'a Vector2<f32>) -> &'a f32 {
        if self.direction == SplitDirection::Horizontal {
            &vector.y
        } else {
            &vector.x
        }
    }

    fn aspect_to_primary_to_secondary(&self, aspect: f32) -> f32 {
        if self.direction == SplitDirection::Horizontal {
            aspect
        } else {
            1.0 / aspect
        }
    }
}

impl Component<SplitInstance, SplitInternalData> for Split {
    fn get_default_data(&self) -> Option<SplitInstance> {
        Some(SplitInstance {
            spacing: 0.9,
            direction: SplitDirection::Horizontal,
            mode: SplitMode::EqualSide,
        })
    }

    fn init_instance(
        &self,
        __ctx: &mut frontend::PresentationContext,
        __data: &SplitInstance,
    ) -> SplitInternalData {
        SplitInternalData {
            aspects: Vec::new(),
            primary_width: 0.0,
            primary_aspect: 1.0
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
        ctx: &mut frontend::PresentationContext,
        zone: DrawZone,
        children: &mut [DrawChild],
        internal_data: &mut SplitInternalData,
        data: &SplitInstance,
    ) {
        let zone = zone.constraint_to_aspect((zone.aspect() *internal_data.primary_aspect).sqrt());
        let mut new_aspects = Vec::new();
        let mut new_primary_width = 0.0;
        let mut new_total_aspect = 0.0;

        if internal_data.aspects.len() != children.len() {
            internal_data.primary_width = 0.0;
            internal_data.aspects.clear();

            for _ in 0..children.len() {
                internal_data.aspects.push(1.0);
                internal_data.primary_width += 1.0;
            }
        }

        if data.mode == SplitMode::EqualSide {
            let space_per_unit = data.p(&zone.size) / internal_data.primary_width;
            let mut primary_cursor = *data.p(&zone.top_left());

            for i in 0..children.len() {
                let mut top_left = Vector2::new(0.0, 0.0);
                let mut bottom_right = Vector2::new(0.0, 0.0);

                *data.pm(&mut top_left) = primary_cursor;
                *data.sm(&mut top_left) = *data.s(&zone.top_left());

                *data.pm(&mut bottom_right) =
                    primary_cursor + internal_data.aspects[i] * space_per_unit;
                *data.sm(&mut bottom_right) = *data.s(&zone.bottom_right());

                let zone = DrawZone::from_rect(top_left, bottom_right);

                let result = draw_with_spacing(ctx, &mut children[i], &zone, data.spacing);

                let new_aspect = data.aspect_to_primary_to_secondary(result.aspect());
                new_total_aspect += new_aspect;
                new_aspects.push(new_aspect);
                new_primary_width += new_aspect;
                primary_cursor += internal_data.aspects[i] * space_per_unit;

            }
        } else {
            panic!();
            /*
            AfterInit{
                aspect: sizes[0].aspect,
                internal_data: SplitInternalData{
                    sizes:
                }
            }
            */
        }

        internal_data.aspects = new_aspects;
        internal_data.primary_width = new_primary_width;

        if data.direction == SplitDirection::Horizontal {
            internal_data.primary_aspect = new_total_aspect
        } else {
            internal_data.primary_aspect = 1.0 / new_total_aspect
        }
    }
}

// ===========================

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub enum GroupingBoxTitleSize {
    RelativeToHeight(f32),
    Absolute(f32),
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct GroupingBoxData {
    pub spacing: f32,
    pub title_size: GroupingBoxTitleSize,
    pub title: String,
}

pub struct GroupingBox {}

impl Component<GroupingBoxData, Option<f32>> for GroupingBox {
    // primary dimension = along split direction
    fn max_children(&self) -> Option<u32> {
        Some(1)
    }

    fn get_name(&self) -> &'static str {
        "GroupingBox"
    }

    fn get_default_data(&self) -> Option<GroupingBoxData> {
        Some(GroupingBoxData {
            spacing: 0.9,
            title_size: GroupingBoxTitleSize::RelativeToHeight(0.2),
            title: "GroupingBox".to_string(),
        })
    }

    fn init_instance(
        &self,
        __ctx: &mut frontend::PresentationContext,
        __data: &GroupingBoxData,
    ) -> Option<f32> {
        None
    }

    fn draw(
        &self,
        ctx: &mut frontend::PresentationContext,
        zone: DrawZone,
        children: &mut [DrawChild],
        last_aspect: &mut Option<f32>,
        public_data: &GroupingBoxData,
    ) {
        let ref palette = ctx.resources.palette;

        let zone = match last_aspect {
            None => zone,
            Some(aspect) => zone.constraint_to_aspect((*aspect * zone.aspect()).sqrt()),
        };

        let title_height = match public_data.title_size {
            GroupingBoxTitleSize::RelativeToHeight(height) => zone.size.y * height,
            GroupingBoxTitleSize::Absolute(height) => height,
        };

        let child_height = zone.size.y - title_height;

        let child_zone = DrawZone::from_rect(
            zone.top_left() + Vector2::new(0.0, title_height),
            zone.bottom_right(),
        );

        let text_zone = DrawZone::from_rect(
            zone.top_left(),
            zone.bottom_right() - Vector2::new(0.0, child_height),
        );

        let text_zone = DrawZone {
            m: text_zone.m,
            size: text_zone.size,
            empty: false,
        };

        let child_zone = DrawZone {
            m: child_zone.m,
            size: child_zone.size * public_data.spacing,
            empty: false,
        };

        let text_opts = nanovg::TextOptions {
            color: ctx.resources.palette.soft_front_color(),
            size: text_zone.size.y * 1.0,
            align: nanovg::Alignment::new().center().middle(),
            line_height: text_zone.size.y * 1.0,
            line_max_width: text_zone.size.x * 1.0,
            ..Default::default()
        };

        ctx.frame.text_box(
            ctx.resources.font,
            (text_zone.left(), text_zone.m.y),
            public_data.title.as_str(),
            text_opts,
        );

        let bounds = ctx.frame.text_box_bounds(
            ctx.resources.font,
            (0.0, 0.0),
            public_data.title.as_str(),
            text_opts,
        );

        let w = match public_data.title == "" {
            false => (bounds.max_x - text_zone.size.x / 2.0) * 1.2,
            true => 0.0,
        };

        ctx.frame.path(
            |path| {
                path.move_to((text_zone.m.x - w, text_zone.m.y));
                path.line_to((zone.left(), text_zone.m.y));
                path.line_to((zone.left(), zone.bottom()));
                path.line_to((zone.right(), zone.bottom()));
                path.line_to((zone.right(), text_zone.m.y));
                path.line_to((text_zone.m.x + w, text_zone.m.y));

                path.stroke(
                    palette.soft_front_color(),
                    nanovg::StrokeOptions {
                        width: 3.0,
                        ..Default::default()
                    },
                );
            },
            Default::default(),
        );

        *last_aspect = Some(children[0].as_mut()(ctx, child_zone).aspect());
    }
}

// ===========================

#[derive(Copy, Clone)]
struct ChildGeometryProperties {
    min_w: Option<f32>,
    max_w: Option<f32>,
    min_h: Option<f32>,
    max_h: Option<f32>,
}

static DEFAULT_GEOMETRY_DATA: ChildGeometryProperties = ChildGeometryProperties {
    min_w: None,
    max_w: None,
    min_h: None,
    max_h: None,
};

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub enum GridDimensions {
    Fixed((usize, usize)), //tuple for better serialization
    Auto,
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub enum GridLayoutStrategy {
    Static,
    MinimizeWastedSpace,
    EqualUsedSpace,
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct GridData {
    pub spacing: f32,
    pub dimensions: GridDimensions,
    pub layout_strategy: GridLayoutStrategy,
}

pub struct Grid {}

struct GridInternalData {
    geometry_data: Vec<ChildGeometryProperties>,
    dims: (usize, usize),
    rows_sizes: Vec<f32>,
    columns_sizes: Vec<f32>,
    spacing: f32,
}

impl GridInternalData {
    pub fn gd<'a>(&'a self, x: usize, y: usize) -> &'a ChildGeometryProperties {
        if y * self.dims.1 + x < self.geometry_data.len() {
            &self.geometry_data[y * self.dims.1 + x]
        } else {
            &DEFAULT_GEOMETRY_DATA
        }
    }

    pub fn get_column_sizes(&self, column: usize) -> (Option<f32>, Option<f32>) {
        let mut min = f32::MAX;
        let mut max = 0.0;

        for i in 0..self.dims.1 {
            match self.gd(column, i).min_w {
                Some(w) => {
                    if w < min {
                        min = w;
                    }
                }
                None => {}
            };

            match self.gd(column, i).max_h {
                Some(w) => {
                    if w > max {
                        max = w;
                    }
                }
                None => {}
            };
        }

        (
            match min {
                f32::MAX => None,
                _ => Some(min),
            },
            match max {
                0.0 => None,
                _ => Some(max),
            },
        )
    }

    pub fn get_row_sizes(&self, row: usize) -> (Option<f32>, Option<f32>) {
        let mut min = f32::MAX;
        let mut max = 0.0;

        for i in 0..self.dims.0 {
            match self.gd(row, i).min_h {
                Some(w) => {
                    if w < min {
                        min = w;
                    }
                }
                None => {}
            };

            match self.gd(row, i).max_h {
                Some(w) => {
                    if w > max {
                        max = w;
                    }
                }
                None => {}
            };
        }

        (
            match min {
                f32::MAX => None,
                _ => Some(min),
            },
            match max {
                0.0 => None,
                _ => Some(max),
            },
        )
    }
}

impl Component<GridData, GridInternalData> for Grid {
    fn max_children(&self) -> Option<u32> {
        None
    }

    fn get_name(&self) -> &'static str {
        "Grid"
    }

    fn get_default_data(&self) -> Option<GridData> {
        Some(GridData {
            spacing: 0.9,
            dimensions: GridDimensions::Auto,
            layout_strategy: GridLayoutStrategy::MinimizeWastedSpace,
        })
    }

    fn init_instance(
        &self,
        __ctx: &mut frontend::PresentationContext,
        __data: &GridData,
    ) -> GridInternalData {
        GridInternalData {
            geometry_data: Vec::new(),
            dims: (0, 0),
            columns_sizes: Vec::new(),
            rows_sizes: Vec::new(),
            spacing: 0.9,
        }
    }

    fn draw(
        &self,
        ctx: &mut frontend::PresentationContext,
        zone: DrawZone,
        children: &mut [DrawChild],
        internal_data: &mut GridInternalData,
        public_data: &GridData,
    ) {
        let dims = match public_data.dimensions {
            GridDimensions::Fixed((w, h)) => (w, h),
            GridDimensions::Auto => (
                round::ceil((children.len() as f64).sqrt(), 0) as usize,
                round::ceil((children.len() as f64).sqrt(), 0) as usize,
            ),
        };

        if internal_data.dims != dims {
            internal_data.dims = dims;
            internal_data.geometry_data.clear();
            internal_data.columns_sizes = vec![1.0 / (dims.0 as f32); dims.0];
            internal_data.rows_sizes = vec![1.0 / (dims.1 as f32); dims.1];
        }

        if internal_data.dims.0 == 0 || internal_data.dims.1 == 0 {
            return;
        }

        let mut child_id = 0;
        let mut cursor = zone.top_left();

        let mut new_columns_sizes = vec![0.0; internal_data.dims.0];
        let mut new_rows_sizes = vec![0.0; internal_data.dims.1];

        let mut new_rows_total_size = 0.0;
        let mut new_columns_total_size = 0.0;

        for y in 0..internal_data.dims.1 {
            for x in 0..internal_data.dims.0 {
                if child_id < children.len() {
                    let childzone_size = Vector2::new(
                        internal_data.columns_sizes[x] * zone.size.x,
                        internal_data.rows_sizes[y] * zone.size.y,
                    );

                    let childzone = DrawZone::from_rect(cursor, cursor + childzone_size);
                    let result = draw_with_spacing(
                        ctx,
                        &mut children[child_id],
                        &childzone,
                        internal_data.spacing,
                    );

                    if result.aspect() > childzone.aspect() {
                        // we are wasting vertical space
                        new_rows_sizes[y] +=
                            internal_data.rows_sizes[y] * childzone.aspect() / result.aspect();
                        new_columns_sizes[x] += internal_data.columns_sizes[x];
                    } else {
                        // we are wasting vertical space
                        new_columns_sizes[x] +=
                            internal_data.columns_sizes[x] * result.aspect() / childzone.aspect();
                        new_rows_sizes[y] += internal_data.rows_sizes[y];
                    }

                    child_id += 1;
                    cursor.x += childzone_size.x;
                } else {
                    break;
                }
            }

            cursor.x = zone.top_left().x;
            cursor.y += internal_data.rows_sizes[y] * zone.size.y;
        }

        for y in 0..internal_data.dims.1 {
            new_rows_total_size += new_rows_sizes[y];
        }

        for x in 0..internal_data.dims.0 {
            new_columns_total_size += new_columns_sizes[x];
        }

        for y in 0..internal_data.dims.1 {
            new_rows_sizes[y] /= new_rows_total_size;
        }

        for x in 0..internal_data.dims.0 {
            new_columns_sizes[x] /= new_columns_total_size;
        }

        internal_data.columns_sizes = new_columns_sizes;
        internal_data.rows_sizes = new_rows_sizes;
    }
}

// =========================== UTILS ===========================

pub fn components() -> impl Fn(&mut Manager) {
    |manager: &mut Manager| {
        let split = Box::new(Split {});
        let spacer = Box::new(Spacer {});
        let grouping_box = Box::new(GroupingBox {});
        let grid = Box::new(Grid {});

        manager.register_component_type(split);
        manager.register_component_type(spacer);
        manager.register_component_type(grouping_box);
        manager.register_component_type(grid);
    }
}

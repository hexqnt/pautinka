//! Ядро Pautinka: геометрия схем без зависимости от UI и предметной модели сети.
//!
//! ```
//! use pautinka_core::{NodeShape, RoutingMode, SpatialIndex, Vec2, pos2};
//!
//! let shape = NodeShape::rectangle(Vec2::new(80.0, 40.0))?;
//! let outlet = shape.anchor(pos2(0.0, 0.0), Vec2::X)?;
//! let inlet = shape.anchor(pos2(200.0, 80.0), -Vec2::X)?;
//! let path = RoutingMode::Curved.route_anchors(outlet, inlet, 40.0, 0.0)?;
//! let mut index = SpatialIndex::default();
//! let handle = index.insert(7_u64, path.bounds())?;
//! let mut found = false;
//! index.query(path.bounds(), |id| found |= id == 7);
//! assert!(found);
//! index.update(handle, path.bounds().translate(Vec2::X))?;
//! index.remove(handle)?;
//! assert!(index.is_empty());
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

mod alignment;
mod anchor;
mod changes;
mod group;
mod hit_point;
mod identity;
mod math;
mod path;
mod ribbon;
mod routing;
mod shape;
mod snapping;
mod spatial;

pub use emath::{Pos2, Rect, Vec2, pos2, vec2};

pub use alignment::{Alignment, AlignmentIndex, AlignmentSnap};
pub use anchor::{Anchor, AnchorError, Port};
pub use changes::{ChangeCoverage, GeometryChanges};
pub use group::{GroupFrame, GroupFrameError};
pub use hit_point::HitPoint;
pub use identity::{
    EdgeId, ElementId, ElementIndex, GraphId, GraphRevision, GroupId, NodeId, PortId,
};
pub use path::{CornerRadius, Curve, EdgePath, PathError, PathSample};
pub use ribbon::{RibbonLane, RibbonSpacing, RibbonSpacingError};
pub use routing::{LoopOptions, RouteEndpoints, RoutingMode};
pub use shape::{NodeShape, ShapeError, ShapeKind};
pub use snapping::{
    SnapGrid, SnapGuide, SnapIndex, SnapKind, SnapKinds, SnapOptions, SnapResult, SnapTarget,
};
pub use spatial::{SpatialError, SpatialHandle, SpatialIndex};

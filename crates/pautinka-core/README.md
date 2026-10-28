# pautinka-core

UI-independent geometry, routing and spatial queries for positioned network diagrams. Depends on `emath`; usable without the [egui frontend](../pautinka-egui).

```toml
[dependencies]
pautinka-core = "0.1"
```

```rust
use pautinka_core::{NodeShape, RoutingMode, Vec2, pos2};

let pump = NodeShape::rectangle(Vec2::new(80.0, 40.0))?;
let tank = NodeShape::circle(30.0)?;
let outlet = pump.anchor(pos2(0.0, 0.0), Vec2::X)?;
let inlet = tank.anchor(pos2(200.0, 80.0), -Vec2::X)?;
let path = RoutingMode::Curved.route_anchors(outlet, inlet, 40.0, 0.0)?;
assert_eq!(path.points()[0], outlet.position());
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Building blocks

- `EdgePath`: validated lines, Bézier curves and polylines with bounds, length, directional samples and distance queries. Lines and curves allocate no heap memory.
- `RoutingMode`: straight, curved, orthogonal, octilinear and 15-degree routes; explicit loops, oriented anchors and parallel ribbons.
- `NodeShape`, `Anchor`, `Port`: circle, ellipse, rectangle, triangle, diamond and regular polygon boundaries with optional rounded corners, plus typed connection points.
- `SnapIndex`: horizontal, vertical and diagonal guides, intersections and hysteresis; O(log n) queries without allocations. `AlignmentIndex` provides the axis-only interface.
- `SnapGrid`: a regular origin-centered grid with a validated positive finite `f64` step. `snap` captures whole intersections using a radial tolerance, rounding midpoint ties away from zero. `SnapIndex::snap_with_grid` compares the grid against the guide result: held targets take priority, otherwise the nearest wins and guides break ties. Grid targets use `SnapTarget::Grid` and provide no guides. Querying a grid needs no source index or heap allocations.
- `SpatialIndex<K>`: dynamic BVH with local updates and generation-checked handles.
- Typed IDs, topology/geometry/appearance revisions and `GeometryChanges` for incremental consumers.

Use `NodeShape::ellipse(size)`, `rectangle(size)`, `triangle(size)`, `diamond(size)`, `circle(radius)` or `regular_polygon(circumradius, sides)`. Polygon side counts range from 3 to 65535; regular polygons have equal sides and one vertex pointing up. Apply `.with_corner_radius(CornerRadius::try_from(radius)?)` to any shape with corners; trims are capped at half the adjacent side lengths. `anchor` and `contains` use analytic boundaries, including circular corner arcs; `contains_precise` preserves local pointer offsets at large positions. `outline` visits the convex contour without heap allocation, approximating arcs at steps no larger than 5°. All sizes and radii use world units. See the frontend's [shape gallery](../pautinka-egui/examples/shapes.rs).

Geometry uses `f32`; precise `HitPoint` queries use `f64`. Choose representable origins and units. Bézier lengths and `points()` use 25 cached samples; picking refines the actual curve. Routing does not perform layout or obstacle avoidance: supply external routes with `EdgePath::try_polyline` when needed. Spatial handles belong to one index and are invalidated by removal or full rebuild.

See the [API documentation](https://docs.rs/pautinka-core) for contracts and the [project README](../../README.md) for development checks. Version 0.1 is experimental and declares no MSRV. Licensed under [MIT](LICENSE-MIT) OR [Apache-2.0](LICENSE-APACHE).

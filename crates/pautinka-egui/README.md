# pautinka-egui

[English](README.md) · [Русский](README.ru.md)

An interactive `egui` canvas for network diagrams. Implement `Graph` for your data: the widget borrows labels and paths, while your application owns layout, routes and editing history. Shared geometry and routing live in [pautinka-core](../pautinka-core).

```toml
[dependencies]
egui = "0.37"
pautinka-egui = "0.1"
```

Keep a `Canvas` between frames and prepare routes when data changes:

```rust
use pautinka_egui::{Canvas, Graph, InteractionOptions};

fn network_ui(ui: &mut egui::Ui, canvas: &mut Canvas, graph: &dyn Graph) {
    canvas.options.interaction = InteractionOptions::read_only();
    if let Err(error) = canvas.try_show(ui, graph) {
        eprintln!("Cannot display network: {error}");
    }
}
```

## Features and examples

- Multiple and rectangle selection, keyboard navigation, tooltips and context actions.
- Event-driven dragging, cancellation, pinned nodes and nested group frames.
- Custom renderers, node shapes, labels, patterned edges and crossing gaps or bridges.
- Pan, zoom, fit/focus, minimap, adaptive grids and axis, diagonal or grid snapping.
- Cached picking/culling, incremental spatial updates and background preparation.
- Optional SVG/PNG export without a window or GPU.

Run any example with `cargo run -p pautinka-egui --example <name>`:

| Example | Demonstrates |
| --- | --- |
| [basic](examples/basic.rs) | Routing and camera controls |
| [custom](examples/custom.rs) | Equipment symbols, ports and styled connections |
| [shapes](examples/shapes.rs) | Node shapes and contour-based connections |
| [editing](examples/editing.rs) | Moving selected nodes, cancellation and Undo |
| [petgraph](examples/petgraph.rs) | Adapting a sparse `StableDiGraph` |
| [groups](examples/groups.rs) | Nested groups, dragging and collapsed views |
| [labels](examples/labels.rs) | Automatic label placement |

Drag the background to pan, scroll/pinch to zoom, and double-click the background or press Home to fit. Ctrl/Command + click toggles selection; Shift + click adds to it; Shift + drag selects a region. With canvas focus, arrows navigate, Ctrl/Command + A selects all, and F focuses the primary selection. Escape cancels a gesture; otherwise it clears selection.

## Integration

`Graph` uses dense, temporary indices and stable IDs. Increment topology revision for insertion/deletion/reordering, geometry revision for positions/routes/bounds, and appearance revision for visual changes that preserve bounds. Cache routes and `Graph::bounds` outside rendering. Call `Canvas::reset` when switching documents and use distinct `ui.push_id` scopes for multiple canvases.

Apply `CanvasEvent`s to your model and history before the next show clears the event buffer. The canvas never mutates your graph. `try_show` / `try_show_with` return errors; `show` / `show_with` display them. Connection creation and reconnection belong to the application; routing does not avoid obstacles. Geometry uses `f32`, so choose suitable origins and units.

A custom `Renderer` must provide bounds and hit-testing matching its painted geometry. Use the same renderer for preparation, picking and drawing; increment its revision when spatial geometry changes. In `node_hit_test`, use `HitPoint::relative_to(node.position)` to preserve pointer precision. Declare screen margins for painting outside element bounds.

`Graph::geometry_changes` enables local spatial updates. `PreparedGraph::new` can prepare an index in a worker thread; `Canvas::install_prepared` accepts it for the same graph instance and revision. `Canvas::stats()` exposes cache and index statistics.

## Shapes, edges and labels

Return a prepared `NodeShape` from `Graph::node_shape` for circles, ellipses, rectangles, triangles, diamonds or regular polygons. Dimensions use world coordinates; shapes with corners support rounding. Drawing, picking and `anchor` follow the contour.

```rust
use egui::Vec2;
use pautinka_egui::{CornerRadius, NodeShape};

let rectangle = NodeShape::rectangle(Vec2::new(80.0, 40.0))?
    .with_corner_radius(CornerRadius::try_from(12.0)?);
# Ok::<(), Box<dyn std::error::Error>>(())
```

Set `EdgeStyle::pattern` in `Renderer::edge_style` to `LinePattern::{Solid, Dashed, Dotted, DashDot, DashDotDot}`. Dash and gap lengths use screen points. `Style::crossings` configures `CrossingMode::{None, Gap, Bridge}`, sizes and ordering for polylines and Bézier curves. `Canvas::set_edge_crossings` overrides priority and style by `EdgeId`. Loops, self-intersections and overlaps are excluded.

Enable automatic labels with `canvas.options.labels = Some(labels::LabelLayoutOptions::default())`. Placement avoids labels, nodes and viewport edges; edges are not obstacles. `Renderer::node_label_options` controls priority, positions and visibility. Custom `paint_node` implementations must respect `show_label = false`; automatic labels use `Renderer::paint_node_label`. `Canvas::node_labels()` exposes the resulting rectangles.

## Groups

Implement `Graph::group_count` and `group` to return borrowed `Group` views, outer frames before inner ones. Build frames with `GroupFrame::new` or `GroupFrame::enclosing`. Groups support selection, focus, minimap and export; the application owns membership and hierarchy.

Header dragging emits `GroupDragStarted`, `GroupDragged`, then `GroupDragFinished` or `GroupDragCancelled`. Apply the total world-space `delta` against a snapshot from gesture start and restore it on cancellation. Group dragging does not snap. `Group::state` enables a collapse control that emits `GroupCollapseRequested`; the application rebuilds the visible graph and routes. The canvas does not move or hide elements itself.

Use `Graph::group_interaction` and global `interaction.group_drag`, `group_drag_modifier` and `group_toggle` to configure gestures. `read_only()` disables drag and collapse requests. Frame changes require a geometry revision and `Selection::Group(index)` in complete geometry journals. Include frames in `Graph::bounds`.

## Camera, grid and snapping

Enable the minimap with `canvas.options.navigator.enabled = true`; `navigator.corner` selects its corner. Clicking or dragging it pans the camera. `interaction.pan = false` makes it read only.

Set `canvas.options.grid = Some(GridOptions::default())` for an adaptive world-space grid. `GridKind::{Lines, Dots}`, colors and widths control appearance. `GridOptions::new(base_step, min_spacing)` accepts a positive finite world step and screen spacing of at least 8 points. Steps follow `base_step × 5ⁿ` as the camera zooms. Override `Renderer::paint_background` for a custom underlay; call `paint_grid` to retain the grid. This hook is not used by export.

```rust
use pautinka_egui::{Canvas, GridOptions, GridSnapMode, SnapKinds};

let mut canvas = Canvas::default();
canvas.options.grid = Some(GridOptions::default());
canvas.options.interaction.snapping = true;
canvas.options.snapping.kinds = SnapKinds::AXES | SnapKinds::DIAGONALS;
canvas.options.grid_snap = GridSnapMode::Fixed;
```

Snapping is off by default. `GridSnapMode::Fixed` uses the base step; `Adaptive` uses the rendered step; `Disabled` leaves only vertex guides. `SnapKinds::NONE` allows grid-only snapping. Grid visibility is independent: `grid.visible = false` retains settings, while `grid = None` uses default settings for snapping. Capture/release tolerances default to 6/9 screen points. Drag events contain snapped positions; the application applies the leader's displacement to other selected nodes.

## SVG and PNG export

Enable `svg` for the `export` module, or `png` for both formats. The facade crate `pautinka` forwards these features. Export returns an SVG `String` or encoded PNG `Vec<u8>` for the application to save.

```rust
# #[cfg(feature = "png")]
# fn save(graph: &impl pautinka_egui::Graph) -> Result<(), Box<dyn std::error::Error>> {
use pautinka_egui::{DefaultRenderer, Style, export::{ExportOptions, Exporter}};

let exporter = Exporter::default();
let options = ExportOptions::new(2.0, 24.0)?;
let style = Style::default();
std::fs::write("diagram.svg", exporter.svg(graph, &DefaultRenderer, &style, &options)?)?;
std::fs::write("diagram.png", exporter.png(graph, &DefaultRenderer, &style, &options, 2.0)?)?;
# Ok(())
# }
```

`ExportOptions` controls scale, padding, cropping, background, labels and loops. Automatic label placement is enabled separately with `with_label_layout(Some(options))?`. Export excludes interactive controls and selection. Labels become vector outlines; use `Exporter::new` with your `egui::FontDefinitions` for custom fonts. Unsupported painting, such as textures or GPU callbacks, returns `ExportError::UnsupportedShape`.

```sh
cargo run -p pautinka-egui --features png --example export -- /tmp/pautinka-export
```

use std::time::Duration;

use egui::{Ui, WidgetText};
use pautinka::egui::{CornerRadius, CrossingMode, Direction, GridOptions, RoutingMode};

use crate::scene::{Appearance, Load, Motion, NodeKind, SceneStyle, Topology};

use super::{Demo, Pattern};

pub(super) struct Controls {
    pub load: Load,
    pub style: SceneStyle,
    pub topology: Topology,
    pub restart: bool,
    pub changed: bool,
}

impl Demo {
    pub(super) fn controls(&mut self, ui: &mut Ui) -> Controls {
        let mut controls = Controls {
            load: self.scene.load(),
            style: self.scene.style(),
            topology: self.scene.topology(),
            restart: false,
            changed: false,
        };
        let frame =
            egui::Frame::side_top_panel(ui.style()).inner_margin(egui::Margin::symmetric(8, 6));
        let panel = egui::Panel::top("stress-controls").frame(frame);
        panel.show(ui, |ui| {
            ui.spacing_mut().scroll = egui::style::ScrollStyle::solid();
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.hyperlink_to("GitHub", "https://github.com/hexqnt/pautinka");
                ui.weak(concat!("pautinka v", env!("CARGO_PKG_VERSION")));
                ui.separator();
                egui::ScrollArea::horizontal()
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                            let animation_icon = if self.running {
                                egui::include_image!("../../assets/player-pause.svg")
                            } else {
                                egui::include_image!("../../assets/player-play.svg")
                            };
                            controls.changed |=
                                toggle_button(ui, &mut self.running, "Animate", animation_icon)
                                    .changed();
                            if toggle_button(
                                ui,
                                &mut self.growth.enabled,
                                "Auto grow",
                                egui::include_image!("../../assets/trending-up.svg"),
                            )
                            .changed()
                            {
                                self.growth.reset();
                            }
                            controls.restart = ui
                                .add(icon_button(
                                    "Restart",
                                    egui::include_image!("../../assets/reload.svg"),
                                ))
                                .clicked();
                            controls.changed |= toggle_button(
                                ui,
                                &mut self.auto_fit,
                                "Auto fit",
                                egui::include_image!("../../assets/maximize.svg"),
                            )
                            .on_hover_text(
                                "Keep the whole graph in view. Disable to pan and zoom freely.",
                            )
                            .changed();
                            ui.separator();
                            menu(ui, "Load", |ui| self.load_controls(ui, &mut controls.load));
                            menu(ui, "Topology", |ui| {
                                Self::topology_controls(ui, &mut controls.topology);
                            });
                            menu(ui, "Motion", |ui| {
                                controls.changed |= self.motion_controls(ui);
                            });
                            menu(ui, "Nodes", |ui| {
                                Self::node_controls(ui, &mut controls.style.appearance);
                            });
                            menu(ui, "Edges", |ui| {
                                controls.changed |= self.edge_controls(ui, &mut controls.style);
                            });
                            menu(ui, "View", |ui| controls.changed |= self.view_controls(ui));
                        });
                    });
            });
        });
        controls.changed |= controls.load != self.scene.load()
            || controls.style != self.scene.style()
            || controls.topology != self.scene.topology();
        controls
    }

    fn load_controls(&mut self, ui: &mut Ui, load: &mut Load) {
        ui.weak("Growth keeps existing node positions.");
        let mut nodes = load.nodes();
        let mut edges = load.edges_per_node();
        let manual = ui
            .add(
                egui::Slider::new(&mut nodes, *Load::NODES.start()..=self.growth.maximum())
                    .logarithmic(true)
                    .text("Nodes"),
            )
            .changed();
        let density = ui
            .add(
                egui::Slider::new(&mut edges, Load::EDGES_PER_NODE)
                    .step_by(0.01)
                    .max_decimals(2)
                    .text("Edges / node"),
            )
            .changed();
        if manual || density {
            *load = Load::new(nodes, edges).expect("Sliders must stay within load bounds");
            if manual {
                self.growth.enabled = false;
            }
        }
        ui.weak(format!(
            "{} edges · mean degree {:.2} ignoring arrows",
            load.edge_count(),
            load.edge_count() as f64 * 2.0 / load.nodes() as f64
        ));
        let mut seconds = self.growth.interval.as_secs_f64();
        if ui
            .add(
                egui::Slider::new(&mut seconds, self.growth.seconds_range())
                    .suffix(" s")
                    .text("Step duration"),
            )
            .changed()
        {
            self.growth.interval = Duration::from_secs_f64(seconds);
        }
        ui.separator();
        ui.horizontal(|ui| {
            ui.label("Seed");
            ui.add(egui::DragValue::new(&mut self.seed));
        });
        ui.weak(format!(
            "Restart applies the seed and starts at {} nodes.",
            self.initial_nodes.get()
        ));
    }

    fn topology_controls(ui: &mut Ui, topology: &mut Topology) {
        ui.horizontal_wrapped(|ui| {
            for (label, preset) in [
                ("Local", Topology::LOCAL),
                ("Mixed", Topology::MIXED),
                ("Random", Topology::RANDOM),
                ("Hubs", Topology::HUB),
            ] {
                if ui.selectable_label(*topology == preset, label).clicked() {
                    *topology = preset;
                }
            }
        });
        ui.separator();
        let mut locality = topology.locality();
        let mut radius = topology.local_radius();
        let mut hub_share = topology.hub_share();
        let mut hubs = topology.hubs();
        let mut backbone = topology.backbone();
        ui.add(
            egui::Slider::new(&mut locality, Topology::PERCENT)
                .suffix(" %")
                .text("Local links"),
        );
        ui.add(egui::Slider::new(&mut radius, Topology::LOCAL_RADIUS).text("Local radius"));
        ui.weak("Local links prefer neighbors within ±radius grid cells on each axis.");
        ui.separator();
        ui.add(
            egui::Slider::new(&mut hub_share, Topology::PERCENT)
                .suffix(" %")
                .text("Hub links"),
        );
        ui.add_enabled(
            hub_share > 0,
            egui::Slider::new(&mut hubs, Topology::HUBS).text("Hub nodes"),
        );
        ui.weak("Hub links target a small set of nodes. Locality applies to the remaining links.");
        ui.separator();
        ui.checkbox(&mut backbone, "Connected backbone (E/V ≥ 1)");
        ui.weak("Adds a spanning chain when edges/node is at least 1. Disable it to allow separate components.");
        ui.weak("Shares apply to links beyond the backbone. If eligible targets are exhausted, global links fill the remainder. No self-links or duplicate source-target pairs.");
        *topology = Topology::new(locality, radius, hub_share, hubs, backbone)
            .expect("Sliders must stay within topology bounds");
    }

    fn motion_controls(&mut self, ui: &mut Ui) -> bool {
        ui.weak("Elliptical orbits; dragging moves the orbit center.");
        let mut fraction = self.motion.fraction();
        let mut speed = self.motion.speed();
        let fraction_changed = ui
            .add(
                egui::Slider::new(&mut fraction, Motion::FRACTION)
                    .suffix(" %")
                    .text("Moving nodes"),
            )
            .changed();
        let speed_changed = ui
            .add(
                egui::Slider::new(&mut speed, Motion::SPEED)
                    .suffix("×")
                    .text("Speed"),
            )
            .changed();
        let changed = fraction_changed || speed_changed;
        if changed {
            self.motion =
                Motion::new(fraction, speed).expect("Sliders must stay within motion bounds");
        }
        ui.weak("Auto growth pauses during a node drag.");
        changed
    }

    fn node_controls(ui: &mut Ui, appearance: &mut Appearance) {
        let mut colors = appearance.colors();
        let mut radius = appearance.radius();
        let mut kind = appearance.kind;
        let colors_changed = ui
            .add(egui::Slider::new(&mut colors, Appearance::COLORS).text("Colors"))
            .changed();
        let size_changed = ui
            .add(egui::Slider::new(&mut radius, Appearance::RADIUS).text("Radius"))
            .changed();
        ui.separator();
        radios(
            ui,
            &mut kind,
            [
                (NodeKind::Mixed, "Mixed shapes"),
                (NodeKind::Circle, "Circles"),
                (NodeKind::Rectangle, "Rectangles"),
                (NodeKind::Diamond, "Diamonds"),
                (NodeKind::Triangle, "Triangles"),
                (NodeKind::Hexagon, "Hexagons"),
            ],
        );
        if colors_changed || size_changed || kind != appearance.kind {
            *appearance = Appearance::new(kind, colors, radius)
                .expect("Sliders must stay within appearance bounds");
        }
    }

    fn edge_controls(&mut self, ui: &mut Ui, style: &mut SceneStyle) -> bool {
        let before = (self.pattern, self.canvas.style.crossings.mode);
        ui.strong("Geometry");
        radios(
            ui,
            &mut style.routing,
            [
                (RoutingMode::Straight, "Straight"),
                (RoutingMode::Curved, "Curved"),
                (RoutingMode::Orthogonal, "Orthogonal · 90°"),
                (RoutingMode::Octilinear, "Octilinear · 45°"),
                (RoutingMode::Angular15, "Angular · 15°"),
            ],
        );
        let mut radius = style.corner_radius.get();
        if ui
            .add_enabled(
                !matches!(style.routing, RoutingMode::Straight | RoutingMode::Curved),
                egui::Slider::new(&mut radius, 0.0..=100.0)
                    .clamping(egui::SliderClamping::Edits)
                    .text("Corner radius"),
            )
            .on_hover_text("Rounds angular edges in graph units. Zero disables rounding.")
            .changed()
        {
            style.corner_radius =
                CornerRadius::try_from(radius).expect("Slider must keep the corner radius valid");
        }
        ui.separator();
        radios(
            ui,
            &mut self.pattern,
            [
                (Pattern::Solid, "Solid"),
                (Pattern::Dashed, "Dashed"),
                (Pattern::Dotted, "Dotted"),
            ],
        );
        ui.separator();
        radios(
            ui,
            &mut style.direction,
            [
                (Direction::Undirected, "No arrows"),
                (Direction::Forward, "Forward arrows"),
                (Direction::Both, "Both directions"),
            ],
        );
        ui.separator();
        ui.strong("Crossings");
        radios(
            ui,
            &mut self.canvas.style.crossings.mode,
            [
                (CrossingMode::None, "No decoration"),
                (CrossingMode::Gap, "Gaps"),
                (CrossingMode::Bridge, "Bridges"),
            ],
        );
        before != (self.pattern, self.canvas.style.crossings.mode)
    }

    fn view_controls(&mut self, ui: &mut Ui) -> bool {
        let mut changed = ui
            .checkbox(&mut self.canvas.options.navigator.enabled, "Minimap")
            .changed();
        let mut labels = self.canvas.labels();
        if ui.checkbox(&mut labels, "Labels").changed() {
            self.canvas.set_labels(labels);
            changed = true;
        }
        changed |= ui
            .add(
                egui::Slider::new(&mut self.canvas.style.label_min_zoom, 0.0..=2.0)
                    .text("Label zoom"),
            )
            .changed();
        changed |= ui
            .checkbox(
                &mut self.canvas.options.lod.enabled,
                "Overview simplification (LOD)",
            )
            .changed();
        let mut grid = self.canvas.options.grid.is_some();
        if ui.checkbox(&mut grid, "Grid").changed() {
            self.canvas.options.grid = grid.then(GridOptions::default);
            changed = true;
        }
        ui.weak("Auto fit keeps the whole graph in view. Disable it to pan and zoom freely.");
        ui.separator();
        ui.weak(
            "Drag background to pan; scroll / pinch to zoom. Middle-click, Home or double-click fits the graph.",
        );
        ui.separator();
        ui.weak("HUD: N/E show visible/total counts; edges in view use bounding boxes. FPS follows frame intervals. CPU excludes tessellation and GPU.");
        let stats = self.canvas.stats();
        ui.weak(format!(
            "Index: {} rebuilds / {} updated entries",
            stats.rebuilds, stats.updated_entries
        ));
        changed
    }
}

fn icon_button(label: &'static str, icon: egui::ImageSource<'static>) -> egui::Button<'static> {
    egui::Button::image_and_text(
        egui::Image::new(icon).fit_to_exact_size(egui::Vec2::splat(16.0)),
        label,
    )
    .image_tint_follows_text_color(true)
}

fn toggle_button(
    ui: &mut Ui,
    value: &mut bool,
    label: &'static str,
    icon: egui::ImageSource<'static>,
) -> egui::Response {
    let mut response = ui.add(icon_button(label, icon).selected(*value));
    if response.clicked() {
        *value = !*value;
        response.mark_changed();
    }
    response
}

fn radios<T: PartialEq>(
    ui: &mut Ui,
    value: &mut T,
    choices: impl IntoIterator<Item = (T, &'static str)>,
) {
    for (choice, label) in choices {
        ui.radio_value(value, choice, label);
    }
}

fn menu(ui: &mut Ui, label: impl Into<WidgetText>, contents: impl FnOnce(&mut Ui)) {
    let height = (ui.ctx().content_rect().height() * 0.6).max(140.0);
    egui::containers::menu::MenuButton::new(label)
        .config(
            egui::containers::menu::MenuConfig::new()
                .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside),
        )
        .ui(ui, |ui| {
            ui.set_max_width(280.0);
            egui::ScrollArea::vertical()
                .max_height(height)
                .show(ui, contents);
        });
}

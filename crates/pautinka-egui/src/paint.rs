mod alignment;
mod edge;
mod pattern;

use egui::{Align2, Color32, FontId, Rect, Role, Sense, Ui, Vec2, WidgetInfo};

use crate::{
    Canvas, CanvasEvent, DetailLevel, ElementId, ElementState, Graph, NodeGeometry, NodeVisual,
    RenderContext, Renderer, Selection,
    interaction::{WidgetInput, synthetic_click},
};

impl Canvas {
    pub(super) fn element_state(&self, id: ElementId) -> ElementState {
        ElementState {
            selected: self.is_selected(id),
            highlighted: self.is_highlighted(id),
            hovered: self.hovered == Some(id),
        }
    }

    pub(super) fn paint(
        &mut self,
        ui: &Ui,
        graph: &(impl Graph + ?Sized),
        renderer: &(impl Renderer + ?Sized),
        viewport: Rect,
        pointer_blocked: bool,
    ) -> Option<egui::Response> {
        let mut context_response =
            self.paint_groups(ui, graph, renderer, viewport, pointer_blocked);
        let painter = ui.painter_at(viewport);
        let input = WidgetInput::new(ui, viewport, pointer_blocked);
        let pointer_world = input
            .pointer
            .and_then(|p| self.camera.to_world_precise(p, viewport));
        let node_actions = self.options.interaction.selection
            || self.options.interaction.activation
            || self.options.interaction.context_menu
            || self.options.interaction.keyboard
            || self.options.interaction.node_drag;
        let drag_enabled = !pointer_blocked
            && self.options.interaction.node_drag
            && self
                .options
                .interaction
                .node_drag_modifier
                .matches(input.modifiers);
        self.paint_edges(
            graph,
            renderer,
            &RenderContext {
                painter: &painter,
                camera: self.camera,
                viewport,
                style: &self.style,
                detail: self.detail,
            },
        );
        let mut label_frame = self.begin_labels(graph.id());
        let ordinary_labels = self.detail == DetailLevel::Full
            && self.labels
            && self.camera.zoom >= self.style.label_min_zoom;
        for position in 0..self.spatial.visible_nodes.len() {
            let index = self.spatial.visible_nodes[position];
            let node = graph.node(index);
            let geometry = graph.node_geometry(index);
            let rect = geometry.screen_rect(node.position, self.camera, viewport);
            if !viewport
                .expand(graph.node_screen_margin(index))
                .intersects(rect)
            {
                continue;
            }
            let state = self.element_state(ElementId::Node(node.id));
            let show_label = (state.selected || state.highlighted || ordinary_labels)
                && self.options.labels.is_none();
            let visual = NodeVisual {
                index,
                rect,
                shape: graph.node_shape(index),
                state,
                style: renderer.node_style(index, node, state, &self.style),
                show_label,
            };
            if let Some(frame) = &mut label_frame {
                frame.push(renderer, &painter, node, &visual, ordinary_labels);
            }
            renderer.paint_node(
                &RenderContext {
                    painter: &painter,
                    camera: self.camera,
                    viewport,
                    style: &self.style,
                    detail: self.detail,
                },
                node,
                &visual,
            );
            // Порты входят в геометрию владельца; маркеры имеют экранный размер и исчезают в обзоре.
            if self.detail == DetailLevel::Full || state.emphasized() {
                for port in graph.node_ports(index) {
                    painter.circle_filled(
                        self.camera.to_screen(port.anchor.position(), viewport),
                        2.5,
                        node.color,
                    );
                }
            }
            let policy = graph.node_interaction(index);
            let hit_rect = Rect::from_center_size(
                rect.center(),
                rect.size().max(Vec2::splat(NodeGeometry::MIN_HIT_SIZE)),
            );
            let pointer_matches = !pointer_blocked
                && input
                    .pointer
                    .zip(pointer_world)
                    .is_none_or(|(screen, world)| {
                        hit_rect.contains(screen)
                            && graph.node_hit_test(index, world, self.camera.zoom)
                    });
            let active_drag = self.drag.as_ref().is_some_and(|drag| drag.id == node.id);
            let node_id = ui.scope_id().with(node.id);
            let sense = if !node_actions || pointer_blocked {
                Sense::hover()
            } else if (pointer_matches || active_drag) && drag_enabled && policy.draggable {
                Sense::click_and_drag()
            } else {
                input.click_sense(
                    node_id,
                    pointer_matches || active_drag,
                    self.options.interaction.keyboard,
                )
            };
            let node_response = ui.interact(hit_rect, node_id, sense);
            node_response.widget_info(|| {
                WidgetInfo::selected(
                    Role::Button,
                    node_actions,
                    state.selected,
                    node.accessibility_label,
                )
            });
            if input.accessibility {
                ui.ctx()
                    .accesskit_node_builder(node_response.id, |builder| {
                        builder.set_selected(state.selected);
                    });
            }
            if self.options.interaction.context_menu
                && pointer_matches
                && node_response.secondary_clicked()
            {
                self.request_context(
                    Some(ElementId::Node(node.id)),
                    node_response
                        .interact_pointer_pos()
                        .map(|p| self.camera.to_world(p, viewport)),
                );
                context_response = Some(node_response.clone());
            }
            let synthetic_click = synthetic_click(&node_response);
            if node_actions && (pointer_matches || synthetic_click) && node_response.clicked() {
                self.focus_requested = true;
                self.interactive_select(Some(Selection::Node(index)), input.modifiers, graph);
                if self.options.interaction.activation {
                    let position = if synthetic_click {
                        node.position
                    } else {
                        node_response
                            .interact_pointer_pos()
                            .map_or(node.position, |p| self.camera.to_world(p, viewport))
                    };
                    self.events.push(CanvasEvent::Clicked {
                        target: Some(ElementId::Node(node.id)),
                        position,
                    });
                    if synthetic_click {
                        self.events.push(CanvasEvent::Activated {
                            element: ElementId::Node(node.id),
                            position,
                        });
                    }
                }
                ui.ctx().request_repaint();
            }
            if self.options.interaction.activation
                && pointer_matches
                && node_response.double_clicked()
            {
                self.events.push(CanvasEvent::Activated {
                    element: ElementId::Node(node.id),
                    position: node.position,
                });
            }
            if drag_enabled
                && policy.draggable
                && node_response.drag_started_by(egui::PointerButton::Primary)
                && let Some(origin) = input.origin
                && self
                    .camera
                    .to_world_precise(origin, viewport)
                    .is_some_and(|point| graph.node_hit_test(index, point, self.camera.zoom))
            {
                self.start_node_drag(graph, index, origin, viewport);
            }
            if pointer_matches {
                node_response.on_hover_ui(|ui| graph.node_tooltip(ui, index));
            }
        }
        self.label_cache = label_frame.map(|frame| frame.finish(viewport));
        self.paint_labels(graph, renderer, &painter, viewport);
        if graph.node_count() == 0 && graph.edge_count() == 0 && graph.group_count() == 0 {
            painter.text(
                viewport.center(),
                Align2::CENTER_CENTER,
                &self.style.empty_text,
                FontId::proportional(18.0),
                Color32::LIGHT_GRAY,
            );
        }
        context_response
    }
}

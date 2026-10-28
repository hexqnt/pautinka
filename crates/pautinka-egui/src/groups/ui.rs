use egui::{Rect, Response, Role, Sense, Ui, WidgetInfo};

use crate::{
    Canvas, CanvasEvent, ElementId, Graph, Group, GroupInteraction, RenderContext, Renderer,
    Selection,
    interaction::{WidgetInput, synthetic_click},
};

use super::{GroupPart, GroupScreenGeometry};

struct GroupWidget<'a> {
    index: usize,
    group: Group<'a>,
    geometry: GroupScreenGeometry,
    hit: Option<GroupPart>,
    policy: GroupInteraction,
}

impl Canvas {
    pub(crate) fn paint_groups(
        &mut self,
        ui: &Ui,
        graph: &(impl Graph + ?Sized),
        renderer: &(impl Renderer + ?Sized),
        viewport: Rect,
        pointer_blocked: bool,
    ) -> Option<Response> {
        if self.spatial.visible_groups.is_empty() {
            return None;
        }
        let painter = ui.painter_at(viewport);
        let input = WidgetInput::new(ui, viewport, pointer_blocked);
        let mut context_response = None;
        for position in 0..self.spatial.visible_groups.len() {
            let index = self.spatial.visible_groups[position];
            let group = graph.group(index);
            let state = self.element_state(ElementId::Group(group.id));
            let geometry = group.screen_geometry(self.camera, viewport);
            renderer.paint_group(
                &RenderContext {
                    painter: &painter,
                    camera: self.camera,
                    viewport,
                    style: &self.style,
                    detail: self.detail,
                },
                group,
                &crate::GroupVisual {
                    index,
                    rect: geometry.rect,
                    header_rect: geometry.header,
                    toggle_rect: geometry.toggle,
                    state,
                    style: renderer.group_style(index, group, state, &self.style),
                },
            );
            let widget = GroupWidget {
                index,
                group,
                geometry,
                policy: graph.group_interaction(index),
                hit: input
                    .pointer
                    .filter(|_| !input.blocked && state.hovered)
                    .and_then(|point| geometry.part_at(point)),
            };
            if let Some(response) = self.interact_group(ui, graph, &widget, &input) {
                context_response = Some(response);
            }
        }
        context_response
    }

    fn interact_group(
        &mut self,
        ui: &Ui,
        graph: &(impl Graph + ?Sized),
        widget: &GroupWidget<'_>,
        input: &WidgetInput,
    ) -> Option<Response> {
        let GroupWidget {
            index,
            group,
            geometry,
            hit,
            policy,
        } = *widget;
        let options = self.options.interaction;
        let id = ElementId::Group(group.id);
        let widget_id = ui.scope_id().with(("group", group.id));
        let actions = options.selection
            || options.activation
            || options.context_menu
            || options.keyboard
            || options.group_drag;
        let drag_enabled = options.group_drag
            && policy.draggable
            && options.group_drag_modifier.matches(input.modifiers);
        let active = self
            .group_drag
            .as_ref()
            .is_some_and(|drag| drag.id == group.id);
        let hovered = hit.is_some() && hit != Some(GroupPart::Toggle);
        let sense = if input.blocked || !actions {
            Sense::hover()
        } else if active || (drag_enabled && hit == Some(GroupPart::Header)) {
            Sense::click_and_drag()
        } else {
            input.click_sense(widget_id, hovered, options.keyboard)
        };
        let response = ui.interact(
            geometry.interaction_rect(hit).intersect(input.viewport),
            widget_id,
            sense,
        );
        let selected = self.is_selected(id);
        response.widget_info(|| {
            WidgetInfo::selected(Role::Button, actions, selected, group.accessibility_label)
        });
        if input.accessibility {
            ui.ctx().accesskit_node_builder(response.id, |builder| {
                builder.set_selected(selected);
            });
        }
        let synthetic = synthetic_click(&response);
        if response.clicked() && (hovered || synthetic) {
            self.focus_requested = true;
            self.interactive_select(Some(Selection::Group(index)), input.modifiers, graph);
            if options.activation {
                let position = if synthetic {
                    group.frame.header().center()
                } else {
                    response.interact_pointer_pos().map_or_else(
                        || group.frame.header().center(),
                        |p| self.camera.to_world(p, input.viewport),
                    )
                };
                self.events.push(CanvasEvent::Clicked {
                    target: Some(id),
                    position,
                });
                if synthetic || response.double_clicked() {
                    self.events.push(CanvasEvent::Activated {
                        element: id,
                        position,
                    });
                }
            }
            ui.ctx().request_repaint();
        }
        let context = options.context_menu && hit.is_some() && response.secondary_clicked();
        if context {
            self.request_context(
                Some(id),
                response
                    .interact_pointer_pos()
                    .map(|p| self.camera.to_world(p, input.viewport)),
            );
        }
        if !input.blocked
            && drag_enabled
            && response.drag_started_by(egui::PointerButton::Primary)
            && let Some(origin) = input.origin
            && geometry.part_at(origin) == Some(GroupPart::Header)
            && self.drag.is_none()
            && self.group_drag.is_none()
        {
            self.start_group_drag(graph, index, origin, input.viewport);
        }
        self.interact_group_toggle(ui, widget, input);
        let context_response = context.then(|| response.clone());
        if hit.is_some() {
            response.on_hover_ui(|ui| graph.group_tooltip(ui, index));
        }
        context_response
    }

    fn interact_group_toggle(&mut self, ui: &Ui, widget: &GroupWidget<'_>, input: &WidgetInput) {
        let Some(toggle) = widget.geometry.toggle else {
            return;
        };
        let options = self.options.interaction;
        let enabled = options.group_toggle && !input.blocked;
        let hovered = widget.hit == Some(GroupPart::Toggle);
        let id = ui.scope_id().with(("group-collapse", widget.group.id));
        let sense = if enabled {
            input.click_sense(id, hovered, options.keyboard)
        } else {
            Sense::hover()
        };
        let response = ui.interact(toggle.intersect(input.viewport), id, sense);
        let collapsed = widget.geometry.collapsed;
        response.widget_info(|| {
            WidgetInfo::labeled(
                Role::Button,
                enabled,
                if collapsed {
                    "Expand group"
                } else {
                    "Collapse group"
                },
            )
        });
        if enabled && response.clicked() && (hovered || synthetic_click(&response)) {
            self.focus_requested = true;
            self.events.push(CanvasEvent::GroupCollapseRequested {
                group: widget.group.id,
                collapsed: !collapsed,
            });
            ui.ctx().request_repaint();
        }
    }
}

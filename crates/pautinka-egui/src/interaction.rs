mod widgets;

pub(crate) use widgets::{WidgetInput, synthetic_click};

use egui::{CursorIcon, Key, Modifiers, Pos2, Rect, Response, Stroke, StrokeKind, Ui, Vec2};

use crate::{Canvas, CanvasEvent, ElementId, Graph, NodeId, Selection, snapping::DragSnap};

pub(super) struct NodeDrag {
    pub id: NodeId,
    pub index: usize,
    pub offset: Vec2,
    pub position: Pos2,
    pub snap: DragSnap,
}

pub(super) struct Marquee {
    origin: Pos2,
    current: Pos2,
    additive: bool,
}

impl Canvas {
    pub(super) fn update_cursor(
        &self,
        ui: &Ui,
        graph: &(impl Graph + ?Sized),
        viewport: Rect,
        panning: bool,
    ) {
        if !ui.is_enabled() {
            return;
        }
        let cursor = if self.drag.is_some() || self.group_drag.is_some() || panning {
            CursorIcon::Grabbing
        } else if self.marquee.is_some() {
            CursorIcon::Crosshair
        } else if ui.rect_contains_pointer(viewport) {
            let interaction = self.options.interaction;
            let (modifiers, pointer) =
                ui.input(|input| (input.modifiers, input.pointer.hover_pos()));
            let hit = self.hit_cache.and_then(|(_, hit)| hit);
            let group_cursor = match hit {
                Some(Selection::Group(index)) => pointer
                    .and_then(|point| self.group_cursor(graph, index, point, viewport, modifiers)),
                _ => None,
            };
            if let Some(cursor) = group_cursor {
                cursor
            } else {
                match hit {
                    Some(Selection::Node(index))
                        if interaction.node_drag
                            && interaction.node_drag_modifier.matches(modifiers)
                            && graph.node_interaction(index).draggable =>
                    {
                        CursorIcon::Grab
                    }
                    Some(selection)
                        if interaction.activation
                            || (interaction.selection
                                && match selection {
                                    Selection::Node(index) => {
                                        graph.node_interaction(index).selectable
                                    }
                                    Selection::Edge(index) => graph.edge_selectable(index),
                                    Selection::Group(index) => {
                                        graph.group_interaction(index).selectable
                                    }
                                }) =>
                    {
                        CursorIcon::PointingHand
                    }
                    None if interaction.selection
                        && interaction.marquee_modifier.matches(modifiers) =>
                    {
                        CursorIcon::Crosshair
                    }
                    None if interaction.pan && interaction.pan_modifier.matches(modifiers) => {
                        CursorIcon::Grab
                    }
                    _ => return,
                }
            }
        } else {
            return;
        };
        ui.ctx().set_cursor_icon(cursor);
    }

    pub(super) fn request_context(&mut self, target: Option<ElementId>, position: Option<Pos2>) {
        self.context_target = target;
        self.context_position = position;
        self.events.push(CanvasEvent::ContextRequested(target));
        if let Some(position) = position {
            self.events
                .push(CanvasEvent::ContextPositionRequested { target, position });
        }
    }

    pub(super) fn start_node_drag(
        &mut self,
        graph: &(impl Graph + ?Sized),
        index: usize,
        origin: Pos2,
        viewport: Rect,
    ) {
        let node = graph.node(index);
        self.focus_requested = true;
        self.initial_fit_complete = true;
        self.drag = Some(NodeDrag {
            id: node.id,
            index,
            offset: node.position - self.camera.to_world(origin, viewport),
            position: node.position,
            snap: DragSnap::default(),
        });
        if self.options.interaction.selection && !self.is_selected(ElementId::Node(node.id)) {
            self.select(Some(Selection::Node(index)), graph);
        }
        self.events.push(CanvasEvent::NodeDragStarted {
            node: node.id,
            position: node.position,
        });
    }

    pub(super) fn update_node_drag(
        &mut self,
        ui: &Ui,
        graph: &(impl Graph + ?Sized),
        viewport: Rect,
    ) {
        let Some(drag) = &mut self.drag else {
            return;
        };
        let resolved =
            if graph.identity(Selection::Node(drag.index)) == Some(ElementId::Node(drag.id)) {
                Some(Selection::Node(drag.index))
            } else {
                graph.resolve(ElementId::Node(drag.id))
            };
        if let Some(Selection::Node(index)) = resolved {
            drag.index = index;
        }
        let cancelled = resolved.is_none()
            || !self.options.interaction.node_drag
            || !graph.node_interaction(drag.index).draggable;
        let Some((pointer, released)) = drag_pointer(ui).filter(|_| !cancelled) else {
            self.events
                .push(CanvasEvent::NodeDragCancelled { node: drag.id });
            self.drag = None;
            ui.ctx().request_repaint();
            return;
        };
        let position = self.camera.to_world(pointer, viewport) + drag.offset;
        drag.snap.result =
            drag.snap
                .prepare(&self.options, self.camera, viewport)
                .map(|(options, grid)| {
                    if !options.kinds.is_empty() {
                        self.snapping
                            .synchronize(graph, drag.id, &self.selection, options.kinds);
                    }
                    self.snapping
                        .index
                        .snap_with_grid(position, options, grid, drag.snap.result)
                });
        let position = drag.snap.result.map_or(position, |snap| snap.position);
        if released {
            self.events.push(CanvasEvent::NodeDragFinished {
                node: drag.id,
                position,
            });
            self.drag = None;
        } else if position != drag.position {
            drag.position = position;
            self.events.push(CanvasEvent::NodeDragged {
                node: drag.id,
                position,
            });
        } else {
            return;
        }
        ui.ctx().request_repaint();
    }

    pub(super) fn update_zoom(&mut self, ui: &Ui, viewport: Rect, minimum_zoom: f32) {
        if !self.options.interaction.zoom {
            return;
        }
        // Проверка слоя учитывает дочерние кнопки узлов, но исключает перекрывающие окна.
        if !ui.rect_contains_pointer(viewport) {
            return;
        }
        let Some((pointer, factor)) = ui.input(|input| {
            let pointer = input.pointer.hover_pos()?;
            let factor = if input.smooth_scroll_delta.y != 0.0
                && !input.modifiers.ctrl
                && !input.modifiers.command
            {
                (input.smooth_scroll_delta.y * self.options.camera.wheel_sensitivity).exp()
            } else {
                input.zoom_delta()
            };
            Some((pointer, factor))
        }) else {
            return;
        };
        if factor != 1.0 && !factor.is_nan() && factor >= 0.0 {
            self.initial_fit_complete = true;
            let zoom = self
                .options
                .camera
                .zoom
                .clamp((self.camera.zoom * factor).max(minimum_zoom));
            self.camera.set_zoom_at(pointer, zoom, viewport);
        }
    }

    pub(super) fn update_marquee(
        &mut self,
        ui: &Ui,
        graph: &(impl Graph + ?Sized),
        viewport: Rect,
        response: &Response,
    ) {
        if !self.options.interaction.selection {
            self.marquee = None;
            return;
        }
        if response.drag_started()
            && self.group_drag.is_none()
            && self.drag.is_none()
            && self
                .options
                .interaction
                .marquee_modifier
                .matches(ui.input(|input| input.modifiers))
            && let Some(origin) = ui.input(|input| input.pointer.press_origin())
        {
            let origin = self.camera.to_world(origin, viewport);
            self.marquee = Some(Marquee {
                origin,
                current: origin,
                additive: self
                    .options
                    .interaction
                    .toggle_modifier
                    .matches(ui.input(|input| input.modifiers)),
            });
            self.focus_requested = true;
        }
        if let Some(marquee) = &mut self.marquee {
            if let Some((pointer, released)) = drag_pointer(ui) {
                let current = self.camera.to_world(pointer, viewport);
                if current != marquee.current {
                    marquee.current = current;
                    ui.ctx().request_repaint();
                }
                if released {
                    let bounds = Rect::from_two_pos(marquee.origin, marquee.current);
                    let additive = marquee.additive;
                    self.marquee = None;
                    self.select_rect(bounds, additive, graph);
                    ui.ctx().request_repaint();
                }
            } else {
                self.marquee = None;
                ui.ctx().request_repaint();
            }
        }
    }

    pub(super) fn paint_marquee(&self, ui: &Ui, viewport: Rect) {
        if let Some(marquee) = &self.marquee {
            let rect = Rect::from_two_pos(
                self.camera.to_screen(marquee.origin, viewport),
                self.camera.to_screen(marquee.current, viewport),
            );
            let painter = ui.painter_at(viewport);
            painter.rect_filled(rect, 0.0, self.style.selection.gamma_multiply(0.12));
            painter.rect_stroke(
                rect,
                0.0,
                Stroke::new(1.0, self.style.selection),
                StrokeKind::Inside,
            );
        }
    }

    pub(super) fn navigate(
        &mut self,
        ui: &Ui,
        graph: &(impl Graph + ?Sized),
        viewport: Rect,
        response: &Response,
    ) {
        if !self.options.interaction.keyboard
            || !(response.has_focus()
                || (response.lost_focus() && ui.input(|input| input.key_pressed(Key::Escape))))
            || ui.ctx().text_edit_focused()
            || self.group_drag.is_some()
            || self.drag.is_some()
            || self.marquee.is_some()
        {
            return;
        }
        if self.options.interaction.selection
            && ui.input_mut(|input| input.consume_key(Modifiers::COMMAND, Key::A))
        {
            self.select_all(graph);
            ui.ctx().request_repaint();
        }
        if self.options.interaction.selection
            && ui.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Escape))
        {
            self.select(None, graph);
            ui.ctx().request_repaint();
        }
        if ui.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Home)) {
            self.fit_graph(graph.bounds(), viewport);
            ui.ctx().request_repaint();
        }
        if ui.input_mut(|input| input.consume_key(Modifiers::NONE, Key::F))
            && let Some(selection) = self.selected()
        {
            self.pending_focus = Some(selection);
            ui.ctx().request_repaint();
        }
        for (key, direction) in [
            (Key::ArrowLeft, -Vec2::X),
            (Key::ArrowRight, Vec2::X),
            (Key::ArrowUp, -Vec2::Y),
            (Key::ArrowDown, Vec2::Y),
        ] {
            if !self.options.interaction.selection {
                break;
            }
            let modifiers = ui.input(|input| input.modifiers);
            if !ui.input_mut(|input| input.consume_key(modifiers, key)) {
                continue;
            }
            let origin = match self.selected() {
                Some(Selection::Node(index)) => graph.node(index).position,
                Some(Selection::Edge(index)) => graph.edge(index).path.midpoint(),
                Some(Selection::Group(index)) => graph.group(index).frame.header().center(),
                None => self.camera.center,
            };
            let nodes = (0..graph.node_count())
                .filter(|&index| graph.node_interaction(index).selectable)
                .map(|index| {
                    let node = graph.node(index);
                    (
                        Selection::Node(index),
                        ElementId::Node(node.id),
                        node.position,
                    )
                });
            let groups = (0..graph.group_count())
                .filter(|&index| graph.group_interaction(index).selectable)
                .map(|index| {
                    let group = graph.group(index);
                    (
                        Selection::Group(index),
                        ElementId::Group(group.id),
                        group.frame.header().center(),
                    )
                });
            let candidate = nodes
                .chain(groups)
                .filter_map(|(selection, id, position)| {
                    let delta = position - origin;
                    let forward = delta.dot(direction);
                    (forward > 0.0).then(|| {
                        (
                            selection,
                            id,
                            delta.length() * (1.0 + delta.dot(direction.rot90()).abs() / forward),
                        )
                    })
                })
                .min_by(|a, b| a.2.total_cmp(&b.2).then_with(|| a.1.cmp(&b.1)));
            if let Some((selection, _, _)) = candidate {
                self.interactive_select(Some(selection), modifiers, graph);
                self.pending_focus = Some(selection);
                self.initial_fit_complete = true;
                ui.ctx().request_repaint();
            }
        }
    }
}
pub(crate) fn drag_pointer(ui: &Ui) -> Option<(Pos2, bool)> {
    ui.input_mut(|input| {
        if input.consume_key(input.modifiers, Key::Escape) {
            return None;
        }
        let released = input.pointer.primary_released();
        if !input.focused || !(input.pointer.primary_down() || released) {
            return None;
        }
        Some((input.pointer.interact_pos()?, released))
    })
}

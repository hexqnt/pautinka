use egui::{Color32, Rect, Response, Role, Sense, Ui, Vec2, WidgetInfo};

use crate::{
    Canvas, CanvasError, CanvasEvent, CanvasResponse, DefaultRenderer, DetailLevel, ElementId,
    Graph, RenderContext, Renderer, Selection,
    geometry::Camera,
    render::{RenderedGraph, RendererKey},
};

impl Canvas {
    pub fn show(&mut self, ui: &mut Ui, graph: &(impl Graph + ?Sized)) -> Response {
        self.show_with(ui, graph, &DefaultRenderer).response
    }

    pub fn try_show(
        &mut self,
        ui: &mut Ui,
        graph: &(impl Graph + ?Sized),
    ) -> Result<CanvasResponse, CanvasError> {
        self.try_show_with(ui, graph, &DefaultRenderer)
    }

    /// Вариант с собственным представлением; ошибка отображается в UI и сохраняется в результате.
    pub fn show_with(
        &mut self,
        ui: &mut Ui,
        graph: &(impl Graph + ?Sized),
        renderer: &(impl Renderer + ?Sized),
    ) -> CanvasResponse {
        match self.try_show_with(ui, graph, renderer) {
            Ok(response) => response,
            Err(error) => {
                let response =
                    ui.colored_label(Color32::LIGHT_RED, format!("Cannot show graph: {error}"));
                CanvasResponse {
                    viewport: response.rect,
                    response,
                    camera: self.camera,
                    hovered: None,
                    context_target: None,
                    context_position: None,
                    detail: self.detail,
                    selection_changed: false,
                    camera_changed: false,
                    error: Some(error),
                }
            }
        }
    }

    /// Ошибка подготовки возвращается приложению без паники и без частичной отрисовки графа.
    pub fn try_show_with(
        &mut self,
        ui: &mut Ui,
        graph: &(impl Graph + ?Sized),
        renderer: &(impl Renderer + ?Sized),
    ) -> Result<CanvasResponse, CanvasError> {
        self.events.clear();
        if self.options.labels.is_some_and(|options| !options.valid()) {
            return Err(CanvasError::InvalidOptions);
        }
        let options = self.options.camera;
        if !self.camera.center.is_finite()
            || !self.camera.zoom.is_finite()
            || self.camera.zoom <= 0.0
        {
            return Err(CanvasError::InvalidCamera);
        }
        if !options.fit_margin.is_finite()
            || options.fit_margin < 0.0
            || !options.focus_margin.is_finite()
            || options.focus_margin < 0.0
            || !options.focus_scale.is_finite()
            || options.focus_scale <= 0.0
            || !options.wheel_sensitivity.is_finite()
            || options.wheel_sensitivity < 0.0
        {
            return Err(CanvasError::InvalidOptions);
        }
        let bounds = graph.bounds();
        if !crate::geometry::valid_rect(bounds) {
            return Err(CanvasError::InvalidGraphBounds);
        }
        self.use_renderer(renderer);
        let graph = RenderedGraph { graph, renderer };
        self.show_graph(ui, &graph, renderer, bounds)
    }

    pub(super) fn use_renderer<R: Renderer + ?Sized>(&mut self, renderer: &R) {
        let key = RendererKey::new(renderer);
        if self.renderer_key != Some(key) {
            self.spatial.invalidate();
            self.hit_cache = None;
            self.renderer_key = Some(key);
        }
    }

    pub(super) fn fit_graph(&mut self, bounds: Rect, viewport: Rect) {
        let bounds = bounds.union(self.spatial.bounds());
        self.camera
            .fit(bounds, viewport.shrink(self.options.camera.fit_margin));
        self.camera.zoom = self.options.camera.zoom.clamp(self.camera.zoom);
    }

    fn show_graph(
        &mut self,
        ui: &mut Ui,
        graph: &(impl Graph + ?Sized),
        renderer: &(impl Renderer + ?Sized),
        bounds: Rect,
    ) -> Result<CanvasResponse, CanvasError> {
        self.focus_requested = false;
        let previous_selection = self.notified_selection;
        let previous_camera = self.camera;
        self.synchronize(graph);
        self.spatial.synchronize(graph)?;
        let scene_bounds = bounds.union(self.spatial.bounds());
        if !scene_bounds.size().is_finite() {
            return Err(CanvasError::InvalidGraphBounds);
        }
        let (viewport, mut response) =
            ui.allocate_exact_size(ui.available_size(), Sense::click_and_drag());
        response.widget_info(|| {
            WidgetInfo::labeled(Role::Unknown, true, &self.style.accessibility_label)
        });
        if self.camera.fit || !self.initial_fit_complete {
            self.fit_graph(bounds, viewport);
            if !self.initial_fit_complete {
                self.initial_fit_complete = self.viewport_size == viewport.size();
                if !self.initial_fit_complete {
                    ui.ctx().request_repaint();
                }
            }
        }
        let viewport_changed = self.viewport_size != viewport.size();
        self.viewport_size = viewport.size();
        if let Some(selection) = self.pending_focus.take() {
            response.request_focus();
            let (bounds, screen_offset) = match selection {
                Selection::Node(index) => {
                    let geometry = graph.node_geometry(index);
                    (
                        geometry.world_bounds(graph.node(index).position),
                        geometry.focus_offset(),
                    )
                }
                Selection::Edge(index) => (graph.edge(index).path.bounds(), Vec2::ZERO),
                Selection::Group(index) => (graph.group(index).frame.bounds(), Vec2::ZERO),
            };
            let fit_zoom = Camera::fit_zoom(
                scene_bounds,
                viewport.shrink(self.options.camera.fit_margin),
            );
            self.camera
                .fit(bounds, viewport.shrink(self.options.camera.focus_margin));
            if bounds.size() == Vec2::ZERO {
                self.camera.zoom = fit_zoom * self.options.camera.focus_scale;
            }
            self.camera.zoom = self.options.camera.zoom.clamp(
                self.camera
                    .zoom
                    .min(fit_zoom * self.options.camera.focus_scale),
            );
            self.camera.center += screen_offset / self.camera.zoom;
        }
        let navigator = if self.options.navigator.enabled
            && (graph.node_count() > 0 || graph.edge_count() > 0 || graph.group_count() > 0)
        {
            crate::navigator::Navigator::new(viewport, scene_bounds, self.options.navigator.corner)
        } else {
            None
        };
        let navigator_input = navigator.as_ref().is_some_and(|nav| nav.blocks_pointer(ui));
        if let Some(target) = navigator
            .as_ref()
            .and_then(|nav| nav.interact(ui, self.options.interaction.pan))
        {
            self.camera.center = target;
            self.camera.fit = false;
            self.initial_fit_complete = true;
            ui.ctx().request_repaint();
        }
        if !navigator_input || self.marquee.is_some() {
            self.update_marquee(ui, graph, viewport, &response);
        }
        let alternate_pan = self.options.interaction.pan_button != egui::PointerButton::Primary
            && ui.rect_contains_pointer(viewport)
            && ui.input(|input| {
                input
                    .pointer
                    .button_down(self.options.interaction.pan_button)
                    && input
                        .pointer
                        .press_origin()
                        .is_some_and(|p| viewport.contains(p))
            });
        let panning = !navigator_input
            && self.options.interaction.pan
            && (response.dragged_by(self.options.interaction.pan_button) || alternate_pan)
            && self.group_drag.is_none()
            && self.drag.is_none()
            && self.marquee.is_none()
            && self
                .options
                .interaction
                .pan_modifier
                .matches(ui.input(|input| input.modifiers));
        if panning {
            self.initial_fit_complete = true;
            let delta = if alternate_pan {
                ui.input(|input| input.pointer.delta())
            } else {
                response.drag_delta()
            };
            self.camera.center -= delta / self.camera.zoom;
        }
        let minimum_zoom = if self.options.camera.limit_zoom_out_to_fit {
            self.options.camera.zoom.clamp(Camera::fit_zoom(
                scene_bounds,
                viewport.shrink(self.options.camera.fit_margin),
            ))
        } else {
            self.options.camera.zoom.min()
        };
        // Правка геометрии не должна менять камеру и сдвигать узел под указателем.
        if viewport_changed {
            self.camera.zoom = self
                .camera
                .zoom
                .clamp(minimum_zoom, self.options.camera.zoom.max());
        }
        if !navigator_input {
            self.update_zoom(ui, viewport, minimum_zoom);
        }
        if !self.camera.center.is_finite()
            || !self.camera.to_world(viewport.min, viewport).is_finite()
            || !self.camera.to_world(viewport.max, viewport).is_finite()
            || !self
                .camera
                .to_screen(scene_bounds.min, viewport)
                .is_finite()
            || !self
                .camera
                .to_screen(scene_bounds.max, viewport)
                .is_finite()
        {
            return Err(CanvasError::InvalidCamera);
        }
        if self.crossings_enabled() {
            self.crossings.prepare(graph, &self.spatial);
        }
        if navigator_input {
            self.hovered = None;
            self.hit_cache = None;
        } else {
            self.update_hover(ui, graph, renderer, viewport);
        }
        if !navigator_input
            && self.options.interaction.double_click_fit
            && response.double_clicked()
            && self.hovered.is_none()
        {
            self.fit_graph(bounds, viewport);
        }
        if !navigator_input
            && (self.options.interaction.selection
                || self.options.interaction.activation
                || self.options.interaction.keyboard)
            && (response.clicked()
                || (ui.rect_contains_pointer(viewport)
                    && ui.input(|input| input.pointer.primary_clicked())))
            && let Some(pointer) = response
                .interact_pointer_pos()
                .or_else(|| ui.input(|input| input.pointer.interact_pos()))
        {
            response.request_focus();
            let selected = self
                .camera
                .to_world_precise(pointer, viewport)
                .and_then(|point| self.pick_source(point, graph, renderer, true));
            if !matches!(selected, Some(Selection::Node(_) | Selection::Group(_))) {
                self.interactive_select(selected, ui.input(|input| input.modifiers), graph);
                if self.options.interaction.activation {
                    self.events.push(CanvasEvent::Clicked {
                        target: selected.and_then(|s| graph.identity(s)),
                        position: self.camera.to_world(pointer, viewport),
                    });
                }
            }
            ui.ctx().request_repaint();
        }
        if !navigator_input
            && self.options.interaction.activation
            && response.double_clicked()
            && let Some(element) = self.hovered
            && let Some(pointer) = response.interact_pointer_pos()
        {
            self.events.push(CanvasEvent::Activated {
                element,
                position: self.camera.to_world(pointer, viewport),
            });
        }
        self.navigate(ui, graph, viewport, &response);
        self.spatial.visible(
            graph,
            self.camera
                .rect_to_world(viewport.expand(self.crossing_margin()), viewport),
            self.camera.zoom,
            self.show_loops,
        );
        self.detail = if self.options.lod.enabled
            && (self.camera.zoom < self.options.lod.overview_below_zoom
                || self.spatial.visible_nodes.len() + self.spatial.visible_edges.len()
                    > self.options.lod.overview_above_elements)
        {
            DetailLevel::Overview
        } else {
            DetailLevel::Full
        };
        if !navigator_input && self.options.interaction.context_menu && response.secondary_clicked()
        {
            self.request_context(
                self.hovered,
                response
                    .interact_pointer_pos()
                    .map(|p| self.camera.to_world(p, viewport)),
            );
        }
        let painter = ui.painter_at(viewport);
        painter.rect_filled(viewport, 4.0, self.style.background);
        renderer.paint_background(
            &RenderContext {
                painter: &painter,
                camera: self.camera,
                viewport,
                style: &self.style,
                detail: self.detail,
            },
            self.options.grid.as_ref(),
        );
        if let Some(context) = self.paint(ui, graph, renderer, viewport, navigator_input) {
            response = response.union(context);
        }
        if let Some(ElementId::Edge(id)) = self.hovered
            && let Some(Selection::Edge(index)) = self.hit_cache.and_then(|(_, hit)| hit)
        {
            let mut hover = response.clone();
            hover.id = response.id.with(("edge-tooltip", id));
            hover.on_hover_ui_at_pointer(|ui| graph.edge_tooltip(ui, index));
        }
        self.paint_marquee(ui, viewport);
        if self.focus_requested {
            response.request_focus();
        }
        self.update_node_drag(ui, graph, viewport);
        self.update_group_drag(ui, graph, viewport);
        if !navigator_input {
            self.update_cursor(ui, graph, viewport, panning);
        }
        self.paint_alignment(ui, viewport);
        if let Some(navigator) = &navigator {
            self.paint_navigator(ui, graph, viewport, navigator);
        }

        let selection_changed = self.selection.epoch != previous_selection;
        if selection_changed {
            self.events
                .push(CanvasEvent::SelectionChanged(self.selected_id()));
        }
        self.notified_selection = self.selection.epoch;
        let camera_changed = self.camera != previous_camera;
        if selection_changed || camera_changed {
            response.mark_changed();
        }
        Ok(CanvasResponse {
            response,
            viewport,
            camera: self.camera,
            hovered: self.hovered,
            context_target: self.context_target,
            context_position: self.context_position,
            detail: self.detail,
            selection_changed,
            camera_changed,
            error: None,
        })
    }
}

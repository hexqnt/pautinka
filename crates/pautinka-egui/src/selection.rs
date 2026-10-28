use rustc_hash::FxHashSet;

use egui::{Modifiers, Rect};

use crate::{Canvas, ElementId, Graph, Selection};

pub(super) struct Primary {
    pub index: Selection,
    pub id: ElementId,
}

#[derive(Default)]
pub(super) struct SelectionState {
    pub primary: Option<Primary>,
    pub members: FxHashSet<ElementId>,
    pub epoch: u64,
}

impl SelectionState {
    fn changed(&mut self) {
        self.epoch = self
            .epoch
            .checked_add(1)
            .expect("Selection revision exhausted");
    }
    pub fn clear(&mut self) {
        if !self.members.is_empty() {
            self.members.clear();
            self.changed();
        }
        self.primary = None;
    }
}

impl Canvas {
    pub fn select_id(&mut self, id: Option<ElementId>, graph: &(impl Graph + ?Sized)) -> bool {
        match id {
            Some(id) => graph
                .resolve(id)
                .is_some_and(|index| self.select(Some(index), graph)),
            None => self.select(None, graph),
        }
    }

    pub fn focus_id(&mut self, id: ElementId, graph: &(impl Graph + ?Sized)) -> bool {
        graph
            .resolve(id)
            .is_some_and(|index| self.focus(index, graph))
    }

    pub fn select_all(&mut self, graph: &(impl Graph + ?Sized)) {
        self.selection.members.clear();
        self.selection.members.extend(
            (0..graph.node_count())
                .filter(|&index| graph.node_interaction(index).selectable)
                .map(|index| ElementId::Node(graph.node(index).id)),
        );
        self.selection.members.extend(
            (0..graph.edge_count())
                .filter(|&index| graph.edge_selectable(index))
                .map(|index| graph.edge(index))
                .filter(|edge| self.show_loops || !edge.is_loop)
                .map(|edge| ElementId::Edge(edge.id)),
        );
        self.selection.members.extend(
            (0..graph.group_count())
                .filter(|&index| graph.group_interaction(index).selectable)
                .map(|index| ElementId::Group(graph.group(index).id)),
        );
        self.selection.changed();
        self.selection.primary = None;
        self.pending_focus = None;
        self.refresh_primary(graph);
    }

    /// Рамка выбирает целиком заключённые в неё формы и пути; Shift начинает жест на фоне.
    pub fn select_rect(&mut self, rect: Rect, additive: bool, graph: &(impl Graph + ?Sized)) {
        if !additive {
            self.selection.members.clear();
            self.selection.primary = None;
        }
        for index in 0..graph.node_count() {
            if !graph.node_interaction(index).selectable {
                continue;
            }
            let node = graph.node(index);
            let geometry = graph.node_geometry(index);
            let screen_bounds = geometry.screen_rect(node.position, self.camera, Rect::ZERO);
            let bounds = self.camera.rect_to_world(screen_bounds, Rect::ZERO);
            if rect.contains_rect(bounds) {
                self.selection.members.insert(ElementId::Node(node.id));
            }
        }
        for index in 0..graph.edge_count() {
            if !graph.edge_selectable(index) {
                continue;
            }
            let edge = graph.edge(index);
            if (self.show_loops || !edge.is_loop) && rect.contains_rect(edge.path.bounds()) {
                self.selection.members.insert(ElementId::Edge(edge.id));
            }
        }
        for index in 0..graph.group_count() {
            let group = graph.group(index);
            if graph.group_interaction(index).selectable && rect.contains_rect(group.frame.bounds())
            {
                self.selection.members.insert(ElementId::Group(group.id));
            }
        }
        self.pending_focus = None;
        self.selection.changed();
        self.refresh_primary(graph);
    }
    pub fn selected(&self) -> Option<Selection> {
        self.selection.primary.as_ref().map(|primary| primary.index)
    }
    pub fn selected_id(&self) -> Option<ElementId> {
        self.selection.primary.as_ref().map(|primary| primary.id)
    }
    pub fn is_selected(&self, id: ElementId) -> bool {
        self.selection.members.contains(&id)
    }

    /// Заимствует набор выбранных ID без гарантии порядка и без промежуточных коллекций.
    pub fn selected_elements(&self) -> impl Iterator<Item = ElementId> + '_ {
        self.selection.members.iter().copied()
    }
    pub fn selection_revision(&self) -> u64 {
        self.selection.epoch
    }

    /// Меняет основной элемент внутри набора, сохраняя остальные выбранные объекты.
    pub fn set_primary(&mut self, id: ElementId, graph: &(impl Graph + ?Sized)) -> bool {
        if !self.is_selected(id) {
            return false;
        }
        let Some(index) = graph.resolve(id) else {
            return false;
        };
        if self.selected_id() != Some(id) {
            self.selection.changed();
        }
        self.selection.primary = Some(Primary { index, id });
        true
    }

    /// Выбирает элемент и вписывает его в область при следующем показе.
    pub fn focus(&mut self, selection: Selection, graph: &(impl Graph + ?Sized)) -> bool {
        if !self.select(Some(selection), graph) {
            return false;
        }
        if let Selection::Edge(index) = selection
            && graph.edge(index).is_loop
        {
            self.show_loops = true;
        }
        self.pending_focus = Some(selection);
        self.initial_fit_complete = true;
        true
    }

    /// Меняет выбор без изменения камеры; отсутствующий элемент не меняет состояние.
    pub fn select(&mut self, selection: Option<Selection>, graph: &(impl Graph + ?Sized)) -> bool {
        let primary = match selection {
            Some(index) => match graph.identity(index) {
                Some(id) => Some(Primary { index, id }),
                None => return false,
            },
            None => None,
        };
        if self.selection.members.len() != usize::from(primary.is_some())
            || primary
                .as_ref()
                .is_some_and(|primary| !self.is_selected(primary.id))
        {
            self.selection.members.clear();
            if let Some(primary) = &primary {
                self.selection.members.insert(primary.id);
            }
            self.selection.changed();
        }
        self.selection.primary = primary;
        self.pending_focus = None;
        true
    }

    /// Полностью заменяет набор. Некорректный ID оставляет предыдущий выбор целиком.
    pub fn select_elements(
        &mut self,
        elements: impl IntoIterator<Item = ElementId>,
        graph: &(impl Graph + ?Sized),
    ) -> bool {
        let members: FxHashSet<_> = elements.into_iter().collect();
        let found = element_ids(graph).filter(|id| members.contains(id)).count();
        if found != members.len() {
            return false;
        }
        if members != self.selection.members {
            self.selection.members = members;
            self.selection.changed();
        }
        self.selection.primary = None;
        self.pending_focus = None;
        self.refresh_primary(graph);
        true
    }

    pub(super) fn interactive_select(
        &mut self,
        selection: Option<Selection>,
        modifiers: Modifiers,
        graph: &(impl Graph + ?Sized),
    ) {
        if !self.options.interaction.selection {
            return;
        }
        if selection.is_some_and(|index| match index {
            Selection::Node(index) => !graph.node_interaction(index).selectable,
            Selection::Edge(index) => !graph.edge_selectable(index),
            Selection::Group(index) => !graph.group_interaction(index).selectable,
        }) {
            return;
        }
        let previous = self.selected_id();
        let epoch = self.selection.epoch;
        let Some(index) = selection else {
            if !self.options.interaction.add_modifier.matches(modifiers)
                && !self.options.interaction.toggle_modifier.matches(modifiers)
            {
                self.select(None, graph);
            }
            return;
        };
        let Some(id) = graph.identity(index) else {
            return;
        };
        if self.options.interaction.toggle_modifier.matches(modifiers) {
            if !self.selection.members.remove(&id) {
                self.selection.members.insert(id);
                self.selection.primary = Some(Primary { index, id });
            }
            self.selection.changed();
            self.refresh_primary(graph);
        } else if self.options.interaction.add_modifier.matches(modifiers) {
            if self.selection.members.insert(id) {
                self.selection.changed();
            }
            self.selection.primary = Some(Primary { index, id });
        } else {
            self.select(Some(index), graph);
        }
        self.pending_focus = None;
        if self.selected_id() != previous && self.selection.epoch == epoch {
            self.selection.changed();
        }
    }

    pub(super) fn refresh_primary(&mut self, graph: &(impl Graph + ?Sized)) {
        let id = self
            .selected_id()
            .filter(|id| self.is_selected(*id))
            .or_else(|| self.selection.members.iter().copied().min());
        self.selection.primary =
            id.and_then(|id| graph.resolve(id).map(|index| Primary { index, id }));
    }

    /// Восстанавливает плотные индексы по ID; изменение геометрии сохраняет набор.
    pub fn synchronize(&mut self, graph: &(impl Graph + ?Sized)) {
        let revision = graph.revision();
        let topology_changed =
            self.revision.map(crate::GraphRevision::topology) != Some(revision.topology());
        if topology_changed && !self.selection.members.is_empty() {
            let count = self.selection.members.len();
            // Перестройка топологии редка; один проход заменяет O(K*N) вызовов default resolve.
            let alive = element_ids(graph)
                .filter(|id| self.selection.members.contains(id))
                .collect();
            self.selection.members = alive;
            if self.selection.members.len() != count {
                self.selection.changed();
            }
        }
        if topology_changed
            || self
                .selected()
                .and_then(|selection| graph.identity(selection))
                != self.selected_id()
        {
            self.refresh_primary(graph);
            self.pending_focus = self.pending_focus.and_then(|_| self.selected());
        }
        self.revision = Some(revision);
    }

    pub fn set_show_loops(&mut self, show: bool, graph: &(impl Graph + ?Sized)) {
        self.show_loops = show;
        if !show {
            let count = self.selection.members.len();
            self.selection
                .members
                .retain(|&id| match graph.resolve(id) {
                    Some(Selection::Edge(index)) => !graph.edge(index).is_loop,
                    Some(_) => true,
                    None => false,
                });
            if self.selection.members.len() != count {
                self.selection.changed();
                self.pending_focus = None;
            }
            self.refresh_primary(graph);
        }
    }
}

fn element_ids(graph: &(impl Graph + ?Sized)) -> impl Iterator<Item = ElementId> {
    (0..graph.node_count())
        .map(|index| ElementId::Node(graph.node(index).id))
        .chain((0..graph.edge_count()).map(|index| ElementId::Edge(graph.edge(index).id)))
        .chain((0..graph.group_count()).map(|index| ElementId::Group(graph.group(index).id)))
}

#[cfg(test)]
mod tests;

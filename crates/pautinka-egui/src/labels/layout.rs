use egui::{Rect, Vec2};
use rustc_hash::FxHashMap;

use crate::NodeId;

use super::{
    LabelLayoutOptions, LabelPosition, NodeLabelOptions, PlacedNodeLabel,
    grid::{Grid, area},
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Request {
    pub id: NodeId,
    pub index: usize,
    pub node: Rect,
    pub size: Vec2,
    pub options: NodeLabelOptions,
    pub forced: bool,
    pub importance: u8,
}

#[derive(Clone, Copy)]
struct Previous {
    position: LabelPosition,
    epoch: u64,
}

#[derive(Clone, Copy)]
struct Candidate {
    position: LabelPosition,
    rect: Rect,
    collision: f64,
    cost: f64,
}

#[derive(Default)]
pub(super) struct Layout {
    pub requests: Vec<Request>,
    pub nodes: Vec<Rect>,
    pub placed: Vec<PlacedNodeLabel>,
    previous_requests: Vec<Request>,
    previous_nodes: Vec<Rect>,
    previous_options: Option<LabelLayoutOptions>,
    viewport: Option<Rect>,
    order: Vec<usize>,
    history: FxHashMap<NodeId, Previous>,
    grid: Grid,
    epoch: u64,
    pub solves: u64,
}

impl Layout {
    pub fn solve(&mut self, viewport: Rect, options: LabelLayoutOptions) {
        if self.previous_options == Some(options)
            && self.viewport == Some(viewport)
            && self.previous_requests == self.requests
            && self.previous_nodes == self.nodes
        {
            return;
        }
        if self.reuse_pan(viewport, options) {
            self.remember_inputs(viewport, options);
            return;
        }
        self.solves += 1;
        self.epoch = self.epoch.wrapping_add(1);
        self.placed.clear();
        if self.requests.is_empty() {
            self.history.clear();
            self.remember_inputs(viewport, options);
            return;
        }
        if options.weights.nodes > 0.0 || options.weights.labels > 0.0 {
            self.grid
                .clear(viewport.expand(options.label_padding * 0.5));
        }
        if options.weights.nodes > 0.0 {
            for &node in &self.nodes {
                self.grid
                    .insert(node.expand(options.node_padding), options.weights.nodes);
            }
        }
        self.order.clear();
        self.order.extend(0..self.requests.len());
        self.order.sort_unstable_by(|&a, &b| {
            let a = self.requests[a];
            let b = self.requests[b];
            b.forced
                .cmp(&a.forced)
                .then_with(|| b.importance.cmp(&a.importance))
                .then_with(|| b.options.priority.cmp(&a.options.priority))
                .then_with(|| a.id.cmp(&b.id))
        });
        for &source in &self.order {
            let request = self.requests[source];
            // Обязательные подписи отсортированы первыми; дальше бюджет уже не может освободиться.
            if !request.forced && self.placed.len() >= options.max_labels {
                break;
            }
            let previous = self
                .history
                .get(&request.id)
                .map(|previous| previous.position);
            let threshold = f64::from(if previous.is_some() {
                options.hide_above
            } else {
                options.reveal_below
            });
            let Some(candidate) = Self::choose(
                &mut self.grid,
                request,
                previous,
                options,
                viewport,
                threshold,
            ) else {
                continue;
            };
            if !request.forced && candidate.collision > threshold {
                continue;
            }
            self.history.insert(
                request.id,
                Previous {
                    position: candidate.position,
                    epoch: self.epoch,
                },
            );
            if options.weights.labels > 0.0 {
                self.grid.insert(
                    candidate.rect.expand(options.label_padding * 0.5),
                    options.weights.labels,
                );
            }
            self.placed.push(PlacedNodeLabel {
                id: request.id,
                index: request.index,
                rect: candidate.rect,
                position: candidate.position,
                forced: request.forced,
                conflicted: candidate.collision > f64::from(options.hide_above),
                source,
            });
        }
        self.history
            .retain(|_, previous| previous.epoch == self.epoch);
        self.remember_inputs(viewport, options);
    }

    fn remember_inputs(&mut self, viewport: Rect, options: LabelLayoutOptions) {
        self.previous_requests.clone_from(&self.requests);
        self.previous_nodes.clone_from(&self.nodes);
        self.previous_options = Some(options);
        self.viewport = Some(viewport);
    }

    fn choose(
        grid: &mut Grid,
        request: Request,
        previous: Option<LabelPosition>,
        options: LabelLayoutOptions,
        viewport: Rect,
        threshold: f64,
    ) -> Option<Candidate> {
        let mut best: Option<Candidate> = None;
        let mut retained: Option<Candidate> = None;
        let rank = |candidate: Candidate| {
            (
                !request.forced && candidate.collision > threshold,
                candidate.cost,
            )
        };
        for position in request.options.placement.positions().iter() {
            let rect = position.rect(request.node, request.size, options.gap);
            if !rect.is_finite() || !rect.is_positive() {
                continue;
            }
            let padded = rect.expand(options.label_padding * 0.5);
            if !padded.is_finite() {
                continue;
            }
            let outside = if options.weights.viewport > 0.0 {
                (1.0 - area(rect.intersect(viewport)) / area(rect)).clamp(0.0, 1.0)
                    * f64::from(options.weights.viewport)
            } else {
                0.0
            };
            let collision = outside
                + if options.weights.nodes > 0.0 || options.weights.labels > 0.0 {
                    grid.overlap(padded)
                } else {
                    0.0
                };
            let candidate = Candidate {
                position,
                rect,
                collision,
                cost: collision
                    + position.distance(request.options.placement.preferred())
                        * f64::from(options.weights.preference),
            };
            if best.is_none_or(|best| rank(candidate) < rank(best)) {
                best = Some(candidate);
            }
            if previous == Some(position) {
                retained = Some(candidate);
            }
        }
        let best = best?;
        // Плохая старая позиция не должна скрывать подпись, когда существует допустимая новая.
        Some(
            retained
                .filter(|old| {
                    old.collision <= f64::from(options.hide_above)
                        && old.cost <= best.cost + f64::from(options.switch_margin)
                })
                .unwrap_or(best),
        )
    }

    /// Чистый перенос сохраняет взаимные пересечения. У границ экрана решение пересчитывается.
    fn reuse_pan(&mut self, viewport: Rect, options: LabelLayoutOptions) -> bool {
        if self.previous_options != Some(options)
            || self.viewport != Some(viewport)
            || self.requests.len() != self.previous_requests.len()
            || self.nodes.len() != self.previous_nodes.len()
        {
            return false;
        }
        let Some((first, old_first)) = self.requests.first().zip(self.previous_requests.first())
        else {
            return false;
        };
        let delta = first.node.min - old_first.node.min;
        if !self
            .nodes
            .iter()
            .zip(&self.previous_nodes)
            .all(|(&node, old)| node == old.translate(delta))
        {
            return false;
        }
        for (&request, old) in self.requests.iter().zip(&self.previous_requests) {
            if request
                != (Request {
                    node: old.node.translate(delta),
                    ..*old
                })
            {
                return false;
            }
            for position in request.options.placement.positions().iter() {
                if !viewport.contains_rect(position.rect(old.node, old.size, options.gap))
                    || !viewport.contains_rect(position.rect(
                        request.node,
                        request.size,
                        options.gap,
                    ))
                {
                    return false;
                }
            }
        }
        for placed in &mut self.placed {
            placed.rect = placed.rect.translate(delta);
        }
        true
    }
}

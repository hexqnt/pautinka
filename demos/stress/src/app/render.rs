use pautinka::egui::{
    DefaultRenderer, Edge, EdgeStyle, ElementState, LinePattern, Renderer, Style,
};

use crate::scenario::Pattern;

impl Renderer for Pattern {
    fn edge_style(
        &self,
        index: usize,
        edge: Edge<'_>,
        state: ElementState,
        style: &Style,
    ) -> EdgeStyle {
        EdgeStyle {
            pattern: match self {
                Self::Solid => LinePattern::Solid,
                Self::Dashed => LinePattern::Dashed {
                    dash: 8.0,
                    gap: 5.0,
                },
                Self::Dotted => LinePattern::Dotted { gap: 5.0 },
            },
            ..DefaultRenderer.edge_style(index, edge, state, style)
        }
    }
}

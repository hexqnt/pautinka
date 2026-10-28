//! Общий API Pautinka: геометрия доступна всегда, egui-холст включается опцией `egui`.
//!
//! ```
//! use pautinka::{EdgePath, SpatialIndex, pos2};
//!
//! let path = EdgePath::try_line(pos2(0.0, 0.0), pos2(100.0, 50.0))?;
//! let mut index = SpatialIndex::default();
//! index.insert(7_u64, path.bounds())?;
//! let mut found = false;
//! index.query(path.bounds(), |id| found |= id == 7);
//! assert!(found);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

pub use pautinka_core::*;

/// Интерактивный холст egui; доступен с опцией `egui`.
///
/// ```
/// use pautinka::{EdgePath, NodeId, egui::Canvas, pos2};
///
/// let canvas = Canvas::default();
/// assert!(canvas.selected_id().is_none());
/// let _node = NodeId::new(1);
/// let _path = EdgePath::try_line(pos2(0.0, 0.0), pos2(100.0, 50.0))?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[cfg(feature = "egui")]
pub use pautinka_egui as egui;

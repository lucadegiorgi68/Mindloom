//! Mindloom core: document model, organic geometry, layout, history.
//! No UI code lives here, so everything is unit testable.
pub mod fonts;
pub mod geom;
pub mod history;
pub mod layout;
pub mod model;
pub mod organic;
pub mod scene;

pub use geom::{Pt, Rect};
pub use history::Document;
pub use model::{BranchStyle, MindMap, Node, NodeId, Shape, Side};
pub use scene::{build_scene, Scene};

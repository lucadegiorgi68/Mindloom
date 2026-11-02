//! Mindloom core: document model, organic geometry, layout, history.
//! No UI code lives here, so everything is unit testable.
pub mod fonts;
pub mod geom;
pub mod history;
pub mod layout;
pub mod links;
pub mod model;
pub mod organic;
pub mod scene;

pub use geom::{Pt, Rect};
pub use history::Document;
pub use model::{
    BranchStyle, FreeImage, ImageAsset, Link, LinkArrow, LinkStyle, MindMap, Node, NodeId,
    NodeImage, Shape, Side,
};
pub use scene::{build_scene, Scene};

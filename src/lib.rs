pub mod document;

mod arena;
mod owned;

pub mod layout;
pub mod lines;
pub mod render;

pub use crate::{
    arena::{Doc, DocBuilder},
    document::GroupPolicy,
    layout::{LayoutEngine, LayoutMode, LayoutSettings, WidthConstraint},
    owned::OwnedDoc,
    render::{RenderAdaptor, RenderAdaptorExt, Renderer},
};
pub use typed_arena::{self, Arena};

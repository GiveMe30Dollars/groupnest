pub mod document;

mod builder;
mod owned;

pub mod layout;
pub mod lines;
pub mod render;

pub use crate::{
    builder::{Doc, DocBuilder},
    document::GroupPolicy,
    layout::{LayoutEngine, LayoutMode, LayoutSettings, WidthConstraint},
    owned::OwnedDoc,
    render::{RenderAdaptorExt, Renderer},
};
pub use typed_arena::{self, Arena};

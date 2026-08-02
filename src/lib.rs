pub mod document;

mod builder;
mod owned;

pub mod layout;
pub mod lines;
pub mod renderer;

pub use crate::{
    builder::{Doc, DocBuilder},
    document::GroupPolicy,
    layout::{LayoutEngine, LayoutMode, LayoutSettings, LayoutWidthConstraint},
    owned::OwnedDoc,
    renderer::{
        RenderAdaptorExt, Renderer, RenderError, 
        PlaintextRenderer
    },
};
pub use typed_arena::{self, Arena};

#[cfg(feature = "termcolor")]
pub use termcolor;
#[cfg(feature = "termcolor")]
pub use crate::renderer::{
    ColorPatch, TermcolorRenderer
};
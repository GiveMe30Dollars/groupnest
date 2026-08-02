//! [`Document`]: crate::document::Document
#![doc = include_str!("../README.md")]

pub mod document;

mod builder;
mod owned;

pub mod layout;
pub mod lines;
pub mod renderer;

pub use crate::{
    builder::{Doc, DocBuilder},
    document::GroupPolicy,
    owned::OwnedDoc,
    renderer::{PlaintextRenderer, RenderAdaptorExt, Renderer},
};
pub use typed_arena::{self, Arena};

#[cfg(feature = "termcolor")]
pub use crate::renderer::{ColorPatch, TermcolorRenderer};
#[cfg(feature = "termcolor")]
pub use termcolor;

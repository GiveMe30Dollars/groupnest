//! [`Document`]: crate::document::Document
//! [`Renderer`]: crate::renderer::Renderer
//! [`LayoutEngine`]: crate::layout::LayoutEngine
//! [`Document::as_layout`]: crate::document::Document::as_layout
//! [`Document::as_layout_with`]: crate::document::Document::as_layout_with
//! [`Document::to_plaintext`]: crate::document::Document::to_plaintext
//! [`Document::to_plaintext_with`]: crate::document::Document::to_plaintext_with
//! [`ColorPatch`]: crate::renderer::ColorPatch
//! [`TermcolorRenderer`]: crate::renderer::TermcolorRenderer
#![doc = include_str!("../README.md")]
#![cfg_attr(docsrs, feature(doc_cfg))]

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

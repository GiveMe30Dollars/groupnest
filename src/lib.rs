//! [`Document`]: crate::document::Document
//! [`RefDoc`]: crate::RefDoc
//! [`RefDocBuilder`]: crate::RefDocBuilder
//! [`BoxDoc`]: crate::BoxDoc
//! [`Arc`]: std::sync::Arc
//! [`ArcDoc`]: crate::ArcDoc
//! [`ArcDocBuilder`]: crate::ArcDocBuilder
//! [`Renderer`]: crate::renderer::Renderer
//! [`LayoutEngine`]: crate::layout::LayoutEngine
//! [`Document::as_layout`]: crate::document::Document::as_layout
//! [`Document::as_layout_with`]: crate::document::Document::as_layout_with
//! [`Document::to_plaintext`]: crate::document::Document::to_plaintext
//! [`Document::to_plaintext_with`]: crate::document::Document::to_plaintext_with
//! [`ColorPatch`]: crate::renderer::ColorPatch
//! [`TermcolorRenderer`]: crate::renderer::TermcolorRenderer
#![doc = include_str!("README.rustdoc.md")]
#![cfg_attr(docsrs, feature(doc_cfg))]

pub mod document;

mod handle;

pub mod layout;
pub mod lines;
pub mod renderer;

pub use crate::{
    document::GroupPolicy,
    handle::{
        boxdoc::{BoxDoc},
        refdoc::{RefDoc, RefDocBuilder},
        arcdoc::{ArcDoc, ArcDocBuilder},
    },
    renderer::{PlaintextRenderer, RenderAdaptorExt, Renderer},
};
pub use typed_arena::{self, Arena};

#[cfg(feature = "termcolor")]
pub use crate::renderer::{ColorPatch, TermcolorRenderer};
#[cfg(feature = "termcolor")]
pub use termcolor;

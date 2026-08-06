//! Definition of the types wrapping [`Document`](crate::document::Document).
//!
//! The contents of this module are `pub use` in the crate root.

mod common;

pub(crate) mod arcdoc;
pub(crate) mod boxdoc;
pub(crate) mod refdoc;

pub use {arcdoc::*, boxdoc::*, refdoc::*};

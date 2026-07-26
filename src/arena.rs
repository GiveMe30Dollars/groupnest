use std::{borrow::Cow, ops::Deref};

use crate::{Doc, Document, document::TextFragment};

/// The interning key for leaf nodes of a `Doc`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum LeafKey {

}

pub struct Arena<'s, 'doc, A> {
    inner: typed_arena::Arena<Doc<'s, 'doc, A>>,
}
impl<'s, 'doc, A> Arena<'s, 'doc, A> {
    fn nil(&'doc self) -> &'doc Doc<'s, 'doc, A> {
        self.inner.alloc(Document::Nil.into())
    }
    // fn text(&'doc self, payload: impl Into<Cow<'s, str>>) -> &'doc Doc<'s, 'doc, A> {
    //     let fragments = TextFragment::text(payload);
    // }
}
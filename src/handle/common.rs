use std::ops::Deref;

use crate::document::{Break, Document, FlatFragment};

/// The interning key for leaf nodes of a `Doc`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum LeafKey<'s> {
    Nil,
    Text(FlatFragment<'s>),
    Break(Break<'s>),
    HardLinebreak,
}
impl<'s, 'doc, A, D> TryFrom<&'doc Document<'s, D, A>> for LeafKey<'s>
where
    D: Deref<Target = Document<'s, D, A>>,
{
    type Error = ();
    fn try_from(value: &'doc Document<'s, D, A>) -> Result<Self, Self::Error> {
        match value {
            Document::Nil => Ok(LeafKey::Nil),
            Document::Text(inner) => Ok(LeafKey::Text(inner.clone())),
            Document::Break(inner) => Ok(LeafKey::Break(inner.clone())),
            Document::HardLinebreak => Ok(LeafKey::HardLinebreak),
            _ => Err(()),
        }
    }
}

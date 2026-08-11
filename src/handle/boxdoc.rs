use std::{
    borrow::Cow,
    ops::{Deref, DerefMut},
};

use derive_more::{AsMut, AsRef};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::{
    ArcDoc, ArcDocBuilder,
    document::{BreakNodeInvalid, ContainsTab, Document, FragmentError, GroupPolicy},
};

/// The notation document format, owning and heap-allocating all of its children and fragments via [`Box`].
///
/// This type is `Send + Sync` if the annotation type `A` is `Send + Sync`.
///
/// Refer to [`Document`](crate::document::Document) for more information on smart constructors.
#[repr(transparent)]
#[derive(Debug, Clone, PartialEq, Eq, Hash, AsRef, AsMut)]
#[as_ref(forward)]
#[as_mut(forward)]
pub struct BoxDoc<A = ()>(pub Box<Document<'static, Self, A>>);
impl<A> Deref for BoxDoc<A> {
    type Target = Document<'static, Self, A>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<A> DerefMut for BoxDoc<A> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
impl<A> From<Document<'static, BoxDoc<A>, A>> for BoxDoc<A> {
    fn from(value: Document<'static, BoxDoc<A>, A>) -> Self {
        BoxDoc(Box::new(value))
    }
}
impl<A> From<BoxDoc<A>> for Document<'static, BoxDoc<A>, A> {
    fn from(val: BoxDoc<A>) -> Self {
        *val.0
    }
}

#[cfg(feature = "serde")]
impl<A> Serialize for BoxDoc<A>
where
    A: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}
#[cfg(feature = "serde")]
impl<'de, A> Deserialize<'de> for BoxDoc<A>
where
    A: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(BoxDoc(Box::deserialize(deserializer)?))
    }
}

impl<A> BoxDoc<A> {
    /// The 'smart' constructor for nil nodes.
    pub fn nil() -> Self {
        Document::nil(Into::into)
    }
    /// The smart constructor for literal text.
    ///
    /// # Panics
    ///
    /// Panics if the payload contains tabs.
    pub fn from_text<'s>(payload: impl Into<Cow<'s, str>>) -> Self {
        let text = payload.into().into_owned();
        Document::from_text(text, Into::into)
            .map_err(|err| panic!("{err}"))
            .unwrap()
    }
    /// The non-panicking smart constructor for literal text.
    pub fn from_text_<'s>(payload: impl Into<Cow<'s, str>>) -> Result<Self, ContainsTab> {
        let text = payload.into().into_owned();
        Document::from_text(text, Into::into)
    }
    /// The smart constructor for literal text.
    ///
    /// # Panics
    ///
    /// Panics if the payload contains tabs.
    pub fn flat_text<'s>(payload: impl Into<Cow<'s, str>>) -> Self {
        let text = payload.into().into_owned();
        Document::flat_text(text, Into::into)
            .map_err(|err| panic!("{err}"))
            .unwrap()
    }
    /// The non-panicking smart constructor for flat text fragments.
    pub fn flat_text_<'s>(payload: impl Into<Cow<'s, str>>) -> Result<Self, FragmentError> {
        let text = payload.into().into_owned();
        Document::flat_text(text, Into::into)
    }
    /// The smart constructor for break nodes.
    ///
    /// # Panics
    ///
    /// Panics if `flat` contains newline sequences, `broken` does not contain any, and either contain tabs.
    pub fn breaker<'s>(flat: impl Into<Cow<'s, str>>, broken: impl Into<Cow<'s, str>>) -> Self {
        let flat = flat.into().into_owned();
        let broken = broken.into().into_owned();
        Document::breaker(flat, broken, Into::into)
            .map_err(|err| panic!("{err}"))
            .unwrap()
    }
    /// The non-panicking smart constructor for break nodes.
    pub fn breaker_<'s>(
        flat: impl Into<Cow<'s, str>>,
        broken: impl Into<Cow<'s, str>>,
    ) -> Result<Self, BreakNodeInvalid> {
        let flat = flat.into().into_owned();
        let broken = broken.into().into_owned();
        Document::breaker(flat, broken, Into::into)
    }
    /// The `smart` constructor for hard linebreaks.
    pub fn hard_linebreak() -> Self {
        Document::hard_linebreak(Into::into)
    }
    /// The smart constructor for a group, using the default policy.
    pub fn group(child: Self) -> Self {
        Document::group(child, Into::into)
    }
    /// The smart constructor for a group with the specified policy.
    pub fn group_with(policy: GroupPolicy, child: Self) -> Self {
        Document::group_with(policy, child, Into::into)
    }
    /// The smart constructor for a grouped sequence, using the default policy.
    pub fn grouped_sequence(children: Vec<Self>) -> Self {
        Document::grouped_sequence(children, Into::into)
    }
    /// The smart constructor for a grouped sequence with the specified policy.
    pub fn grouped_sequence_with(policy: GroupPolicy, children: Vec<Self>) -> Self {
        Document::grouped_sequence_with(policy, children, Into::into)
    }
    /// The smart constructor for a collection sequence.
    pub fn sequence(children: Vec<Self>) -> Self {
        Document::sequence(children, Into::into)
    }
    /// The smart constructor for a collection sequence with interspersion.
    pub fn sequence_intersperse_with(children: Vec<Self>, separator: Self) -> Self
    where
        A: Clone,
    {
        Document::sequence_intersperse_with(children, separator, Into::into)
    }
    /// The smart constructor for nesting.
    pub fn nest(indentation: usize, inner: Self) -> Self {
        Document::nest(indentation, inner, Into::into)
    }
    /// The 'smart' constructor for annotations.
    pub fn annotation(annotation: A, inner: Self) -> Self {
        Document::annotation(annotation, inner, Into::into)
    }

    /// Converts self to an [`ArcDoc`] with deduplication, using a fresh [`ArcDocBuilder`].
    pub fn into_arc(self) -> ArcDoc<A> {
        self.into_arc_with(&ArcDocBuilder::default())
    }
    /// Converts self to an [`ArcDoc`] with deduplication via a given [`ArcDocBuilder`].
    pub fn into_arc_with(self, builder: &ArcDocBuilder<A>) -> ArcDoc<A> {
        match *self.0 {
            Document::Nil => builder.nil(),
            Document::Text(fragment) => builder.alloc(Document::Text(fragment)),
            Document::Break(breaker) => builder.alloc(Document::Break(breaker)),
            Document::HardLinebreak => builder.hard_linebreak(),
            Document::Group(policy, child)
                => builder.group_with(policy, child.into_arc_with(builder)),
            Document::Sequence(sequence) => {
                let children = sequence
                    .into_children()
                    .into_iter()
                    .map(|child| child.into_arc_with(builder))
                    .collect::<Vec<_>>();
                builder.sequence(children)
            }
            Document::Nest(indentation, inner) => {
                builder.nest(indentation, inner.into_arc_with(builder))
            }
            Document::Annotation(annotation, inner) => builder.alloc(Document::Annotation(
                annotation,
                inner.into_arc_with(builder),
            )),
        }
    }
}

/// Const assertion of `BoxDoc` `Send + Sync`.
const _: () = {
    #[allow(unused)]
    use std::rc::Rc;
    const fn assert_send<T: Send + Sync>() {}
    assert_send::<BoxDoc<()>>();
    assert_send::<BoxDoc<String>>();
};

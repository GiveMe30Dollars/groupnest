use std::{
    borrow::Cow,
    ops::{Deref, DerefMut},
};

use crate::document::{Document, GroupPolicy};

/// The notation document format, owning all of its children and fragments.
/// This type is `Send + Sync` if the annotation type `A` is `Send + Sync`,
/// but may suffer from memory fragmentation.
///
/// Refer to `Document` for more information on smart constructors.
#[repr(transparent)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OwnedDoc<A = ()>(pub Box<Document<'static, Self, A>>);
impl<A> Deref for OwnedDoc<A> {
    type Target = Document<'static, Self, A>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<A> DerefMut for OwnedDoc<A> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
impl<A> From<Document<'static, OwnedDoc<A>, A>> for OwnedDoc<A> {
    fn from(value: Document<'static, OwnedDoc<A>, A>) -> Self {
        OwnedDoc(Box::new(value))
    }
}
impl<A> From<OwnedDoc<A>> for Document<'static, OwnedDoc<A>, A> {
    fn from(val: OwnedDoc<A>) -> Self {
        *val.0
    }
}

impl<A> OwnedDoc<A> {
    /// The 'smart' constructor for nil nodes.
    pub fn nil() -> Self {
        Document::nil(Into::into)
    }
    /// The smart constructor for literal text.
    pub fn from_text(payload: impl Into<Cow<'static, str>>) -> Self {
        Document::from_text(payload, Into::into)
            .map_err(|err| panic!("{err}")).unwrap()
    }
    /// The smart constructor for literal text.
    /// 
    /// # Panics
    /// 
    /// Panics if the payload contains tabs.
    pub fn flat_text(payload: impl Into<Cow<'static, str>>) -> Self {
        Document::flat_text(payload, Into::into)
            .map_err(|err| panic!("{err}"))
            .unwrap()
    }
    /// The smart constructor for break nodes.
    /// 
    /// # Panics
    /// 
    /// Panics if `flat` contains newline sequences, `broken` does not contain any, and either contain tabs.
    pub fn breaker(
        flat: impl Into<Cow<'static, str>>,
        broken: impl Into<Cow<'static, str>>,
    ) -> Self {
        Document::breaker(flat, broken, Into::into)
            .map_err(|err| panic!("{err}"))
            .unwrap()
    }
    /// The `smart` constructor for hard linebreaks.
    pub fn hard_linebreak() -> Self {
        Document::hard_linebreak(Into::into)
    }
    /// The smart constructor for groups with a specified policy.
    pub fn group(child: Self, policy: GroupPolicy) -> Self {
        Document::group(child, policy, Into::into)
    }
    /// The smart constructor for a grouped sequence.
    pub fn grouped_sequence(children: Vec<Self>, policy: GroupPolicy) -> Self {
        Document::grouped_sequence(children, policy, Into::into)
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
}

/// Const assertion of `OwnedDoc` `Send + Sync`.
const _: () = {
    #[allow(unused)]
    use std::rc::Rc;
    const fn assert_send<T: Send + Sync>() {}
    assert_send::<OwnedDoc<()>>();
    assert_send::<OwnedDoc<String>>();
};

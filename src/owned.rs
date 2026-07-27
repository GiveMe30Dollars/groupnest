use std::{
    borrow::Cow,
    ops::{Deref, DerefMut},
};

use crate::document::{BreakNodeInvalid, ContainsLinebreak, Document, GroupPolicy};

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
        Document::nil().into()
    }
    /// The smart constructor for literal text.
    pub fn from_text<F>(payload: impl Into<Cow<'static, str>>) -> Self {
        Document::from_text(payload, Into::into).into()
    }
    /// The smart constructor for flat text fragments.
    pub fn flat_text(payload: impl Into<Cow<'static, str>>) -> Result<Self, ContainsLinebreak> {
        Document::flat_text(payload).map(Into::into)
    }
    /// The smart constructor for break nodes.
    pub fn breaker(
        flat: impl Into<Cow<'static, str>>,
        broken: impl Into<Cow<'static, str>>,
    ) -> Result<Self, BreakNodeInvalid> {
        Document::breaker(flat, broken).map(Into::into)
    }
    /// The `smart` constructor for hard linebreaks.
    pub fn hard_linebreak() -> Self {
        Document::hard_linebreak().into()
    }
    /// The smart constructor for groups using the default policy.
    pub fn group<F>(children: Vec<Self>) -> Self {
        Document::group(children).into()
    }
    /// The smart constructor for groups with a specified policy.
    pub fn group_with_policy<F>(children: Vec<Self>, policy: GroupPolicy) -> Self {
        Document::group_with_policy(children, policy).into()
    }
    /// The smart constructor for nesting.
    pub fn nest<F>(indentation: usize, inner: Self) -> Self {
        Document::nest(indentation, inner).into()
    }
    /// The 'smart' constructor for annotations.
    pub fn annotation<F>(annotation: A, inner: Self) -> Self {
        Document::annotation(annotation, inner).into()
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

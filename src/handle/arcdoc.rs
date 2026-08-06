use std::{borrow::Cow, collections::HashMap, ops::Deref, sync::{Arc, Mutex}};

use derive_more::{AsRef};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::{GroupPolicy, BoxDoc, handle::common::LeafKey, document::{BreakNodeInvalid, ContainsTab, Document, FragmentError}};

/// The notation document format, heap-allocated and reference-counted via [`Arc`].
///
/// This type is `Send + Sync` as long as annotation type `A` is.
///
/// ## Note on [`Document`](crate::document::Document) Smart Constructors
///
/// This type can construct itself, but it is not recommended due to duplication of leaf nodes and string fragments.
/// Prefer construction via [`ArcDocBuilder`].
/// 
/// ## Note on [`serde`] Support
/// 
/// Due to using [`Arc`], shared pointer equality is currently not preserved across serialization and deserialization.
/// 
/// Future support for shared deserialization would entail implementing [`serde::de::DeserializeSeed`] for [`ArcDocBuilder`].
#[repr(transparent)]
#[derive(Debug, PartialEq, Eq, Hash, AsRef)]
#[as_ref(forward)]
pub struct ArcDoc<A = ()>(pub Arc<Document<'static, Self, A>>);
impl<A> Deref for ArcDoc<A> {
    type Target = Document<'static, Self, A>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<A> Clone for ArcDoc<A> {
    fn clone(&self) -> Self {
        ArcDoc(Arc::clone(&self.0))
    }
}
impl<A> From<Document<'static, Self, A>> for ArcDoc<A> {
    fn from(value: Document<'static, Self, A>) -> Self {
        ArcDoc(Arc::new(value))
    }
}
impl<A> From<ArcDoc<A>> for BoxDoc<A> where A : Clone {
    fn from(arc_doc: ArcDoc<A>) -> Self {
        arc_doc.into_box()
    }
}

#[cfg(feature = "serde")]
impl<A> Serialize for ArcDoc<A> where A : Serialize {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer {
        self.0.serialize(serializer)
    }
}
#[cfg(feature = "serde")]
impl<'de, A> Deserialize<'de> for ArcDoc<A> where A : Deserialize<'de> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>
    {
        Ok(ArcDoc(Arc::deserialize(deserializer)?))
    }
}

impl<A> ArcDoc<A> {
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
    pub fn breaker<'s>(
        flat: impl Into<Cow<'s, str>>,
        broken: impl Into<Cow<'s, str>>,
    ) -> Self {
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

    /// Produces an equivalent `BoxDoc` from `&self`, cloning where required.
    pub fn to_box(&self) -> BoxDoc<A> where A : Clone {
        match self.deref() {
            Document::Nil => BoxDoc::nil(),
            Document::Text(fragment) => {
                BoxDoc(Box::new(Document::Text(fragment.clone())))
            },
            Document::Break(breaker) => {
                BoxDoc(Box::new(Document::Break(breaker.clone())))
            },
            Document::HardLinebreak => BoxDoc::hard_linebreak(),

            Document::Group(policy, child) => {
                BoxDoc::group(child.to_box(), *policy)
            },
            Document::Sequence(sequence) => {
                let children = sequence.children().iter()
                    .map(|child| child.to_box())
                    .collect::<Vec<_>>();
                BoxDoc::sequence(children)
            },
            Document::Nest(indentation, child) => {
                BoxDoc::nest(*indentation, child.to_box())
            },
            Document::Annotation(annotation, child) => {
                BoxDoc::annotation((**annotation).clone(), child.to_box())
            },
        }
    }

    /// Produces an equivalent `BoxDoc` from `self`, taking where possible and cloning otherwise.
    pub fn into_box(self) -> BoxDoc<A> where A : Clone {
        let taken = match Arc::<_>::try_unwrap(self.0) {
            Ok(taken) => taken,
            Err(inner) => {
                return ArcDoc(inner).to_box();
            },
        };
        match taken {
            Document::Nil => BoxDoc::nil(),
            Document::Text(fragment) => {
                BoxDoc(Box::new(Document::Text(fragment)))
            },
            Document::Break(breaker) => {
                BoxDoc(Box::new(Document::Break(breaker)))
            },
            Document::HardLinebreak => BoxDoc::hard_linebreak(),

            Document::Group(policy, child) => {
                BoxDoc::group(child.into_box(), policy)
            },
            Document::Sequence(sequence) => {
                let children = sequence.into_children().into_iter()
                    .map(|child| child.into_box())
                    .collect::<Vec<_>>();
                BoxDoc::sequence(children)
            },
            Document::Nest(indentation, child) => {
                BoxDoc::nest(indentation, child.into_box())
            },
            Document::Annotation(annotation, child) => {
                BoxDoc::annotation(*annotation, child.into_box())
            },
        }
    }
}

/// The builder structure for [`ArcDoc`].
/// 
/// Unlike [`RefDocBuilder`](crate::RefDocBuilder), this builder allocates onto the heap via [`Arc`],
/// and serves to deduplicate leaf nodes already present in the current document.
/// 
/// ## Note on [`Document`](crate::document::Document) Smart Constructors (ie. Why `&self`?)
///
/// Due to interning, a `&self` parameter is taken for all smart constructors.
/// See [`Self::alloc`] for more information.
pub struct ArcDocBuilder<A> {
    intern: Mutex<HashMap< LeafKey<'static>, ArcDoc<A> >>,
}
impl<A> Default for ArcDocBuilder<A> {
    fn default() -> Self {
        Self { intern: Mutex::new(HashMap::new()) }
    }
}
impl<A> ArcDocBuilder<A> {
    pub fn new() -> Self where A : Default {
        Self::default()
    }
    
    /// Allocates and deduplicates document nodes interned by the builder.
    /// Hence, the `alloc` closure expected by `Document` is `|inner| self.alloc(inner)`.
    /// 
    /// This is used internally for all smart construction, and is not intended to be used directly.
    /// 
    /// ## Why `&self`?
    /// 
    /// In short: more ergonomic expression-based usage.
    /// 
    /// ```
    /// use groupnest::{ArcDocBuilder, GroupPolicy};
    /// let builder: ArcDocBuilder<()> = ArcDocBuilder::new();
    /// 
    /// // This is possible with immutable reference, but not mutable reference:
    /// let example = builder.group(
    ///     builder.sequence(vec![
    ///         builder.flat_text("Some text here..."),
    ///         builder.flat_text("... and end."),
    ///     ]),
    ///     GroupPolicy::Normal
    /// );
    /// ```
    /// 
    /// Each invokation of the smart constructors above would need to be bound to individual statements
    /// if `builder` is taken via mutable reference.
    /// 
    /// In practice, the interning map is gated behind a synchronization lock.
    /// As Rust has specified execution order, these operations occur in the order you expect them to.
    pub fn alloc(&self, doc: Document<'static, ArcDoc<A>, A>) -> ArcDoc<A> {
        let is_leaf = LeafKey::try_from(&doc);
        // If this is a leaf node, try to find an interned copy, and return that instead.
        if let Ok(leaf_key) = &is_leaf
            && let Some(cached) = self.intern.lock().unwrap().get(leaf_key)
        {
            return (*cached).clone()
        }
        // Allocate (no-op).
        let arc_doc: ArcDoc<A> = doc.into();
        // If this is a leaf node, intern this, which has no copy given the earlier check.
        if let Ok(leaf_key) = is_leaf {
            let mut lock = self.intern.lock().unwrap();
            lock.insert(leaf_key, arc_doc.clone());
        }
        arc_doc
    }

    /// The smart constructor for a nil node.
    pub fn nil(&self) -> ArcDoc<A> {
        Document::nil(|doc| self.alloc(doc))
    }
    /// The smart constructor for literal text.
    ///
    /// # Panics
    ///
    /// Panics if the payload contains tabs.
    pub fn from_text<'s>(&self, payload: impl Into<Cow<'s, str>>) -> ArcDoc<A> {
        let text = payload.into().into_owned();
        Document::from_text(text, |inner| self.alloc(inner))
            .map_err(|err| panic!("{err}"))
            .unwrap()
    }
    /// The non-panicking smart constructor for literal text.
    pub fn from_text_<'s>(
        &self,
        payload: impl Into<Cow<'s, str>>,
    ) -> Result<ArcDoc<A>, ContainsTab> {
        let text = payload.into().into_owned();
        Document::from_text(text, |inner| self.alloc(inner))
    }
    /// The smart constructor for flat text fragments.
    ///
    /// # Panics
    ///
    /// Panics if the payload contains newline sequences or tabs.
    pub fn flat_text<'s>(&self, payload: impl Into<Cow<'s, str>>) -> ArcDoc<A> {
        let text = payload.into().into_owned();
        Document::flat_text(text, |inner| self.alloc(inner))
            .map_err(|err| panic!("{err}"))
            .unwrap()
    }
    /// The non-panicking smart constructor for flat text fragments.
    pub fn flat_text_<'s>(
        &self,
        payload: impl Into<Cow<'s, str>>,
    ) -> Result<ArcDoc<A>, FragmentError> {
        let text = payload.into().into_owned();
        Document::flat_text(text, |inner| self.alloc(inner))
    }
    /// The smart constructor for break nodes.
    ///
    /// # Panics
    ///
    /// Panics if `flat` contains newline sequences, `broken` does not contain any, and either contain tabs.
    pub fn breaker<'s>(
        &self,
        flat: impl Into<Cow<'s, str>>,
        broken: impl Into<Cow<'s, str>>,
    ) -> ArcDoc<A> {
        let flat = flat.into().into_owned();
        let broken = broken.into().into_owned();
        Document::breaker(flat, broken, |inner| self.alloc(inner))
            .map_err(|err| panic!("{err}"))
            .unwrap()
    }
    /// The non-panicking smart constructor for break nodes.
    pub fn breaker_<'s>(
        &self,
        flat: impl Into<Cow<'s, str>>,
        broken: impl Into<Cow<'s, str>>,
    ) -> Result<ArcDoc<A>, BreakNodeInvalid> {
        let flat = flat.into().into_owned();
        let broken = broken.into().into_owned();
        Document::breaker(flat, broken, |inner| self.alloc(inner))
    }
    /// The smart constructor for a hard linebreak.
    pub fn hard_linebreak(&self) -> ArcDoc<A> {
        Document::hard_linebreak(|inner| self.alloc(inner))
    }
    /// The smart constructor for a group with the specified policy.
    pub fn group(&self, child: ArcDoc<A>, policy: GroupPolicy) -> ArcDoc<A> {
        Document::group(child, policy, |inner| self.alloc(inner))
    }
    /// The smart constructor for a grouped sequence.
    pub fn grouped_sequence(
        &self,
        children: Vec<ArcDoc<A>>,
        policy: GroupPolicy,
    ) -> ArcDoc<A> {
        Document::grouped_sequence(children, policy, |inner| self.alloc(inner))
    }
    /// The smart constructor for a collection sequence.
    pub fn sequence(&self, children: Vec<ArcDoc<A>>) -> ArcDoc<A> {
        Document::sequence(children, |inner| self.alloc(inner))
    }
    /// The smart constructor for a collection sequence with interspersion.
    pub fn sequence_intersperse_with(
        &self,
        children: Vec<ArcDoc<A>>,
        separator: ArcDoc<A>,
    ) -> ArcDoc<A>
    where
        A: Clone,
    {
        Document::sequence_intersperse_with(children, separator, |inner| self.alloc(inner))
    }
    /// The smart constructor for nesting.
    pub fn nest(&self, indentation: usize, inner: ArcDoc<A>) -> ArcDoc<A> {
        Document::nest(indentation, inner, |inner| self.alloc(inner))
    }
    /// The smart constructor for annotations.
    pub fn annotation(&self, annotation: A, inner: ArcDoc<A>) -> ArcDoc<A> {
        Document::annotation(annotation, inner, |inner| self.alloc(inner))
    }
}

/// Const assertion of `ArcDoc` `Send + Sync`.
const _: () = {
    #[allow(unused)]
    use std::rc::Rc;
    const fn assert_send<T: Send + Sync>() {}
    assert_send::<ArcDoc<()>>();
    assert_send::<ArcDoc<String>>();
};
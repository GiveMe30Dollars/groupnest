use std::{
    borrow::Cow,
    collections::HashMap,
    ops::Deref,
    sync::{Arc, Mutex},
};

use derive_more::AsRef;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::{
    BoxDoc, GroupPolicy,
    document::{BreakNodeInvalid, ContainsTab, Document, FragmentError},
    handle::common::LeafKey,
};

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
/// Prefer [`ArcDocBuilder`] instead.
/// 
/// [`RefDoc`](crate::RefDoc), [`BoxDoc`](crate::BoxDoc) and [`ArcDoc`](crate::ArcDoc) erase their wrappers during serialization,
/// and hence share identical serialized representations.
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
impl<A> From<ArcDoc<A>> for BoxDoc<A>
where
    A: Clone,
{
    fn from(arc_doc: ArcDoc<A>) -> Self {
        arc_doc.into_box()
    }
}

#[cfg(feature = "serde")]
impl<A> Serialize for ArcDoc<A>
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
impl<'de, A> Deserialize<'de> for ArcDoc<A>
where
    A: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
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

    /// Produces an equivalent [`BoxDoc`] from `&self`, cloning where required.
    ///
    /// Identical to an equivalent [`Document::to_representation`] invokation.
    pub fn to_box(&self) -> BoxDoc<A>
    where
        A: Clone,
    {
        self.0.to_representation(&mut Into::into)
    }

    /// Produces an equivalent [`BoxDoc`] from `self`, taking where possible and cloning otherwise.
    pub fn into_box(self) -> BoxDoc<A>
    where
        A: Clone,
    {
        let taken = match Arc::<_>::try_unwrap(self.0) {
            Ok(taken) => taken,
            Err(inner) => {
                return ArcDoc(inner).to_box();
            }
        };
        match taken {
            Document::Nil => BoxDoc::nil(),
            Document::Text(fragment) => BoxDoc(Box::new(Document::Text(fragment))),
            Document::Break(breaker) => BoxDoc(Box::new(Document::Break(breaker))),
            Document::HardLinebreak => BoxDoc::hard_linebreak(),

            Document::Group(policy, child) => BoxDoc::group_with(policy, child.into_box()),
            Document::Sequence(sequence) => {
                let children = sequence
                    .into_children()
                    .into_iter()
                    .map(|child| child.into_box())
                    .collect::<Vec<_>>();
                BoxDoc::sequence(children)
            }
            Document::Nest(indentation, child) => BoxDoc::nest(indentation, child.into_box()),
            Document::Annotation(annotation, child) => {
                BoxDoc::annotation(*annotation, child.into_box())
            }
        }
    }
}

/// The builder structure for [`ArcDoc`].
///
/// Unlike [`ArcDocBuilder`](crate::ArcDocBuilder), this builder allocates onto the heap via [`Arc`],
/// and serves to deduplicate leaf nodes already present in the current document.
///
/// ## Note on [`Document`](crate::document::Document) Smart Constructors (ie. Why `&self`?)
///
/// Due to interning, a `&self` parameter is taken for all smart constructors.
/// See [`Self::alloc`] for more information.
pub struct ArcDocBuilder<A = ()> {
    intern: Mutex<HashMap<LeafKey<'static>, ArcDoc<A>>>,
}
impl<A> Default for ArcDocBuilder<A> {
    fn default() -> Self {
        Self {
            intern: Mutex::new(HashMap::new()),
        }
    }
}
impl<A> ArcDocBuilder<A> {
    pub fn new() -> Self
    where
        A: Default,
    {
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
            return (*cached).clone();
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
    /// The smart constructor for a group, using the default policy.
    pub fn group(&self, child: ArcDoc<A>) -> ArcDoc<A> {
        Document::group(child, |inner| self.alloc(inner))
    }
    /// The smart constructor for a group with the specified policy.
    pub fn group_with(&self, policy: GroupPolicy, child: ArcDoc<A>) -> ArcDoc<A> {
        Document::group_with(policy, child, |inner| self.alloc(inner))
    }
    /// The smart constructor for a grouped sequence, using the default policy.
    pub fn grouped_sequence(&self, children: Vec<ArcDoc<A>>) -> ArcDoc<A> {
        Document::grouped_sequence(children, |inner| self.alloc(inner))
    }
    /// The smart constructor for a grouped sequence with the specified policy.
    pub fn grouped_sequence_with(
        &self,
        policy: GroupPolicy,
        children: Vec<ArcDoc<A>>,
    ) -> ArcDoc<A> {
        Document::grouped_sequence_with(policy, children, |inner| self.alloc(inner))
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

/// This module adds `DeserializeSeed` support to `ArcDocBuilder`.
///
/// This mostly entailed threading `ArcDocBuilder` anywhere an `ArcDoc` would need to be constructed,
/// then invoking the respective smart construction methods on it.
///
/// This is made verbose by `serde` being verbose.
#[cfg(feature = "serde")]
mod serde_support {
    use crate::{
        ArcDoc, ArcDocBuilder, GroupPolicy,
        document::{Break, Document, FlatFragment},
    };
    use derive_more::From;
    use serde::{
        Deserialize,
        de::{DeserializeSeed, VariantAccess, Visitor},
    };
    use std::{any::type_name, marker::PhantomData};

    impl<'de, A> DeserializeSeed<'de> for &ArcDocBuilder<A>
    where
        A: Deserialize<'de>,
    {
        type Value = ArcDoc<A>;

        fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            deserializer.deserialize_enum(
                "Document",
                &[
                    "Nil",
                    "Text",
                    "Break",
                    "HardLinebreak",
                    "Group",
                    "Sequence",
                    "Nest",
                    "Annotation",
                ],
                ArcDocVisitor::from(self),
            )
        }
    }

    // SANITY CHECK:
    // pub enum Document<'s, D, A = ()> {
    //     Nil,
    //     Text(FlatFragment<'s>),
    //     Break(Break<'s>),
    //     HardLinebreak,
    //     Group(GroupPolicy, D),
    //     Sequence(Sequence<D>),
    //     Nest(usize, D),
    //     Annotation(Box<A>, D),
    // }

    #[derive(Deserialize)]
    enum DocumentVariant {
        Nil,
        Text,
        Break,
        HardLinebreak,
        Group,
        Sequence,
        Nest,
        Annotation,
    }

    /// The main ArcDocVisitor.
    #[derive(From)]
    struct ArcDocVisitor<'b, A> {
        builder: &'b ArcDocBuilder<A>,
    }

    impl<'de, 'b, A> DeserializeSeed<'de> for ArcDocVisitor<'b, A>
    where
        A: Deserialize<'de>,
    {
        type Value = ArcDoc<A>;

        fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            deserializer.deserialize_enum(
                "Document",
                &[
                    "Nil",
                    "Text",
                    "Break",
                    "HardLinebreak",
                    "Group",
                    "Sequence",
                    "Nest",
                    "Annotation",
                ],
                self,
            )
        }
    }
    impl<'de, 'b, A> Visitor<'de> for ArcDocVisitor<'b, A>
    where
        A: Deserialize<'de>,
    {
        type Value = ArcDoc<A>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a Document")
        }
        fn visit_enum<B>(self, data: B) -> Result<Self::Value, B::Error>
        where
            B: serde::de::EnumAccess<'de>,
        {
            let (variant, values) = data.variant::<DocumentVariant>()?;
            match variant {
                // Nullary variants
                DocumentVariant::Nil => {
                    values.unit_variant()?;
                    Ok(self.builder.nil())
                }
                DocumentVariant::HardLinebreak => {
                    values.unit_variant()?;
                    Ok(self.builder.hard_linebreak())
                }

                // Singleton variants, with the payload implementing `Deserialize`.
                DocumentVariant::Text => {
                    let text = values.newtype_variant::<FlatFragment<'static>>()?;
                    Ok(self.builder.alloc(Document::Text(text)))
                }
                DocumentVariant::Break => {
                    let breaker = values.newtype_variant::<Break<'static>>()?;
                    Ok(self.builder.alloc(Document::Break(breaker)))
                }

                // Two-tuple variants, with a `Deserialize`-able first payload and a subsequent recursive paylaod.
                DocumentVariant::Group => {
                    let (policy, child) = values
                        .tuple_variant(2, TwoTupleVisitor::<A, GroupPolicy>::from(self.builder))?;
                    Ok(self.builder.group_with(policy, child))
                }
                DocumentVariant::Nest => {
                    let (indentation, inner) =
                        values.tuple_variant(2, TwoTupleVisitor::<A, usize>::from(self.builder))?;
                    Ok(self.builder.nest(indentation, inner))
                }
                DocumentVariant::Annotation => {
                    let (annotation, inner) =
                        values.tuple_variant(2, TwoTupleVisitor::<A, A>::from(self.builder))?;
                    Ok(self.builder.annotation(annotation, inner))
                }

                // Sequence, which carries a collection of recursive children.
                DocumentVariant::Sequence => {
                    let children =
                        values.newtype_variant_seed(SequenceVisitor::from(self.builder))?;
                    Ok(self.builder.sequence(children))
                }
            }
        }
    }

    /// A visitor for a two-arity tuple which of type (O, ArcDoc).
    /// O must implement Deserialize<'s> for this to be meaningful.
    struct TwoTupleVisitor<'b, A, O> {
        builder: &'b ArcDocBuilder<A>,
        _other: PhantomData<O>,
    }
    impl<'b, A, O> From<&'b ArcDocBuilder<A>> for TwoTupleVisitor<'b, A, O> {
        fn from(builder: &'b ArcDocBuilder<A>) -> Self {
            TwoTupleVisitor {
                builder,
                _other: PhantomData,
            }
        }
    }

    impl<'de, 'b, A, O> Visitor<'de> for TwoTupleVisitor<'b, A, O>
    where
        A: Deserialize<'de>,
        O: Deserialize<'de>,
    {
        type Value = (O, ArcDoc<A>);

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str(&format!(
                "a two-arity tuple of type ({}, ArcDoc)",
                type_name::<O>()
            ))
        }
        fn visit_seq<B>(self, mut seq: B) -> Result<Self::Value, B::Error>
        where
            B: serde::de::SeqAccess<'de>,
        {
            let first_element = seq
                .next_element()?
                .ok_or_else(|| serde::de::Error::invalid_length(1, &self))?;
            let recursive_element = seq
                .next_element_seed(self.builder)?
                .ok_or_else(|| serde::de::Error::invalid_length(2, &self))?;
            Ok((first_element, recursive_element))
        }
    }

    #[derive(From)]
    struct SequenceVisitor<'b, A> {
        builder: &'b ArcDocBuilder<A>,
    }
    impl<'de, 'b, A> DeserializeSeed<'de> for SequenceVisitor<'b, A>
    where
        A: Deserialize<'de>,
    {
        type Value = Vec<ArcDoc<A>>;

        fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            deserializer.deserialize_seq(self)
        }
    }
    impl<'de, 'b, A> Visitor<'de> for SequenceVisitor<'b, A>
    where
        A: Deserialize<'de>,
    {
        type Value = Vec<ArcDoc<A>>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a sequence of RefDocs")
        }
        fn visit_seq<B>(self, mut seq: B) -> Result<Self::Value, B::Error>
        where
            B: serde::de::SeqAccess<'de>,
        {
            let mut children = Vec::new();
            while let Some(child) = seq.next_element_seed(ArcDocVisitor::from(self.builder))? {
                children.push(child);
            }
            Ok(children)
        }
    }
}

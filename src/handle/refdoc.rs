use std::{borrow::Cow, collections::HashMap, fmt::Debug, ops::Deref, sync::Mutex};

use derive_more::{AsRef, From, Into};
use typed_arena::Arena;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize, de::DeserializeSeed};

use crate::{
    document::{BreakNodeInvalid, ContainsTab, Document, FragmentError, GroupPolicy},
    handle::common::LeafKey,
};

/// The notation document format, allocated via arena and taking immutable reference to its children and fragments.
///
/// Due to reliance on arena allocation, this type may not be `Send + Sync`.
///
/// ## Note on [`Document`](crate::document::Document) Smart Constructors
///
/// Due to not owning its internals, this type cannot construct itself.
/// It is the responsibility of [`RefDocBuilder`] and similar data structures to implement the [`Document`] smart constructors.
///
/// ## Note on [`serde`] Support
///
/// As this type is unable to contruct itself, it currently only implements [`serde::Serialize`]. Use [`RefDocBuilder`] instead.
/// 
/// [`RefDoc`], [`BoxDoc`](crate::BoxDoc) and [`ArcDoc`](crate::ArcDoc) erase their wrappers during serialization,
/// and hence share identical serialized representations.
#[repr(transparent)]
#[derive(Debug, Clone, PartialEq, Eq, Hash, From, Into, AsRef)]
pub struct RefDoc<'s, 'doc, A = ()>(pub &'doc Document<'s, Self, A>);
impl<'s, 'doc, A> Deref for RefDoc<'s, 'doc, A> {
    type Target = Document<'s, Self, A>;
    fn deref(&self) -> &Self::Target {
        self.0
    }
}
#[cfg(feature = "serde")]
impl<'s, 'doc, A> Serialize for RefDoc<'s, 'doc, A>
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

/// The builder structure for [`RefDoc`].
///
/// ## Note on [`Document`](crate::document::Document) Smart Constructors (ie. Why `&self`?)
///
/// Due to interning and arena allocation, a `&self` parameter is taken for all smart constructors.
/// See [`Self::alloc`] for more information.
pub struct RefDocBuilder<'s, 'doc, A = ()> {
    arena: &'doc Arena<Document<'s, RefDoc<'s, 'doc, A>, A>>,
    intern: Mutex<HashMap<LeafKey<'s>, &'doc Document<'s, RefDoc<'s, 'doc, A>, A>>>,
}
impl<'s, 'doc, A> Debug for RefDocBuilder<'s, 'doc, A>
where
    A: Debug,
{
    /// Debug information on the backing arena is omitted because [`typed_arena::Arena`] does not implement Debug.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DocBuilder")
            .field("arena", &"[Debug representation hidden]")
            .field("intern", &self.intern)
            .finish()
    }
}

impl<'s, 'doc, A> RefDocBuilder<'s, 'doc, A> {
    /// Create a new [`RefDocBuilder`]. The backing arena must remain live.
    /// Despite the type parameter `A` defaulting to `()` in `Doc` and `Document`,
    /// it must be specified here.
    /// If you intend to consume the document in the same scope as the builder and arena:
    /// ```
    /// use groupnest::{Arena, RefDocBuilder};
    /// let arena = Arena::new();
    /// let builder: RefDocBuilder<'_, '_, ()> = RefDocBuilder::new(&arena);
    /// ```
    pub fn new(arena: &'doc Arena<Document<'s, RefDoc<'s, 'doc, A>, A>>) -> Self {
        Self {
            arena,
            intern: Mutex::new(HashMap::new()),
        }
    }

    /// Allocates and/or interns a document node into the held arena.
    /// Hence, the `alloc` closure expected by `Document` is `|inner| self.alloc(inner)`.
    ///
    /// This is used internally for all smart construction, and is not intended to be used directly.
    ///
    /// ## Why `&self`?
    ///
    /// In short: more ergonomic expression-based usage.
    ///
    /// ```
    /// use groupnest::{Arena, RefDocBuilder, GroupPolicy};
    /// let arena = Arena::new();
    /// let builder: RefDocBuilder<'_, '_, ()> = RefDocBuilder::new(&arena);
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
    /// In practice, [`typed_arena::Arena`] already observes interior mutability by allocating via `&self`,
    /// so the only change this necessitates is gating the interning map behind a synchronization lock.
    /// As Rust has specified execution order, these operations occur in the order you expect them to.
    pub fn alloc(&self, doc: Document<'s, RefDoc<'s, 'doc, A>, A>) -> RefDoc<'s, 'doc, A> {
        let is_leaf = LeafKey::try_from(&doc);
        // If this is a leaf node, try to find an interned copy, and return that instead.
        if let Ok(leaf_key) = &is_leaf
            && let Some(cached) = self.intern.lock().unwrap().get(leaf_key)
        {
            return RefDoc(*cached);
        }
        // Allocate.
        let reference = self.arena.alloc(doc);
        // If this is a leaf node, intern this, which has no copy given the earlier check.
        if let Ok(leaf_key) = is_leaf {
            let mut lock = self.intern.lock().unwrap();
            lock.insert(leaf_key, reference);
        }
        RefDoc(reference)
    }

    /// The smart constructor for a nil node.
    pub fn nil(&self) -> RefDoc<'s, 'doc, A> {
        Document::nil(|doc| self.alloc(doc))
    }
    /// The smart constructor for literal text.
    ///
    /// # Panics
    ///
    /// Panics if the payload contains tabs.
    pub fn from_text(&self, payload: impl Into<Cow<'s, str>>) -> RefDoc<'s, 'doc, A> {
        Document::from_text(payload, |inner| self.alloc(inner))
            .map_err(|err| panic!("{err}"))
            .unwrap()
    }
    /// The non-panicking smart constructor for literal text.
    pub fn from_text_(
        &self,
        payload: impl Into<Cow<'s, str>>,
    ) -> Result<RefDoc<'s, 'doc, A>, ContainsTab> {
        Document::from_text(payload, |inner| self.alloc(inner))
    }
    /// The smart constructor for flat text fragments.
    ///
    /// # Panics
    ///
    /// Panics if the payload contains newline sequences or tabs.
    pub fn flat_text(&self, payload: impl Into<Cow<'s, str>>) -> RefDoc<'s, 'doc, A> {
        Document::flat_text(payload, |inner| self.alloc(inner))
            .map_err(|err| panic!("{err}"))
            .unwrap()
    }
    /// The non-panicking smart constructor for flat text fragments.
    pub fn flat_text_(
        &self,
        payload: impl Into<Cow<'s, str>>,
    ) -> Result<RefDoc<'s, 'doc, A>, FragmentError> {
        Document::flat_text(payload, |inner| self.alloc(inner))
    }
    /// The smart constructor for break nodes.
    ///
    /// # Panics
    ///
    /// Panics if `flat` contains newline sequences, `broken` does not contain any, and either contain tabs.
    pub fn breaker(
        &self,
        flat: impl Into<Cow<'s, str>>,
        broken: impl Into<Cow<'s, str>>,
    ) -> RefDoc<'s, 'doc, A> {
        Document::breaker(flat, broken, |inner| self.alloc(inner))
            .map_err(|err| panic!("{err}"))
            .unwrap()
    }
    /// The non-panicking smart constructor for break nodes.
    pub fn breaker_(
        &self,
        flat: impl Into<Cow<'s, str>>,
        broken: impl Into<Cow<'s, str>>,
    ) -> Result<RefDoc<'s, 'doc, A>, BreakNodeInvalid> {
        Document::breaker(flat, broken, |inner| self.alloc(inner))
    }
    /// The smart constructor for a hard linebreak.
    pub fn hard_linebreak(&self) -> RefDoc<'s, 'doc, A> {
        Document::hard_linebreak(|inner| self.alloc(inner))
    }
    /// The smart constructor for a group, using the default policy.
    pub fn group(&self, child: RefDoc<'s, 'doc, A>) -> RefDoc<'s, 'doc, A> {
        Document::group(child, |inner| self.alloc(inner))
    }
    /// The smart constructor for a group with the specified policy.
    pub fn group_with(
        &self,
        policy: GroupPolicy,
        child: RefDoc<'s, 'doc, A>,
    ) -> RefDoc<'s, 'doc, A> {
        Document::group_with(policy, child, |inner| self.alloc(inner))
    }
    /// The smart constructor for a grouped sequence, using the default policy.
    pub fn grouped_sequence(&self, children: Vec<RefDoc<'s, 'doc, A>>) -> RefDoc<'s, 'doc, A> {
        Document::grouped_sequence(children, |inner| self.alloc(inner))
    }
    /// The smart constructor for a grouped sequence with the specified policy.
    pub fn grouped_sequence_with(
        &self,
        policy: GroupPolicy,
        children: Vec<RefDoc<'s, 'doc, A>>,
    ) -> RefDoc<'s, 'doc, A> {
        Document::grouped_sequence_with(policy, children, |inner| self.alloc(inner))
    }
    /// The smart constructor for a collection sequence.
    pub fn sequence(&self, children: Vec<RefDoc<'s, 'doc, A>>) -> RefDoc<'s, 'doc, A> {
        Document::sequence(children, |inner| self.alloc(inner))
    }
    /// The smart constructor for a collection sequence with interspersion.
    pub fn sequence_intersperse_with(
        &self,
        children: Vec<RefDoc<'s, 'doc, A>>,
        separator: RefDoc<'s, 'doc, A>,
    ) -> RefDoc<'s, 'doc, A>
    where
        A: Clone,
    {
        Document::sequence_intersperse_with(children, separator, |inner| self.alloc(inner))
    }
    /// The smart constructor for nesting.
    pub fn nest(&self, indentation: usize, inner: RefDoc<'s, 'doc, A>) -> RefDoc<'s, 'doc, A> {
        Document::nest(indentation, inner, |inner| self.alloc(inner))
    }
    /// The smart constructor for annotations.
    pub fn annotation(&self, annotation: A, inner: RefDoc<'s, 'doc, A>) -> RefDoc<'s, 'doc, A> {
        Document::annotation(annotation, inner, |inner| self.alloc(inner))
    }
}
#[cfg(feature = "serde")]
impl<'s, 'doc, A> DeserializeSeed<'s> for &'doc RefDocBuilder<'s, 'doc, A>
where
    A: Deserialize<'s>,
{
    type Value = RefDoc<'s, 'doc, A>;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'s>,
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
            serde_support::RefDocVisitor::from(self),
        )
    }
}

/// This module adds `DeserializeSeed` support to `RefDocBuilder`.
///
/// This mostly entailed threading `RefDocBuilder` anywhere a `RefDoc` would need to be constructed,
/// then invoking the respective smart construction methods on it.
///
/// This is made verbose by `serde` being verbose.
#[cfg(feature = "serde")]
mod serde_support {
    use crate::{
        GroupPolicy, RefDoc, RefDocBuilder,
        document::{Break, Document, FlatFragment},
    };
    use derive_more::From;
    use serde::{
        Deserialize,
        de::{DeserializeSeed, VariantAccess, Visitor},
    };
    use std::{any::type_name, marker::PhantomData};

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

    /// The main RefDocVisitor.
    #[derive(From)]
    pub(crate) struct RefDocVisitor<'s, 'doc, A> {
        builder: &'doc RefDocBuilder<'s, 'doc, A>,
    }

    impl<'s, 'doc, A> DeserializeSeed<'s> for RefDocVisitor<'s, 'doc, A>
    where
        A: Deserialize<'s>,
    {
        type Value = RefDoc<'s, 'doc, A>;

        fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
        where
            D: serde::Deserializer<'s>,
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
    impl<'s, 'doc, A> Visitor<'s> for RefDocVisitor<'s, 'doc, A>
    where
        A: Deserialize<'s>,
    {
        type Value = RefDoc<'s, 'doc, A>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a Document")
        }
        fn visit_enum<B>(self, data: B) -> Result<Self::Value, B::Error>
        where
            B: serde::de::EnumAccess<'s>,
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
                    let text = values.newtype_variant::<FlatFragment<'s>>()?;
                    Ok(self.builder.alloc(Document::Text(text)))
                }
                DocumentVariant::Break => {
                    let breaker = values.newtype_variant::<Break<'s>>()?;
                    Ok(self.builder.alloc(Document::Break(breaker)))
                }

                // Two-tuple variants, with a `Deserialize`-able first payload and a subsequent recursive paylaod.
                DocumentVariant::Group => {
                    let (policy, child) = values.tuple_variant(
                        2,
                        TwoTupleVisitor::<'s, 'doc, A, GroupPolicy>::from(self.builder),
                    )?;
                    Ok(self.builder.group_with(policy, child))
                }
                DocumentVariant::Nest => {
                    let (indentation, inner) = values.tuple_variant(
                        2,
                        TwoTupleVisitor::<'s, 'doc, A, usize>::from(self.builder),
                    )?;
                    Ok(self.builder.nest(indentation, inner))
                }
                DocumentVariant::Annotation => {
                    let (annotation, inner) = values
                        .tuple_variant(2, TwoTupleVisitor::<'s, 'doc, A, A>::from(self.builder))?;
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

    /// A visitor for a two-arity tuple which of type (O, RefDoc).
    /// O must implement Deserialize<'s> for this to be meaningful.
    struct TwoTupleVisitor<'s, 'doc, A, O> {
        builder: &'doc RefDocBuilder<'s, 'doc, A>,
        _other: PhantomData<O>,
    }
    impl<'s, 'doc, A, O> From<&'doc RefDocBuilder<'s, 'doc, A>> for TwoTupleVisitor<'s, 'doc, A, O> {
        fn from(builder: &'doc RefDocBuilder<'s, 'doc, A>) -> Self {
            TwoTupleVisitor {
                builder,
                _other: PhantomData,
            }
        }
    }

    impl<'s, 'doc, A, O> Visitor<'s> for TwoTupleVisitor<'s, 'doc, A, O>
    where
        A: Deserialize<'s>,
        O: Deserialize<'s>,
    {
        type Value = (O, RefDoc<'s, 'doc, A>);

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str(&format!(
                "a two-arity tuple of type ({}, RefDoc)",
                type_name::<O>()
            ))
        }
        fn visit_seq<B>(self, mut seq: B) -> Result<Self::Value, B::Error>
        where
            B: serde::de::SeqAccess<'s>,
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
    struct SequenceVisitor<'s, 'doc, A> {
        builder: &'doc RefDocBuilder<'s, 'doc, A>,
    }
    impl<'s, 'doc, A> DeserializeSeed<'s> for SequenceVisitor<'s, 'doc, A>
    where
        A: Deserialize<'s>,
    {
        type Value = Vec<RefDoc<'s, 'doc, A>>;

        fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
        where
            D: serde::Deserializer<'s>,
        {
            deserializer.deserialize_seq(self)
        }
    }
    impl<'s, 'doc, A> Visitor<'s> for SequenceVisitor<'s, 'doc, A>
    where
        A: Deserialize<'s>,
    {
        type Value = Vec<RefDoc<'s, 'doc, A>>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a sequence of RefDocs")
        }
        fn visit_seq<B>(self, mut seq: B) -> Result<Self::Value, B::Error>
        where
            B: serde::de::SeqAccess<'s>,
        {
            let mut children = Vec::new();
            while let Some(child) = seq.next_element_seed(RefDocVisitor::from(self.builder))? {
                children.push(child);
            }
            Ok(children)
        }
    }
}

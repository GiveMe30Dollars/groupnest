//! Definitions for render events, the [`Renderer`] trait and some implementors.

use thiserror::Error;

/// A data structure representing rendering events, produced by a [`LayoutEngine`](crate::layout::LayoutEngine)
/// and to be consumed by a [`Renderer`].
/// 
/// # Note on Lifetime `'payload`
/// 
/// The lifetime `'payload` refers to the lifetime of the string reference that [`RenderEvent::Text`] holds.
/// Because this typically points to a [`Document`](crate::document::Document) type,
/// `'payload` is *not* `'s` but the lifetime of the document reference `'doc`,
/// where `'doc : 's` necessarily holds by construction.
/// 
/// Implementors may shorten `'payload` to `'p`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum RenderEvent<'payload, A = ()> {
    /// Print the following text, as is. Includes its width as calculated via Unicode width conventions.
    /// This text fragment may not contain newline sequences.
    Text(usize, &'payload str),
    /// Insert a newline character.
    Linebreak,
    /// Insert whitespace padding of the following character width.
    Padding(usize),
    /// Marks the beginning of the scope of the following annotation. Follows stack semantics.
    PushAnnotation(A),
    /// Marks the end of the scope of the most recent annotation. Follows stack semantics.
    PopAnnotation,
    /// A signalling error. Emitted in the following situations:
    /// - After a complete line where a [`LayoutWidthConstraint::Strict`](crate::layout::LayoutWidthConstraint) width constraint is violated.
    Error(LayoutError),
}

/// Errors resulting from layout rendering.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Error)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum LayoutError {
    #[error(
        "Line {line_num} has a width of {line_width} characters, exceeding the maximum width {max_width}"
    )]
    WidthExceeded {
        /// One-indexed.
        line_num: usize,
        max_width: usize,
        line_width: usize,
    },
}

/// An iterator adaptor for mapping onto the annotation type of each item.
///
/// This type is not intended to be used directly.
/// The idiomatic usage would be to use [`RenderAdaptorExt`] for method chaining.
pub struct RenderAdaptor<'p, A, I, F>
where
    I: Iterator<Item = RenderEvent<'p, A>>,
{
    iterator: I,
    closure: F,
}
impl<'p, A, I, F, B> RenderAdaptor<'p, A, I, F>
where
    I: Iterator<Item = RenderEvent<'p, A>>,
    F: FnMut(A) -> B,
{
    pub fn new<'c>(iterator: I, closure: F) -> Self
    where
        Self: 'c,
        F: 'c,
    {
        Self { iterator, closure }
    }
}
impl<'p, A, I, F, B> Iterator for RenderAdaptor<'p, A, I, F>
where
    I: Iterator<Item = RenderEvent<'p, A>>,
    F: FnMut(A) -> B,
{
    type Item = RenderEvent<'p, B>;
    fn next(&mut self) -> Option<Self::Item> {
        self.iterator.next().map(|event| match event {
            RenderEvent::PushAnnotation(annotation) => {
                RenderEvent::PushAnnotation((self.closure)(annotation))
            }
            // Yes, we do need to deconstruct + reconstruct every variant
            // because `B` might have a different size from `A`.
            // Most fields here are [`Copy`] anyways so it isn't too bad.
            RenderEvent::Text(span, payload) => RenderEvent::Text(span, payload),
            RenderEvent::Linebreak => RenderEvent::Linebreak,
            RenderEvent::Padding(num) => RenderEvent::Padding(num),
            RenderEvent::PopAnnotation => RenderEvent::PopAnnotation,
            RenderEvent::Error(error) => RenderEvent::Error(error),
        })
    }
}

/// Extension trait for iterator chaining of `.map_annotation(f)` and `.strip_annotation()`,
/// allowing conversion between different annotation types with similar or less expressive semantics.
///
/// This trait is blanket-implemented for compatible iterators.
///
/// ```
/// use groupnest::{RenderAdaptorExt, renderer::RenderEvent};
///
/// enum Formatting {
///     Normal,
///     Bold,
/// }
/// fn map_to_bold<'p>(layout: impl Iterator<Item = RenderEvent<'p, bool>>)
/// -> impl Iterator<Item = RenderEvent<'p, Formatting>> {
///     layout.map_annotation(|is_bold| if is_bold {Formatting::Bold} else {Formatting::Normal})
/// }
/// ```
/// ```
/// use groupnest::{RenderAdaptorExt, renderer::RenderEvent};
///
/// enum LargeComplicatedAnnotation {
///     // ...
/// }
/// fn map_to_unit<'p, >(layout: impl Iterator<Item = RenderEvent<'p, LargeComplicatedAnnotation>>)
/// -> impl Iterator<Item = RenderEvent<'p, ()>> {
///     layout.strip_annotation()
/// }
///
/// ```
pub trait RenderAdaptorExt<'p, A>
where
    Self: Iterator<Item = RenderEvent<'p, A>> + Sized,
{
    /// Given a closure, `map` annotations from self, leaving all other events intact.
    fn map_annotation<'c, B, F>(self, closure: F) -> RenderAdaptor<'p, A, Self, F>
    where
        Self: 'c,
        F: FnMut(A) -> B + 'c,
    {
        RenderAdaptor::new(self, closure)
    }
    /// `map` annotations from self to the unit, leaving all other events intact.
    fn strip_annotation(self) -> RenderAdaptor<'p, A, Self, Box<dyn FnMut(A)>> {
        let strip: Box<dyn FnMut(A)> = Box::new(|_| ());
        RenderAdaptor::new(self, strip)
    }
}
impl<'p, A, I> RenderAdaptorExt<'p, A> for I where I: Iterator<Item = RenderEvent<'p, A>> {}

/// Trait describing renderers that receive render events to stream or store text of the formatted document.
///
/// # Invariants
///
/// The render event stream supplied to a [`Renderer`] implementor must be synchronous and well-formed,
/// usually from direct emission of [`LayoutEngine`](crate::layout::LayoutEngine)
/// and/or after transformation of annotation type `A`.
///
/// [`Renderer`] implementors are not obliged to handle malformed input gracefully.
pub trait Renderer<A> {
    /// The type signalling the end-of-rendering.
    type Finish;
    /// The type signalling a render error.
    type Error;

    /// Receives a render event.
    fn receive<'p>(&mut self, event: RenderEvent<'p, A>) -> Result<(), Self::Error>;
    /// Signals the end of the event stream. Defaults to no-op.
    /// This may be used to flush buffers etc.
    fn finish(&mut self) -> Result<Self::Finish, Self::Error>;

    /// Consumes a render event iterator.
    fn consume<'p>(
        &mut self,
        iterator: impl Iterator<Item = RenderEvent<'p, A>>,
    ) -> Result<Self::Finish, Self::Error> {
        for event in iterator {
            self.receive(event)?;
        }
        self.finish()
    }
}

/// The (typical) error type of rendering.
///
/// [`Renderer`] implementors may choose to implement alternate error representations.
#[derive(Debug, Error)]
pub enum RenderError {
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error(transparent)]
    LayoutError(#[from] LayoutError),
}

/// A plaintext renderer implementing the [`Renderer`] protocol.
/// 
/// Layout errors are eagerly reported. It is assumed the underlyinng buffer does not have a width constraint.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct PlaintextRenderer<W> {
    pub inner: W,
}
impl<W> PlaintextRenderer<W> {
    pub fn new(inner: W) -> Self {
        Self { inner }
    }
    /// Renders the given stream into an initial default value of `W`, then returns that value.
    ///
    /// ## Note on [`String`]
    ///
    /// `String` does not implement [`std::io::Write`], and hence is incompatible for this function.
    /// Use [`Self::render_to_string()`] instead.
    pub fn render<'p, A>(
        iterator: impl Iterator<Item = RenderEvent<'p, A>>,
    ) -> Result<W, <Self as Renderer<A>>::Error>
    where
        Self: Default + Renderer<A>,
    {
        let mut renderer = Self::default();
        renderer.consume(iterator)?;
        Ok(renderer.inner)
    }
}
impl PlaintextRenderer<Vec<u8>> {
    /// Renders the given stream into the empty string, returning the result.
    ///
    /// It is known that all render events express valid UTF-8 text, hence the `String` conversion is infallible.
    pub fn render_to_string<'p, A>(
        iterator: impl Iterator<Item = RenderEvent<'p, A>>,
    ) -> Result<String, <Self as Renderer<A>>::Error>
    where
        Self: Renderer<A>,
    {
        Ok(String::try_from(Self::render(iterator)?).unwrap())
    }
}

/// Private implementation.
impl<W> PlaintextRenderer<W>
where
    W: std::io::Write,
{
    /// Receives a render event, discarding the annotation payload.
    fn receive<'p, A>(&mut self, event: RenderEvent<'p, A>) -> Result<(), RenderError> {
        match event {
            RenderEvent::Text(_, payload) => {
                self.inner.write_all(payload.as_bytes())?;
            }
            RenderEvent::Linebreak => {
                self.inner.write_all("\n".as_bytes())?;
            }
            RenderEvent::Padding(num) => {
                self.inner.write_all(" ".repeat(num).as_bytes())?;
            }
            RenderEvent::PushAnnotation(_) => (),
            RenderEvent::PopAnnotation => (),
            RenderEvent::Error(e) => return Err(RenderError::LayoutError(e)),
        }
        Ok(())
    }
    /// Finishes a render event stream by flushing self.
    fn finish(&mut self) -> Result<(), RenderError> {
        self.inner.flush()?;
        Ok(())
    }
}

impl<W> Renderer<()> for PlaintextRenderer<W>
where
    W: std::io::Write,
{
    type Finish = ();
    type Error = RenderError;
    fn receive<'p>(&mut self, event: RenderEvent<'p, ()>) -> Result<(), Self::Error> {
        self.receive(event)
    }
    fn finish(&mut self) -> Result<Self::Finish, Self::Error> {
        self.finish()
    }
}
impl<'a, W> Renderer<&'a ()> for PlaintextRenderer<W>
where
    W: std::io::Write,
{
    type Finish = ();
    type Error = RenderError;
    fn receive<'p>(&mut self, event: RenderEvent<'p, &'a ()>) -> Result<(), Self::Error> {
        self.receive(event)
    }
    fn finish(&mut self) -> Result<Self::Finish, Self::Error> {
        self.finish()
    }
}

#[cfg(feature = "termcolor")]
mod termcolor_renderer {
    use crate::{
        Renderer,
        renderer::{RenderError, RenderEvent},
    };
    use termcolor::{self, Color, ColorSpec, WriteColor};

    /// A patch to be applied to a [`termcolor::ColorSpec`](ColorSpec) object.
    /// Useful as an annotation type for a [`Document`](crate::document::Document)
    /// to be streamed to a [`termcolor::WriteColor`](WriteColor) implementor.
    ///
    /// ## Usage with [`termcolor::ColorSpec`](ColorSpec)
    ///
    /// [`ColorSpec`] is, by design, a total specification.
    /// This simplifies streaming but is contrary to the intuition that formatting options are cumulative;
    /// a bold annotation followed by an italic annotation should cause enclosed text to be bold *and* italic.
    ///
    /// Hence, this data type expresses cumulative semantics.
    /// You may consider these to be equivalent to the methods on [`ColorSpec`].
    ///
    /// ## Note on Fields
    ///
    /// All fields are [`Option<T>`], where `T` corresponds to the equivalent payload in [`ColorSpec`].
    /// `None` will leave a [`ColorSpec`] field unchanged,
    /// whereas `Some(value)` will override the existing field in the `ColorSpec` with `value`.
    ///
    /// ## Note on [`Default`]
    ///
    /// The default value for this type is a patch that does nothing; the identity patch that,
    /// when applied to a [`ColorSpec`], returns it unchanged. This results in more ergonomic usage patterns:
    /// ```rust
    /// # use groupnest::termcolor_renderer::ColorPatch;
    /// let bold_patch = ColorPatch {
    ///     bold: Some(true),
    ///     ..Default::default()
    /// };
    /// ```
    ///
    /// For the patch that is semantically equivalent to [`termcolor::ColorSpec::clear`],
    /// use [`ColorPatch::clear_all()`].
    /// 
    /// ## Note on [`serde`] Support
    /// 
    /// Due to containing [`termcolor::Color`], which does not support `serde` serialization and deserialization,
    /// this type does not implement `serde` support.
    #[derive(Debug, Clone, Default, PartialEq, Eq)]
    pub struct ColorPatch {
        /// Foreground color.
        /// - `None`: apply no color changes.
        /// - `Some(None)`: apply color change to `None`; clear color of given `ColorSpec`.
        /// - `Some(Some(color))`: apply color change to `Some(color)`.
        pub fg_color: Option<Option<Color>>,
        /// Background color.
        /// - `None`: apply no color changes.
        /// - `Some(None)`: apply color change to `None`; clear color of given `ColorSpec`.
        /// - `Some(Some(color))`: apply color change to `Some(color)`.
        pub bg_color: Option<Option<Color>>,

        pub bold: Option<bool>,
        pub italic: Option<bool>,
        pub underline: Option<bool>,
        pub strikethrough: Option<bool>,
        pub intense: Option<bool>,
        pub dimmed: Option<bool>,
        pub reset: Option<bool>,
    }

    impl ColorPatch {
        /// Creates a default patch, which applies no changes.
        pub fn new() -> Self {
            Self::default()
        }
        /// Creates a patch that clears all `ColorSpec` fields to default values.
        /// ```rust
        /// # use groupnest::ColorPatch;
        /// # use ::termcolor::ColorSpec;
        ///
        /// /// This function will never panic.
        /// fn invariant(colorspec: ColorSpec) {
        ///     assert_eq!(ColorPatch::clear_all().apply_to(colorspec), ColorSpec::default())
        /// }
        /// ```
        pub fn clear_all() -> Self {
            Self {
                fg_color: Some(None),
                bg_color: Some(None),
                bold: Some(false),
                italic: Some(false),
                underline: Some(false),
                strikethrough: Some(false),
                intense: Some(false),
                dimmed: Some(false),
                reset: Some(true),
            }
        }

        /// Consumes self, applying it onto a [`ColorSpec`].
        pub fn apply_to(self, mut to_modify: ColorSpec) -> ColorSpec {
            // As `termcolor::ColorSpec` doesn't expose its internals...
            if let Some(inner) = self.fg_color {
                to_modify.set_fg(inner);
            }
            if let Some(inner) = self.bg_color {
                to_modify.set_bg(inner);
            }

            if let Some(inner) = self.bold {
                to_modify.set_bold(inner);
            }
            if let Some(inner) = self.italic {
                to_modify.set_italic(inner);
            }
            if let Some(inner) = self.underline {
                to_modify.set_underline(inner);
            }
            if let Some(inner) = self.strikethrough {
                to_modify.set_strikethrough(inner);
            }

            if let Some(inner) = self.intense {
                to_modify.set_intense(inner);
            }
            if let Some(inner) = self.dimmed {
                to_modify.set_dimmed(inner);
            }
            if let Some(inner) = self.reset {
                to_modify.set_reset(inner);
            }

            to_modify
        }
    }

    /// A renderer that supports [`termcolor`] styling via [`Renderer<ColorPatch>`].
    /// 
    /// Layout errors are eagerly reported. It is assumed the underlyinng buffer does not have a width constraint.
    #[derive(Debug, Clone, Default)]
    #[non_exhaustive]
    pub struct TermcolorRenderer<W> {
        pub inner: W,
        colorstack: Vec<ColorSpec>,
    }
    impl<W> TermcolorRenderer<W>
    where
        W: WriteColor,
    {
        /// Creates a new [`TermcolorRenderer`], setting the `inner` stream to use the default color specification.
        ///
        /// Due to invoking [`WriteColor::set_color`], this function is fallible.
        pub fn new(mut inner: W) -> Result<Self, std::io::Error> {
            let first = ColorSpec::default();
            inner.reset()?;
            Ok(Self {
                inner,
                colorstack: vec![first],
            })
        }
        /// Creates a new [`TermcolorRenderer`], setting the `inner` stream to use the given `ColorSpec` specification.
        ///
        /// Due to invoking [`WriteColor::set_color`], this function is fallible.
        pub fn with_colorspec(mut inner: W, spec: ColorSpec) -> Result<Self, std::io::Error> {
            inner.set_color(&spec)?;
            Ok(Self {
                inner,
                colorstack: vec![spec],
            })
        }
    }

    impl<W> Renderer<ColorPatch> for TermcolorRenderer<W>
    where
        W: WriteColor,
    {
        type Finish = ();
        type Error = RenderError;
        fn receive<'p>(
            &mut self,
            event: crate::renderer::RenderEvent<'p, ColorPatch>,
        ) -> Result<(), Self::Error> {
            match event {
                RenderEvent::Text(_, payload) => {
                    self.inner.write_all(payload.as_bytes())?;
                }
                RenderEvent::Linebreak => {
                    self.inner.write_all("\n".as_bytes())?;
                }
                RenderEvent::Padding(num) => {
                    self.inner.write_all(" ".repeat(num).as_bytes())?;
                }
                RenderEvent::PushAnnotation(patch) => {
                    // The default case shouldn't be encountered, ever, unless event stream is malformed.
                    // In which case we don't give out any guarantees anyways.
                    let enclosing_spec = self.colorstack.last().cloned().unwrap_or_default();
                    let new_spec = patch.apply_to(enclosing_spec);
                    self.inner.set_color(&new_spec)?;
                    self.colorstack.push(new_spec);
                }
                RenderEvent::PopAnnotation => {
                    self.colorstack.pop();
                }
                RenderEvent::Error(e) => return Err(RenderError::LayoutError(e)),
            }
            Ok(())
        }
        fn finish(&mut self) -> Result<(), Self::Error> {
            Ok(self.inner.flush()?)
        }
    }
}
#[cfg(feature = "termcolor")]
pub use termcolor_renderer::*;

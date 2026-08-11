//! Definition and configuration options for layout generation.
//!
//! Refer to [`LayoutEngine`] for further explanation.

use std::{collections::VecDeque, ops::Deref};

use crate::{
    document::{BreakStatus, Document, GroupPolicy, TextFragment},
    renderer::{LayoutError, RenderEvent},
};

/// The layout modes to be applied for notation nodes.
/// Used internally within [`LayoutEngine`] as well as [`LayoutSettings`] configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LayoutMode {
    /// Display components flat.
    Flat,
    /// Display components broken. Sub-components within may be displayed flat.
    Broken,
}

/// The constraint mode of the width of a layout as used by [`LayoutSettings`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LayoutWidthConstraint {
    /// The maximal width is treated as a best-effort soft constraint.
    /// No error is signalled if this width is exceeded and no breaking can occur.
    Relaxed,
    /// The maximal width is treated as a hard constraint.
    /// In the event this constraint is violated, a [`RenderEvent::Error`] is emitted at the **end** of the line.
    /// It is the responsibility of the receiving [`Renderer`](crate::Renderer) to handle errors.
    Strict,
}
/// The layout settings for `LayoutEngine`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LayoutSettings {
    /// The minimal width within a line in which groups must display flat,
    /// where width refers to the display width of the text in terminal columns,
    /// according to Unicode East Asian Width rules.
    ///
    /// This setting may cause width constraints to be exceeded,
    /// and hence is ignored if `WidthConstraint::Strict` is applied.
    pub min_width: usize,
    /// The maximal width within a line in which groups can display flat,
    /// where width refers to the display width of the text in terminal columns,
    /// according to Unicode East Asian Width rules.
    pub max_width: usize,
    /// The width constraint mode used.
    pub width_constraint: LayoutWidthConstraint,
    /// The initial layout mode used.
    pub initial_mode: LayoutMode,
}
/// The default rendering options, suited for IDE usage.
impl Default for LayoutSettings {
    fn default() -> Self {
        LayoutSettings {
            min_width: 20,
            max_width: 100,
            width_constraint: LayoutWidthConstraint::Relaxed,
            initial_mode: LayoutMode::Flat,
        }
    }
}

/// The layout frame internal to [`LayoutEngine`].
/// This is analagous to a call frame for a function call of `fn render(&document, indentation, mode)`
/// for the classical formulation of Wadler's layout algorithm.
#[derive(Debug, Clone)]
enum LayoutFrame<'s, 'doc, D, A> {
    Annotation,
    Nest(usize),
    CallFrame(CallFrame<'s, 'doc, D, A>),
}
#[derive(Debug, Clone)]
struct CallFrame<'s, 'doc, D, A> {
    pub indentation: usize,
    pub mode: LayoutMode,
    pub document: &'doc Document<'s, D, A>,
    pub force_flat: bool,
}

/// The layout engine, which borrows a Wadler-style notation document and functions as an iterator over resultant render events.
///
/// Parameterized over:
/// - `'s`: the lifetime of owned or borrowed string fragments, which are of type `Cow<'s, str>`.
/// - `'doc`: the lifetime of the document. `'s` **must** outlive `'doc`, and this would be the case for well-formed documents.
/// - `D`: the type of children. This should be a fixed-point reference or smart pointer, wrapped in a Rust newtype.
/// - `A`: the type of annotations. Defaults to unit type `()`.
///
/// # Note on [`Iterator::Item`]
///
/// Due to accessing the `'s`-valid string fragments *through* document references of lifetime `'doc`, and `'s : 'doc`,
/// the item returned must be of the shorter lifetime, hence `'doc`.
///
/// Refer to [`Renderer`](crate::Renderer) "Note on Lifetime `'payload`" for more information.
#[derive(Debug, Clone)]
pub struct LayoutEngine<'s, 'doc, D, A>
where
    D: Deref<Target = Document<'s, D, A>>,
    's: 'doc,
{
    /// FIFO queue of pending events, to be emitted. Annotations are borrowed.
    pending: VecDeque<RenderEvent<'doc, &'doc A>>,
    /// Cached padding. These are batched and accumulated separately,
    /// only released upon a subsequent `Text` event.
    /// This means that [`RenderEvent::Padding`] events in the `pending` field carry the semantic meaning of
    /// "add to the `pending_padding` field"
    pending_padding: usize,
    /// Internal layout state. This is analogous to a LIFO stack of call frames.
    state: Vec<LayoutFrame<'s, 'doc, D, A>>,
    /// (line, col) cursor (zero-indexed) tracking the position of a cursor from emitted events.
    cursor: (usize, usize),
    /// Layout settings.
    settings: LayoutSettings,
}

impl<'s, 'doc, D, A> LayoutEngine<'s, 'doc, D, A>
where
    D: Deref<Target = Document<'s, D, A>>,
    's: 'doc,
{
    /// Creates a document with the given document, using default layout settings.
    pub fn new(document: &'doc Document<'s, D, A>) -> Self
    where
        Self: 'doc,
    {
        let settings = LayoutSettings::default();
        Self::with_settings(document, settings)
    }
    /// Creates a document with the given document and layout settings.
    pub fn with_settings(document: &'doc Document<'s, D, A>, settings: LayoutSettings) -> Self
    where
        Self: 'doc,
    {
        Self {
            pending: VecDeque::new(),
            state: vec![LayoutFrame::CallFrame(CallFrame {
                indentation: 0,
                mode: settings.initial_mode,
                document,
                force_flat: false,
            })],
            pending_padding: 0,
            cursor: (0, 0),
            settings,
        }
    }

    /// Emits the render event, updating the internal cursor as it does so.
    /// - May cause addition of pending events.
    /// - The returned event is not guaranteed to be the same event as the input,
    ///   and may cause further calls to [`LayoutEngine::next_event`].
    /// - [`LayoutEngine`] is free to reorganize or omit [`RenderEvent::Padding`] to strip unecessary whitespace padding.
    /// - `RenderEvent::Text(0, "")` is eliminated, and does not cause emission of text.
    fn emit(&mut self, next: RenderEvent<'doc, &'doc A>) -> Option<RenderEvent<'doc, &'doc A>> {
        match &next {
            RenderEvent::Text(span, _) => {
                if *span == 0 {
                    // The empty string. DO NOT actualize emission. Get next event.
                    return self.next_event();
                }
                // `Padding` is only actualized here.
                if self.pending_padding > 0 {
                    let padding = self.pending_padding;
                    self.pending_padding = 0;
                    let padding_event: RenderEvent<'s, &'doc A> = RenderEvent::Padding(padding);
                    self.pending.push_front(next);
                    self.cursor.1 += padding;
                    return Some(padding_event);
                } else {
                    self.cursor.1 += *span;
                }
            }
            RenderEvent::Padding(padding) => {
                // For the `pending` event cache, this results in batching.
                self.pending_padding += *padding;
                return self.next_event();
            }
            RenderEvent::Linebreak => {
                self.pending_padding = 0;
                let line_width = self.cursor.1;
                if line_width > self.settings.max_width
                    && self.settings.width_constraint == LayoutWidthConstraint::Strict
                {
                    self.pending
                        .push_front(RenderEvent::Error(LayoutError::WidthExceeded {
                            line_num: self.cursor.0 + 1,
                            max_width: self.settings.max_width,
                            line_width,
                        }));
                }
                self.cursor.0 += 1;
                self.cursor.1 = 0;
            }
            RenderEvent::PushAnnotation(_) | RenderEvent::PopAnnotation => (),
            RenderEvent::Error(_) => (),
        }
        // This is the default, and only `Padding` events may change this.
        Some(next)
    }

    /// Determines for `GroupPolicy::Normal` whether this group should display flat or broken.
    fn determine_mode(
        &self,
        child: &Document<'s, D, A>,
        callframe: &CallFrame<'s, 'doc, D, A>,
    ) -> LayoutMode {
        let status = child.break_status();
        if callframe.force_flat {
            // Invariant: when force_flat is flipped, it is asserted that this group contains no forced breaks.
            return LayoutMode::Flat;
        }

        // Relaxed min-width checking.
        if self.settings.width_constraint == LayoutWidthConstraint::Relaxed
            && matches!(status, BreakStatus::FlatLength(span) if span <= self.settings.min_width)
        {
            return LayoutMode::Flat;
        }

        // Normal fits checking and bounds-checking.
        if matches!(status, BreakStatus::FlatLength(span)
            if span + callframe.indentation <= self.settings.max_width
        ) {
            LayoutMode::Flat
        } else {
            LayoutMode::Broken
        }
    }

    /// Computes the next render event by mutating internals.
    fn next_event(&mut self) -> Option<RenderEvent<'doc, &'doc A>> {
        if let Some(next) = self.pending.pop_front() {
            return self.emit(next);
        }
        let frame = self.state.pop()?;
        let callframe = match frame {
            LayoutFrame::CallFrame(inner) => inner,
            LayoutFrame::Annotation => {
                return Some(RenderEvent::PopAnnotation);
            }
            LayoutFrame::Nest(dedent) => {
                let next_event = self.next_event();
                if let Some(RenderEvent::Padding(current)) = next_event {
                    return Some(RenderEvent::Padding(current - dedent));
                } else {
                    return next_event;
                }
            }
        };

        match callframe.document {
            Document::Nil => {
                // discard this frame, get next.
                self.next_event()
            }
            Document::Text(fragment) => {
                let event: RenderEvent<'doc, &'doc A> =
                    RenderEvent::Text(fragment.unicode_width(), fragment.inner());
                self.emit(event)
            }
            Document::Break(breaker) => {
                if callframe.mode == LayoutMode::Flat {
                    self.pending.push_back(RenderEvent::Text(
                        breaker.flat().unicode_width(),
                        breaker.flat().inner(),
                    ));
                } else {
                    self.pending
                        .extend(breaker.broken().iter().flat_map(|elem| match elem {
                            TextFragment::Text(fragment) => {
                                vec![RenderEvent::Text(
                                    fragment.unicode_width(),
                                    fragment.inner(),
                                )]
                            }
                            TextFragment::Linebreak => vec![
                                RenderEvent::Linebreak,
                                RenderEvent::Padding(callframe.indentation),
                            ],
                        }));
                }
                self.next_event()
            }
            Document::HardLinebreak => {
                self.pending.extend(vec![
                    RenderEvent::Linebreak,
                    RenderEvent::Padding(callframe.indentation),
                ]);
                self.next_event()
            }

            Document::Group(policy, child) => {
                // Generally, all children must be supplied in reverse order due to LIFO.
                match policy {
                    GroupPolicy::Normal => {
                        let mode = self.determine_mode(child, &callframe);
                        self.state.push(LayoutFrame::CallFrame(CallFrame {
                            mode,
                            document: child,
                            ..callframe
                        }));
                    }
                    GroupPolicy::FlatIfPossible => {
                        let child_can_flat = child.break_status() != BreakStatus::MustBreak;
                        self.state.push(LayoutFrame::CallFrame(CallFrame {
                            mode: if child_can_flat {
                                LayoutMode::Flat
                            } else {
                                LayoutMode::Broken
                            },
                            document: child,
                            force_flat: child_can_flat,
                            ..callframe
                        }));
                    }
                    GroupPolicy::ForceBreak => {
                        self.state.push(LayoutFrame::CallFrame(CallFrame {
                            mode: LayoutMode::Broken,
                            document: child,
                            ..callframe
                        }));
                    }
                }
                self.next_event()
            }

            Document::Sequence(sequence) => {
                self.state
                    .extend(sequence.children().iter().rev().map(|child| {
                        LayoutFrame::CallFrame(CallFrame {
                            document: child,
                            ..callframe
                        })
                    }));
                self.next_event()
            }

            Document::Nest(indent, inner) => {
                if callframe.mode == LayoutMode::Broken {
                    self.state.push(LayoutFrame::Nest(*indent));
                    self.state.push(LayoutFrame::CallFrame(CallFrame {
                        document: inner,
                        indentation: callframe.indentation + *indent,
                        ..callframe
                    }));
                    // If we are at the start of a newline (cursor at column 0 with pending padding)
                    // Add to pending padding.
                    if self.cursor.1 == 0 {
                        self.pending_padding += *indent;
                    }
                } else {
                    // Do not add any indentation.
                    self.state.push(LayoutFrame::CallFrame(CallFrame {
                        document: inner,
                        ..callframe
                    }));
                }
                self.next_event()
            }

            Document::Annotation(annotation, inner) => {
                self.state.push(LayoutFrame::Annotation);
                self.state.push(LayoutFrame::CallFrame(CallFrame {
                    document: inner,
                    ..callframe
                }));
                self.emit(RenderEvent::PushAnnotation(annotation))
            }
        }
    }
}

impl<'s, 'doc, D, A> Iterator for LayoutEngine<'s, 'doc, D, A>
where
    D: Deref<Target = Document<'s, D, A>>,
{
    type Item = RenderEvent<'doc, &'doc A>;
    fn next(&mut self) -> Option<Self::Item> {
        self.next_event()
    }
}

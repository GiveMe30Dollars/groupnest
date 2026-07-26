use std::{collections::VecDeque, ops::Deref};

use crate::{
    document::{BreakStatus, Document, Group, GroupPolicy, TextFragment},
    render::{RenderError, RenderEvent},
};

/// The layout modes to be applied for notation nodes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LayoutMode {
    /// Display components flat.
    Flat,
    /// Display components broken. Sub-components within may be displayed flat.
    Broken,
}

/// The constraint mode of the width of a layout as used by `LayoutSettings`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WidthConstraint {
    /// The maximal width is treated as a best-effort soft constraint.
    /// No error is signalled if this width is exceeded and no breaking can occur.
    Relaxed,
    /// The maximal width is treated as a hard constraint.
    /// In the event this constraint is violated, a `RenderEvent::Error` is emitted at the end of line.
    /// It is the responsibility of the receiving Renderer to handle any errors.
    Strict,
}
/// The layout settings for `LayoutEngine`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LayoutSettings {
    /// The minimal width within a line in which groups must display flat.
    ///
    /// This setting may cause width constraints to be exceeded,
    /// and hence is ignored if `WidthConstraint::Strict` is applied.
    pub min_width: usize,
    /// The maximal width within a line in which groups can display flat.
    pub max_width: usize,
    /// The width constraint mode used.
    pub width_constraint: WidthConstraint,
    /// The initial layout mode used.
    pub initial_mode: LayoutMode,
}
/// The default rendering options, suited for IDE usage.
impl Default for LayoutSettings {
    fn default() -> Self {
        LayoutSettings {
            min_width: 20,
            max_width: 100,
            width_constraint: WidthConstraint::Relaxed,
            initial_mode: LayoutMode::Flat,
        }
    }
}

/// The layout frame internal to `LayoutEngine`.
/// This is analagous to a call frame for a function call of `fn render(&document, indentation, mode)`.
#[derive(Debug, Clone)]
enum LayoutFrame<'s, D, A> {
    Annotation,
    CallFrame(CallFrame<'s, D, A>),
}
#[derive(Debug, Clone)]
struct CallFrame<'s, D, A> {
    pub indentation: usize,
    pub mode: LayoutMode,
    pub document: &'s Document<'s, D, A>,
    pub force_flat: bool,
}

/// The layout engine, which borrows a notation document and functions as an iterator over resultant render events.
#[derive(Debug, Clone)]
pub struct LayoutEngine<'s, D, A>
where
    D: Deref<Target = Document<'s, D, A>>,
{
    /// The root of the document.
    document: &'s Document<'s, D, A>,
    /// FIFO queue of pending events, to be emitted. Annotations are borrowed.
    pending: VecDeque<RenderEvent<'s, &'s A>>,
    /// Internal layout state. This is analogous to a LIFO stack of call frames.
    state: Vec<LayoutFrame<'s, D, A>>,
    /// (line, col) cursor (zero-indexed) tracking the position of a cursor from emitted events.
    cursor: (usize, usize),
    /// Layout settings.
    settings: LayoutSettings,
}

impl<'s, D, A> LayoutEngine<'s, D, A>
where
    D: Deref<Target = Document<'s, D, A>>,
{
    /// Creates a document with the given document, using default layout settings.
    pub fn new(document: &'s Document<'s, D, A>) -> Self {
        let settings = LayoutSettings::default();
        Self::with_settings(document, settings)
    }
    /// Creates a document with the given document and layout settings.
    pub fn with_settings(document: &'s Document<'s, D, A>, settings: LayoutSettings) -> Self {
        Self {
            pending: VecDeque::new(),
            state: vec![LayoutFrame::CallFrame(CallFrame {
                indentation: 0,
                mode: settings.initial_mode,
                document,
                force_flat: false,
            })],
            cursor: (0, 0),
            document,
            settings,
        }
    }

    /// Emits the render event, updating the internal cursor as it does so.
    /// May cause addition of pending events.
    fn emit(&mut self, next: RenderEvent<'s, &'s A>) -> RenderEvent<'s, &'s A> {
        match &next {
            RenderEvent::Text(span, _) | RenderEvent::Padding(span) => {
                self.cursor.1 += *span;
            }
            RenderEvent::Linebreak => {
                let line_width = self.cursor.1;
                if line_width > self.settings.max_width {
                    self.pending
                        .push_front(RenderEvent::Error(RenderError::WidthExceeded {
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
        next
    }

    /// Determines for `GroupPolicy::Default` whether this group should display flat or broken.
    fn determine_mode(
        &self,
        group: &Group<D>,
        callframe: &CallFrame<'s, D, A>,
    ) -> LayoutMode {
        if callframe.force_flat {
            // Invariant: when force_flat is flipped, it is asserted that this group contains no forced breaks.
            return LayoutMode::Flat;
        }

        // Relaxed min-width checking.
        if self.settings.width_constraint == WidthConstraint::Relaxed
            && matches!(group.status, BreakStatus::FlatLength(span) if span <= self.settings.min_width)
        {
            return LayoutMode::Flat;
        }

        // Normal fits checking and bounds-checking.
        if matches!(group.status, BreakStatus::FlatLength(span)
            if span + callframe.indentation <= self.settings.max_width
        ) {
            LayoutMode::Flat
        } else {
            LayoutMode::Broken
        }
    }

    /// Computes the next render event by mutating internals.
    fn next_event(&mut self) -> Option<RenderEvent<'s, &'s A>> {
        if let Some(next) = self.pending.pop_front() {
            return Some(self.emit(next));
        }
        let frame = self.state.pop()?;
        let LayoutFrame::CallFrame(callframe) = frame else {
            return Some(RenderEvent::PopAnnotation);
        };
        match callframe.document {
            Document::Nil => {
                // discard this frame, get next.
                self.next_event()
            }
            Document::Text(fragment) => {
                let event = RenderEvent::Text(fragment.width, fragment.inner());
                Some(self.emit(event))
            }
            Document::Break(breaker) => {
                if callframe.mode == LayoutMode::Flat {
                    self.pending.push_back(RenderEvent::Text(
                        breaker.flat().width,
                        breaker.flat().inner(),
                    ));
                } else {
                    self.pending
                        .extend(breaker.broken().iter().flat_map(|elem| match elem {
                            TextFragment::Text(fragment) => {
                                vec![RenderEvent::Text(fragment.width, fragment.inner())]
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

            Document::Group(group) => {
                // Generally, all children must be supplied in reverse order due to LIFO.
                match group.policy() {
                    GroupPolicy::Default => {
                        let mode = self.determine_mode(group, &callframe);
                        self.push_group_children(group, |child| {
                            LayoutFrame::CallFrame(CallFrame {
                                mode,
                                document: child,
                                ..callframe
                            })
                        });
                    }
                    GroupPolicy::ForceFlat => {
                        self.push_group_children(group, |child| {
                            LayoutFrame::CallFrame(CallFrame {
                                mode: if group.status != BreakStatus::MustBreak {
                                    LayoutMode::Flat
                                } else { LayoutMode::Broken },
                                document: child,
                                force_flat: group.status != BreakStatus::MustBreak,
                                ..callframe
                            })
                        });
                    }
                    GroupPolicy::ForceBreak => {
                        self.push_group_children(group, |child| {
                            LayoutFrame::CallFrame(CallFrame {
                                mode: LayoutMode::Broken,
                                document: child,
                                ..callframe
                            })
                        });
                    }
                }
                self.next_event()
            }
            Document::Nest(nest, inner) => {
                self.state.push(LayoutFrame::CallFrame(CallFrame {
                    document: inner,
                    indentation: callframe.indentation + *nest,
                    ..callframe
                }));
                self.next_event()
            }

            Document::Annotation(annotation, inner) => {
                self.state.push(LayoutFrame::Annotation);
                self.state.push(LayoutFrame::CallFrame(CallFrame {
                    document: inner,
                    ..callframe
                }));
                Some(self.emit(RenderEvent::PushAnnotation(annotation)))
            }
        }
    }

    /// Helper function for `next_event`: pushes all group children to the state stack.
    fn push_group_children<F>(&mut self, group: &'s Group<D>, callback: F)
    where
        F: Fn(&'s D) -> LayoutFrame<'s, D, A>,
    {
        self.state
            .extend(group.children().iter().rev().map(callback));
    }
}

impl<'s, D, A> Iterator for LayoutEngine<'s, D, A>
where
    D: Deref<Target = Document<'s, D, A>>,
{
    type Item = RenderEvent<'s, &'s A>;
    fn next(&mut self) -> Option<Self::Item> {
        self.next_event()
    }
}

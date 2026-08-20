//! This module adds fuzzing implementations for `BoxDoc` and `LayoutSettings`.

use groupnest::{
    BoxDoc, GroupPolicy,
    layout::{LayoutMode, LayoutSettings, LayoutWidthConstraint},
};
use proptest::prelude::*;

pub fn arbitrary_boxdoc_leaf() -> impl Strategy<Value = BoxDoc<()>> {
    prop_oneof![
        Just(BoxDoc::nil()),
        Just(BoxDoc::hard_linebreak()),
        "[a-zA-Z0-9]{0,30}".prop_map(BoxDoc::flat_text),
        (
            "[a-zA-Z0-9]{0,30}",
            "[a-zA-Z0-9\n]{0,30}\n[a-zA-Z0-9]{0,30}"
        )
            .prop_map(|(flat, broken)| BoxDoc::breaker(flat, broken))
    ]
}

pub fn arbitrary_boxdoc() -> impl Strategy<Value = BoxDoc<()>> {
    let leaf = arbitrary_boxdoc_leaf();

    leaf.prop_recursive(8, 256, 10, |inner| {
        prop_oneof![
            // Take the inner strategy and make the two recursive cases.
            prop::collection::vec(inner.clone(), 0..10).prop_map(BoxDoc::sequence),
            (
                prop_oneof![
                    Just(GroupPolicy::Normal),
                    Just(GroupPolicy::ForceBreak),
                    Just(GroupPolicy::FlatIfPossible),
                ],
                inner.clone()
            )
                .prop_map(|(policy, child)| BoxDoc::group_with(policy, child)),
            (0..50, inner.clone())
                .prop_map(|(indentation, inner)| BoxDoc::nest(indentation as usize, inner)),
            inner
                .clone()
                .prop_map(|inner| BoxDoc::annotation((), inner))
        ]
    })
}

pub fn arbitrary_settings() -> impl Strategy<Value = LayoutSettings> {
    (
        0..50,
        0..100,
        prop_oneof![
            Just(LayoutWidthConstraint::Relaxed),
            Just(LayoutWidthConstraint::Strict),
        ],
        prop_oneof![Just(LayoutMode::Flat), Just(LayoutMode::Broken),],
    )
        .prop_map(
            |(min_width, max_width, width_constraint, initial_mode)| LayoutSettings {
                min_width: min_width as usize,
                max_width: max_width as usize,
                width_constraint,
                initial_mode,
            },
        )
}

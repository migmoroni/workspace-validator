//! Declarative presentation modes independent from concrete terminal hues.

use super::{
    style::{StyleModifier, BOLD, BRIGHT, DIM, UNDERLINE},
    PresentationProfile, ROLE_COUNT,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct PresentationMetrics {
    pub(super) progress_width: usize,
    pub(super) section_gap: usize,
    pub(super) item_gap: usize,
    pub(super) execution_gap: usize,
}

pub(super) struct PresentationSpec {
    pub(super) modifiers: [StyleModifier; ROLE_COUNT],
    pub(super) metrics: PresentationMetrics,
}

pub(super) fn specification(profile: PresentationProfile) -> &'static PresentationSpec {
    match profile {
        PresentationProfile::Standard => &STANDARD,
        PresentationProfile::LowVision => &LOW_VISION,
    }
}

const N: StyleModifier = StyleModifier::none();
const fn m(remove: u8, add: u8) -> StyleModifier {
    StyleModifier::new(remove, add)
}

const REGULAR: PresentationMetrics = PresentationMetrics {
    progress_width: 14,
    section_gap: 0,
    item_gap: 0,
    execution_gap: 0,
};

const STANDARD: PresentationSpec = PresentationSpec {
    modifiers: [N; ROLE_COUNT],
    metrics: REGULAR,
};

const LOW_VISION: PresentationSpec = PresentationSpec {
    modifiers: [
        m(DIM, BOLD | BRIGHT),
        m(DIM, BOLD | UNDERLINE | BRIGHT),
        m(DIM, BOLD),
        m(DIM, BOLD),
        m(DIM, BRIGHT),
        m(DIM, UNDERLINE | BRIGHT),
        m(DIM, BOLD | UNDERLINE),
        m(DIM, BOLD | BRIGHT),
        m(DIM, BOLD | BRIGHT),
        m(DIM, BOLD | BRIGHT),
        m(DIM, BOLD | BRIGHT),
        m(DIM, BOLD | BRIGHT),
        m(DIM, BOLD | BRIGHT),
        m(DIM, BOLD | UNDERLINE | BRIGHT),
        m(DIM, BOLD | UNDERLINE | BRIGHT),
        m(DIM, BOLD | UNDERLINE),
        m(DIM, BOLD | UNDERLINE),
        m(DIM, BRIGHT),
        m(DIM, BOLD | BRIGHT),
        m(DIM, BOLD | BRIGHT),
        m(DIM, BRIGHT),
    ],
    metrics: PresentationMetrics {
        progress_width: 22,
        section_gap: 2,
        item_gap: 1,
        execution_gap: 2,
    },
};

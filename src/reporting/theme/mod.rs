//! Composable, accessible visual themes for human reporting.

mod palette;
mod presentation;
mod style;

use presentation::PresentationMetrics;
use style::StyleSpec;

const FRAME_WIDTH: usize = 78;
const ROLE_COUNT: usize = Role::ALL.len();

/// Color palette selected independently from report presentation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PaletteProfile {
    /// Terminal foreground and typographic presentation only.
    Plain,
    /// Familiar terminal colors.
    Standard,
    /// Bright terminal colors selected for strong visual separation.
    HighContrast,
    /// Palette avoiding red-green distinctions for reduced red perception.
    Protanopia,
    /// Palette avoiding red-green distinctions for reduced green perception.
    Deuteranopia,
    /// Palette avoiding documented blue-green, purple-red, and yellow-pink pairs.
    Tritanopia,
    /// Non-chromatic palette using typographic attributes.
    Achromatopsia,
}

#[cfg(test)]
impl PaletteProfile {
    pub(super) const ALL: [Self; 7] = [
        Self::Plain,
        Self::Standard,
        Self::HighContrast,
        Self::Protanopia,
        Self::Deuteranopia,
        Self::Tritanopia,
        Self::Achromatopsia,
    ];
}

/// Layout and emphasis selected independently from color.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PresentationProfile {
    /// Regular density and emphasis.
    Standard,
    /// Expanded spacing, progress width, and short-label emphasis.
    LowVision,
}

#[cfg(test)]
impl PresentationProfile {
    pub(super) const ALL: [Self; 2] = [Self::Standard, Self::LowVision];
}

/// A local, immutable combination of one palette and one presentation.
#[derive(Clone)]
pub struct Theme {
    palette_profile: PaletteProfile,
    presentation_profile: PresentationProfile,
    styles: [StyleSpec; ROLE_COUNT],
    metrics: PresentationMetrics,
}

impl Theme {
    /// Creates the default theme with no ANSI output and regular density.
    pub fn plain() -> Self {
        Self::resolve(PaletteProfile::Plain, PresentationProfile::Standard)
    }

    /// Resolves palette styles and presentation modifiers deterministically.
    pub fn resolve(
        palette_profile: PaletteProfile,
        presentation_profile: PresentationProfile,
    ) -> Self {
        let palette = palette::specification(palette_profile);
        let presentation = presentation::specification(presentation_profile);
        let styles =
            std::array::from_fn(|index| palette.styles[index].merge(presentation.modifiers[index]));
        Self {
            palette_profile,
            presentation_profile,
            styles,
            metrics: presentation.metrics,
        }
    }

    /// Returns the palette used by this resolved theme.
    pub fn palette_profile(&self) -> PaletteProfile {
        self.palette_profile
    }

    /// Returns the presentation used by this resolved theme.
    pub fn presentation_profile(&self) -> PresentationProfile {
        self.presentation_profile
    }

    pub(super) fn paint(&self, role: Role, text: impl std::fmt::Display) -> String {
        let style = self.styles[role as usize];
        if style == StyleSpec::plain() {
            text.to_string()
        } else {
            style.console_style().apply_to(text).to_string()
        }
    }

    pub(super) fn indicatif_modifier(&self, role: Role) -> String {
        self.styles[role as usize].indicatif_modifier()
    }

    pub(super) fn progress_width(&self) -> usize {
        self.metrics.progress_width
    }

    pub(super) fn section_gap(&self) -> usize {
        self.metrics.section_gap
    }

    pub(super) fn item_gap(&self) -> usize {
        self.metrics.item_gap
    }

    pub(super) fn execution_gap(&self) -> usize {
        self.metrics.execution_gap
    }

    pub(super) fn frame_rule(&self, edge: char, label: &str, label_role: Role) -> String {
        let leading = format!("{edge}─ ");
        let occupied = leading.chars().count() + label.chars().count() + 1;
        let trailing = format!(" {}", "─".repeat(FRAME_WIDTH.saturating_sub(occupied)));
        format!(
            "{}{}{}",
            self.paint(Role::Border, leading),
            self.paint(label_role, label),
            self.paint(Role::Border, trailing)
        )
    }
}

#[derive(Clone, Copy, Debug)]
#[repr(usize)]
pub(super) enum Role {
    Heading,
    Section,
    Border,
    Metadata,
    Duration,
    Path,
    Shared,
    Tool,
    Group,
    Suite,
    Check,
    Gate,
    Success,
    Failure,
    Blocked,
    Skipped,
    DiagnosticLabel,
    DiagnosticContent,
    ActiveSpinner,
    ProgressComplete,
    ProgressPending,
}

impl Role {
    const ALL: [Self; 21] = [
        Self::Heading,
        Self::Section,
        Self::Border,
        Self::Metadata,
        Self::Duration,
        Self::Path,
        Self::Shared,
        Self::Tool,
        Self::Group,
        Self::Suite,
        Self::Check,
        Self::Gate,
        Self::Success,
        Self::Failure,
        Self::Blocked,
        Self::Skipped,
        Self::DiagnosticLabel,
        Self::DiagnosticContent,
        Self::ActiveSpinner,
        Self::ProgressComplete,
        Self::ProgressPending,
    ];
}

#[cfg(test)]
mod tests {
    use super::style::DIM;
    use super::*;
    use console::{measure_text_width, strip_ansi_codes};

    #[test]
    fn all_fourteen_combinations_resolve_every_role_without_leaking() {
        for palette in PaletteProfile::ALL {
            for presentation in PresentationProfile::ALL {
                let first = Theme::resolve(palette, presentation);
                let second = Theme::resolve(palette, presentation);
                assert_eq!(first.styles, second.styles);
                for role in Role::ALL {
                    assert_eq!(
                        strip_ansi_codes(&first.paint(role, "token")),
                        "token",
                        "{palette:?} + {presentation:?} + {role:?}"
                    );
                }
            }
        }
        let colored = Theme::resolve(PaletteProfile::Standard, PresentationProfile::LowVision);
        assert!(colored.paint(Role::Failure, "FAIL").contains('\u{1b}'));
        assert_eq!(Theme::plain().paint(Role::Failure, "FAIL"), "FAIL");
    }

    #[test]
    fn plain_standard_is_ansi_free_and_plain_presentations_never_add_hue() {
        for role in Role::ALL {
            assert!(!Theme::plain().paint(role, "token").contains('\u{1b}'));
        }
        let theme = Theme::resolve(PaletteProfile::Plain, PresentationProfile::LowVision);
        assert!(theme.styles.iter().all(|style| style.foreground.is_none()));
        assert!(theme.paint(Role::Heading, "heading").contains('\u{1b}'));
    }

    #[test]
    fn low_vision_removes_dim_after_every_palette() {
        for palette in PaletteProfile::ALL {
            let theme = Theme::resolve(palette, PresentationProfile::LowVision);
            assert!(theme.styles.iter().all(|style| style.attributes & DIM == 0));
        }
    }

    #[test]
    fn palettes_never_change_presentation_metrics() {
        for presentation in PresentationProfile::ALL {
            let expected = Theme::resolve(PaletteProfile::Plain, presentation).metrics;
            for palette in PaletteProfile::ALL {
                assert_eq!(Theme::resolve(palette, presentation).metrics, expected);
            }
        }
        assert_eq!(Theme::plain().progress_width(), 14);
        assert_eq!(
            Theme::resolve(PaletteProfile::Plain, PresentationProfile::LowVision).progress_width(),
            22
        );
        let low_vision = Theme::resolve(PaletteProfile::Plain, PresentationProfile::LowVision);
        assert_eq!(low_vision.section_gap(), 2);
        assert_eq!(low_vision.item_gap(), 1);
        assert_eq!(low_vision.execution_gap(), 2);
        assert_eq!(Theme::plain().item_gap(), 0);
    }

    #[test]
    fn presentations_preserve_each_palette_structural_color() {
        for palette in PaletteProfile::ALL {
            let specification = palette::specification(palette);
            for presentation in PresentationProfile::ALL {
                let theme = Theme::resolve(palette, presentation);
                assert_eq!(
                    theme.styles[Role::Border as usize].foreground,
                    specification.styles[Role::Border as usize].foreground
                );
            }
        }
    }

    #[test]
    fn frame_width_is_stable_for_all_combinations() {
        for palette in PaletteProfile::ALL {
            for presentation in PresentationProfile::ALL {
                let theme = Theme::resolve(palette, presentation);
                let line = theme.frame_rule('├', "Tools", Role::Section);
                assert_eq!(measure_text_width(&line), FRAME_WIDTH);
                assert_eq!(strip_ansi_codes(&line).chars().count(), FRAME_WIDTH);
                assert!(!line.contains('\n'));
            }
        }
    }
}

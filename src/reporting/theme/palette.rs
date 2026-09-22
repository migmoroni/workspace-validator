//! Declarative color palettes for semantic reporting roles.

use super::{
    style::{StyleSpec, TerminalColor, BOLD, BRIGHT, DIM, REVERSE, UNDERLINE},
    PaletteProfile, ROLE_COUNT,
};

#[derive(Debug)]
pub(super) struct PaletteSpec {
    pub(super) styles: [StyleSpec; ROLE_COUNT],
}

pub(super) fn specification(profile: PaletteProfile) -> &'static PaletteSpec {
    match profile {
        PaletteProfile::Plain => &PLAIN,
        PaletteProfile::Standard => &STANDARD,
        PaletteProfile::HighContrast => &HIGH_CONTRAST,
        PaletteProfile::Protanopia => &PROTANOPIA,
        PaletteProfile::Deuteranopia => &DEUTERANOPIA,
        PaletteProfile::Tritanopia => &TRITANOPIA,
        PaletteProfile::Achromatopsia => &ACHROMATOPSIA,
    }
}

const P: StyleSpec = StyleSpec::plain();
const fn c(color: TerminalColor, attributes: u8) -> StyleSpec {
    StyleSpec::color(color, attributes)
}
const fn a(attributes: u8) -> StyleSpec {
    StyleSpec::attributes(attributes)
}

// Entries follow `Role::ALL`, so every palette decides every semantic role.
const PLAIN: PaletteSpec = PaletteSpec {
    styles: [P; ROLE_COUNT],
};

const STANDARD: PaletteSpec = PaletteSpec {
    styles: [
        c(TerminalColor::White, BOLD),
        c(TerminalColor::Cyan, BOLD),
        P,
        c(TerminalColor::Cyan, 0),
        c(TerminalColor::White, DIM),
        c(TerminalColor::Blue, 0),
        c(TerminalColor::Magenta, UNDERLINE),
        c(TerminalColor::Cyan, BOLD),
        c(TerminalColor::Blue, BOLD),
        c(TerminalColor::Magenta, BOLD),
        c(TerminalColor::Cyan, BOLD),
        c(TerminalColor::Yellow, BOLD),
        c(TerminalColor::Green, BOLD),
        c(TerminalColor::Red, BOLD | UNDERLINE),
        c(TerminalColor::Yellow, BOLD),
        c(TerminalColor::Yellow, UNDERLINE),
        c(TerminalColor::Red, BOLD),
        c(TerminalColor::White, 0),
        c(TerminalColor::Cyan, BOLD),
        c(TerminalColor::Green, BOLD),
        c(TerminalColor::Blue, DIM),
    ],
};

const HIGH_CONTRAST: PaletteSpec = PaletteSpec {
    styles: [
        c(TerminalColor::White, BOLD | BRIGHT),
        c(TerminalColor::Cyan, BOLD | BRIGHT | UNDERLINE),
        c(TerminalColor::Cyan, BRIGHT),
        c(TerminalColor::White, BOLD),
        c(TerminalColor::White, BRIGHT),
        c(TerminalColor::Cyan, BRIGHT | UNDERLINE),
        c(TerminalColor::Magenta, BOLD | UNDERLINE),
        c(TerminalColor::Cyan, BOLD | BRIGHT),
        c(TerminalColor::White, BOLD | BRIGHT),
        c(TerminalColor::Magenta, BOLD | BRIGHT),
        c(TerminalColor::Cyan, BOLD | BRIGHT),
        c(TerminalColor::Yellow, BOLD | BRIGHT),
        c(TerminalColor::White, BOLD | BRIGHT),
        c(TerminalColor::Yellow, BOLD | REVERSE),
        c(TerminalColor::Cyan, BOLD | UNDERLINE),
        c(TerminalColor::White, UNDERLINE),
        c(TerminalColor::Yellow, BOLD | UNDERLINE),
        c(TerminalColor::White, BRIGHT),
        c(TerminalColor::Cyan, BOLD | BRIGHT),
        c(TerminalColor::White, BOLD | BRIGHT),
        c(TerminalColor::White, BRIGHT),
    ],
};

const PROTANOPIA: PaletteSpec = PaletteSpec {
    styles: [
        c(TerminalColor::White, BOLD),
        c(TerminalColor::Cyan, BOLD),
        P,
        c(TerminalColor::Cyan, 0),
        c(TerminalColor::White, DIM),
        c(TerminalColor::Blue, UNDERLINE),
        c(TerminalColor::Magenta, UNDERLINE),
        c(TerminalColor::Cyan, BOLD),
        c(TerminalColor::Blue, BOLD),
        c(TerminalColor::Magenta, BOLD),
        c(TerminalColor::Cyan, BOLD),
        c(TerminalColor::Yellow, BOLD),
        c(TerminalColor::Blue, BOLD),
        c(TerminalColor::Yellow, BOLD | REVERSE | UNDERLINE),
        c(TerminalColor::Magenta, BOLD),
        c(TerminalColor::Cyan, UNDERLINE),
        c(TerminalColor::Yellow, BOLD),
        c(TerminalColor::White, 0),
        c(TerminalColor::Cyan, BOLD),
        c(TerminalColor::Blue, BOLD),
        c(TerminalColor::White, DIM),
    ],
};

const DEUTERANOPIA: PaletteSpec = PaletteSpec {
    styles: [
        c(TerminalColor::White, BOLD),
        c(TerminalColor::Blue, BOLD),
        P,
        c(TerminalColor::Cyan, 0),
        c(TerminalColor::White, DIM),
        c(TerminalColor::Blue, UNDERLINE),
        c(TerminalColor::Magenta, UNDERLINE),
        c(TerminalColor::Cyan, BOLD),
        c(TerminalColor::Blue, BOLD),
        c(TerminalColor::Magenta, BOLD),
        c(TerminalColor::Cyan, BOLD),
        c(TerminalColor::Yellow, BOLD),
        c(TerminalColor::Cyan, BOLD),
        c(TerminalColor::Magenta, BOLD | REVERSE | UNDERLINE),
        c(TerminalColor::Yellow, BOLD),
        c(TerminalColor::Blue, UNDERLINE),
        c(TerminalColor::Magenta, BOLD),
        c(TerminalColor::White, 0),
        c(TerminalColor::Cyan, BOLD),
        c(TerminalColor::Cyan, BOLD),
        c(TerminalColor::White, DIM),
    ],
};

const TRITANOPIA: PaletteSpec = PaletteSpec {
    styles: [
        c(TerminalColor::White, BOLD),
        c(TerminalColor::White, BOLD | UNDERLINE),
        P,
        c(TerminalColor::White, 0),
        c(TerminalColor::White, DIM),
        c(TerminalColor::White, UNDERLINE),
        c(TerminalColor::White, REVERSE),
        c(TerminalColor::White, BOLD),
        c(TerminalColor::White, BOLD | UNDERLINE),
        c(TerminalColor::White, BOLD | REVERSE),
        c(TerminalColor::White, UNDERLINE),
        c(TerminalColor::Red, BOLD),
        c(TerminalColor::White, BOLD),
        c(TerminalColor::Red, BOLD | REVERSE | UNDERLINE),
        c(TerminalColor::White, BOLD | UNDERLINE),
        c(TerminalColor::White, REVERSE),
        c(TerminalColor::Red, BOLD | UNDERLINE),
        c(TerminalColor::White, 0),
        c(TerminalColor::White, BOLD),
        c(TerminalColor::White, BOLD),
        c(TerminalColor::White, DIM),
    ],
};

const ACHROMATOPSIA: PaletteSpec = PaletteSpec {
    styles: [
        a(BOLD),
        a(BOLD | UNDERLINE),
        P,
        a(UNDERLINE),
        a(DIM),
        a(UNDERLINE),
        a(REVERSE),
        a(BOLD),
        a(BOLD | UNDERLINE),
        a(BOLD | REVERSE),
        a(UNDERLINE),
        a(BOLD | REVERSE),
        a(BOLD),
        a(BOLD | REVERSE | UNDERLINE),
        a(BOLD | UNDERLINE),
        a(REVERSE),
        a(BOLD | UNDERLINE),
        a(BOLD),
        a(BOLD),
        a(BOLD | REVERSE),
        a(DIM),
    ],
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reporting::theme::Role;

    #[test]
    fn palette_color_invariants_are_explicit() {
        assert!(HIGH_CONTRAST
            .styles
            .iter()
            .all(|style| style.attributes & DIM == 0));
        assert!(HIGH_CONTRAST
            .styles
            .iter()
            .any(|style| style.attributes & BRIGHT != 0));
        assert!(ACHROMATOPSIA
            .styles
            .iter()
            .all(|style| style.foreground.is_none()));
        for palette in [&PROTANOPIA, &DEUTERANOPIA] {
            assert!(palette.styles.iter().all(|style| !matches!(
                style.foreground,
                Some(TerminalColor::Red | TerminalColor::Green)
            )));
        }
        assert!(TRITANOPIA.styles.iter().all(|style| !matches!(
            style.foreground,
            Some(
                TerminalColor::Blue
                    | TerminalColor::Green
                    | TerminalColor::Yellow
                    | TerminalColor::Magenta
            )
        )));
    }

    #[test]
    fn regular_palettes_use_terminal_borders_and_high_contrast_colors_its_border() {
        for profile in [
            PaletteProfile::Plain,
            PaletteProfile::Standard,
            PaletteProfile::Protanopia,
            PaletteProfile::Deuteranopia,
            PaletteProfile::Tritanopia,
            PaletteProfile::Achromatopsia,
        ] {
            assert_eq!(
                specification(profile).styles[Role::Border as usize].foreground,
                None
            );
        }
        assert_eq!(
            specification(PaletteProfile::HighContrast).styles[Role::Border as usize].foreground,
            Some(TerminalColor::Cyan)
        );
    }
}

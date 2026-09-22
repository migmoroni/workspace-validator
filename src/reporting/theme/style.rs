//! Private styling primitives used to resolve palettes and presentations.

use console::{Attribute, Color, Style};

pub(super) const BOLD: u8 = 1 << 0;
pub(super) const DIM: u8 = 1 << 1;
pub(super) const UNDERLINE: u8 = 1 << 2;
pub(super) const REVERSE: u8 = 1 << 3;
pub(super) const BRIGHT: u8 = 1 << 4;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum TerminalColor {
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
}

impl TerminalColor {
    const fn console(self) -> Color {
        match self {
            Self::Red => Color::Red,
            Self::Green => Color::Green,
            Self::Yellow => Color::Yellow,
            Self::Blue => Color::Blue,
            Self::Magenta => Color::Magenta,
            Self::Cyan => Color::Cyan,
            Self::White => Color::White,
        }
    }

    const fn indicatif(self) -> &'static str {
        match self {
            Self::Red => "red",
            Self::Green => "green",
            Self::Yellow => "yellow",
            Self::Blue => "blue",
            Self::Magenta => "magenta",
            Self::Cyan => "cyan",
            Self::White => "white",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct StyleSpec {
    pub(super) foreground: Option<TerminalColor>,
    pub(super) attributes: u8,
}

impl StyleSpec {
    pub(super) const fn plain() -> Self {
        Self {
            foreground: None,
            attributes: 0,
        }
    }

    pub(super) const fn color(foreground: TerminalColor, attributes: u8) -> Self {
        Self {
            foreground: Some(foreground),
            attributes,
        }
    }

    pub(super) const fn attributes(attributes: u8) -> Self {
        Self {
            foreground: None,
            attributes,
        }
    }

    pub(super) fn merge(self, modifier: StyleModifier) -> Self {
        let attributes = (self.attributes & !modifier.remove) | modifier.add;
        Self {
            foreground: self.foreground,
            attributes,
        }
    }

    pub(super) fn console_style(self) -> Style {
        let mut style = Style::new().force_styling(self != Self::plain());
        if let Some(color) = self.foreground {
            style = style.fg(color.console());
        }
        for (flag, attribute) in [
            (BOLD, Attribute::Bold),
            (DIM, Attribute::Dim),
            (UNDERLINE, Attribute::Underlined),
            (REVERSE, Attribute::Reverse),
        ] {
            if self.attributes & flag != 0 {
                style = style.attr(attribute);
            }
        }
        if self.attributes & BRIGHT != 0 {
            style = style.bright();
        }
        style
    }

    pub(super) fn indicatif_modifier(self) -> String {
        let mut parts = Vec::new();
        if let Some(color) = self.foreground {
            parts.push(color.indicatif());
        }
        for (flag, name) in [
            (BOLD, "bold"),
            (DIM, "dim"),
            (UNDERLINE, "underlined"),
            (REVERSE, "reverse"),
            (BRIGHT, "bright"),
        ] {
            if self.attributes & flag != 0 {
                parts.push(name);
            }
        }
        if parts.is_empty() {
            String::new()
        } else {
            format!(".{}", parts.join("."))
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct StyleModifier {
    pub(super) remove: u8,
    pub(super) add: u8,
}

impl StyleModifier {
    pub(super) const fn none() -> Self {
        Self { remove: 0, add: 0 }
    }

    pub(super) const fn new(remove: u8, add: u8) -> Self {
        Self { remove, add }
    }
}

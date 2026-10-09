//! Firmware-agnostic core: knows how to draw, knows nothing about UEFI.
//! HTML tokenizer, CSS parser, text rendering, BLS parser, and UI layout.
#![cfg_attr(not(test), no_std)]

extern crate alloc;

pub mod bls;
pub mod css;
pub mod font;
pub mod html;
pub mod ui;

pub use bls::BootEntry;
pub use css::{ComputedStyle, Stylesheet};
pub use font::{FONT_DATA, FONT_HEIGHT, FONT_WIDTH};
pub use html::{Element, HtmlDocument, Node};
pub use ui::{
    draw_char, draw_cursor, draw_rect_outline, draw_text, render_menu_ui, HtmlTheme, MenuState,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect {
    pub x: usize,
    pub y: usize,
    pub w: usize,
    pub h: usize,
}

/// Anything the renderer can draw onto: a GOP framebuffer, a test buffer,
/// a desktop window, a PNG writer...
pub trait Surface {
    fn size(&self) -> (usize, usize);
    fn fill_rect(&mut self, rect: Rect, color: Color);
}

/// Parses `#rrggbb` or `#rgb`.
pub fn parse_hex_color(s: &str) -> Option<Color> {
    let s = s.trim().strip_prefix('#')?;
    let pair = |i: usize| -> Option<u8> { u8::from_str_radix(s.get(i..i + 2)?, 16).ok() };
    let nibble = |i: usize| -> Option<u8> {
        u8::from_str_radix(s.get(i..i + 1)?, 16).ok().map(|v| v * 17)
    };
    match s.len() {
        6 => Some(Color::rgb(pair(0)?, pair(2)?, pair(4)?)),
        3 => Some(Color::rgb(nibble(0)?, nibble(1)?, nibble(2)?)),
        _ => None,
    }
}

/// Helper CSS lookup for backward compatibility.
pub fn css_lookup<'a>(css: &'a str, selector: &str, property: &str) -> Option<&'a str> {
    let mut rest = css;
    while let Some(open) = rest.find('{') {
        let head = rest[..open].trim();
        let after = &rest[open + 1..];
        let close = after.find('}')?;
        if head == selector {
            for decl in after[..close].split(';') {
                if let Some((key, value)) = decl.split_once(':') {
                    if key.trim() == property {
                        return Some(value.trim());
                    }
                }
            }
        }
        rest = &after[close + 1..];
    }
    None
}

/// The colors read from the theme CSS.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DemoTheme {
    pub background: Color,
    pub card: Color,
}

impl DemoTheme {
    /// Loud red: shown when theme files are missing, so failure is obvious.
    pub const MISSING: Self = Self {
        background: Color::rgb(0xcc, 0x00, 0x00),
        card: Color::rgb(0x66, 0x00, 0x00),
    };

    /// Fallback for a CSS file that exists but doesn't set a value.
    pub const FALLBACK: Self = Self {
        background: Color::rgb(0x18, 0x18, 0x25),
        card: Color::rgb(0x31, 0x32, 0x44),
    };

    /// Reads `body { background-color }` and `.card { background-color }`.
    pub fn from_css(css: &str) -> Self {
        let get = |sel: &str| {
            css_lookup(css, sel, "background-color").and_then(parse_hex_color)
        };
        Self {
            background: get("body").unwrap_or(Self::FALLBACK.background),
            card: get(".card").unwrap_or(Self::FALLBACK.card),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;
    use alloc::vec::Vec;

    struct Recorder {
        calls: Vec<(Rect, Color)>,
    }

    impl Surface for Recorder {
        fn size(&self) -> (usize, usize) {
            (800, 600)
        }
        fn fill_rect(&mut self, rect: Rect, color: Color) {
            self.calls.push((rect, color));
        }
    }

    #[test]
    fn hex_colors() {
        assert_eq!(parse_hex_color("#1e1e2e"), Some(Color::rgb(0x1e, 0x1e, 0x2e)));
        assert_eq!(parse_hex_color(" #fff "), Some(Color::rgb(255, 255, 255)));
        assert_eq!(parse_hex_color("#12"), None);
        assert_eq!(parse_hex_color("red"), None);
        assert_eq!(parse_hex_color("#gggggg"), None);
    }

    #[test]
    fn test_render_menu_with_theme() {
        let mut s = Recorder { calls: Vec::new() };
        let entries = vec![BootEntry {
            id: "test.conf".into(),
            title: "Test Linux".into(),
            version: None,
            linux: "/vmlinuz".into(),
            initrd: Some("/initramfs.gz".into()),
            options: None,
        }];
        let menu = MenuState::new(entries);
        let theme = HtmlTheme::from_html_and_css(
            Some("<h1>My OS Loader</h1>"),
            Some("body { background-color: #1e1e2e; } .card { background-color: #313244; }"),
        );
        render_menu_ui(&mut s, &theme, &menu);
        assert!(!s.calls.is_empty());
    }
}

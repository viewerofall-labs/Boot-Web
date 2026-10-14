extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

use crate::bls::BootEntry;
use crate::css::Stylesheet;
use crate::font::{FONT_DATA, FONT_HEIGHT, FONT_WIDTH};
use crate::html::HtmlDocument;
use crate::{Color, Rect, Surface};

// ─────────────────────────────────────────────────────────────
// Mouse cursor
// ─────────────────────────────────────────────────────────────

const CURSOR_PATTERN: [&str; 19] = [
    "#                  ",
    "##                 ",
    "#.#                ",
    "#..#               ",
    "#...#              ",
    "#....#             ",
    "#.....#            ",
    "#......#           ",
    "#.......#          ",
    "#........#         ",
    "#.....#####        ",
    "#..#..#            ",
    "#.# #..#           ",
    "##   #..#          ",
    "#     #..#         ",
    "       #..#        ",
    "        ##         ",
    "                   ",
    "                   ",
];

// ─────────────────────────────────────────────────────────────
// Layout configuration (read from CSS custom properties)
// ─────────────────────────────────────────────────────────────

/// How entries are rotated when drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EntryRotation {
    /// Normal, left-to-right text (default).
    None,
    /// 90° clockwise: text reads top-to-bottom.
    Cw90,
    /// 180°: text is upside-down.
    Rot180,
    /// 270° clockwise (= 90° counter-clockwise): text reads bottom-to-top.
    Cw270,
    /// Diagonal going down-right (shear: each row shifts right by 1 px per row).
    DiagonalDown,
    /// Diagonal going up-right (shear: each row shifts right by -1 px per row).
    DiagonalUp,
}

impl EntryRotation {
    fn from_str(s: &str) -> Self {
        match s.trim() {
            "90" | "90deg" | "cw90" => Self::Cw90,
            "180" | "180deg" => Self::Rot180,
            "270" | "270deg" | "ccw90" | "cw270" => Self::Cw270,
            "diagonal" | "diagonal-down" => Self::DiagonalDown,
            "diagonal-up" => Self::DiagonalUp,
            _ => Self::None,
        }
    }
}

/// Whether entries are laid out in a column (default) or a row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LayoutMode {
    Vertical,
    Horizontal,
}

impl LayoutMode {
    fn from_str(s: &str) -> Self {
        match s.trim() {
            "horizontal" | "row" => Self::Horizontal,
            _ => Self::Vertical,
        }
    }
}

/// Background fill style.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BgFill {
    /// Solid color (default).
    Solid,
    /// Alternating squares (like a chessboard).
    Checkerboard,
    /// Horizontal stripes alternating body_bg and bg_pattern_color.
    Stripes,
    /// Dot grid.
    Dots,
    /// Fine grid lines.
    Grid,
}

impl BgFill {
    fn from_str(s: &str) -> Self {
        match s.trim() {
            "checkerboard" | "chess" => Self::Checkerboard,
            "stripes" | "horizontal-stripes" => Self::Stripes,
            "dots" => Self::Dots,
            "grid" => Self::Grid,
            _ => Self::Solid,
        }
    }
}

/// All layout + decoration values read from `theme.css` `:root` custom properties.
#[derive(Clone, Debug)]
pub struct LayoutConfig {
    /// Card left edge as a percentage of screen width (0–100). Default 10.
    pub card_x_pct: i32,
    /// Card top edge as a percentage of screen height (0–100). Default 20.
    pub card_y_pct: i32,
    /// Card width as a percentage of screen width (0–100). Default 80.
    pub card_w_pct: i32,
    /// Height of each entry row in pixels. Default 44.
    pub item_height: i32,
    /// Vertical (or horizontal) gap between entries in pixels. Default 12.
    pub item_margin: i32,
    /// Font scale for entry text (1–4). Default 1.
    pub item_text_scale: usize,
    /// Font scale for the heading. Default 2.
    pub title_scale: usize,
    /// Border thickness for the card and entries. Default 2.
    pub border_thickness: usize,
    /// Width of the selected-entry accent bar in pixels. Default 6.
    pub accent_bar_w: usize,
    /// How entry text is rotated. Default EntryRotation::None.
    pub entry_rotate: EntryRotation,
    /// Whether entries are stacked vertically or horizontally. Default Vertical.
    pub layout: LayoutMode,
    /// Background fill style. Default Solid.
    pub bg_fill: BgFill,
    /// Secondary color for pattern fills (e.g. alternating stripe color).
    pub bg_pattern_color: Option<Color>,
    /// Extra padding inside entry boxes (left + top). Default 16, 14.
    pub entry_pad_x: usize,
    pub entry_pad_y: usize,
}

impl Default for LayoutConfig {
    fn default() -> Self {
        Self {
            card_x_pct: 10,
            card_y_pct: 20,
            card_w_pct: 80,
            item_height: 44,
            item_margin: 12,
            item_text_scale: 1,
            title_scale: 2,
            border_thickness: 2,
            accent_bar_w: 6,
            entry_rotate: EntryRotation::None,
            layout: LayoutMode::Vertical,
            bg_fill: BgFill::Solid,
            bg_pattern_color: None,
            entry_pad_x: 16,
            entry_pad_y: 14,
        }
    }
}

impl LayoutConfig {
    pub fn from_stylesheet(sheet: &Stylesheet) -> Self {
        let mut cfg = Self::default();

        if let Some(v) = sheet.get_var_pct("--card-x-pct") { cfg.card_x_pct = v; }
        if let Some(v) = sheet.get_var_pct("--card-y-pct") { cfg.card_y_pct = v; }
        if let Some(v) = sheet.get_var_pct("--card-w-pct") { cfg.card_w_pct = v; }

        if let Some(v) = sheet.get_var_int("--item-height") {
            if v > 0 { cfg.item_height = v; }
        }
        if let Some(v) = sheet.get_var_int("--item-margin") {
            if v >= 0 { cfg.item_margin = v; }
        }
        if let Some(v) = sheet.get_var_int("--item-text-scale") {
            let s = v.clamp(1, 8) as usize;
            cfg.item_text_scale = s;
        }
        if let Some(v) = sheet.get_var_int("--title-scale") {
            let s = v.clamp(1, 8) as usize;
            cfg.title_scale = s;
        }
        if let Some(v) = sheet.get_var_int("--border-thickness") {
            if v >= 0 { cfg.border_thickness = v as usize; }
        }
        if let Some(v) = sheet.get_var_int("--accent-bar-w") {
            if v >= 0 { cfg.accent_bar_w = v as usize; }
        }
        if let Some(v) = sheet.get_var_int("--entry-pad-x") {
            if v >= 0 { cfg.entry_pad_x = v as usize; }
        }
        if let Some(v) = sheet.get_var_int("--entry-pad-y") {
            if v >= 0 { cfg.entry_pad_y = v as usize; }
        }
        if let Some(v) = sheet.get_var("--entry-rotate") {
            cfg.entry_rotate = EntryRotation::from_str(v);
        }
        if let Some(v) = sheet.get_var("--layout") {
            cfg.layout = LayoutMode::from_str(v);
        }
        if let Some(v) = sheet.get_var("--bg-fill") {
            cfg.bg_fill = BgFill::from_str(v);
        }
        if let Some(v) = sheet.get_var("--bg-pattern-color") {
            cfg.bg_pattern_color = crate::parse_hex_color(v);
        }

        cfg
    }
}

// ─────────────────────────────────────────────────────────────
// Theme (colours + text)
// ─────────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub struct HtmlTheme {
    pub page_title: String,
    pub heading: String,
    /// Subtitle shown under the heading. Sourced from the first `<p>` in `<body>`.
    pub subtitle: String,
    /// Label shown at the top of the boot-entry card. Sourced from `<h2>`.
    pub card_title: String,
    pub body_bg: Color,
    pub body_fg: Color,
    pub card_bg: Color,
    pub card_border: Color,
    pub card_fg: Color,
    pub entry_bg: Color,
    pub entry_border: Color,
    pub entry_fg: Color,
    pub selected_bg: Color,
    pub selected_border: Color,
    pub selected_fg: Color,
    pub accent_color: Color,
    pub footer_fg: Color,
    pub layout: LayoutConfig,
}

impl HtmlTheme {
    pub fn from_html_and_css(html_str: Option<&str>, css_str: Option<&str>) -> Self {
        let mut page_title = String::from("htmlboot");
        let mut heading = String::from("HTMLBoot Loader");
        let mut subtitle = String::from("Select an operating system to boot:");
        let mut card_title = String::from("Boot Options");

        if let Some(html) = html_str {
            let doc = HtmlDocument::parse(html);
            if let Some(t) = doc.title {
                if !t.is_empty() {
                    page_title = t;
                }
            }
            if let Some(h1) = doc.root.find_by_tag("h1") {
                let text = String::from(h1.text_content().trim());
                if !text.is_empty() {
                    heading = text;
                }
            }
            // <h2> → card panel label ("Boot Options" by default)
            if let Some(h2) = doc.root.find_by_tag("h2") {
                let text = String::from(h2.text_content().trim());
                if !text.is_empty() {
                    card_title = text;
                }
            }
            // First <p> in <body> → subtitle line under the heading
            if let Some(p) = doc.root.find_by_tag("p") {
                let text = String::from(p.text_content().trim());
                if !text.is_empty() {
                    subtitle = text;
                }
            }
        }

        let sheet = css_str.map(Stylesheet::parse).unwrap_or_default();
        let layout = LayoutConfig::from_stylesheet(&sheet);

        let body_bg = sheet
            .get_color("body", "background-color")
            .unwrap_or(Color::rgb(0x1e, 0x1e, 0x2e));
        let body_fg = sheet
            .get_color("body", "color")
            .unwrap_or(Color::rgb(0xcd, 0xd6, 0xf4));

        let card_bg = sheet
            .get_color(".card", "background-color")
            .unwrap_or(Color::rgb(0x31, 0x32, 0x44));
        let card_border = sheet
            .get_color(".card", "border-color")
            .unwrap_or(Color::rgb(0x45, 0x47, 0x5a));
        let card_fg = sheet
            .get_color(".card", "color")
            .unwrap_or(Color::rgb(0xcd, 0xd6, 0xf4));

        let entry_bg = sheet
            .get_color(".entry", "background-color")
            .unwrap_or(Color::rgb(0x18, 0x18, 0x25));
        let entry_border = sheet
            .get_color(".entry", "border-color")
            .unwrap_or(Color::rgb(0x31, 0x32, 0x44));
        let entry_fg = sheet
            .get_color(".entry", "color")
            .unwrap_or(Color::rgb(0xcd, 0xd6, 0xf4));

        let selected_bg = sheet
            .get_color(".entry.selected", "background-color")
            .or_else(|| sheet.get_color(".selected", "background-color"))
            .unwrap_or(Color::rgb(0x31, 0x32, 0x44));
        let selected_border = sheet
            .get_color(".entry.selected", "border-color")
            .or_else(|| sheet.get_color(".selected", "border-color"))
            .unwrap_or(Color::rgb(0x89, 0xb4, 0xfa));
        let selected_fg = sheet
            .get_color(".entry.selected", "color")
            .or_else(|| sheet.get_color(".selected", "color"))
            .unwrap_or(Color::rgb(0x89, 0xb4, 0xfa));

        let accent_color = sheet
            .get_color("h1", "color")
            .or_else(|| sheet.get_color(":root", "--accent"))
            .unwrap_or(Color::rgb(0x89, 0xb4, 0xfa));

        let footer_fg = sheet
            .get_color("footer", "color")
            .or_else(|| sheet.get_color("p", "color"))
            .unwrap_or(Color::rgb(0x93, 0x99, 0xb2));

        Self {
            page_title,
            heading,
            subtitle,
            card_title,
            body_bg,
            body_fg,
            card_bg,
            card_border,
            card_fg,
            entry_bg,
            entry_border,
            entry_fg,
            selected_bg,
            selected_border,
            selected_fg,
            accent_color,
            footer_fg,
            layout,
        }
    }
}

// ─────────────────────────────────────────────────────────────
// Menu state
// ─────────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub struct MenuState {
    pub entries: Vec<BootEntry>,
    pub selected: usize,
    pub cursor_pos: (usize, usize),
    pub cursor_visible: bool,
    pub status_message: Option<String>,
}

impl MenuState {
    pub fn new(entries: Vec<BootEntry>) -> Self {
        Self {
            entries,
            selected: 0,
            cursor_pos: (100, 100),
            cursor_visible: false,
            status_message: None,
        }
    }

    pub fn select_next(&mut self) {
        if !self.entries.is_empty() {
            self.selected = (self.selected + 1) % self.entries.len();
        }
    }

    pub fn select_prev(&mut self) {
        if !self.entries.is_empty() {
            if self.selected == 0 {
                self.selected = self.entries.len() - 1;
            } else {
                self.selected -= 1;
            }
        }
    }

    pub fn selected_entry(&self) -> Option<&BootEntry> {
        self.entries.get(self.selected)
    }

    pub fn entry_bounds(
        &self,
        index: usize,
        screen_w: usize,
        screen_h: usize,
        cfg: &LayoutConfig,
    ) -> Option<Rect> {
        if index >= self.entries.len() {
            return None;
        }

        let card_x = screen_w * cfg.card_x_pct as usize / 100;
        let card_y = screen_h * cfg.card_y_pct as usize / 100;
        let card_w = screen_w * cfg.card_w_pct as usize / 100;

        let list_y = card_y + 80;
        let list_x = card_x + 20;
        let item_h = cfg.item_height as usize;
        let item_m = cfg.item_margin as usize;

        let (item_x, item_y, item_w, item_h_out) = match cfg.layout {
            LayoutMode::Vertical => {
                let iy = list_y + index * (item_h + item_m);
                let iw = card_w.saturating_sub(40);
                (list_x, iy, iw, item_h)
            }
            LayoutMode::Horizontal => {
                // Each entry is a column; width is capped so they fit in the card
                let n = self.entries.len().max(1);
                let total_m = item_m * n.saturating_sub(1);
                let col_w = card_w.saturating_sub(40 + total_m) / n;
                let ix = list_x + index * (col_w + item_m);
                // Use a taller box for horizontal layout
                let h = screen_h.saturating_sub(list_y + 60).min(item_h * 3);
                (ix, list_y, col_w, h)
            }
        };

        Some(Rect {
            x: item_x,
            y: item_y,
            w: item_w,
            h: item_h_out,
        })
    }

    pub fn hit_test(
        &self,
        x: usize,
        y: usize,
        screen_w: usize,
        screen_h: usize,
        cfg: &LayoutConfig,
    ) -> Option<usize> {
        for (i, _) in self.entries.iter().enumerate() {
            if let Some(rect) = self.entry_bounds(i, screen_w, screen_h, cfg) {
                if x >= rect.x
                    && x < rect.x + rect.w
                    && y >= rect.y
                    && y < rect.y + rect.h
                {
                    return Some(i);
                }
            }
        }
        None
    }
}

// ─────────────────────────────────────────────────────────────
// Primitive drawing
// ─────────────────────────────────────────────────────────────

pub fn draw_char<S: Surface>(
    s: &mut S,
    x: usize,
    y: usize,
    ch: char,
    fg: Color,
    bg: Option<Color>,
    scale: usize,
) {
    let (w, h) = s.size();
    let char_idx = (ch as usize).min(255);
    let glyph = &FONT_DATA[char_idx * FONT_HEIGHT..(char_idx + 1) * FONT_HEIGHT];

    for (row, &byte) in glyph.iter().enumerate() {
        for col in 0..FONT_WIDTH {
            let px = x + col * scale;
            let py = y + row * scale;
            if px >= w || py >= h {
                continue;
            }

            let is_set = (byte & (0x80 >> col)) != 0;
            if is_set {
                s.fill_rect(Rect { x: px, y: py, w: scale, h: scale }, fg);
            } else if let Some(bg_color) = bg {
                s.fill_rect(Rect { x: px, y: py, w: scale, h: scale }, bg_color);
            }
        }
    }
}

pub fn draw_text<S: Surface>(
    s: &mut S,
    mut x: usize,
    mut y: usize,
    text: &str,
    fg: Color,
    bg: Option<Color>,
    scale: usize,
) {
    let start_x = x;
    for ch in text.chars() {
        if ch == '\n' {
            x = start_x;
            y += FONT_HEIGHT * scale;
            continue;
        }
        draw_char(s, x, y, ch, fg, bg, scale);
        x += FONT_WIDTH * scale;
    }
}

pub fn draw_rect_outline<S: Surface>(
    s: &mut S,
    rect: Rect,
    color: Color,
    thickness: usize,
) {
    if rect.w == 0 || rect.h == 0 {
        return;
    }
    // Top
    s.fill_rect(Rect { x: rect.x, y: rect.y, w: rect.w, h: thickness.min(rect.h) }, color);
    // Bottom
    let b_y = rect.y + rect.h.saturating_sub(thickness);
    s.fill_rect(Rect { x: rect.x, y: b_y, w: rect.w, h: thickness.min(rect.h) }, color);
    // Left
    s.fill_rect(Rect { x: rect.x, y: rect.y, w: thickness.min(rect.w), h: rect.h }, color);
    // Right
    let r_x = rect.x + rect.w.saturating_sub(thickness);
    s.fill_rect(Rect { x: r_x, y: rect.y, w: thickness.min(rect.w), h: rect.h }, color);
}

pub fn draw_cursor<S: Surface>(s: &mut S, cursor_x: usize, cursor_y: usize) {
    let (w, h) = s.size();
    let border_color = Color::rgb(0, 0, 0);
    let fill_color = Color::rgb(255, 255, 255);

    for (row, line) in CURSOR_PATTERN.iter().enumerate() {
        for (col, byte) in line.bytes().enumerate() {
            let px = cursor_x + col;
            let py = cursor_y + row;
            if px >= w || py >= h {
                continue;
            }
            match byte {
                b'#' => { s.fill_rect(Rect { x: px, y: py, w: 1, h: 1 }, border_color); }
                b'.' => { s.fill_rect(Rect { x: px, y: py, w: 1, h: 1 }, fill_color); }
                _ => {}
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────
// Rotated / transformed text rendering
// ─────────────────────────────────────────────────────────────

/// Render one character with a 90° clockwise rotation around (origin_x, origin_y).
/// The bounding box after rotation is FONT_HEIGHT wide and FONT_WIDTH tall.
fn draw_char_rot90<S: Surface>(
    s: &mut S,
    origin_x: usize,
    origin_y: usize,
    ch: char,
    fg: Color,
    bg: Option<Color>,
    scale: usize,
) {
    let (sw, sh) = s.size();
    let char_idx = (ch as usize).min(255);
    let glyph = &FONT_DATA[char_idx * FONT_HEIGHT..(char_idx + 1) * FONT_HEIGHT];

    // After 90° CW: (col, row) → (FONT_HEIGHT-1-row, col)  in new coords
    for (row, &byte) in glyph.iter().enumerate() {
        for col in 0..FONT_WIDTH {
            let is_set = (byte & (0x80 >> col)) != 0;
            let nx = origin_x + (FONT_HEIGHT - 1 - row) * scale;
            let ny = origin_y + col * scale;
            if nx >= sw || ny >= sh {
                continue;
            }
            if is_set {
                s.fill_rect(Rect { x: nx, y: ny, w: scale, h: scale }, fg);
            } else if let Some(bg_color) = bg {
                s.fill_rect(Rect { x: nx, y: ny, w: scale, h: scale }, bg_color);
            }
        }
    }
}

/// 90° CW text: advances origin_y by FONT_WIDTH per character.
fn draw_text_rot90<S: Surface>(
    s: &mut S,
    x: usize,
    mut y: usize,
    text: &str,
    fg: Color,
    bg: Option<Color>,
    scale: usize,
) {
    for ch in text.chars() {
        draw_char_rot90(s, x, y, ch, fg, bg, scale);
        y += FONT_WIDTH * scale;
    }
}

/// 180° rotation: text is upside-down. Advances x leftward.
fn draw_text_rot180<S: Surface>(
    s: &mut S,
    mut x: usize,
    y: usize,
    text: &str,
    fg: Color,
    bg: Option<Color>,
    scale: usize,
) {
    let (sw, sh) = s.size();
    let char_idx_fn = |ch: char| -> usize { (ch as usize).min(255) };

    for ch in text.chars() {
        let char_idx = char_idx_fn(ch);
        let glyph = &FONT_DATA[char_idx * FONT_HEIGHT..(char_idx + 1) * FONT_HEIGHT];
        for (row, &byte) in glyph.iter().enumerate() {
            for col in 0..FONT_WIDTH {
                let is_set = (byte & (0x80 >> col)) != 0;
                // 180°: flip both axes
                let nx = x + (FONT_WIDTH - 1 - col) * scale;
                let ny = y + (FONT_HEIGHT - 1 - row) * scale;
                if nx >= sw || ny >= sh {
                    continue;
                }
                if is_set {
                    s.fill_rect(Rect { x: nx, y: ny, w: scale, h: scale }, fg);
                } else if let Some(bg_color) = bg {
                    s.fill_rect(Rect { x: nx, y: ny, w: scale, h: scale }, bg_color);
                }
            }
        }
        // Advance right even though text is flipped (goes right-to-left visually,
        // but we lay out chars left-to-right in memory order; caller positions the
        // text block and the flip brings it into view).
        x += FONT_WIDTH * scale;
    }
}

/// 270° CW (90° CCW) rotation: text reads bottom to top. Advances origin_y down.
fn draw_char_rot270<S: Surface>(
    s: &mut S,
    origin_x: usize,
    origin_y: usize,
    ch: char,
    fg: Color,
    bg: Option<Color>,
    scale: usize,
) {
    let (sw, sh) = s.size();
    let char_idx = (ch as usize).min(255);
    let glyph = &FONT_DATA[char_idx * FONT_HEIGHT..(char_idx + 1) * FONT_HEIGHT];
    // 270° CW: (col, row) → (row, FONT_WIDTH-1-col)
    for (row, &byte) in glyph.iter().enumerate() {
        for col in 0..FONT_WIDTH {
            let is_set = (byte & (0x80 >> col)) != 0;
            let nx = origin_x + row * scale;
            let ny = origin_y + (FONT_WIDTH - 1 - col) * scale;
            if nx >= sw || ny >= sh {
                continue;
            }
            if is_set {
                s.fill_rect(Rect { x: nx, y: ny, w: scale, h: scale }, fg);
            } else if let Some(bg_color) = bg {
                s.fill_rect(Rect { x: nx, y: ny, w: scale, h: scale }, bg_color);
            }
        }
    }
}

fn draw_text_rot270<S: Surface>(
    s: &mut S,
    x: usize,
    mut y: usize,
    text: &str,
    fg: Color,
    bg: Option<Color>,
    scale: usize,
) {
    // Characters advance downward; y decreases visually (bottom-to-top reading)
    // but since we start at top of the rect and go down, we reverse the string.
    let chars: Vec<char> = text.chars().collect();
    for ch in chars.into_iter().rev() {
        draw_char_rot270(s, x, y, ch, fg, bg, scale);
        y += FONT_WIDTH * scale;
    }
}

/// Diagonal-down: each successive character row shifts +1 px to the right.
fn draw_text_diagonal<S: Surface>(
    s: &mut S,
    mut x: usize,
    y: usize,
    text: &str,
    fg: Color,
    bg: Option<Color>,
    scale: usize,
    shear_sign: i32, // +1 = down-right, -1 = up-right
) {
    let (sw, sh) = s.size();
    let char_idx_fn = |ch: char| -> usize { (ch as usize).min(255) };
    let char_count = text.chars().count();

    // Compute total shear offset so we can position starting y to keep text in bounds
    let total_shear = FONT_HEIGHT * scale * char_count;

    for ch in text.chars() {
        let char_idx = char_idx_fn(ch);
        let glyph = &FONT_DATA[char_idx * FONT_HEIGHT..(char_idx + 1) * FONT_HEIGHT];
        for (row, &byte) in glyph.iter().enumerate() {
            for col in 0..FONT_WIDTH {
                let is_set = (byte & (0x80 >> col)) != 0;
                let px = x + col * scale;
                // Shear: each row is offset diagonally
                let shear_offset = row * scale;
                let py = if shear_sign > 0 {
                    y + shear_offset
                } else {
                    y.saturating_sub(shear_offset) + total_shear
                };
                if px >= sw || py >= sh {
                    continue;
                }
                if is_set {
                    s.fill_rect(Rect { x: px, y: py, w: scale, h: scale }, fg);
                } else if let Some(bg_color) = bg {
                    s.fill_rect(Rect { x: px, y: py, w: scale, h: scale }, bg_color);
                }
            }
        }
        x += FONT_WIDTH * scale;
    }
    let _ = total_shear;
}

/// Unified entry label renderer that respects the configured rotation.
fn draw_entry_text<S: Surface>(
    s: &mut S,
    rect: &Rect,
    text: &str,
    fg: Color,
    bg: Option<Color>,
    scale: usize,
    rotation: EntryRotation,
    pad_x: usize,
    pad_y: usize,
) {
    match rotation {
        EntryRotation::None => {
            draw_text(s, rect.x + pad_x, rect.y + pad_y, text, fg, bg, scale);
        }
        EntryRotation::Cw90 => {
            // Text runs top→bottom inside the entry box
            draw_text_rot90(s, rect.x + pad_y, rect.y + pad_x, text, fg, bg, scale);
        }
        EntryRotation::Rot180 => {
            // Anchor to bottom-right of the entry
            let text_w = text.chars().count() * FONT_WIDTH * scale;
            let start_x = (rect.x + rect.w).saturating_sub(text_w + pad_x);
            let start_y = (rect.y + rect.h).saturating_sub(FONT_HEIGHT * scale + pad_y);
            draw_text_rot180(s, start_x, start_y, text, fg, bg, scale);
        }
        EntryRotation::Cw270 => {
            // Text runs bottom→top inside the entry box
            let start_x = (rect.x + rect.w).saturating_sub(FONT_HEIGHT * scale + pad_y);
            draw_text_rot270(s, start_x, rect.y + pad_x, text, fg, bg, scale);
        }
        EntryRotation::DiagonalDown => {
            draw_text_diagonal(
                s, rect.x + pad_x, rect.y + pad_y, text, fg, bg, scale, 1,
            );
        }
        EntryRotation::DiagonalUp => {
            draw_text_diagonal(
                s, rect.x + pad_x, rect.y + pad_y, text, fg, bg, scale, -1,
            );
        }
    }
}

// ─────────────────────────────────────────────────────────────
// Background fill patterns
// ─────────────────────────────────────────────────────────────

fn draw_background<S: Surface>(
    s: &mut S,
    screen_w: usize,
    screen_h: usize,
    body_bg: Color,
    fill: BgFill,
    pattern_color: Option<Color>,
) {
    // Always fill with body_bg first
    s.fill_rect(Rect { x: 0, y: 0, w: screen_w, h: screen_h }, body_bg);

    let alt = pattern_color.unwrap_or(Color::rgb(
        body_bg.r.saturating_add(20),
        body_bg.g.saturating_add(20),
        body_bg.b.saturating_add(20),
    ));

    match fill {
        BgFill::Solid => {}
        BgFill::Checkerboard => {
            let cell = 40usize;
            let mut row = 0usize;
            while row < screen_h {
                let mut col = 0usize;
                while col < screen_w {
                    let checker = ((row / cell) + (col / cell)) % 2 == 1;
                    if checker {
                        s.fill_rect(
                            Rect {
                                x: col,
                                y: row,
                                w: cell.min(screen_w - col),
                                h: cell.min(screen_h - row),
                            },
                            alt,
                        );
                    }
                    col += cell;
                }
                row += cell;
            }
        }
        BgFill::Stripes => {
            let stripe_h = 20usize;
            let mut y = 0usize;
            let mut toggle = false;
            while y < screen_h {
                if toggle {
                    let h = stripe_h.min(screen_h - y);
                    s.fill_rect(Rect { x: 0, y, w: screen_w, h }, alt);
                }
                y += stripe_h;
                toggle = !toggle;
            }
        }
        BgFill::Dots => {
            let spacing = 20usize;
            let dot_r = 2usize;
            let mut y = spacing / 2;
            while y < screen_h {
                let mut x = spacing / 2;
                while x < screen_w {
                    s.fill_rect(
                        Rect {
                            x: x.saturating_sub(dot_r),
                            y: y.saturating_sub(dot_r),
                            w: dot_r * 2,
                            h: dot_r * 2,
                        },
                        alt,
                    );
                    x += spacing;
                }
                y += spacing;
            }
        }
        BgFill::Grid => {
            let spacing = 40usize;
            // Horizontal lines
            let mut y = 0usize;
            while y < screen_h {
                s.fill_rect(Rect { x: 0, y, w: screen_w, h: 1 }, alt);
                y += spacing;
            }
            // Vertical lines
            let mut x = 0usize;
            while x < screen_w {
                s.fill_rect(Rect { x, y: 0, w: 1, h: screen_h }, alt);
                x += spacing;
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────
// Top-level render
// ─────────────────────────────────────────────────────────────

pub fn render_menu_ui<S: Surface>(
    s: &mut S,
    theme: &HtmlTheme,
    menu: &MenuState,
) {
    let (screen_w, screen_h) = s.size();
    let cfg = &theme.layout;

    // 1. Background
    draw_background(s, screen_w, screen_h, theme.body_bg, cfg.bg_fill, cfg.bg_pattern_color);

    // 2. Heading
    let title_scale = cfg.title_scale;
    let title_x = 40;
    let title_y = 30;
    draw_text(s, title_x, title_y, &theme.heading, theme.accent_color, None, title_scale);

    let sub_text = &theme.subtitle;
    draw_text(
        s,
        title_x,
        title_y + FONT_HEIGHT * title_scale + 8,
        sub_text,
        theme.body_fg,
        None,
        1,
    );

    // 3. Card
    let card_x = screen_w * cfg.card_x_pct as usize / 100;
    let card_y = screen_h * cfg.card_y_pct as usize / 100;
    let card_w = screen_w * cfg.card_w_pct as usize / 100;

    let num_entries = menu.entries.len().max(1);
    let card_h = match cfg.layout {
        LayoutMode::Vertical => {
            90 + num_entries * (cfg.item_height as usize + cfg.item_margin as usize)
        }
        LayoutMode::Horizontal => {
            // Fixed height — enough room for the item boxes + padding
            let item_h = cfg.item_height as usize;
            item_h * 3 + 100
        }
    };

    s.fill_rect(Rect { x: card_x, y: card_y, w: card_w, h: card_h }, theme.card_bg);
    draw_rect_outline(s, Rect { x: card_x, y: card_y, w: card_w, h: card_h }, theme.card_border, cfg.border_thickness);

    draw_text(s, card_x + 20, card_y + 20, &theme.card_title, theme.card_fg, None, 1);

    // Separator
    s.fill_rect(
        Rect { x: card_x + 20, y: card_y + 44, w: card_w.saturating_sub(40), h: 1 },
        theme.card_border,
    );

    // 4. Entries
    if menu.entries.is_empty() {
        draw_text(
            s,
            card_x + 30,
            card_y + 80,
            "No boot entries found in \\loader\\entries",
            Color::rgb(0xf3, 0x8b, 0xa8),
            None,
            1,
        );
    } else {
        for (i, entry) in menu.entries.iter().enumerate() {
            if let Some(rect) = menu.entry_bounds(i, screen_w, screen_h, cfg) {
                let is_selected = i == menu.selected;

                let item_bg = if is_selected { theme.selected_bg } else { theme.entry_bg };
                let item_border = if is_selected { theme.selected_border } else { theme.entry_border };
                let text_color = if is_selected { theme.selected_fg } else { theme.entry_fg };
                let border_t = if is_selected { cfg.border_thickness } else { cfg.border_thickness.saturating_sub(1).max(1) };

                s.fill_rect(rect, item_bg);
                draw_rect_outline(s, rect, item_border, border_t);

                // Accent bar (selected only)
                if is_selected && cfg.accent_bar_w > 0 {
                    s.fill_rect(
                        Rect { x: rect.x, y: rect.y, w: cfg.accent_bar_w, h: rect.h },
                        theme.selected_border,
                    );
                }

                // Entry label
                let prefix = if is_selected { "> " } else { "  " };
                let mut label = String::from(prefix);
                label.push_str(&entry.title);

                draw_entry_text(
                    s,
                    &rect,
                    &label,
                    text_color,
                    None,
                    cfg.item_text_scale,
                    cfg.entry_rotate,
                    cfg.entry_pad_x,
                    cfg.entry_pad_y,
                );

                // Kernel path hint (right-aligned, normal rotation, smaller)
                if cfg.entry_rotate == EntryRotation::None || cfg.entry_rotate == EntryRotation::Rot180 {
                    let info_color = theme.footer_fg;
                    let info_x = (rect.x + rect.w)
                        .saturating_sub(entry.linux.len() * FONT_WIDTH + 20);
                    if info_x > rect.x + 200 {
                        draw_text(s, info_x, rect.y + cfg.entry_pad_y, &entry.linux, info_color, None, 1);
                    }
                }
            }
        }
    }

    // 5. Footer
    let footer_y = screen_h.saturating_sub(40);
    draw_text(
        s,
        40,
        footer_y,
        "[^ / v / Mouse] Navigate   [Enter / Click] Boot Selected",
        theme.footer_fg,
        None,
        1,
    );

    if let Some(ref status) = menu.status_message {
        draw_text(s, 40, footer_y.saturating_sub(25), status, Color::rgb(0xa6, 0xe3, 0xa1), None, 1);
    }

    // 6. Cursor
    if menu.cursor_visible {
        draw_cursor(s, menu.cursor_pos.0, menu.cursor_pos.1);
    }
}

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

use crate::bls::BootEntry;
use crate::css::Stylesheet;
use crate::font::{FONT_DATA, FONT_HEIGHT, FONT_WIDTH};
use crate::html::HtmlDocument;
use crate::{Color, Rect, Surface};

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

#[derive(Clone, Debug)]
pub struct HtmlTheme {
    pub page_title: String,
    pub heading: String,
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
}

impl HtmlTheme {
    pub fn from_html_and_css(html_str: Option<&str>, css_str: Option<&str>) -> Self {
        let mut page_title = String::from("htmlboot");
        let mut heading = String::from("HTMLBoot Loader");

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
        }

        let sheet = css_str.map(Stylesheet::parse).unwrap_or_default();

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
        }
    }
}

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
    ) -> Option<Rect> {
        if index >= self.entries.len() {
            return None;
        }

        let card_w = 640.min(screen_w.saturating_sub(40));
        let card_x = (screen_w.saturating_sub(card_w)) / 2;
        let card_y = 120.min(screen_h / 4);

        let item_margin = 12;
        let item_h = 44;
        let list_y = card_y + 80;

        let item_y = list_y + index * (item_h + item_margin);
        let item_w = card_w.saturating_sub(40);
        let item_x = card_x + 20;

        Some(Rect {
            x: item_x,
            y: item_y,
            w: item_w,
            h: item_h,
        })
    }

    pub fn hit_test(
        &self,
        x: usize,
        y: usize,
        screen_w: usize,
        screen_h: usize,
    ) -> Option<usize> {
        for (i, _) in self.entries.iter().enumerate() {
            if let Some(rect) = self.entry_bounds(i, screen_w, screen_h) {
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
                s.fill_rect(
                    Rect {
                        x: px,
                        y: py,
                        w: scale,
                        h: scale,
                    },
                    fg,
                );
            } else if let Some(bg_color) = bg {
                s.fill_rect(
                    Rect {
                        x: px,
                        y: py,
                        w: scale,
                        h: scale,
                    },
                    bg_color,
                );
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
    s.fill_rect(
        Rect {
            x: rect.x,
            y: rect.y,
            w: rect.w,
            h: thickness.min(rect.h),
        },
        color,
    );
    // Bottom
    let b_y = rect.y + rect.h.saturating_sub(thickness);
    s.fill_rect(
        Rect {
            x: rect.x,
            y: b_y,
            w: rect.w,
            h: thickness.min(rect.h),
        },
        color,
    );
    // Left
    s.fill_rect(
        Rect {
            x: rect.x,
            y: rect.y,
            w: thickness.min(rect.w),
            h: rect.h,
        },
        color,
    );
    // Right
    let r_x = rect.x + rect.w.saturating_sub(thickness);
    s.fill_rect(
        Rect {
            x: r_x,
            y: rect.y,
            w: thickness.min(rect.w),
            h: rect.h,
        },
        color,
    );
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
                b'#' => {
                    s.fill_rect(Rect { x: px, y: py, w: 1, h: 1 }, border_color);
                }
                b'.' => {
                    s.fill_rect(Rect { x: px, y: py, w: 1, h: 1 }, fill_color);
                }
                _ => {}
            }
        }
    }
}

pub fn render_menu_ui<S: Surface>(
    s: &mut S,
    theme: &HtmlTheme,
    menu: &MenuState,
) {
    let (screen_w, screen_h) = s.size();

    // 1. Fill screen background from body { background-color }
    s.fill_rect(
        Rect {
            x: 0,
            y: 0,
            w: screen_w,
            h: screen_h,
        },
        theme.body_bg,
    );

    // 2. Header Title from <h1> / <title> in HTML
    let title_scale = 2;
    let title_x = 40;
    let title_y = 30;
    draw_text(
        s,
        title_x,
        title_y,
        &theme.heading,
        theme.accent_color,
        None,
        title_scale,
    );

    let sub_text = "Select an operating system to boot:";
    draw_text(
        s,
        title_x,
        title_y + (FONT_HEIGHT * title_scale) + 8,
        sub_text,
        theme.body_fg,
        None,
        1,
    );

    // 3. Card container styled by .card in CSS
    let card_w = 640.min(screen_w.saturating_sub(40));
    let card_x = (screen_w.saturating_sub(card_w)) / 2;
    let card_y = 120.min(screen_h / 4);

    let num_entries = menu.entries.len().max(1);
    let item_h = 44;
    let item_margin = 12;
    let card_h = 90 + num_entries * (item_h + item_margin);

    // Draw card background
    s.fill_rect(
        Rect {
            x: card_x,
            y: card_y,
            w: card_w,
            h: card_h,
        },
        theme.card_bg,
    );
    draw_rect_outline(
        s,
        Rect {
            x: card_x,
            y: card_y,
            w: card_w,
            h: card_h,
        },
        theme.card_border,
        2,
    );

    // Card Header label
    draw_text(
        s,
        card_x + 20,
        card_y + 20,
        "Boot Options",
        theme.card_fg,
        None,
        1,
    );

    // Separator line
    s.fill_rect(
        Rect {
            x: card_x + 20,
            y: card_y + 44,
            w: card_w.saturating_sub(40),
            h: 1,
        },
        theme.card_border,
    );

    // 4. Render boot entries from .entries and .entry styles
    if menu.entries.is_empty() {
        let no_entry_y = card_y + 80;
        draw_text(
            s,
            card_x + 30,
            no_entry_y,
            "No boot entries found in \\loader\\entries",
            Color::rgb(0xf3, 0x8b, 0xa8),
            None,
            1,
        );
    } else {
        for (i, entry) in menu.entries.iter().enumerate() {
            if let Some(rect) = menu.entry_bounds(i, screen_w, screen_h) {
                let is_selected = i == menu.selected;

                let item_bg = if is_selected {
                    theme.selected_bg
                } else {
                    theme.entry_bg
                };

                let item_border = if is_selected {
                    theme.selected_border
                } else {
                    theme.entry_border
                };

                // Item background & border
                s.fill_rect(rect, item_bg);
                draw_rect_outline(s, rect, item_border, if is_selected { 2 } else { 1 });

                // Accent indicator bar for selected item
                if is_selected {
                    s.fill_rect(
                        Rect {
                            x: rect.x,
                            y: rect.y,
                            w: 6,
                            h: rect.h,
                        },
                        theme.selected_border,
                    );
                }

                // Text
                let text_color = if is_selected {
                    theme.selected_fg
                } else {
                    theme.entry_fg
                };

                let prefix = if is_selected { "> " } else { "  " };
                let mut label = String::from(prefix);
                label.push_str(&entry.title);

                draw_text(
                    s,
                    rect.x + 16,
                    rect.y + 14,
                    &label,
                    text_color,
                    None,
                    1,
                );

                // Secondary kernel path info on right
                let info_color = theme.footer_fg;
                let info_x = (rect.x + rect.w).saturating_sub(
                    entry.linux.len() * FONT_WIDTH + 20,
                );
                if info_x > rect.x + 200 {
                    draw_text(
                        s,
                        info_x,
                        rect.y + 14,
                        &entry.linux,
                        info_color,
                        None,
                        1,
                    );
                }
            }
        }
    }

    // 5. Status / Footer hints
    let footer_y = screen_h.saturating_sub(40);
    let hint_text = "[^ / v / Mouse] Navigate   [Enter / Click] Boot Selected";
    draw_text(
        s,
        40,
        footer_y,
        hint_text,
        theme.footer_fg,
        None,
        1,
    );

    if let Some(ref status) = menu.status_message {
        draw_text(
            s,
            40,
            footer_y.saturating_sub(25),
            status,
            Color::rgb(0xa6, 0xe3, 0xa1),
            None,
            1,
        );
    }

    // 6. Mouse cursor if visible
    if menu.cursor_visible {
        draw_cursor(s, menu.cursor_pos.0, menu.cursor_pos.1);
    }
}

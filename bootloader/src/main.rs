#![no_main]
#![no_std]

extern crate alloc;

use alloc::format;
use alloc::string::String;
use htmlboot_core::{render_menu_ui, HtmlTheme, MenuState, Surface};
use htmlboot_ueeli::{boot_entry, discover_boot_entries, read_esp_text, GopSurface, InputManager, UserAction};
use uefi::boot;
use uefi::cstr16;
use uefi::prelude::*;

// Alias in case crate name is htmlboot_uefi
use htmlboot_uefi as htmlboot_ueeli;

#[entry]
fn main() -> Status {
    uefi::helpers::init().unwrap();

    let html = read_esp_text(cstr16!("\\EFI\\htmlboot\\theme.html"));
    let css = read_esp_text(cstr16!("\\EFI\\htmlboot\\theme.css"));

    let theme = HtmlTheme::from_html_and_css(html.as_deref(), css.as_deref());

    let entries = discover_boot_entries();
    let mut menu = MenuState::new(entries);
    let mut screen = match GopSurface::open() {
        Ok(s) => s,
        Err(e) => return e.status(),
    };

    let (sw, sh) = screen.size();
    menu.cursor_pos = (sw / 2, sh / 2);

    let mut input = InputManager::new();

    let mut boot_target = None;

    // Initial render
    render_menu_ui(&mut screen, &theme, &menu);

    loop {
        let action = input.poll_action();
        let mut needs_redraw = false;

        match action {
            UserAction::Up => {
                menu.select_prev();
                needs_redraw = true;
            }
            UserAction::Down => {
                menu.select_next();
                needs_redraw = true;
            }
            UserAction::Home => {
                menu.selected = 0;
                needs_redraw = true;
            }
            UserAction::End => {
                if !menu.entries.is_empty() {
                    menu.selected = menu.entries.len() - 1;
                }
                needs_redraw = true;
            }
            UserAction::DirectSelect(idx) => {
                if idx < menu.entries.len() {
                    menu.selected = idx;
                    boot_target = Some(menu.entries[idx].clone());
                    needs_redraw = true;
                }
            }
            UserAction::CursorMove { dx, dy } => {
                menu.cursor_visible = true;
                let cur_x = menu.cursor_pos.0 as i32 + dx;
                let cur_y = menu.cursor_pos.1 as i32 + dy;
                let new_x = cur_x.clamp(0, sw.saturating_sub(1) as i32) as usize;
                let new_y = cur_y.clamp(0, sh.saturating_sub(1) as i32) as usize;
                menu.cursor_pos = (new_x, new_y);

                if let Some(hit) = menu.hit_test(new_x, new_y, sw, sh) {
                    if menu.selected != hit {
                        menu.selected = hit;
                    }
                }
                needs_redraw = true;
            }
            UserAction::Select => {
                if let Some(entry) = menu.selected_entry() {
                    boot_target = Some(entry.clone());
                }
            }
            UserAction::PointerMoved { dx, dy } => {
                menu.cursor_visible = true;
                let cur_x = menu.cursor_pos.0 as i32 + (dx / 2);
                let cur_y = menu.cursor_pos.1 as i32 + (dy / 2);
                let new_x = cur_x.clamp(0, sw.saturating_sub(1) as i32) as usize;
                let new_y = cur_y.clamp(0, sh.saturating_sub(1) as i32) as usize;
                menu.cursor_pos = (new_x, new_y);

                if let Some(hit) = menu.hit_test(new_x, new_y, sw, sh) {
                    if menu.selected != hit {
                        menu.selected = hit;
                    }
                }
                needs_redraw = true;
            }
            UserAction::PointerClick => {
                menu.cursor_visible = true;
                if let Some(hit) = menu.hit_test(menu.cursor_pos.0, menu.cursor_pos.1, sw, sh) {
                    menu.selected = hit;
                    if let Some(entry) = menu.selected_entry() {
                        boot_target = Some(entry.clone());
                    }
                } else if let Some(entry) = menu.selected_entry() {
                    boot_target = Some(entry.clone());
                }
                needs_redraw = true;
            }
            UserAction::None => {
                boot::stall(15_000);
            }
        }

        if let Some(entry) = boot_target.take() {
            menu.status_message = Some(format!("Booting {}...", entry.title));
            render_menu_ui(&mut screen, &theme, &menu);
            boot::stall(300_000); // 0.3s visual confirmation

            // Release exclusive protocol locks (GOP, Keyboard, Mouse) before launching the kernel
            drop(screen);
            drop(input);

            let err = boot_entry(&entry);

            // If boot failed or returned back to bootloader, reopen GOP and input
            screen = GopSurface::open().unwrap();
            input = InputManager::new();

            if let Err((stage, status)) = err {
                menu.status_message = Some(format!("Boot failed at [{}]: {:?}", stage, status));
            } else {
                menu.status_message = Some(String::from("Kernel returned to bootloader"));
            }
            needs_redraw = true;
        }

        if needs_redraw {
            render_menu_ui(&mut screen, &theme, &menu);
        }
    }
}

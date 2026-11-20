mod core;
mod ui;

use std::cell::RefCell;
use std::rc::Rc;

use slint::{ComponentHandle, Weak};

use core::{Editor, AUDIO_EXTS, IMAGE_EXTS};
use ui::MainWindow;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ui = MainWindow::new()?;
    let ui_weak: Weak<MainWindow> = ui.as_weak();

    let editor = Rc::new(RefCell::new(Editor::new()));

    ui.set_show_confirm_close(false);
    ui.set_confirm_close_message("".into());

    ui.on_field_edited({
        let ed = editor.clone();
        let w = ui_weak.clone();
        move || {
            let mut e = ed.borrow_mut();
            if !e.dirty {
                e.dirty = true;
            }
            drop(e);
            if let Some(ui) = w.upgrade() {
                ui.set_dirty(true);
            }
        }
    });

    ui.on_open_file({
        let ed = editor.clone();
        let w = ui_weak.clone();
        move || {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("Audio", AUDIO_EXTS)
                .pick_file()
            {
                if let Some(ui) = w.upgrade() {
                    let mut e = ed.borrow_mut();
                    core::load_file(&ui, &mut e, &path);
                }
            }
        }
    });

    ui.on_save_file({
        let ed = editor.clone();
        let w = ui_weak.clone();
        move || {
            if let Some(ui) = w.upgrade() {
                let mut e = ed.borrow_mut();
                core::save_file(&ui, &mut e);
            }
        }
    });

    ui.on_set_cover({
        let ed = editor.clone();
        let w = ui_weak.clone();
        move || {
            let has_file = ed.borrow().file.is_some();
            if !has_file {
                return;
            }
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("Images", IMAGE_EXTS)
                .pick_file()
            {
                if let Some(ui) = w.upgrade() {
                    let mut e = ed.borrow_mut();
                    ui::set_cover_from_path(&ui, &mut e, &path);
                }
            }
        }
    });

    ui.on_remove_cover({
        let ed = editor.clone();
        let w = ui_weak.clone();
        move || {
            {
                let mut e = ed.borrow_mut();
                e.cover_action = core::CoverAction::Remove;
                e.dirty = true;
            }
            if let Some(ui) = w.upgrade() {
                ui.set_cover_image(slint::Image::default());
                ui.set_has_cover_image(false);
                ui.set_can_remove_cover(false);
                ui.set_dirty(true);
            }
        }
    });

    ui.on_confirm_close_save({
        let ed = editor.clone();
        let w = ui_weak.clone();
        move || {
            if let Some(ui) = w.upgrade() {
                let mut e = ed.borrow_mut();
                core::save_file(&ui, &mut e);
                let _ = ui.window().hide();
            }
        }
    });

    ui.on_confirm_close_discard({
        let w = ui_weak.clone();
        move || {
            if let Some(ui) = w.upgrade() {
                let _ = ui.window().hide();
            }
        }
    });

    ui.on_confirm_close_cancel({
        let w = ui_weak.clone();
        move || {
            if let Some(ui) = w.upgrade() {
                ui.set_show_confirm_close(false);
            }
        }
    });

    let weak_for_close = ui_weak.clone();
    ui.window().on_close_requested(move || {
        if let Some(ui) = weak_for_close.upgrade() {
            if ui.get_dirty() {
                ui.invoke_close_requested();
                return slint::CloseRequestResponse::KeepWindowShown;
            }
        }
        slint::CloseRequestResponse::HideWindow
    });

    ui.on_close_requested({
        let w = ui_weak.clone();
        move || {
            if let Some(ui) = w.upgrade() {
                ui.set_show_confirm_close(true);
                ui.set_confirm_close_message("You have unsaved changes. Do you want to save them?".into());
            }
        }
    });

    ui.run()?;
    Ok(())
}

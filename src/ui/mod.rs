use image::ImageFormat;
use lofty::picture::{MimeType, Picture, PictureType};
use slint::{Image, Rgba8Pixel, SharedPixelBuffer};

use crate::core::Editor;
use crate::core::CoverAction;

slint::include_modules!();

pub fn decode_image(bytes: &[u8]) -> Option<Image> {
    let img = image::load_from_memory(bytes).ok()?;
    let rgba = img.to_rgba8();
    let buf = SharedPixelBuffer::<Rgba8Pixel>::clone_from_slice(
        rgba.as_raw(),
        rgba.width(),
        rgba.height(),
    );
    Some(Image::from_rgba8(buf))
}

fn mime_for_ext(ext: &str) -> MimeType {
    match ext {
        "png" => MimeType::Png,
        "jpg" | "jpeg" => MimeType::Jpeg,
        "gif" => MimeType::Gif,
        "bmp" => MimeType::Bmp,
        _ => MimeType::Unknown("image/png".to_string()),
    }
}

fn prepare_cover(bytes: &[u8], ext: &str) -> Result<(Vec<u8>, MimeType), String> {
    match ext {
        "png" | "jpg" | "jpeg" | "gif" | "bmp" => {
            Ok((bytes.to_vec(), mime_for_ext(ext)))
        }
        _ => {
            let img = image::load_from_memory(bytes).map_err(|e| e.to_string())?;
            let mut out = Vec::new();
            img.write_to(&mut std::io::Cursor::new(&mut out), ImageFormat::Png)
                .map_err(|e| e.to_string())?;
            Ok((out, MimeType::Png))
        }
    }
}

pub fn set_cover_from_path(ui: &MainWindow, editor: &mut Editor, path: &std::path::Path) {
    let bytes = match std::fs::read(path) {
        Ok(b) => b,
        Err(e) => {
            ui.set_error_message(format!("Cannot read image: {e}").into());
            return;
        }
    };
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    let (tag_bytes, mime) = match prepare_cover(&bytes, &ext) {
        Ok(v) => v,
        Err(e) => {
            ui.set_error_message(format!("Not a valid image file: {e}").into());
            return;
        }
    };

    let preview_bytes = tag_bytes.clone();
    let pic = Picture::unchecked(tag_bytes)
        .pic_type(PictureType::CoverFront)
        .mime_type(mime)
        .build();

    editor.cover_action = CoverAction::Set(pic);
    editor.dirty = true;

    if let Some(img) = decode_image(&preview_bytes) {
        ui.set_cover_image(img);
        ui.set_has_cover_image(true);
    }
    ui.set_can_remove_cover(true);
    ui.set_dirty(true);
}

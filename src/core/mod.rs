use std::path::{Path, PathBuf};

use lofty::config::WriteOptions;
use lofty::file::{AudioFile, TaggedFile, TaggedFileExt};
use lofty::picture::{Picture, PictureType};
use lofty::tag::{Accessor, ItemKey, Tag};
use lofty::tag::items::Timestamp;

use crate::ui::MainWindow;

pub const AUDIO_EXTS: &[&str] = &[
    "mp3", "flac", "m4a", "m4b", "ogg", "oga", "opus", "wav", "aif", "aiff", "wma", "ape",
    "mpc", "wv", "tta", "mka", "dsf", "tak",
];

pub const IMAGE_EXTS: &[&str] = &["png", "jpg", "jpeg", "gif", "bmp", "webp"];

#[derive(Clone)]
pub enum CoverAction {
    Keep,
    Set(Picture),
    Remove,
}

pub struct Editor {
    pub file: Option<TaggedFile>,
    pub path: PathBuf,
    pub dirty: bool,
    pub cover_action: CoverAction,
}

impl Editor {
    pub fn new() -> Self {
        Self {
            file: None,
            path: PathBuf::new(),
            dirty: false,
            cover_action: CoverAction::Keep,
        }
    }
}

fn primary_tag(file: &TaggedFile) -> Option<&Tag> {
    file.primary_tag().or_else(|| file.first_tag())
}

fn read_str(tag: Option<&Tag>, get: impl Fn(&Tag) -> Option<std::borrow::Cow<'_, str>>) -> String {
    tag.and_then(|t| get(t)).map(|c| c.into_owned()).unwrap_or_default()
}

pub fn load_file(ui: &MainWindow, editor: &mut Editor, path: &Path) {
    match lofty::read_from_path(path) {
        Ok(file) => {
            editor.file = Some(file);
            editor.path = path.to_path_buf();
            editor.dirty = false;
            editor.cover_action = CoverAction::Keep;

            let title;
            let artist;
            let album;
            let album_artist;
            let genre;
            let date;
            let track;
            let comment;
            let mut cover_pic: Option<Picture> = None;

            {
                let file = match editor.file.as_ref() {
                    Some(f) => f,
                    None => return,
                };
                let tag = primary_tag(file);
                title = read_str(tag, |t| t.title());
                artist = read_str(tag, |t| t.artist());
                album = read_str(tag, |t| t.album());
                album_artist = tag
                    .and_then(|t| t.get_string(ItemKey::AlbumArtist).map(|s| s.to_string()))
                    .unwrap_or_default();
                genre = read_str(tag, |t| t.genre());
                date = tag
                    .and_then(|t| {
                        t.date()
                            .map(|ts| ts.to_string())
                            .or_else(|| t.get_string(ItemKey::RecordingDate).map(|s| s.to_string()))
                    })
                    .unwrap_or_default();
                track = tag
                    .map(|t| {
                        match (t.track(), t.track_total()) {
                            (Some(n), Some(tt)) if tt > 0 => format!("{n}/{tt}"),
                            (Some(n), _) => n.to_string(),
                            _ => String::new(),
                        }
                    })
                    .unwrap_or_default();
                comment = read_str(tag, |t| t.comment());

                if let Some(t) = tag {
                    cover_pic = t
                        .pictures()
                        .iter()
                        .find(|p| p.pic_type() == PictureType::CoverFront)
                        .or_else(|| t.pictures().first())
                        .cloned();
                }
            }

            ui.set_track_title(title.into());
            ui.set_artist(artist.into());
            ui.set_album(album.into());
            ui.set_album_artist(album_artist.into());
            ui.set_genre(genre.into());
            ui.set_date(date.into());
            ui.set_track(track.into());
            ui.set_comment(comment.into());

            let file_name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();

            match cover_pic {
                Some(pic) => {
                    ui.set_can_remove_cover(true);
                    if let Some(img) = crate::ui::decode_image(pic.data()) {
                        ui.set_cover_image(img);
                        ui.set_has_cover_image(true);
                    } else {
                        ui.set_cover_image(slint::Image::default());
                        ui.set_has_cover_image(false);
                    }
                }
                None => {
                    ui.set_can_remove_cover(false);
                    ui.set_cover_image(slint::Image::default());
                    ui.set_has_cover_image(false);
                }
            }

            ui.set_file_name(file_name.clone().into());
            ui.set_has_file(true);
            ui.set_dirty(false);
            ui.set_status_message(format!("Loaded {file_name}").into());
        }
        Err(err) => {
            ui.set_error_message(format!("{err}").into());
        }
    }
}

fn set_str_field(tag: &mut Tag, value: &str, set: impl FnOnce(&mut Tag, String), remove: impl FnOnce(&mut Tag)) {
    if value.is_empty() {
        remove(tag);
    } else {
        set(tag, value.to_string());
    }
}

pub fn save_file(ui: &MainWindow, editor: &mut Editor) {
    if editor.file.is_none() {
        return;
    }
    let path = editor.path.clone();
    let action = editor.cover_action.clone();
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_string();

    let file = match editor.file.as_mut() {
        Some(f) => f,
        None => return,
    };

    let tag = match file.primary_tag_mut() {
        Some(t) => t,
        None => {
            if let Some(t) = file.first_tag_mut() {
                t
            } else {
                let tt = file.primary_tag_type();
                file.insert_tag(Tag::new(tt));
                match file.primary_tag_mut() {
                    Some(t) => t,
                    None => return,
                }
            }
        }
    };

    set_str_field(tag, &ui.get_track_title(), |t, v| t.set_title(v), |t| t.remove_title());
    set_str_field(tag, &ui.get_artist(), |t, v| t.set_artist(v), |t| t.remove_artist());
    set_str_field(tag, &ui.get_album(), |t, v| t.set_album(v), |t| t.remove_album());
    set_str_field(tag, &ui.get_genre(), |t, v| t.set_genre(v), |t| t.remove_genre());
    set_str_field(tag, &ui.get_comment(), |t, v| t.set_comment(v), |t| t.remove_comment());

    let album_artist = ui.get_album_artist();
    if album_artist.is_empty() {
        tag.remove_key(ItemKey::AlbumArtist);
    } else {
        tag.insert_text(ItemKey::AlbumArtist, album_artist.to_string());
    }

    let date = ui.get_date();
    if date.is_empty() {
        tag.remove_date();
    } else if let Ok(ts) = date.parse::<Timestamp>() {
        tag.set_date(ts);
    } else {
        tag.insert_text(ItemKey::RecordingDate, date.to_string());
    }

    let track = ui.get_track();
    if track.is_empty() {
        tag.remove_track();
        tag.remove_track_total();
    } else {
        let mut parts = track.split('/');
        let n = parts.next().and_then(|s| s.trim().parse::<u32>().ok());
        let total = parts.next().and_then(|s| s.trim().parse::<u32>().ok());
        match n {
            Some(num) => {
                tag.set_track(num);
                if let Some(tt) = total {
                    tag.set_track_total(tt);
                }
            }
            None => {
                tag.insert_text(ItemKey::TrackNumber, track.to_string());
            }
        }
    }

    match &action {
        CoverAction::Keep => {}
        CoverAction::Set(pic) => {
            while !tag.pictures().is_empty() {
                tag.remove_picture(0);
            }
            tag.push_picture(pic.clone());
        }
        CoverAction::Remove => {
            while !tag.pictures().is_empty() {
                tag.remove_picture(0);
            }
        }
    }

    match file.save_to_path(&path, WriteOptions::default()) {
        Ok(()) => {
            editor.dirty = false;
            editor.cover_action = CoverAction::Keep;
            ui.set_dirty(false);
            ui.set_status_message(format!("Saved {file_name}").into());
        }
        Err(err) => {
            ui.set_error_message(format!("{err}").into());
        }
    }
}

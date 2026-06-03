use crate::app::AppModel;
use crate::app::view::summary::text_overlay_available;
use crate::services::clipboard::ClipboardEntry;
use cosmic::iced::widget::image::Handle as ImageHandle;

pub(super) fn cache_thumbnail_handle(app: &mut AppModel, entry: &ClipboardEntry) {
    let ClipboardEntry::Image {
        bytes,
        hash,
        thumbnail_png: Some(thumbnail_png),
        ..
    } = entry
    else {
        return;
    };

    app.thumbnail_handles
        .entry((*hash, bytes.len()))
        .or_insert_with(|| ImageHandle::from_bytes(thumbnail_png.clone()));
}

pub(super) fn prune_thumbnail_handles(app: &mut AppModel) {
    app.thumbnail_handles.retain(|key, _| {
        app.history.iter().any(|item| match &item.entry {
            ClipboardEntry::Image { bytes, hash, .. } => key == &(*hash, bytes.len()),
            ClipboardEntry::Text(_) => false,
        })
    });
}

pub(super) fn warm_thumbnail_handles(app: &mut AppModel) {
    for item in app.history.iter() {
        let ClipboardEntry::Image {
            bytes,
            hash,
            thumbnail_png: Some(thumbnail_png),
            ..
        } = &item.entry
        else {
            continue;
        };

        app.thumbnail_handles
            .entry((*hash, bytes.len()))
            .or_insert_with(|| ImageHandle::from_bytes(thumbnail_png.clone()));
    }
}

pub(super) fn row_has_preview(app: &AppModel, idx: usize) -> bool {
    app.history
        .get(idx)
        .and_then(|item| match &item.entry {
            ClipboardEntry::Text(text) => Some(text_overlay_available(text)),
            ClipboardEntry::Image { .. } => None,
        })
        .unwrap_or(false)
}

pub(super) fn parse_usize_field(input: &str) -> Result<usize, &'static str> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err("value is required");
    }
    trimmed
        .parse::<usize>()
        .map_err(|_| "must be a valid positive integer")
}

pub(super) fn parse_u32_field(input: &str) -> Result<u32, &'static str> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err("value is required");
    }
    trimmed
        .parse::<u32>()
        .map_err(|_| "must be a valid positive integer")
}

use super::super::summary::{
    EXPANDED_MAX_CHARS, summarize_one_line, summarize_one_line_with_limit, text_overlay_available,
};
use crate::app::AppModel;
use crate::app::model::{FocusPart, HistoryItem};
use crate::services::clipboard;
use cosmic::iced::widget::image::Handle as ImageHandle;
use std::hash::{Hash, Hasher};

#[derive(Clone)]
pub(in crate::app) enum RowContent {
    Text {
        collapsed_summary: String,
        expanded_summary: String,
        overlay_available: bool,
    },
    Image {
        mime: String,
        bytes_len: usize,
        content_hash: u64,
        thumbnail_handle: Option<ImageHandle>,
    },
}

#[derive(Clone)]
pub(in crate::app) struct RowRenderState {
    pub(in crate::app) idx: usize,
    pub(in crate::app) pinned: bool,
    pub(in crate::app) text_overlay_open: bool,
    pub(in crate::app) row_is_hovered: bool,
    pub(in crate::app) row_keyboard_focus: Option<FocusPart>,
    pub(in crate::app) hovered_focus: Option<FocusPart>,
    pub(in crate::app) content: RowContent,
}

impl RowRenderState {
    pub(in crate::app) fn from_app(app: &AppModel, idx: usize, item: &HistoryItem) -> Self {
        let row_is_hovered = app.hovered_index == Some(idx);
        let row_keyboard_focus = app
            .keyboard_focus
            .and_then(|(focus_idx, part)| (focus_idx == idx).then_some(part));
        let hovered_focus = app
            .hovered_focus
            .and_then(|(focus_idx, part)| (focus_idx == idx).then_some(part));

        let content = match &item.entry {
            clipboard::ClipboardEntry::Text(text) => RowContent::Text {
                collapsed_summary: summarize_one_line(text.as_str()),
                expanded_summary: summarize_one_line_with_limit(text.as_str(), EXPANDED_MAX_CHARS),
                overlay_available: text_overlay_available(text.as_str()),
            },
            clipboard::ClipboardEntry::Image {
                mime,
                bytes,
                hash,
                thumbnail_png: _,
            } => RowContent::Image {
                mime: mime.clone(),
                bytes_len: bytes.len(),
                content_hash: *hash,
                thumbnail_handle: app.thumbnail_handles.get(&(*hash, bytes.len())).cloned(),
            },
        };

        Self {
            idx,
            pinned: item.pinned,
            text_overlay_open: app.text_overlay_index == Some(idx),
            row_is_hovered,
            row_keyboard_focus,
            hovered_focus,
            content,
        }
    }
}

impl Hash for RowRenderState {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.idx.hash(state);
        self.pinned.hash(state);
        self.row_is_hovered.hash(state);
        self.row_keyboard_focus.hash(state);
        self.hovered_focus.hash(state);

        match &self.content {
            RowContent::Text {
                collapsed_summary,
                expanded_summary,
                overlay_available,
            } => {
                0u8.hash(state);
                collapsed_summary.hash(state);
                expanded_summary.hash(state);
                overlay_available.hash(state);
            }
            RowContent::Image {
                mime,
                bytes_len,
                content_hash,
                thumbnail_handle,
            } => {
                1u8.hash(state);
                mime.hash(state);
                bytes_len.hash(state);
                content_hash.hash(state);
                thumbnail_handle.is_some().hash(state);
            }
        }
    }
}

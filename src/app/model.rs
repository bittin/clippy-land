use crate::services::clipboard;
use crate::services::clipboard::ClipboardEntry;
use crate::settings::AppSettings;
use cosmic::iced::widget::image::Handle as ImageHandle;
use cosmic::iced::window::Id;
use std::collections::HashMap;
use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub(super) struct HistoryItem {
    pub(super) entry: clipboard::ClipboardEntry,
    pub(super) pinned: bool,
}

/// The application model stores app-specific state used to describe its interface
#[derive(Default)]
pub struct AppModel {
    pub(super) core: cosmic::Core,
    pub(super) settings: AppSettings,
    pub(super) popup: Option<Id>,
    /// True when the popup was opened via IPC (layer surface), false for icon click (XDG popup).
    pub(super) popup_is_layer_surface: bool,
    /// Current search query for filtering clipboard history.
    pub(super) search_query: String,
    /// Whether settings panel is visible inside popup.
    pub(super) settings_open: bool,
    /// Draft settings form values (text inputs).
    pub(super) settings_draft: SettingsDraft,
    /// Last settings save/validation error shown in UI.
    pub(super) settings_error: Option<String>,
    /// Latest clipboard entries, newest-first (with pinned items kept at the top).
    pub(super) history: VecDeque<HistoryItem>,
    /// Cached filtered indices for the current query.
    pub(super) filtered_indices: Vec<usize>,
    /// Query value used when `filtered_indices` was last computed.
    pub(super) filtered_query_cache: String,
    /// History length used when `filtered_indices` was last computed.
    pub(super) filtered_history_len_cache: usize,
    /// Cached decoded image handles for thumbnails, keyed by (content hash, byte length).
    pub(super) thumbnail_handles: HashMap<(u64, usize), ImageHandle>,
    /// Index of the history entry the mouse is currently hovering over.
    pub(super) hovered_index: Option<usize>,
    /// The specific part of a row the mouse is hovering over (index, part)
    pub(super) hovered_focus: Option<(usize, FocusPart)>,
    pub(super) at_scroll_bottom: bool,
    /// Last observed history scroll viewport, used to keep keyboard selection in view.
    pub(super) history_viewport: Option<cosmic::iced::widget::scrollable::Viewport>,
    /// Keyboard focus within the history: (index, part) where part is Entry/Pin/Remove
    pub(super) keyboard_focus: Option<(usize, FocusPart)>,
}

#[derive(Debug, Clone, Default)]
pub struct SettingsDraft {
    pub max_history: String,
    pub max_pinned: String,
    pub max_image_bytes: String,
    pub max_image_dimension_px: String,
}

impl SettingsDraft {
    pub fn from_settings(settings: &AppSettings) -> Self {
        Self {
            max_history: settings.max_history.to_string(),
            max_pinned: settings.max_pinned.to_string(),
            max_image_bytes: settings.max_image_bytes.to_string(),
            max_image_dimension_px: settings.max_image_dimension_px.to_string(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FocusPart {
    Entry,
    Pin,
    Remove,
}

impl AppModel {
    pub(super) fn compute_filtered_indices_for(
        history: &VecDeque<HistoryItem>,
        search_query: &str,
    ) -> Vec<usize> {
        let query = search_query.to_lowercase();
        if query.is_empty() {
            return (0..history.len()).collect();
        }

        history
            .iter()
            .enumerate()
            .filter(|(_, item)| match &item.entry {
                ClipboardEntry::Text(text) => text.to_lowercase().contains(&query),
                ClipboardEntry::Image { mime, .. } => mime.to_lowercase().contains(&query),
            })
            .map(|(idx, _)| idx)
            .collect()
    }

    pub(super) fn recompute_filtered_indices(&mut self) {
        self.filtered_indices =
            Self::compute_filtered_indices_for(&self.history, &self.search_query);
        self.filtered_query_cache = self.search_query.clone();
        self.filtered_history_len_cache = self.history.len();
    }
}

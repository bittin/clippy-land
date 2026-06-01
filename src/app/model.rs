use crate::services::clipboard;
use crate::services::clipboard::ClipboardEntry;
use crate::settings::AppSettings;
use cosmic::iced::widget::image::Handle as ImageHandle;
use cosmic::iced::window::Id;
use std::cell::RefCell;
use std::collections::HashMap;
use std::collections::VecDeque;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub(super) struct HistoryItem {
    pub(super) entry: clipboard::ClipboardEntry,
    pub(super) pinned: bool,
}

#[derive(Debug)]
pub(super) struct PopupOpenTrace {
    source: &'static str,
    started_at: Instant,
    history_len_at_request: usize,
    visible_len_at_request: usize,
    first_view_logged: bool,
    opened_logged: bool,
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
    /// Explicit text preview overlay target, if open.
    pub(super) text_overlay_index: Option<usize>,
    /// Pending timing trace for popup open diagnostics.
    pub(super) popup_open_trace: RefCell<Option<PopupOpenTrace>>,
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
    Preview,
    Pin,
    Remove,
}

impl AppModel {
    fn filtered_indices_cache_is_valid(&self) -> bool {
        self.filtered_query_cache == self.search_query
            && self.filtered_history_len_cache == self.history.len()
            && self
                .filtered_indices
                .iter()
                .all(|&idx| idx < self.history.len())
    }

    fn current_filtered_len(&self) -> usize {
        if self.filtered_indices_cache_is_valid() {
            self.filtered_indices.len()
        } else {
            Self::compute_filtered_indices_for(&self.history, &self.search_query).len()
        }
    }

    pub(super) fn popup_open_trace_pending(&self) -> bool {
        self.popup_open_trace.borrow().is_some()
    }

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

    pub(super) fn begin_popup_open_trace(&self, source: &'static str) {
        let trace = PopupOpenTrace {
            source,
            started_at: Instant::now(),
            history_len_at_request: self.history.len(),
            visible_len_at_request: self.current_filtered_len(),
            first_view_logged: false,
            opened_logged: false,
        };

        popup_timing_log(format!(
            "popup requested via {}: history={} visible={} search_len={}",
            source,
            trace.history_len_at_request,
            trace.visible_len_at_request,
            self.search_query.len()
        ));

        *self.popup_open_trace.borrow_mut() = Some(trace);
    }

    pub(super) fn note_popup_view_built(
        &self,
        visible_len_now: usize,
        image_rows_now: usize,
        build_elapsed: Duration,
    ) {
        let mut trace_slot = self.popup_open_trace.borrow_mut();
        let Some(trace) = trace_slot.as_mut() else {
            return;
        };

        if trace.first_view_logged {
            return;
        }

        popup_timing_log(format!(
            "first popup view via {} after {:.2}ms (view_build={:.2}ms, history_at_request={}, visible_at_request={}, visible_now={}, image_rows_now={})",
            trace.source,
            duration_ms(trace.started_at.elapsed()),
            duration_ms(build_elapsed),
            trace.history_len_at_request,
            trace.visible_len_at_request,
            visible_len_now,
            image_rows_now
        ));

        trace.first_view_logged = true;
    }

    pub(super) fn note_popup_opened(&self) {
        let mut trace_slot = self.popup_open_trace.borrow_mut();
        let Some(trace) = trace_slot.as_mut() else {
            return;
        };

        if trace.opened_logged {
            return;
        }

        popup_timing_log(format!(
            "popup window opened via {} after {:.2}ms",
            trace.source,
            duration_ms(trace.started_at.elapsed())
        ));

        trace.opened_logged = true;
    }

    pub(super) fn finish_popup_open_trace_on_redraw(&self) {
        let Some(trace) = self.popup_open_trace.borrow_mut().take() else {
            return;
        };

        popup_timing_log(format!(
            "first popup redraw via {} after {:.2}ms (opened_logged={}, first_view_logged={})",
            trace.source,
            duration_ms(trace.started_at.elapsed()),
            trace.opened_logged,
            trace.first_view_logged
        ));
    }

    pub(super) fn cancel_popup_open_trace(&self, reason: &'static str) {
        let Some(trace) = self.popup_open_trace.borrow_mut().take() else {
            return;
        };

        if trace.first_view_logged || trace.opened_logged {
            popup_timing_log(format!(
                "popup trace via {} cleared after {:.2}ms: {}",
                trace.source,
                duration_ms(trace.started_at.elapsed()),
                reason
            ));
            return;
        }

        popup_timing_log(format!(
            "popup timing via {} cancelled after {:.2}ms: {}",
            trace.source,
            duration_ms(trace.started_at.elapsed()),
            reason
        ));
    }

    #[cfg(test)]
    pub(crate) fn popup_open_trace_pending_for_test(&self) -> bool {
        self.popup_open_trace_pending()
    }
}

fn popup_timing_log(message: impl std::fmt::Display) {
    if std::env::var_os("CLIPPY_LAND_DEBUG_TIMING").is_some() {
        eprintln!("[clippy-land timing] {message}");
    }
}

fn duration_ms(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1000.0
}

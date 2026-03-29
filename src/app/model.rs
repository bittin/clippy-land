use crate::services::clipboard;
use cosmic::iced::window::Id;
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
    pub(super) popup: Option<Id>,
    /// Latest clipboard entries, newest-first (with pinned items kept at the top).
    pub(super) history: VecDeque<HistoryItem>,
    /// Index of the history entry the mouse is currently hovering over.
    pub(super) hovered_index: Option<usize>,
    /// The specific part of a row the mouse is hovering over (index, part)
    pub(super) hovered_focus: Option<(usize, FocusPart)>,
    /// Whether the history list is scrolled to the bottom.
    pub(super) at_scroll_bottom: bool,
    /// Keyboard focus within the history: (index, part) where part is Entry/Pin/Remove
    pub(super) keyboard_focus: Option<(usize, FocusPart)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusPart {
    Entry,
    Pin,
    Remove,
}

use crate::services::clipboard;
use cosmic::iced::window::Id;

/// Messages emitted by the application and its widgets.
#[derive(Debug, Clone)]
pub enum Message {
    TogglePopup,
    PopupClosed(Id),
    ClipboardChanged(clipboard::ClipboardEntry),
    TogglePin(usize),
    RemoveHistory(usize),
    ClearHistory,
    CopyFromHistory(usize),
    HoverEntry(Option<(usize, crate::app::model::FocusPart)>),
    /// Move the selection up (keyboard)
    MoveSelectionUp,
    /// Move the selection down (keyboard)
    MoveSelectionDown,
    /// Move sub-focus left (e.g., to actions)
    MoveFocusLeft,
    /// Move sub-focus right (e.g., to actions)
    MoveFocusRight,
    /// Activate the currently selected entry or focused control (Enter)
    ActivateSelection,
}

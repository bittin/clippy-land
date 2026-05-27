use crate::services::clipboard;
use cosmic::iced::widget::scrollable;
use cosmic::iced::window::Id;

/// Messages emitted by the application and its widgets.
#[derive(Debug, Clone)]
pub enum Message {
    TogglePopup,
    /// Toggle popup triggered externally via the --toggle CLI flag.
    ToggleViaIpc,
    PopupClosed(Id),
    /// Sent when a window loses focus, used to close the layer surface popup.
    WindowUnfocused(Id),
    ClipboardChanged(clipboard::ClipboardEntry),
    TogglePin(usize),
    RemoveHistory(usize),
    ClearHistory,
    CopyFromHistory(usize),
    HoverEntry(Option<(usize, crate::app::model::FocusPart)>),
    HistoryScrolled(scrollable::Viewport),
    /// Search query changed — filters the visible history items.
    SearchChanged(String),
    ToggleSettingsPanel,
    SettingsMaxHistoryChanged(String),
    SettingsMaxPinnedChanged(String),
    SettingsMaxImageBytesChanged(String),
    SettingsMaxImageDimensionChanged(String),
    ApplySettings,
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

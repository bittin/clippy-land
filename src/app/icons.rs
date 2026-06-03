use cosmic::widget;
use std::sync::LazyLock;

static REMOVE_ICON: LazyLock<widget::icon::Handle> =
    LazyLock::new(|| symbolic_icon("edit-delete-symbolic"));
static PIN_ICON: LazyLock<widget::icon::Handle> = LazyLock::new(|| symbolic_icon("pin-symbolic"));
static SEARCH_ICON: LazyLock<widget::icon::Handle> =
    LazyLock::new(|| symbolic_icon("system-search-symbolic"));
static SETTINGS_ICON: LazyLock<widget::icon::Handle> =
    LazyLock::new(|| symbolic_icon("preferences-system-symbolic"));
static CLOSE_ICON: LazyLock<widget::icon::Handle> =
    LazyLock::new(|| symbolic_icon("window-close-symbolic"));

fn symbolic_icon(name: &str) -> widget::icon::Handle {
    widget::icon::from_name(name).handle()
}

pub fn named_symbolic_icon(name: &str) -> widget::icon::Handle {
    match name {
        "edit-delete-symbolic" => REMOVE_ICON.clone(),
        "pin-symbolic" => PIN_ICON.clone(),
        "system-search-symbolic" => SEARCH_ICON.clone(),
        "preferences-system-symbolic" => SETTINGS_ICON.clone(),
        "window-close-symbolic" => CLOSE_ICON.clone(),
        _ => symbolic_icon(name),
    }
}

pub fn remove_icon() -> widget::icon::Handle {
    REMOVE_ICON.clone()
}

pub fn pin_icon() -> widget::icon::Handle {
    PIN_ICON.clone()
}

/// Pinned variant uses the same symbolic icon; button styling controls visual emphasis.
pub fn pin_icon_pinned() -> widget::icon::Handle {
    PIN_ICON.clone()
}

pub fn prewarm_popup_icons() {
    let _ = SETTINGS_ICON.clone();
    let _ = CLOSE_ICON.clone();
    let _ = SEARCH_ICON.clone();
    let _ = PIN_ICON.clone();
    let _ = REMOVE_ICON.clone();
}

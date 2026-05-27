use cosmic::widget;

fn symbolic_icon(name: &str) -> widget::icon::Handle {
    widget::icon::from_name(name).handle()
}

pub fn remove_icon() -> widget::icon::Handle {
    symbolic_icon("edit-delete-symbolic")
}

pub fn pin_icon() -> widget::icon::Handle {
    symbolic_icon("pin-symbolic")
}

/// Pinned variant uses the same symbolic icon; button styling controls visual emphasis.
pub fn pin_icon_pinned() -> widget::icon::Handle {
    symbolic_icon("pin-symbolic")
}

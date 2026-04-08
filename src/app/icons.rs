use cosmic::iced::Color;
use cosmic::widget;

const REMOVE_SVG: &[u8] = include_bytes!("../../resources/icons/user-trash-symbolic.svg");
const PIN_SVG: &[u8] = include_bytes!("../../resources/icons/pin-symbolic.svg");

/// Symbolic SVG — inherits its color from the parent widget's `icon_color`.
/// Works automatically inside any button or container that sets `icon_color`.
fn svg_symbolic(bytes: &'static [u8]) -> widget::icon::Handle {
    widget::icon::from_svg_bytes(bytes).symbolic(true)
}

/// SVG with a specific color applied via hex replacement, used when the
/// desired color cannot come from the parent (e.g. accent-colored pinned state).
fn svg_recolored(bytes: &'static [u8], fg: Color) -> widget::icon::Handle {
    let hex = format!(
        "#{:02x}{:02x}{:02x}",
        (fg.r * 255.0) as u8,
        (fg.g * 255.0) as u8,
        (fg.b * 255.0) as u8,
    );
    let mut svg = String::from_utf8_lossy(bytes).into_owned();
    for color in ["#2e3436", "#2e3434", "#232323"] {
        svg = svg.replace(color, &hex);
    }
    svg = svg.replace("fill-opacity=\"0.34902\"", "fill-opacity=\"1\"");
    svg = svg.replace("fill-opacity=\"0.95\"", "fill-opacity=\"1\"");
    widget::icon::from_svg_bytes(svg.into_bytes())
}

pub fn remove_icon() -> widget::icon::Handle {
    svg_symbolic(REMOVE_SVG)
}

pub fn pin_icon() -> widget::icon::Handle {
    svg_symbolic(PIN_SVG)
}

/// Pinned variant uses the theme accent color to signal active state.
pub fn pin_icon_pinned() -> widget::icon::Handle {
    let accent: Color = cosmic::theme::active().cosmic().accent_color().into();
    svg_recolored(PIN_SVG, accent)
}

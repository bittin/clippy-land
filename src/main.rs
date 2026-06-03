mod app;
mod i18n;
mod ipc;
mod services;
mod settings;

fn main() -> cosmic::iced::Result {
    ensure_default_iced_backend();

    let mut open_popup_on_start = std::env::var_os("COSMIC_PANEL_NAME").is_none();

    for arg in std::env::args().skip(1) {
        if arg == "--toggle" || arg == "-t" {
            if let Err(e) = ipc::send_toggle_signal() {
                eprintln!("Failed to toggle clippy-land: {e}");
                std::process::exit(1);
            }
            return Ok(());
        }

        if arg == "--standalone" {
            open_popup_on_start = true;
            continue;
        }

        if arg == "--no-standalone" {
            open_popup_on_start = false;
            continue;
        }

        if arg == "-h" || arg == "--help" {
            println!("Clippy Land - Clipboard history applet for COSMIC");
            println!();
            println!("USAGE:");
            println!("    cosmic-applet-clippy-land [OPTIONS]");
            println!();
            println!("OPTIONS:");
            println!("    -t, --toggle    Toggle the clipboard popup via keyboard shortcut");
            println!("    --standalone    Open popup window immediately on startup");
            println!("    --no-standalone Do not auto-open popup on startup");
            println!("    -h, --help      Print this help message");
            println!();
            println!("KEYBOARD SHORTCUT SETUP:");
            println!("    1. Open COSMIC Settings > Keyboard > Custom Shortcuts");
            println!("    2. Click 'Add Custom Shortcut'");
            println!("    3. Name:    Clipboard History");
            println!("    4. Command: cosmic-applet-clippy-land --toggle");
            println!("    5. Shortcut: Press Super+V (or your preferred shortcut)");
            return Ok(());
        }
    }

    let requested_languages = i18n_embed::DesktopLanguageRequester::requested_languages();
    i18n::init(&requested_languages);
    cosmic::applet::run::<app::AppModel>(app::AppFlags {
        open_popup_on_start,
    })
}

fn ensure_default_iced_backend() {
    if std::env::var_os("ICED_BACKEND").is_some() {
        backend_timing_log("startup keeping existing ICED_BACKEND");
        return;
    }

    // SAFETY: This runs at process startup on the main thread before the app runtime,
    // worker threads, or GUI event loop are initialized, so mutating the process
    // environment here cannot race with concurrent environment access.
    unsafe {
        std::env::set_var("ICED_BACKEND", default_iced_backend());
    }

    backend_timing_log("startup defaulted ICED_BACKEND=tiny-skia");
}

const fn default_iced_backend() -> &'static str {
    "tiny-skia"
}

fn backend_timing_log(message: &str) {
    if std::env::var_os("CLIPPY_LAND_DEBUG_TIMING").is_some() {
        eprintln!("[clippy-land timing] {message}");
    }
}

#[cfg(test)]
mod tests {
    use super::default_iced_backend;

    #[test]
    fn default_backend_matches_fast_panel_runtime_choice() {
        assert_eq!(default_iced_backend(), "tiny-skia");
    }
}

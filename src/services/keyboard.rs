//! Synthesized paste keystrokes via the `zwp_virtual_keyboard_v1` protocol.
//!
//! When an entry is activated, the popup is destroyed first so the compositor
//! can restore focus. Normal apps receive Ctrl+V; known terminals receive
//! Ctrl+Shift+V so their own clipboard/bracketed-paste handling also works for
//! applications such as nvim. Clipboard text is never typed as individual keys:
//! newlines could execute shell commands, and nvim may be in normal mode.
//!
//! The compositor must advertise `zwp_virtual_keyboard_manager_v1`. COSMIC
//! permits it on the panel-granted privileged connection, including for Flatpak
//! applets. Without that grant/protocol, selection gracefully remains copy-only.

use std::fs::File;
use std::io::Write;
use std::os::fd::AsFd;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use wayland_client::globals::{GlobalList, GlobalListContents, registry_queue_init};
use wayland_client::protocol::wl_registry::WlRegistry;
use wayland_client::protocol::wl_seat::WlSeat;
use wayland_client::{Connection, Dispatch, EventQueue, QueueHandle};
use wayland_protocols_misc::zwp_virtual_keyboard_v1::client::{
    zwp_virtual_keyboard_manager_v1::ZwpVirtualKeyboardManagerV1,
    zwp_virtual_keyboard_v1::ZwpVirtualKeyboardV1,
};

/// Linux evdev keycode for `KEY_V` (XKB keycode 55 in the shortcut keymap).
const KEY_V: u32 = 47;
const KEY_LEFTCTRL: u32 = 29;
const KEY_LEFTSHIFT: u32 = 42;
/// `wl_keyboard` key state: released.
const KEY_STATE_RELEASED: u32 = 0;
/// `wl_keyboard` key state: pressed.
const KEY_STATE_PRESSED: u32 = 1;
/// `wl_keyboard` keymap format enum value for `xkb_keymap_format_text_v1`.
const KEYMAP_FORMAT_XKB_V1: u32 = 1;

/// How long a synthesized key press is reported as held (timestamps only).
const KEY_HOLD_MS: u32 = 40;

/// Give the compositor time to destroy the popup and restore keyboard focus.
const DEFAULT_PASTE_DELAY_MS: u64 = 250;
/// Upper bound for the `CLIPPY_LAND_PASTE_DELAY_MS` debug override.
const MAX_PASTE_DELAY_MS: u64 = 5_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PasteShortcut {
    CtrlV,
    CtrlShiftV,
}

impl PasteShortcut {
    fn modifier_keys(self) -> &'static [u32] {
        match self {
            Self::CtrlV => &[KEY_LEFTCTRL],
            Self::CtrlShiftV => &[KEY_LEFTCTRL, KEY_LEFTSHIFT],
        }
    }

    fn for_app_id(app_id: Option<&str>) -> Self {
        // Match application identities, never window titles: a browser page or
        // document mentioning a terminal must not change its paste shortcut.
        let app_id = app_id.unwrap_or_default().to_ascii_lowercase();
        match app_id.as_str() {
            "com.system76.cosmicterm"
            | "alacritty"
            | "org.alacritty.alacritty"
            | "kitty"
            | "net.kovidgoyal.kitty"
            | "foot"
            | "footclient"
            | "org.codeberg.dnkl.foot"
            | "gnome-terminal"
            | "gnome-terminal-server"
            | "org.gnome.terminal"
            | "kgx"
            | "org.gnome.console"
            | "konsole"
            | "org.kde.konsole"
            | "wezterm"
            | "org.wezfurlong.wezterm"
            | "org.wezterm.wezterm"
            | "ghostty"
            | "com.mitchellh.ghostty"
            | "xfce4-terminal"
            | "terminator"
            | "tilix"
            | "com.gexperts.tilix"
            | "blackbox"
            | "com.raggesilver.blackbox" => Self::CtrlShiftV,
            _ => Self::CtrlV,
        }
    }
}

static PROCESS_START: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();

/// Monotonic millisecond timestamp suitable for `wl_keyboard.key` events.
fn monotonic_ms() -> u32 {
    let base = PROCESS_START.get_or_init(Instant::now);
    base.elapsed().as_millis() as u32
}

/// How long to wait after closing the popup before injecting a paste shortcut.
///
/// `CLIPPY_LAND_PASTE_DELAY_MS` overrides the default for debugging.
pub fn paste_delay() -> Duration {
    std::env::var("CLIPPY_LAND_PASTE_DELAY_MS")
        .ok()
        .and_then(|raw| raw.parse::<u64>().ok())
        .filter(|&ms| ms <= MAX_PASTE_DELAY_MS)
        .map(Duration::from_millis)
        .unwrap_or_else(|| Duration::from_millis(DEFAULT_PASTE_DELAY_MS))
}

/// Bind once: the virtual-keyboard manager has no destructor, so rebinding it
/// on every paste would leak objects on the shared, process-long connection.
/// The mutex also prevents overlapping modifier/key sequences.
static KEYBOARD: OnceLock<Mutex<Option<KeyboardClient>>> = OnceLock::new();

struct KeyboardClient {
    conn: Connection,
    queue: EventQueue<KeyboardState>,
    globals: GlobalList,
    manager: Option<ZwpVirtualKeyboardManagerV1>,
    seat: Option<WlSeat>,
}

impl KeyboardClient {
    fn connect() -> Result<Self, String> {
        let conn = super::wayland::connection()?;
        let (globals, queue) = registry_queue_init::<KeyboardState>(&conn)
            .map_err(|err| format!("registry: {err}"))?;
        Ok(Self {
            conn,
            queue,
            globals,
            manager: None,
            seat: None,
        })
    }

    fn bind_keyboard(&mut self) -> Result<(), String> {
        if self.manager.is_some() {
            return Ok(());
        }
        // Refresh globals for retries, but retain every successful binding so
        // neither missing protocols nor partial setup leak objects on retry.
        self.queue
            .roundtrip(&mut KeyboardState)
            .map_err(|err| format!("registry refresh: {err}"))?;
        let qh = self.queue.handle();
        if self.seat.is_none() {
            self.seat = Some(
                self.globals
                    .bind(&qh, 1..=7, ())
                    .map_err(|err| format!("wl_seat bind: {err}"))?,
            );
        }
        self.manager = Some(self.globals.bind(&qh, 1..=1, ()).map_err(|_| {
            "zwp_virtual_keyboard_manager_v1 unavailable (unsupported compositor or sandboxed client)".to_string()
        })?);
        Ok(())
    }

    fn send_paste(&mut self, shortcut: PasteShortcut) -> Result<(), String> {
        self.bind_keyboard()?;
        // Do fallible local work before creating a protocol object.
        let (keymap_text, modifier_mask) = compile_paste_keymap(shortcut)?;
        let mut keymap_bytes = keymap_text.into_bytes();
        // XKB text keymaps are NUL-terminated, including the advertised size.
        keymap_bytes.push(0);
        let keymap_file = create_memfd_with(&keymap_bytes)?;
        let keyboard = self
            .manager
            .as_ref()
            .expect("manager bound above")
            .create_virtual_keyboard(
                self.seat.as_ref().expect("seat bound above"),
                &self.queue.handle(),
                (),
            );
        keyboard.keymap(
            KEYMAP_FORMAT_XKB_V1,
            keymap_file.as_fd(),
            keymap_bytes.len() as u32,
        );

        let result = self.send_keys(&keyboard, shortcut, modifier_mask);
        // Destroy even if a roundtrip fails, rather than leaving a virtual
        // keyboard (and possibly its modifiers) alive.
        keyboard.destroy();
        self.conn
            .flush()
            .map_err(|err| format!("wayland flush: {err}"))?;
        result
    }

    fn send_keys(
        &mut self,
        keyboard: &ZwpVirtualKeyboardV1,
        shortcut: PasteShortcut,
        modifier_mask: u32,
    ) -> Result<(), String> {
        let mut state = KeyboardState;
        self.queue
            .roundtrip(&mut state)
            .map_err(|err| format!("keymap roundtrip: {err}"))?;
        let press_time = monotonic_ms();
        // COSMIC activates the virtual device's keymap on its first key event,
        // which resets a modifier mask sent before that event. Real modifier
        // key presses both activate the map and establish the intended state.
        for &key in shortcut.modifier_keys() {
            keyboard.key(press_time, key, KEY_STATE_PRESSED);
        }
        keyboard.modifiers(modifier_mask, 0, 0, 0);
        keyboard.key(press_time, KEY_V, KEY_STATE_PRESSED);
        let press_result = self
            .queue
            .roundtrip(&mut state)
            .map_err(|err| format!("key press roundtrip: {err}"));
        if press_result.is_ok() {
            std::thread::sleep(Duration::from_millis(2));
        }
        // Always attempt every release, even if the press roundtrip failed.
        let release_time = press_time.wrapping_add(KEY_HOLD_MS);
        keyboard.key(release_time, KEY_V, KEY_STATE_RELEASED);
        for &key in shortcut.modifier_keys().iter().rev() {
            keyboard.key(release_time, key, KEY_STATE_RELEASED);
        }
        keyboard.modifiers(0, 0, 0, 0);
        let release_result = self
            .queue
            .roundtrip(&mut state)
            .map_err(|err| format!("key release roundtrip: {err}"));
        press_result.and(release_result).map(|_| ())
    }
}

/// Look up the focused app only after the popup has closed and the paste delay
/// has elapsed. A snapshot taken when the popup holds focus can be empty, and
/// one taken before opening it can be stale if the user changes windows.
fn send_paste(tracker: &super::toplevel::Tracker) -> Result<(), String> {
    let mut client = KEYBOARD
        .get_or_init(|| Mutex::new(None))
        .lock()
        .map_err(|_| "virtual keyboard lock poisoned".to_string())?;
    if client.is_none() {
        // A failed connection must not permanently disable future pastes.
        *client = Some(KeyboardClient::connect()?);
    }
    let snapshot = tracker.snapshot();
    let app_id = snapshot
        .as_ref()
        .and_then(|snapshot| snapshot.focused_app_id.as_deref());
    let shortcut = PasteShortcut::for_app_id(app_id);
    if std::env::var_os("CLIPPY_LAND_DEBUG_CLIPBOARD").is_some() {
        eprintln!("[clippy-land] paste shortcut: {shortcut:?}, app_id={app_id:?}");
    }
    client
        .as_mut()
        .expect("keyboard initialized above")
        .send_paste(shortcut)
}

/// Wait for focus to return, then ask the focused app to paste the clipboard.
pub async fn delayed_paste(tracker: super::toplevel::Tracker) {
    tokio::time::sleep(paste_delay()).await;
    let result = tokio::task::spawn_blocking(move || send_paste(&tracker))
        .await
        .unwrap_or_else(|err| Err(format!("virtual keyboard task failed: {err}")));
    if let Err(err) = result {
        log_paste_failure(&err);
    }
}

fn log_paste_failure(reason: &str) {
    if std::env::var_os("CLIPPY_LAND_DEBUG_CLIPBOARD").is_some() {
        eprintln!("[clippy-land] paste not synthesized: {reason}");
    }
}

/// Use a deterministic US keymap for this shortcut-only virtual device. The
/// physical KEY_V position need not produce 'v' in XKB_DEFAULT_LAYOUT (Dvorak,
/// for example), and a Shift override must not change the intended shortcut.
fn compile_paste_keymap(shortcut: PasteShortcut) -> Result<(String, u32), String> {
    let context = xkbcommon::xkb::Context::new(xkbcommon::xkb::CONTEXT_NO_FLAGS);
    let keymap = xkbcommon::xkb::Keymap::new_from_names(
        &context,
        "evdev",
        "pc105",
        "us",
        "",
        Some("".to_string()),
        xkbcommon::xkb::KEYMAP_COMPILE_NO_FLAGS,
    )
    .ok_or_else(|| "failed to compile xkb keymap".to_string())?;

    let modifier_mask = |name| {
        let index = keymap.mod_get_index(name);
        if index >= u32::BITS {
            Err(format!("xkb keymap has no {name} modifier"))
        } else {
            Ok(1u32 << index)
        }
    };
    let mut mask = modifier_mask(xkbcommon::xkb::MOD_NAME_CTRL)?;
    if shortcut == PasteShortcut::CtrlShiftV {
        mask |= modifier_mask(xkbcommon::xkb::MOD_NAME_SHIFT)?;
    }

    let text = keymap.get_as_string(xkbcommon::xkb::KEYMAP_FORMAT_TEXT_V1);
    Ok((text, mask))
}

fn create_memfd_with(contents: &[u8]) -> Result<File, String> {
    let fd = rustix::fs::memfd_create("clippy-land-keymap", rustix::fs::MemfdFlags::CLOEXEC)
        .map_err(|err| format!("memfd_create: {err}"))?;

    let mut file = File::from(fd);
    file.write_all(contents)
        .map_err(|err| format!("keymap write: {err}"))?;
    file.flush().map_err(|err| format!("keymap flush: {err}"))?;
    Ok(file)
}

#[derive(Default)]
struct KeyboardState;

impl Dispatch<WlRegistry, GlobalListContents> for KeyboardState {
    fn event(
        _state: &mut Self,
        _proxy: &WlRegistry,
        _event: <WlRegistry as wayland_client::Proxy>::Event,
        _data: &GlobalListContents,
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<WlSeat, ()> for KeyboardState {
    fn event(
        _state: &mut Self,
        _proxy: &WlSeat,
        _event: <WlSeat as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ZwpVirtualKeyboardManagerV1, ()> for KeyboardState {
    fn event(
        _state: &mut Self,
        _proxy: &ZwpVirtualKeyboardManagerV1,
        _event: <ZwpVirtualKeyboardManagerV1 as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ZwpVirtualKeyboardV1, ()> for KeyboardState {
    fn event(
        _state: &mut Self,
        _proxy: &ZwpVirtualKeyboardV1,
        _event: <ZwpVirtualKeyboardV1 as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;

    #[test]
    fn terminals_use_their_clipboard_paste_shortcut() {
        for app_id in [
            "com.system76.CosmicTerm",
            "Alacritty",
            "org.alacritty.Alacritty",
            "kitty",
            "foot",
            "footclient",
            "org.gnome.Terminal",
            "org.gnome.Console",
            "org.kde.konsole",
            "org.wezfurlong.wezterm",
            "com.mitchellh.ghostty",
            "xfce4-terminal",
            "terminator",
            "com.gexperts.Tilix",
            "com.raggesilver.BlackBox",
        ] {
            assert_eq!(
                PasteShortcut::for_app_id(Some(app_id)),
                PasteShortcut::CtrlShiftV,
                "{app_id}"
            );
        }
    }

    #[test]
    fn other_apps_and_missing_tracking_keep_ctrl_v() {
        for app_id in [
            None,
            Some(""),
            Some("firefox"),
            Some("org.gnome.TextEditor"),
            Some("unknown"),
            Some("not-com.system76.CosmicTerm"),
        ] {
            assert_eq!(
                PasteShortcut::for_app_id(app_id),
                PasteShortcut::CtrlV,
                "{app_id:?}"
            );
        }
    }

    #[test]
    fn paste_keymap_produces_v_and_the_requested_modifiers() {
        for (shortcut, expected_mask, expected_keysym) in [
            (PasteShortcut::CtrlV, 1 << 2, 'v' as u32),
            (PasteShortcut::CtrlShiftV, (1 << 2) | 1, 'V' as u32),
        ] {
            let (text, mask) = compile_paste_keymap(shortcut).expect("keymap should compile");
            assert_eq!(mask, expected_mask);
            let context = xkbcommon::xkb::Context::new(xkbcommon::xkb::CONTEXT_NO_FLAGS);
            let keymap = xkbcommon::xkb::Keymap::new_from_string(
                &context,
                text,
                xkbcommon::xkb::KEYMAP_FORMAT_TEXT_V1,
                xkbcommon::xkb::KEYMAP_COMPILE_NO_FLAGS,
            )
            .expect("uploaded keymap should compile");
            let mut state = xkbcommon::xkb::State::new(&keymap);
            for &key in shortcut.modifier_keys() {
                state.update_key((key + 8).into(), xkbcommon::xkb::KeyDirection::Down);
            }
            assert_eq!(
                state.serialize_mods(xkbcommon::xkb::STATE_MODS_DEPRESSED),
                mask
            );
            state.update_mask(mask, 0, 0, 0, 0, 0);
            assert_eq!(
                state.key_get_one_sym((KEY_V + 8).into()).raw(),
                expected_keysym
            );
            for &key in shortcut.modifier_keys().iter().rev() {
                state.update_key((key + 8).into(), xkbcommon::xkb::KeyDirection::Up);
            }
            assert_eq!(
                state.serialize_mods(xkbcommon::xkb::STATE_MODS_DEPRESSED),
                0
            );
        }
    }

    #[test]
    fn memfd_receives_keymap_contents() {
        let file = create_memfd_with(b"xkb_keymap { }").expect("memfd should be writable");

        use std::os::fd::AsRawFd;
        let mut reopened = File::open(format!("/proc/self/fd/{}", file.as_raw_fd()))
            .expect("memfd should be re-openable via /proc");
        let mut contents = String::new();
        reopened
            .read_to_string(&mut contents)
            .expect("memfd contents should be readable");
        assert_eq!(contents, "xkb_keymap { }");
    }

    #[test]
    fn paste_delay_defaults_and_honors_override() {
        let previous = std::env::var_os("CLIPPY_LAND_PASTE_DELAY_MS");
        unsafe { std::env::remove_var("CLIPPY_LAND_PASTE_DELAY_MS") };
        assert_eq!(paste_delay(), Duration::from_millis(DEFAULT_PASTE_DELAY_MS));

        // The override is read per call, so a stale value is not cached.
        unsafe { std::env::set_var("CLIPPY_LAND_PASTE_DELAY_MS", "1000") };
        assert_eq!(paste_delay(), Duration::from_millis(1000));

        unsafe { std::env::set_var("CLIPPY_LAND_PASTE_DELAY_MS", "999999999") };
        assert_eq!(paste_delay(), Duration::from_millis(DEFAULT_PASTE_DELAY_MS));

        match previous {
            Some(value) => unsafe { std::env::set_var("CLIPPY_LAND_PASTE_DELAY_MS", value) },
            None => unsafe { std::env::remove_var("CLIPPY_LAND_PASTE_DELAY_MS") },
        }
    }

    /// Focus a dedicated sink (e.g. `cat > /tmp/clippy-paste-output`) first.
    /// Set CLIPPY_LAND_LIVE_PASTE=1, CLIPPY_LAND_LIVE_PASTE_APP_ID to its app ID,
    /// CLIPPY_LAND_LIVE_PASTE_TEXT to the payload (ending in a newline for cat),
    /// and CLIPPY_LAND_LIVE_PASTE_OUTPUT to the empty sink file's path. Verifies
    /// two successive pastes through the same cached virtual-keyboard client.
    #[tokio::test]
    #[ignore = "requires a live Wayland session and a dedicated focused text sink"]
    async fn live_paste_into_focused_window() {
        // `just test-wayland` runs all ignored tests. Never inject input unless
        // this destructive UI test was explicitly enabled for a prepared sink.
        if std::env::var_os("CLIPPY_LAND_LIVE_PASTE").is_none() {
            return;
        }
        let tracker = super::super::toplevel::Tracker::started();
        let expected_app_id =
            std::env::var("CLIPPY_LAND_LIVE_PASTE_APP_ID").expect("set expected sink app ID");
        tokio::time::sleep(Duration::from_millis(500)).await;
        let snapshot = tracker.snapshot().expect("tracker should be ready");
        assert_eq!(
            snapshot.focused_app_id.as_deref(),
            Some(expected_app_id.as_str())
        );
        let text = std::env::var("CLIPPY_LAND_LIVE_PASTE_TEXT").expect("set live paste payload");
        let output = std::env::var("CLIPPY_LAND_LIVE_PASTE_OUTPUT").expect("set sink output path");
        assert!(
            std::fs::read(&output)
                .expect("sink must already exist")
                .is_empty(),
            "sink must start empty"
        );
        assert!(super::super::clipboard::write_clipboard_text(&text));
        tokio::time::sleep(paste_delay()).await;
        for _ in 0..2 {
            let tracker = tracker.clone();
            tokio::task::spawn_blocking(move || send_paste(&tracker))
                .await
                .unwrap()
                .expect("paste should succeed");
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        let expected = text.repeat(2);
        // Submission alone does not prove a paste. Keep the source alive and
        // compare the receiving sink's actual bytes, detecting missing/wrong
        // shortcuts, duplicate input, and accidental newline transformations.
        for _ in 0..40 {
            tokio::time::sleep(Duration::from_millis(50)).await;
            if std::fs::read(&output).unwrap_or_default() == expected.as_bytes() {
                break;
            }
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert_eq!(std::fs::read(&output).unwrap(), expected.as_bytes());
    }
}

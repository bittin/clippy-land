//! IPC mechanism for external toggle functionality via file-based signaling.
//!
//! When the `--toggle` command is invoked, it writes a timestamp to a signal file
//! in XDG_RUNTIME_DIR. A 250ms polling loop in the running applet detects the file,
//! deletes it, and sends a ToggleViaIpc message to open/close the popup.

use std::fs;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::time::SystemTime;

use crate::app::Message;
use cosmic::iced::Subscription;
use cosmic::iced::futures::SinkExt;
use cosmic::iced::futures::channel::mpsc;
use cosmic::iced::stream::channel;

/// Get the signal file path for IPC toggle functionality.
/// Returns None if XDG_RUNTIME_DIR is not set.
pub fn get_signal_file_path() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("CLIPPY_LAND_SIGNAL_FILE") {
        let path = PathBuf::from(path);
        if !path.as_os_str().is_empty() {
            return Some(path);
        }
    }

    std::env::var("XDG_RUNTIME_DIR")
        .ok()
        .map(|runtime_dir| PathBuf::from(runtime_dir).join("clippy-land-toggle"))
}

/// Send a toggle signal by writing a timestamp to the signal file.
pub fn send_toggle_signal() -> std::io::Result<()> {
    let signal_file = get_signal_file_path().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "XDG_RUNTIME_DIR not set - cannot send toggle signal",
        )
    })?;

    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?
        .as_millis()
        .to_string();

    fs::write(&signal_file, timestamp)?;
    Ok(())
}

struct SignalFileWatcher;

impl Hash for SignalFileWatcher {
    fn hash<H: Hasher>(&self, state: &mut H) {
        "clippy-land-signal-file-watcher".hash(state);
    }
}

/// Poll for the signal file at a fixed interval (250ms).
///
/// Uses simple polling instead of filesystem notifications to avoid CPU busy-loops
/// caused by inotify on a busy /run/user/ directory.
pub fn signal_file_watcher() -> Subscription<Message> {
    Subscription::run_with(SignalFileWatcher, |_| {
        channel(1, |mut output: mpsc::Sender<Message>| async move {
            let signal_file = match get_signal_file_path() {
                Some(path) => path,
                None => {
                    futures_util::future::pending::<()>().await;
                    unreachable!();
                }
            };

            loop {
                tokio::time::sleep(tokio::time::Duration::from_millis(250)).await;
                if signal_file.exists() {
                    let _ = std::fs::remove_file(&signal_file);
                    output.send(Message::ToggleViaIpc).await.ok();
                }
            }
        })
    })
}

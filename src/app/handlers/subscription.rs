use crate::app::{AppModel, Message};
use crate::ipc;
use crate::services::clipboard;
use cosmic::iced::Subscription;
use cosmic::iced::futures::channel::mpsc;
use futures_util::SinkExt;
use std::time::Duration;

pub(super) fn subscription(app: &AppModel) -> Subscription<Message> {
    struct ClipboardSubscription;

    let mut subs: Vec<Subscription<Message>> = vec![
        Subscription::run_with(std::any::TypeId::of::<ClipboardSubscription>(), |_| {
            cosmic::iced::stream::channel(1, move |mut channel: mpsc::Sender<Message>| async move {
                let mut last_seen: Option<clipboard::ClipboardFingerprint> = None;

                loop {
                    tokio::time::sleep(Duration::from_millis(500)).await;

                    let next = tokio::task::spawn_blocking(clipboard::read_clipboard_entry)
                        .await
                        .ok()
                        .flatten();

                    let Some(next) = next else {
                        continue;
                    };

                    let next_fp = next.fingerprint();
                    if last_seen.as_ref() == Some(&next_fp) {
                        continue;
                    }

                    last_seen = Some(next_fp);

                    if channel.send(Message::ClipboardChanged(next)).await.is_err() {
                        break;
                    }
                }
            })
        }),
        ipc::signal_file_watcher(),
    ];

    if app.popup.is_some() {
        use cosmic::iced::core::keyboard;
        use cosmic::iced::core::keyboard::key::Named as NamedKey;
        use cosmic::iced::event::{listen_raw, listen_with};
        use cosmic::iced::{Event, event};

        let unfocus_sub = listen_with(|event, _status, window_id| {
            if let Event::Window(cosmic::iced::window::Event::Unfocused) = event {
                Some(Message::WindowUnfocused(window_id))
            } else {
                None
            }
        });
        subs.push(unfocus_sub);

        if app.popup_open_trace_pending() {
            let popup_lifecycle_sub = listen_with(|event, _status, window_id| match event {
                Event::Window(cosmic::iced::window::Event::Opened { .. }) => {
                    Some(Message::PopupOpened(window_id))
                }
                Event::Window(cosmic::iced::window::Event::RedrawRequested(_)) => {
                    Some(Message::PopupRedraw(window_id))
                }
                _ => None,
            });
            subs.push(popup_lifecycle_sub);
        }

        let key_sub = listen_raw(move |event, status, _| {
            if event::Status::Ignored != status {
                return None;
            }

            match event {
                Event::Keyboard(keyboard::Event::KeyPressed {
                    key: keyboard::Key::Named(named),
                    ..
                }) => match named {
                    NamedKey::ArrowUp => return Some(Message::MoveSelectionUp),
                    NamedKey::ArrowDown => return Some(Message::MoveSelectionDown),
                    NamedKey::ArrowLeft => return Some(Message::MoveFocusLeft),
                    NamedKey::ArrowRight => return Some(Message::MoveFocusRight),
                    NamedKey::Enter => return Some(Message::ActivateSelection),
                    NamedKey::Escape => return Some(Message::EscapePressed),
                    _ => (),
                },
                Event::Keyboard(keyboard::Event::KeyPressed {
                    key: keyboard::Key::Character(c),
                    physical_key,
                    ..
                }) => {
                    let key_obj = keyboard::Key::Character(c.clone());
                    if let Some(ch) = key_obj.to_latin(physical_key) {
                        match ch {
                            'j' | 'J' => return Some(Message::MoveSelectionDown),
                            'k' | 'K' => return Some(Message::MoveSelectionUp),
                            'h' | 'H' => return Some(Message::MoveFocusLeft),
                            'l' | 'L' => return Some(Message::MoveFocusRight),
                            '\n' | '\r' => return Some(Message::ActivateSelection),
                            _ => (),
                        }
                    }
                }
                _ => (),
            }

            None
        });
        subs.push(key_sub);
    }

    Subscription::batch(subs)
}

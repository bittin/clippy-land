#[test]
fn ipc_signal_path_is_none_when_env_var_unset() {
    unsafe { std::env::remove_var("CLIPPY_LAND_SIGNAL_FILE") };

    let saved = std::env::var("XDG_RUNTIME_DIR").ok();
    unsafe { std::env::remove_var("XDG_RUNTIME_DIR") };

    let result = crate::ipc::get_signal_file_path();
    assert!(result.is_none());

    if let Some(val) = saved {
        unsafe { std::env::set_var("XDG_RUNTIME_DIR", val) };
    }
}

#[test]
fn ipc_signal_path_appends_filename_to_runtime_dir() {
    unsafe { std::env::remove_var("CLIPPY_LAND_SIGNAL_FILE") };
    unsafe { std::env::set_var("XDG_RUNTIME_DIR", "/tmp/test-runtime") };

    let result = crate::ipc::get_signal_file_path();

    unsafe { std::env::remove_var("XDG_RUNTIME_DIR") };

    let path = result.expect("should return a path");
    assert_eq!(
        path.file_name().and_then(|n| n.to_str()),
        Some("clippy-land-toggle")
    );
    assert!(path.to_string_lossy().starts_with("/tmp/test-runtime"));
}

#[test]
fn ipc_signal_path_prefers_override_env_var() {
    unsafe {
        std::env::set_var("CLIPPY_LAND_SIGNAL_FILE", "/tmp/clippy-land-test-signal");
        std::env::set_var("XDG_RUNTIME_DIR", "/tmp/test-runtime-ignored");
    }

    let result = crate::ipc::get_signal_file_path();

    unsafe {
        std::env::remove_var("CLIPPY_LAND_SIGNAL_FILE");
        std::env::remove_var("XDG_RUNTIME_DIR");
    }

    let path = result.expect("override signal path should be returned");
    assert_eq!(
        path,
        std::path::PathBuf::from("/tmp/clippy-land-test-signal")
    );
}

#[test]
fn ipc_signal_payload_parser_accepts_millisecond_timestamp() {
    let parsed = crate::ipc::parse_signal_timestamp_ms_for_test("123456789");
    assert_eq!(parsed, Some(123456789));

    let invalid = crate::ipc::parse_signal_timestamp_ms_for_test("not-a-timestamp");
    assert_eq!(invalid, None);
}

#[test]
fn send_toggle_signal_writes_parseable_timestamp_payload() {
    use std::time::{SystemTime, UNIX_EPOCH};

    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be after unix epoch")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("clippy-land-signal-{unique}"));

    unsafe {
        std::env::set_var("CLIPPY_LAND_SIGNAL_FILE", &path);
    }

    crate::ipc::send_toggle_signal().expect("toggle signal should be written");

    let payload = std::fs::read_to_string(&path).expect("signal payload should be readable");
    let parsed = crate::ipc::parse_signal_timestamp_ms_for_test(payload.trim());
    assert!(
        parsed.is_some(),
        "payload should contain a millisecond timestamp"
    );

    let _ = std::fs::remove_file(&path);
    unsafe {
        std::env::remove_var("CLIPPY_LAND_SIGNAL_FILE");
    }
}

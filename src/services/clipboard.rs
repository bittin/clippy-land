use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};

use wl_clipboard_rs::{
    copy::{MimeType as CopyMimeType, Options as CopyOptions, Source},
    paste::{get_contents, ClipboardType, MimeType as PasteMimeType, Seat},
};

const MAX_IMAGE_BYTES: usize = 8 * 1024 * 1024;
const THUMBNAIL_SIZE_PX: u32 = 400;

#[derive(Debug, Clone)]
pub enum ClipboardEntry {
    Text(String),
    Image {
        mime: String,
        bytes: Vec<u8>,
        hash: u64,
        thumbnail_png: Option<Vec<u8>>,
    },
}

impl ClipboardEntry {
    pub fn fingerprint(&self) -> ClipboardFingerprint {
        match self {
            ClipboardEntry::Text(text) => ClipboardFingerprint::Text(text.clone()),
            ClipboardEntry::Image {
                mime,
                bytes,
                hash,
                thumbnail_png: _,
            } => ClipboardFingerprint::Image {
                mime: mime.clone(),
                bytes_len: bytes.len(),
                hash: *hash,
            },
        }
    }
}

impl PartialEq for ClipboardEntry {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (ClipboardEntry::Text(a), ClipboardEntry::Text(b)) => a == b,
            (
                ClipboardEntry::Image {
                    mime: am,
                    bytes: ab,
                    hash: ah,
                    ..
                },
                ClipboardEntry::Image {
                    mime: bm,
                    bytes: bb,
                    hash: bh,
                    ..
                },
            ) => ah == bh && am == bm && ab.len() == bb.len(),
            _ => false,
        }
    }
}

impl Eq for ClipboardEntry {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClipboardFingerprint {
    Text(String),
    Image {
        mime: String,
        bytes_len: usize,
        hash: u64,
    },
}

pub fn read_clipboard_entry() -> Option<ClipboardEntry> {
    read_clipboard_image()
        .or_else(read_clipboard_image_from_uri_list)
        .or_else(|| read_clipboard_text().map(ClipboardEntry::Text))
}

pub fn read_clipboard_text() -> Option<String> {
    let result = get_contents(
        ClipboardType::Regular,
        Seat::Unspecified,
        PasteMimeType::Text,
    );

    let (mut pipe, _) = match result {
        Ok(ok) => ok,
        Err(err) => {
            if std::env::var_os("CLIPPY_LAND_DEBUG_CLIPBOARD").is_some() {
                eprintln!("[clippy-land] clipboard read get_contents error: {err:?}");
            }
            return None;
        }
    };

    let mut bytes = Vec::new();
    if let Err(err) = pipe.read_to_end(&mut bytes) {
        if std::env::var_os("CLIPPY_LAND_DEBUG_CLIPBOARD").is_some() {
            eprintln!("[clippy-land] clipboard read pipe error: {err:?}");
        }
        return None;
    }

    let text = match String::from_utf8(bytes) {
        Ok(ok) => ok,
        Err(err) => {
            if std::env::var_os("CLIPPY_LAND_DEBUG_CLIPBOARD").is_some() {
                eprintln!("[clippy-land] clipboard read utf8 error: {err:?}");
            }
            return None;
        }
    };
    let text = text.trim_end_matches(['\n', '\r']).to_string();
    (!text.is_empty()).then_some(text)
}

pub fn read_clipboard_image() -> Option<ClipboardEntry> {
    // Try common image formats first.
    const IMAGE_MIMES: [&str; 3] = ["image/png", "image/jpeg", "image/webp"];

    for mime in IMAGE_MIMES {
        let result = get_contents(
            ClipboardType::Regular,
            Seat::Unspecified,
            PasteMimeType::Specific(mime),
        );

        let (pipe, actual_mime) = match result {
            Ok(ok) => ok,
            Err(_) => continue,
        };

        let mut bytes = Vec::new();
        let mut limited = pipe.take((MAX_IMAGE_BYTES + 1) as u64);
        if limited.read_to_end(&mut bytes).is_err() {
            continue;
        }
        if bytes.len() > MAX_IMAGE_BYTES {
            if std::env::var_os("CLIPPY_LAND_DEBUG_CLIPBOARD").is_some() {
                eprintln!(
                    "[clippy-land] clipboard image ignored (too large): {} bytes (max {})",
                    bytes.len(),
                    MAX_IMAGE_BYTES
                );
            }
            continue;
        }
        if bytes.is_empty() {
            continue;
        }

        let mut hasher = DefaultHasher::new();
        actual_mime.hash(&mut hasher);
        bytes.hash(&mut hasher);
        let hash = hasher.finish();

        let thumbnail_png = make_thumbnail_png(&actual_mime, &bytes);

        return Some(ClipboardEntry::Image {
            mime: actual_mime,
            bytes,
            hash,
            thumbnail_png,
        });
    }

    None
}

fn read_clipboard_image_from_uri_list() -> Option<ClipboardEntry> {
    let result = get_contents(
        ClipboardType::Regular,
        Seat::Unspecified,
        PasteMimeType::Specific("text/uri-list"),
    );

    let (mut pipe, _) = result.ok()?;
    let mut bytes = Vec::new();
    if pipe.read_to_end(&mut bytes).is_err() {
        return None;
    }

    let uris = String::from_utf8(bytes).ok()?;
    let path = parse_first_local_path_from_uri_list(&uris)?;
    clipboard_entry_from_image_path(&path)
}

fn parse_first_local_path_from_uri_list(uri_list: &str) -> Option<PathBuf> {
    for line in uri_list.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some(path) = path_from_file_uri(trimmed) {
            return Some(path);
        }
    }
    None
}

fn path_from_file_uri(uri: &str) -> Option<PathBuf> {
    let without_scheme = uri.strip_prefix("file://")?;
    let path_part = without_scheme.split(['\r', '\n']).next()?.trim();
    if path_part.is_empty() {
        return None;
    }
    percent_decode_to_path(path_part)
}

fn percent_decode_to_path(input: &str) -> Option<PathBuf> {
    let mut out = Vec::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' if i + 2 < bytes.len() => {
                let h = hex_val(bytes[i + 1])?;
                let l = hex_val(bytes[i + 2])?;
                out.push((h << 4) | l);
                i += 3;
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    let decoded = String::from_utf8(out).ok()?;
    Some(PathBuf::from(decoded))
}

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(10 + b - b'a'),
        b'A'..=b'F' => Some(10 + b - b'A'),
        _ => None,
    }
}

fn clipboard_entry_from_image_path(path: &Path) -> Option<ClipboardEntry> {
    let ext = path.extension()?.to_string_lossy().to_ascii_lowercase();
    let mime = match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        _ => return None,
    };

    let bytes = std::fs::read(path).ok()?;
    if bytes.is_empty() || bytes.len() > MAX_IMAGE_BYTES {
        return None;
    }

    let mut hasher = DefaultHasher::new();
    mime.hash(&mut hasher);
    bytes.hash(&mut hasher);
    let hash = hasher.finish();
    let thumbnail_png = make_thumbnail_png(mime, &bytes);

    Some(ClipboardEntry::Image {
        mime: mime.to_string(),
        bytes,
        hash,
        thumbnail_png,
    })
}

fn make_thumbnail_png(mime: &str, bytes: &[u8]) -> Option<Vec<u8>> {
    let format = match mime {
        "image/png" => image::ImageFormat::Png,
        "image/jpeg" => image::ImageFormat::Jpeg,
        "image/webp" => image::ImageFormat::WebP,
        _ => {
            // Let the decoder guess if we don't recognize the exact mime.
            return image::load_from_memory(bytes)
                .ok()
                .and_then(|img| encode_thumbnail_png(img));
        }
    };

    let decoded = image::load_from_memory_with_format(bytes, format)
        .or_else(|_| image::load_from_memory(bytes))
        .ok()?;

    encode_thumbnail_png(decoded)
}

fn encode_thumbnail_png(decoded: image::DynamicImage) -> Option<Vec<u8>> {
    let thumb = decoded.thumbnail(THUMBNAIL_SIZE_PX, THUMBNAIL_SIZE_PX);
    let mut out = Vec::new();
    let mut cursor = Cursor::new(&mut out);
    thumb.write_to(&mut cursor, image::ImageFormat::Png).ok()?;
    Some(out)
}

pub fn write_clipboard_text(text: &str) -> bool {
    let opts = CopyOptions::new();
    match opts.copy(
        Source::Bytes(text.as_bytes().to_vec().into()),
        CopyMimeType::Autodetect,
    ) {
        Ok(()) => true,
        Err(err) => {
            if std::env::var_os("CLIPPY_LAND_DEBUG_CLIPBOARD").is_some() {
                eprintln!("[clippy-land] clipboard write error: {err:?}");
            }
            false
        }
    }
}

pub fn write_clipboard_image(mime: &str, bytes: &[u8]) -> bool {
    let opts = CopyOptions::new();
    match opts.copy(
        Source::Bytes(bytes.to_vec().into_boxed_slice()),
        CopyMimeType::Specific(mime.to_string()),
    ) {
        Ok(()) => true,
        Err(err) => {
            if std::env::var_os("CLIPPY_LAND_DEBUG_CLIPBOARD").is_some() {
                eprintln!("[clippy-land] clipboard image write error: {err:?}");
            }
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};
    fn image_entry(
        mime: &str,
        bytes: Vec<u8>,
        hash: u64,
        thumbnail_png: Option<Vec<u8>>,
    ) -> ClipboardEntry {
        ClipboardEntry::Image {
            mime: mime.to_string(),
            bytes,
            hash,
            thumbnail_png,
        }
    }

    #[test]
    fn text_fingerprint_matches_text_content() {
        let entry = ClipboardEntry::Text("hello".to_string());
        assert_eq!(
            entry.fingerprint(),
            ClipboardFingerprint::Text("hello".to_string())
        );
    }

    #[test]
    fn image_fingerprint_tracks_mime_hash_and_length() {
        let entry = image_entry("image/png", vec![1, 2, 3, 4], 99, None);
        assert_eq!(
            entry.fingerprint(),
            ClipboardFingerprint::Image {
                mime: "image/png".to_string(),
                bytes_len: 4,
                hash: 99,
            }
        );
    }

    #[test]
    fn image_equality_uses_hash_mime_and_length() {
        let a = image_entry("image/png", vec![1, 2, 3], 7, None);
        let b = image_entry("image/png", vec![9, 8, 7], 7, Some(vec![0, 1, 2]));
        let c = image_entry("image/png", vec![9, 8], 7, None);
        let d = image_entry("image/png", vec![9, 8, 7], 8, None);

        assert_eq!(a, b);
        assert_ne!(a, c);
        assert_ne!(a, d);
    }

    #[test]
    fn parses_first_local_file_uri() {
        let list = "# comment\n\nfile:///tmp/hello%20world.png\nfile:///tmp/second.jpg\n";
        let parsed =
            parse_first_local_path_from_uri_list(list).expect("first path should parse");
        assert_eq!(parsed, PathBuf::from("/tmp/hello world.png"));
    }

    #[test]
    fn creates_image_entry_from_local_file_path() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time should be after unix epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("clippy-land-test-{unique}.png"));

        let img = image::RgbaImage::from_pixel(2, 2, image::Rgba([0, 255, 0, 255]));
        image::DynamicImage::ImageRgba8(img)
            .save_with_format(&path, image::ImageFormat::Png)
            .expect("test image should save");

        let entry = clipboard_entry_from_image_path(&path).expect("image entry should be created");
        match entry {
            ClipboardEntry::Image { mime, bytes, .. } => {
                assert_eq!(mime, "image/png");
                assert!(!bytes.is_empty());
            }
            ClipboardEntry::Text(_) => panic!("expected image"),
        }

        let _ = std::fs::remove_file(path);
    }

    fn wayland_tests_enabled() -> bool {
        std::env::var("CLIPPY_LAND_RUN_WAYLAND_TESTS")
            .map(|v| v == "1")
            .unwrap_or(false)
            && std::env::var_os("WAYLAND_DISPLAY").is_some()
            && std::env::var_os("XDG_RUNTIME_DIR").is_some()
    }

    fn read_text_with_retry(expected: &str, retries: usize) -> bool {
        for _ in 0..retries {
            if read_clipboard_text().as_deref() == Some(expected) {
                return true;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        false
    }

    fn read_image_with_retry(retries: usize) -> Option<ClipboardEntry> {
        for _ in 0..retries {
            if let Some(entry) = read_clipboard_image() {
                return Some(entry);
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        None
    }

    #[test]
    #[ignore = "requires CLIPPY_LAND_RUN_WAYLAND_TESTS=1 and a live Wayland session"]
    fn wayland_clipboard_text_round_trip() {
        if !wayland_tests_enabled() {
            return;
        }

        let text = "clippy-land-wayland-text";
        assert!(write_clipboard_text(text));
        assert!(read_text_with_retry(text, 20));
    }

    #[test]
    #[ignore = "requires CLIPPY_LAND_RUN_WAYLAND_TESTS=1 and a live Wayland session"]
    fn wayland_clipboard_image_round_trip() {
        if !wayland_tests_enabled() {
            return;
        }

        let img = image::RgbaImage::from_pixel(2, 2, image::Rgba([255, 0, 0, 255]));
        let mut png = Vec::new();
        image::DynamicImage::ImageRgba8(img)
            .write_to(&mut Cursor::new(&mut png), image::ImageFormat::Png)
            .expect("test image encoding should work");

        assert!(write_clipboard_image("image/png", &png));

        let read = read_image_with_retry(20).expect("clipboard image should be readable");
        match read {
            ClipboardEntry::Image { mime, bytes, .. } => {
                assert_eq!(mime, "image/png");
                assert_eq!(bytes, png);
            }
            ClipboardEntry::Text(_) => panic!("expected image entry"),
        }
    }
}

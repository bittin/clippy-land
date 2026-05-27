use super::{
    ClipboardEntry, THUMBNAIL_SIZE_PX, debug_log, max_image_bytes, max_image_dimension_px,
};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::io::Cursor;
use std::path::Path;

pub(super) fn clipboard_entry_from_image_bytes(
    mime: String,
    bytes: Vec<u8>,
) -> Option<ClipboardEntry> {
    let max_image_bytes = max_image_bytes();
    if bytes.is_empty() || bytes.len() > max_image_bytes {
        return None;
    }

    let mut hasher = DefaultHasher::new();
    mime.hash(&mut hasher);
    bytes.hash(&mut hasher);
    let hash = hasher.finish();
    let thumbnail_png = make_thumbnail_png(&mime, &bytes);

    Some(ClipboardEntry::Image {
        mime,
        bytes,
        hash,
        thumbnail_png,
    })
}

pub(super) fn clipboard_entry_from_image_path(path: &Path) -> Option<ClipboardEntry> {
    let ext = path.extension()?.to_string_lossy().to_ascii_lowercase();
    let mime = match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        _ => return None,
    };

    let bytes = std::fs::read(path).ok()?;
    clipboard_entry_from_image_bytes(mime.to_string(), bytes)
}

fn make_thumbnail_png(mime: &str, bytes: &[u8]) -> Option<Vec<u8>> {
    let max_dimension = max_image_dimension_px();
    if !image_dimensions_within_limit(bytes, max_dimension) {
        debug_log(format!(
            "clipboard image ignored (dimensions exceed {} px)",
            max_dimension
        ));
        return None;
    }

    let format = match mime {
        "image/png" => image::ImageFormat::Png,
        "image/jpeg" => image::ImageFormat::Jpeg,
        "image/webp" => image::ImageFormat::WebP,
        _ => {
            return image::load_from_memory(bytes)
                .ok()
                .and_then(encode_thumbnail_png);
        }
    };

    let decoded = image::load_from_memory_with_format(bytes, format)
        .or_else(|_| image::load_from_memory(bytes))
        .ok()?;

    encode_thumbnail_png(decoded)
}

fn image_dimensions_within_limit(bytes: &[u8], max_dimension: u32) -> bool {
    let Ok(reader) = image::ImageReader::new(std::io::Cursor::new(bytes)).with_guessed_format()
    else {
        return false;
    };

    let Ok((width, height)) = reader.into_dimensions() else {
        return false;
    };

    width > 0 && height > 0 && width <= max_dimension && height <= max_dimension
}

fn encode_thumbnail_png(decoded: image::DynamicImage) -> Option<Vec<u8>> {
    let thumb = decoded.thumbnail(THUMBNAIL_SIZE_PX, THUMBNAIL_SIZE_PX);
    let mut out = Vec::new();
    let mut cursor = Cursor::new(&mut out);
    thumb.write_to(&mut cursor, image::ImageFormat::Png).ok()?;
    Some(out)
}

pub(super) fn log_image_too_large(len: usize) {
    let max_image_bytes = max_image_bytes();
    debug_log(format!(
        "clipboard image ignored (too large): {} bytes (max {})",
        len, max_image_bytes
    ));
}

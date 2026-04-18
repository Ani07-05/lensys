use image::{imageops::FilterType, DynamicImage};
use std::io::Cursor;
use base64::Engine;

/// 256-bit perceptual hash stored as 4×u64.
pub type ScreenHash = [u64; 4];

/// Average-hash (aHash): downsample to 16×16 grayscale, bit = pixel > mean.
/// Robust to minor scrolls/cursor movement — small shifts change only a few bits.
fn perceptual_hash(img: &image::RgbaImage) -> ScreenHash {
    let gray = DynamicImage::ImageRgba8(img.clone())
        .resize_exact(16, 16, FilterType::Triangle)
        .to_luma8();

    let pixels = gray.as_raw();
    let mean = pixels.iter().map(|&p| p as u32).sum::<u32>() / 256;

    let mut hash = [0u64; 4];
    for (i, &px) in pixels.iter().enumerate() {
        if (px as u32) > mean {
            hash[i / 64] |= 1u64 << (i % 64);
        }
    }
    hash
}

/// Hamming distance between two 256-bit hashes.
/// Threshold of 10/256 (≈4%) — sensitive enough to catch tab switches and code
/// file changes while ignoring sub-pixel cursor blink or minor chrome redraws.
pub fn screens_differ(a: &ScreenHash, b: &ScreenHash) -> bool {
    let dist: u32 = a.iter().zip(b.iter()).map(|(x, y)| (x ^ y).count_ones()).sum();
    dist > 10
}

/// Find the monitor that contains the given (x, y) cursor position.
/// Falls back to the first monitor if no match (single monitor or edge case).
fn find_monitor_at(cursor: (i32, i32)) -> Result<xcap::Monitor, String> {
    let monitors = xcap::Monitor::all().map_err(|e| e.to_string())?;
    let (cx, cy) = cursor;

    for m in &monitors {
        let x = m.x();
        let y = m.y();
        let w = m.width() as i32;
        let h = m.height() as i32;
        if cx >= x && cx < x + w && cy >= y && cy < y + h {
            return Ok(m.clone());
        }
    }

    // Fallback: first monitor
    monitors
        .into_iter()
        .next()
        .ok_or_else(|| "No monitor found".to_string())
}

/// Returns (base64_jpeg, perceptual_hash).
/// Captures the monitor the cursor is currently on.
pub fn capture_screen_at_cursor(cursor: (i32, i32)) -> Result<(String, ScreenHash), String> {
    let monitor = find_monitor_at(cursor)?;
    let img = monitor.capture_image().map_err(|e| e.to_string())?;

    let resized = if img.width() > 1920 {
        let h = (1920u32 * img.height()) / img.width();
        DynamicImage::ImageRgba8(img)
            .resize(1920, h, FilterType::Lanczos3)
            .into_rgba8()
    } else {
        img
    };

    let hash = perceptual_hash(&resized);

    // JPEG — 5-10x faster encoding than PNG, vision models handle it fine
    let rgb = DynamicImage::ImageRgba8(resized).into_rgb8();
    let mut buf = Vec::new();
    let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, 80);
    rgb.write_with_encoder(encoder).map_err(|e| e.to_string())?;

    Ok((base64::engine::general_purpose::STANDARD.encode(&buf), hash))
}

/// Legacy wrapper — captures primary (first) monitor. Kept for compatibility.
pub fn capture_primary_screen() -> Result<(String, ScreenHash), String> {
    capture_screen_at_cursor((0, 0))
}

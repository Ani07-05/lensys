use base64::Engine;
use image::{imageops::FilterType, DynamicImage};
use std::io::Cursor;

pub fn capture_primary_screen() -> Result<String, String> {
    let monitors = xcap::Monitor::all().map_err(|e| e.to_string())?;
    let monitor = monitors
        .into_iter()
        .next()
        .ok_or_else(|| "No monitor found".to_string())?;

    let img = monitor.capture_image().map_err(|e| e.to_string())?;

    // Resize to max 1920px — preserves text sharpness while keeping payload reasonable
    let resized = if img.width() > 1920 {
        let h = (1920u32 * img.height()) / img.width();
        DynamicImage::ImageRgba8(img)
            .resize(1920, h, FilterType::Lanczos3)
            .into_rgba8()
    } else {
        img
    };

    // PNG — lossless, text stays sharp and readable by vision models
    let mut buf = Vec::new();
    resized
        .write_to(&mut Cursor::new(&mut buf), image::ImageFormat::Png)
        .map_err(|e| e.to_string())?;

    Ok(base64::engine::general_purpose::STANDARD.encode(&buf))
}

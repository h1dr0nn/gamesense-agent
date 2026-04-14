use crate::adb::AdbExecutor;
use crate::command_utils::hidden_command;
use crate::error::AppError;
use base64::Engine;
use std::io::Cursor;
use std::time::{Duration, Instant};

/// Capture device screen as raw PNG bytes
pub fn capture_screen(device_id: &str) -> Result<Vec<u8>, AppError> {
    let executor = AdbExecutor::new();
    let adb_path = executor.get_adb_path();

    let output = hidden_command(adb_path)
        .args(["-s", device_id, "exec-out", "screencap", "-p"])
        .output()
        .map_err(|e| {
            AppError::new(
                "SCREEN_CAPTURE_FAILED",
                &format!("Failed to capture screen: {}", e),
            )
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AppError::new(
            "SCREEN_CAPTURE_FAILED",
            &format!("Screencap failed: {}", stderr),
        ));
    }

    if output.stdout.is_empty() {
        return Err(AppError::new(
            "SCREEN_CAPTURE_FAILED",
            "Screencap returned empty data",
        ));
    }

    Ok(output.stdout)
}

/// Capture device screen and encode as base64 string (plain, no overlay).
pub fn capture_as_base64(device_id: &str) -> Result<String, AppError> {
    let png_bytes = capture_screen(device_id)?;
    Ok(base64::engine::general_purpose::STANDARD.encode(&png_bytes))
}

/// Capture device screen, draw a 10×10 labelled grid overlay, and return as base64.
///
/// The grid divides the screen into 100 cells (columns A–J, rows 1–10).
/// Each cell is labelled at its top-left corner (e.g. "A1", "B3").
/// When the agent describes a tap as "column B, row 3" or "cell B3", we can
/// map it back to `tap:15%:25%` reliably, reducing LLM spatial guessing error.
///
/// Falls back to `capture_as_base64` (no overlay) if image decoding fails.
pub fn capture_as_base64_with_grid(device_id: &str) -> Result<String, AppError> {
    let png_bytes = capture_screen(device_id)?;
    match draw_grid_overlay(&png_bytes) {
        Ok(overlaid) => Ok(base64::engine::general_purpose::STANDARD.encode(&overlaid)),
        Err(_) => {
            // Graceful fallback: send raw screenshot without grid
            Ok(base64::engine::general_purpose::STANDARD.encode(&png_bytes))
        }
    }
}

/// Draw a semi-transparent 10×10 grid with cell labels onto a PNG byte buffer.
///
/// Grid lines are drawn in light gray with 40% opacity so they don't obscure
/// game content. Cell labels are white with a dark outline for legibility.
pub fn draw_grid_overlay(png_bytes: &[u8]) -> Result<Vec<u8>, image::ImageError> {
    use image::{DynamicImage, GenericImageView, Rgba};

    let img = image::load_from_memory(png_bytes)?;
    let (width, height) = img.dimensions();
    let mut rgba = img.to_rgba8();

    const COLS: u32 = 10;
    const ROWS: u32 = 10;

    let cell_w = width / COLS;
    let cell_h = height / ROWS;

    // Grid line color: light gray at ~50% opacity
    let line_color = Rgba([200u8, 200, 200, 128]);

    // Draw vertical grid lines
    for col in 0..=COLS {
        let x = col * cell_w;
        for y in 0..height {
            if x < width {
                blend_pixel(&mut rgba, x, y, line_color);
            }
        }
    }

    // Draw horizontal grid lines
    for row in 0..=ROWS {
        let y = row * cell_h;
        for x in 0..width {
            if y < height {
                blend_pixel(&mut rgba, x, y, line_color);
            }
        }
    }

    // Draw cell labels with center % coordinates: "F7(55,65)"
    // Painted at top-left of each cell so they don't obscure the center tap point.
    for row in 0..ROWS {
        for col in 0..COLS {
            // Center of this cell as percentage (matches tap:X%:Y% exactly)
            let cx_pct = (col * 10 + 5) as u32;
            let cy_pct = (row * 10 + 5) as u32;
            let label = format!("{}{} {},{}", col_letter(col), row + 1, cx_pct, cy_pct);
            let px = col * cell_w + 3;
            let py = row * cell_h + 3;
            draw_label(&mut rgba, px, py, width, height, &label);
        }
    }

    let mut out = Vec::new();
    DynamicImage::ImageRgba8(rgba).write_to(&mut Cursor::new(&mut out), image::ImageFormat::Png)?;
    Ok(out)
}

/// Map column index 0–9 to letters A–J.
fn col_letter(col: u32) -> char {
    (b'A' + col as u8) as char
}

/// Alpha-blend `color` onto pixel `(x, y)` of `img`.
fn blend_pixel(img: &mut image::RgbaImage, x: u32, y: u32, color: image::Rgba<u8>) {
    let existing = img.get_pixel(x, y);
    let a = color[3] as f32 / 255.0;
    let blended = image::Rgba([
        (existing[0] as f32 * (1.0 - a) + color[0] as f32 * a) as u8,
        (existing[1] as f32 * (1.0 - a) + color[1] as f32 * a) as u8,
        (existing[2] as f32 * (1.0 - a) + color[2] as f32 * a) as u8,
        255,
    ]);
    img.put_pixel(x, y, blended);
}

/// Draw a text label using a minimal 5×7 pixel bitmap font.
/// Renders white text with a 1px black outline for readability on any background.
fn draw_label(img: &mut image::RgbaImage, x: u32, y: u32, img_w: u32, img_h: u32, label: &str) {
    // Minimal 5×7 bitmap glyphs for characters we need: A-J, 0-9
    // Each glyph is a &[u8; 7] where each byte encodes one row (bit 4=leftmost).
    const GLYPH_W: u32 = 5;
    const GLYPH_H: u32 = 7;
    const CHAR_SPACING: u32 = 6; // px between characters

    for (ci, ch) in label.chars().enumerate() {
        let glyph = char_glyph(ch);
        let char_x = x + ci as u32 * CHAR_SPACING;

        for row in 0..GLYPH_H {
            let bits = glyph[row as usize];
            for col in 0..GLYPH_W {
                if bits & (1 << (4 - col)) != 0 {
                    // Draw 1px black outline then white center
                    for dy in -1i32..=1 {
                        for dx in -1i32..=1 {
                            let ox = char_x as i32 + col as i32 + dx;
                            let oy = y as i32 + row as i32 + dy;
                            if ox >= 0 && oy >= 0 && (ox as u32) < img_w && (oy as u32) < img_h {
                                if dx != 0 || dy != 0 {
                                    img.put_pixel(ox as u32, oy as u32, image::Rgba([0, 0, 0, 255]));
                                }
                            }
                        }
                    }
                    if char_x + col < img_w && y + row < img_h {
                        img.put_pixel(char_x + col, y + row, image::Rgba([255, 255, 255, 255]));
                    }
                }
            }
        }
    }
}

/// Return a 5×7 bitmap glyph for a character (rows top-to-bottom, bits left-to-right).
/// Bit 4 (0x10) = leftmost column, bit 0 (0x01) = rightmost column.
fn char_glyph(ch: char) -> [u8; 7] {
    match ch {
        'A' => [0x0E, 0x11, 0x11, 0x1F, 0x11, 0x11, 0x11],
        'B' => [0x1E, 0x11, 0x11, 0x1E, 0x11, 0x11, 0x1E],
        'C' => [0x0E, 0x11, 0x10, 0x10, 0x10, 0x11, 0x0E],
        'D' => [0x1E, 0x09, 0x09, 0x09, 0x09, 0x09, 0x1E],
        'E' => [0x1F, 0x10, 0x10, 0x1E, 0x10, 0x10, 0x1F],
        'F' => [0x1F, 0x10, 0x10, 0x1E, 0x10, 0x10, 0x10],
        'G' => [0x0E, 0x11, 0x10, 0x13, 0x11, 0x11, 0x0F],
        'H' => [0x11, 0x11, 0x11, 0x1F, 0x11, 0x11, 0x11],
        'I' => [0x1F, 0x04, 0x04, 0x04, 0x04, 0x04, 0x1F],
        'J' => [0x1F, 0x02, 0x02, 0x02, 0x02, 0x12, 0x0C],
        '0' => [0x0E, 0x11, 0x13, 0x15, 0x19, 0x11, 0x0E],
        '1' => [0x04, 0x0C, 0x04, 0x04, 0x04, 0x04, 0x0E],
        '2' => [0x0E, 0x11, 0x01, 0x02, 0x04, 0x08, 0x1F],
        '3' => [0x1F, 0x02, 0x04, 0x02, 0x01, 0x11, 0x0E],
        '4' => [0x02, 0x06, 0x0A, 0x12, 0x1F, 0x02, 0x02],
        '5' => [0x1F, 0x10, 0x1E, 0x01, 0x01, 0x11, 0x0E],
        '6' => [0x06, 0x08, 0x10, 0x1E, 0x11, 0x11, 0x0E],
        '7' => [0x1F, 0x01, 0x02, 0x04, 0x08, 0x08, 0x08],
        '8' => [0x0E, 0x11, 0x11, 0x0E, 0x11, 0x11, 0x0E],
        '9' => [0x0E, 0x11, 0x11, 0x0F, 0x01, 0x02, 0x0C],
        ',' => [0x00, 0x00, 0x00, 0x00, 0x04, 0x04, 0x08],
        _   => [0x00; 7], // blank for unknown chars (space = blank)
    }
}

/// Wait until the screen stabilizes (animation finished)
/// Compares consecutive screenshots; returns when they match or timeout
pub fn wait_for_stable(device_id: &str, timeout_ms: u64) -> Result<Vec<u8>, AppError> {
    let mut prev = capture_screen(device_id)?;
    let deadline = Instant::now() + Duration::from_millis(timeout_ms);

    while Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(300));
        let curr = capture_screen(device_id)?;

        if frames_similar(&prev, &curr) {
            return Ok(curr);
        }
        prev = curr;
    }

    // Timeout — return last captured frame
    Ok(prev)
}

/// Check if two PNG byte arrays are similar enough (same length + >99% identical bytes)
fn frames_similar(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    if a.is_empty() {
        return true;
    }
    let matching = a.iter().zip(b.iter()).filter(|(x, y)| x == y).count();
    let ratio = matching as f64 / a.len() as f64;
    ratio > 0.99
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Create a minimal valid PNG (1×1 red pixel) for testing.
    fn minimal_png() -> Vec<u8> {
        use image::{ImageBuffer, Rgba};
        let img: ImageBuffer<Rgba<u8>, Vec<u8>> =
            ImageBuffer::from_fn(100, 200, |_, _| Rgba([255u8, 0, 0, 255]));
        let mut buf = Vec::new();
        image::DynamicImage::ImageRgba8(img)
            .write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png)
            .unwrap();
        buf
    }

    #[test]
    fn col_letter_a_through_j() {
        assert_eq!(col_letter(0), 'A');
        assert_eq!(col_letter(9), 'J');
    }

    #[test]
    fn draw_grid_overlay_produces_valid_png() {
        let png = minimal_png();
        let result = draw_grid_overlay(&png);
        assert!(result.is_ok());
        // Output is still valid PNG
        let overlaid = result.unwrap();
        assert!(image::load_from_memory(&overlaid).is_ok());
    }

    #[test]
    fn draw_grid_overlay_same_dimensions() {
        use image::GenericImageView;
        let png = minimal_png();
        let original = image::load_from_memory(&png).unwrap();
        let overlaid_bytes = draw_grid_overlay(&png).unwrap();
        let overlaid = image::load_from_memory(&overlaid_bytes).unwrap();
        assert_eq!(original.dimensions(), overlaid.dimensions());
    }

    #[test]
    fn draw_grid_overlay_invalid_input_returns_error() {
        let result = draw_grid_overlay(b"not a png");
        assert!(result.is_err());
    }

    #[test]
    fn identical_frames_are_similar() {
        let a = vec![1, 2, 3, 4, 5];
        assert!(frames_similar(&a, &a));
    }

    #[test]
    fn different_length_frames_not_similar() {
        let a = vec![1, 2, 3];
        let b = vec![1, 2, 3, 4];
        assert!(!frames_similar(&a, &b));
    }

    #[test]
    fn slightly_different_frames_are_similar() {
        let a: Vec<u8> = (0..1000).map(|i| (i % 256) as u8).collect();
        let mut b = a.clone();
        // Change 5 bytes out of 1000 (0.5% difference < 1% threshold)
        b[0] = 255;
        b[100] = 255;
        b[200] = 255;
        b[300] = 255;
        b[400] = 255;
        assert!(frames_similar(&a, &b));
    }

    #[test]
    fn very_different_frames_not_similar() {
        let a = vec![0u8; 100];
        let b = vec![255u8; 100];
        assert!(!frames_similar(&a, &b));
    }

    #[test]
    fn empty_frames_are_similar() {
        assert!(frames_similar(&[], &[]));
    }
}

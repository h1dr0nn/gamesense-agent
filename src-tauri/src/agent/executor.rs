use crate::adb::AdbExecutor;
use crate::command_utils::hidden_command;
use crate::error::AppError;

/// Parse a `tap:X%:Y%` action string into absolute pixel coordinates.
/// X and Y are percentages (0–100) of screen width/height.
/// Returns `None` if the string is not a valid tap action.
pub fn parse_tap(action: &str, screen_w: u32, screen_h: u32) -> Option<(i32, i32)> {
    let s = action.strip_prefix("tap:")?;
    let mut parts = s.split(':');
    let x_pct: f64 = parts.next()?.trim_end_matches('%').parse().ok()?;
    let y_pct: f64 = parts.next()?.trim_end_matches('%').parse().ok()?;
    if !(0.0..=100.0).contains(&x_pct) || !(0.0..=100.0).contains(&y_pct) {
        return None;
    }
    let x = (screen_w as f64 * x_pct / 100.0) as i32;
    let y = (screen_h as f64 * y_pct / 100.0) as i32;
    Some((x, y))
}

/// Map an agent action string to screen swipe coordinates.
/// Returns `(x1, y1, x2, y2)` for a swipe from the center of the screen.
pub fn action_to_swipe(action: &str, screen_w: u32, screen_h: u32) -> Option<(i32, i32, i32, i32)> {
    let cx = screen_w as i32 / 2;
    let cy = screen_h as i32 / 2;
    let dist = screen_h as i32 / 3;

    match action {
        "swipe_up" => Some((cx, cy + dist / 2, cx, cy - dist / 2)),
        "swipe_down" => Some((cx, cy - dist / 2, cx, cy + dist / 2)),
        "swipe_left" => Some((cx + dist / 2, cy, cx - dist / 2, cy)),
        "swipe_right" => Some((cx - dist / 2, cy, cx + dist / 2, cy)),
        _ => None,
    }
}

/// Execute a game action via ADB.
///
/// Supported action formats:
/// - `swipe_up / swipe_down / swipe_left / swipe_right` — swipe from screen center
/// - `tap:X%:Y%` — tap at (X%, Y%) of screen dimensions (e.g. `tap:50%:75%`)
pub fn execute_action(
    device_id: &str,
    action: &str,
    screen_w: u32,
    screen_h: u32,
) -> Result<(), AppError> {
    let executor = AdbExecutor::new();
    let adb_path = executor.get_adb_path();

    if let Some((x, y)) = parse_tap(action, screen_w, screen_h) {
        let output = hidden_command(adb_path)
            .args(["-s", device_id, "shell", "input", "tap", &x.to_string(), &y.to_string()])
            .output()
            .map_err(|e| AppError::new("ACTION_EXECUTION_FAILED", &format!("Failed to execute tap: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(AppError::new("ACTION_EXECUTION_FAILED", &format!("Tap failed: {}", stderr)));
        }
        return Ok(());
    }

    let (x1, y1, x2, y2) = action_to_swipe(action, screen_w, screen_h).ok_or_else(|| {
        AppError::new("INVALID_ACTION", &format!("Unknown action: {}", action))
    })?;

    let duration_ms = 200;
    let output = hidden_command(adb_path)
        .args([
            "-s", device_id, "shell", "input", "swipe",
            &x1.to_string(), &y1.to_string(),
            &x2.to_string(), &y2.to_string(),
            &duration_ms.to_string(),
        ])
        .output()
        .map_err(|e| AppError::new("ACTION_EXECUTION_FAILED", &format!("Failed to execute swipe: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AppError::new("ACTION_EXECUTION_FAILED", &format!("Swipe failed: {}", stderr)));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn swipe_up_coordinates() {
        let (x1, y1, x2, y2) = action_to_swipe("swipe_up", 1080, 2340).unwrap();
        assert_eq!(x1, 540); // center x
        assert_eq!(x2, 540); // center x (vertical swipe)
        assert!(y1 > y2); // start below, end above
    }

    #[test]
    fn swipe_down_coordinates() {
        let (x1, y1, x2, y2) = action_to_swipe("swipe_down", 1080, 2340).unwrap();
        assert_eq!(x1, 540);
        assert!(y2 > y1); // end below start
    }

    #[test]
    fn swipe_left_coordinates() {
        let (x1, y1, x2, y2) = action_to_swipe("swipe_left", 1080, 2340).unwrap();
        assert_eq!(y1, 1170); // center y
        assert!(x1 > x2); // start right, end left
    }

    #[test]
    fn swipe_right_coordinates() {
        let (x1, y1, x2, y2) = action_to_swipe("swipe_right", 1080, 2340).unwrap();
        assert_eq!(y1, 1170);
        assert!(x2 > x1); // end right of start
    }

    #[test]
    fn unknown_action_returns_none() {
        assert!(action_to_swipe("tap:50%:50%", 1080, 2340).is_none());
        assert!(action_to_swipe("", 1080, 2340).is_none());
        assert!(action_to_swipe("swipe_diagonal", 1080, 2340).is_none());
    }

    #[test]
    fn parse_tap_center() {
        let (x, y) = parse_tap("tap:50%:50%", 1080, 2340).unwrap();
        assert_eq!(x, 540);
        assert_eq!(y, 1170);
    }

    #[test]
    fn parse_tap_top_left() {
        let (x, y) = parse_tap("tap:0%:0%", 1080, 2340).unwrap();
        assert_eq!(x, 0);
        assert_eq!(y, 0);
    }

    #[test]
    fn parse_tap_bottom_right() {
        let (x, y) = parse_tap("tap:100%:100%", 1080, 2340).unwrap();
        assert_eq!(x, 1080);
        assert_eq!(y, 2340);
    }

    #[test]
    fn parse_tap_invalid_returns_none() {
        assert!(parse_tap("tap:", 1080, 2340).is_none());
        assert!(parse_tap("tap:50%", 1080, 2340).is_none());
        assert!(parse_tap("tap:abc%:50%", 1080, 2340).is_none());
        assert!(parse_tap("tap:150%:50%", 1080, 2340).is_none());
        assert!(parse_tap("swipe_up", 1080, 2340).is_none());
    }

    #[test]
    fn coordinates_scale_with_screen_size() {
        let (_, _, _, y2_small) = action_to_swipe("swipe_up", 720, 1280).unwrap();
        let (_, _, _, y2_large) = action_to_swipe("swipe_up", 1440, 3200).unwrap();
        // Larger screen should have larger coordinate values
        assert!(y2_large.abs() > y2_small.abs());
    }
}

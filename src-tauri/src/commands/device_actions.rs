// Device Actions - Device control and input commands
// Provides reboot, input, dark mode, show taps, and animation commands

use crate::adb::AdbExecutor;
use crate::command_utils::hidden_command;
use crate::error::AppError;
use std::process::Stdio;

/// Reboot device with optional mode (recovery, bootloader)
#[tauri::command]
pub fn reboot_device(device_id: String, mode: Option<String>) -> Result<(), AppError> {
    let executor = AdbExecutor::new();
    let adb_path = executor.get_adb_path();

    let mut cmd = hidden_command(adb_path);
    cmd.args(["-s", &device_id, "reboot"]);

    if let Some(m) = &mode {
        if !m.is_empty() {
            cmd.arg(m);
        }
    }

    cmd.stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| AppError::new("REBOOT_FAILED", &format!("Failed to reboot: {}", e)))?;

    Ok(())
}

/// Input text to device's current focused input
#[tauri::command]
pub fn input_text(device_id: String, text: String) -> Result<(), AppError> {
    let executor = AdbExecutor::new();
    let adb_path = executor.get_adb_path();

    // Escape special characters for adb shell input text
    // Space -> %s, special chars need escaping
    let escaped_text = text
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\'', "\\'")
        .replace(' ', "%s")
        .replace('&', "\\&")
        .replace('|', "\\|")
        .replace(';', "\\;")
        .replace('(', "\\(")
        .replace(')', "\\)")
        .replace('<', "\\<")
        .replace('>', "\\>");

    let output = hidden_command(adb_path)
        .args(["-s", &device_id, "shell", "input", "text", &escaped_text])
        .output()
        .map_err(|e| AppError::new("INPUT_FAILED", &format!("Failed to input text: {}", e)))?;

    // Android shell outputs errors to stdout, not stderr
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Check for error in both stdout and stderr
    if stdout.contains("Exception") || stdout.contains("Error") || stdout.contains("error") {
        return Err(AppError::new(
            "INPUT_FAILED",
            &format!("Input failed: {}", stdout.trim()),
        ));
    }

    if !output.status.success() {
        return Err(AppError::new(
            "INPUT_FAILED",
            &format!(
                "Input failed: {}",
                if stderr.is_empty() {
                    stdout.trim()
                } else {
                    stderr.trim()
                }
                .to_string()
            ),
        ));
    }

    Ok(())
}

/// Input tap at specific coordinates
#[tauri::command]
pub fn input_tap(device_id: String, x: i32, y: i32) -> Result<(), AppError> {
    let executor = AdbExecutor::new();
    let adb_path = executor.get_adb_path();

    let output = hidden_command(adb_path)
        .args([
            "-s",
            &device_id,
            "shell",
            "input",
            "tap",
            &x.to_string(),
            &y.to_string(),
        ])
        .output()
        .map_err(|e| AppError::new("INPUT_TAP_FAILED", &format!("Failed to input tap: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(AppError::new(
            "INPUT_TAP_FAILED",
            &format!("Input tap failed: {} {}", stdout, stderr),
        ));
    }

    Ok(())
}

/// Input swipe from one point to another
#[tauri::command]
pub fn input_swipe(
    device_id: String,
    x1: i32,
    y1: i32,
    x2: i32,
    y2: i32,
    duration_ms: u32,
) -> Result<(), AppError> {
    let executor = AdbExecutor::new();
    let adb_path = executor.get_adb_path();

    let output = hidden_command(adb_path)
        .args([
            "-s",
            &device_id,
            "shell",
            "input",
            "swipe",
            &x1.to_string(),
            &y1.to_string(),
            &x2.to_string(),
            &y2.to_string(),
            &duration_ms.to_string(),
        ])
        .output()
        .map_err(|e| {
            AppError::new(
                "INPUT_SWIPE_FAILED",
                &format!("Failed to input swipe: {}", e),
            )
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AppError::new(
            "INPUT_SWIPE_FAILED",
            &format!("Input swipe failed: {}", stderr),
        ));
    }

    Ok(())
}

/// Set system UI night mode (Dark Mode)
#[tauri::command]
pub fn set_dark_mode(device_id: String, enabled: bool) -> Result<(), AppError> {
    let executor = AdbExecutor::new();
    let adb_path = executor.get_adb_path();
    let mode = if enabled { "yes" } else { "no" };

    hidden_command(adb_path)
        .args(["-s", &device_id, "shell", "cmd", "uimode", "night", mode])
        .output()
        .map_err(|e| {
            AppError::new(
                "DARK_MODE_FAILED",
                &format!("Failed to set dark mode: {}", e),
            )
        })?;

    Ok(())
}

/// Toggle "Show Taps" in developer options
#[tauri::command]
pub fn set_show_taps(device_id: String, enabled: bool) -> Result<(), AppError> {
    let executor = AdbExecutor::new();
    let adb_path = executor.get_adb_path();
    let val = if enabled { "1" } else { "0" };

    hidden_command(adb_path)
        .args([
            "-s",
            &device_id,
            "shell",
            "settings",
            "put",
            "system",
            "show_touches",
            val,
        ])
        .output()
        .map_err(|e| {
            AppError::new(
                "SHOW_TAPS_FAILED",
                &format!("Failed to set show taps: {}", e),
            )
        })?;

    Ok(())
}

/// Set global animation scales
#[tauri::command]
pub fn set_animations(device_id: String, scale: f32) -> Result<(), AppError> {
    let executor = AdbExecutor::new();
    let adb_path = executor.get_adb_path();
    let val = scale.to_string();

    let commands = [
        ["settings", "put", "global", "window_animation_scale", &val],
        [
            "settings",
            "put",
            "global",
            "transition_animation_scale",
            &val,
        ],
        ["settings", "put", "global", "animator_duration_scale", &val],
    ];

    for args in commands {
        let mut full_args = vec!["-s", &device_id, "shell"];
        full_args.extend_from_slice(&args);

        hidden_command(adb_path)
            .args(&full_args)
            .output()
            .map_err(|e| {
                AppError::new(
                    "ANIM_SCALE_FAILED",
                    &format!("Failed to set animation scale: {}", e),
                )
            })?;
    }

    Ok(())
}

/// Get the package name of the currently focused (foreground) app
#[tauri::command]
pub fn get_foreground_app(device_id: String) -> Result<String, AppError> {
    let executor = AdbExecutor::new();
    let adb_path = executor.get_adb_path();

    // Use dumpsys window which is faster and more reliable
    let output = hidden_command(adb_path)
        .args(["-s", &device_id, "shell",
            "dumpsys", "window", "windows"])
        .output()
        .map_err(|e| AppError::new("FOREGROUND_FAILED", &format!("Failed to get foreground app: {}", e)))?;

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Parse mCurrentFocus or mFocusedApp lines
    for line in stdout.lines() {
        if (line.contains("mCurrentFocus") || line.contains("mFocusedApp")) && line.contains('/') {
            if let Some(pkg_part) = line.split_whitespace().find(|s| s.contains('/')) {
                if let Some(pkg) = pkg_part.split('/').next() {
                    let pkg = pkg.trim_matches('{').trim_matches('}');
                    if !pkg.is_empty() && pkg.contains('.') {
                        return Ok(pkg.to_string());
                    }
                }
            }
        }
    }

    Ok(String::new())
}

/// Get the human-readable display name of an app by package name.
/// Returns the package name itself if the label cannot be determined.
#[tauri::command]
pub fn get_app_label(device_id: String, package_name: String) -> Result<String, AppError> {
    let executor = AdbExecutor::new();
    let adb_path = executor.get_adb_path();

    // `cmd package resolve-activity --brief -a android.intent.action.MAIN <pkg>` is unreliable.
    // `dumpsys package <pkg>` contains a line like:
    //   nonLocalizedLabel=Pocket Sort: Coin Puzzle
    // which is the fastest reliable way to get the display name over ADB.
    let output = hidden_command(adb_path)
        .args(["-s", &device_id, "shell", "dumpsys", "package", &package_name])
        .output()
        .map_err(|e| AppError::new("APP_LABEL_FAILED", &format!("dumpsys failed: {}", e)))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        let trimmed = line.trim();
        if let Some(val) = trimmed.strip_prefix("nonLocalizedLabel=") {
            let label = val.trim();
            if !label.is_empty() && label != "null" {
                return Ok(label.to_string());
            }
        }
    }

    // Fallback: return package name unchanged
    Ok(package_name)
}

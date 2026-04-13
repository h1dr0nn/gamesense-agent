// Package Actions - App package management commands
// Provides install, uninstall, list, clear data, and permission commands

use crate::adb::AdbExecutor;
use crate::command_utils::hidden_command;
use crate::error::AppError;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct AppPackage {
    pub id: String,
    pub label: Option<String>,
    pub icon: Option<String>,
}

/// Uninstall an app by package name
#[tauri::command]
pub fn uninstall_app(device_id: String, package_name: String) -> Result<String, AppError> {
    let executor = AdbExecutor::new();
    let adb_path = executor.get_adb_path();

    let output = hidden_command(adb_path)
        .args(["-s", &device_id, "uninstall", &package_name])
        .output()
        .map_err(|e| AppError::new("UNINSTALL_FAILED", &format!("Failed to uninstall: {}", e)))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();

    if stdout.contains("Success") {
        Ok("Successfully uninstalled".to_string())
    } else {
        Err(AppError::new(
            "UNINSTALL_FAILED",
            &format!("Uninstall failed: {}", stdout.trim()),
        ))
    }
}

/// List installed packages
#[tauri::command]
pub fn list_packages(device_id: String, include_system: bool) -> Result<Vec<AppPackage>, AppError> {
    let executor = AdbExecutor::new();
    let adb_path = executor.get_adb_path();

    let mut args = vec!["-s", &device_id, "shell", "pm", "list", "packages"];

    // -3 = third party only, no flag = all packages
    if !include_system {
        args.push("-3");
    }

    let output = hidden_command(adb_path).args(&args).output().map_err(|e| {
        AppError::new(
            "LIST_PACKAGES_FAILED",
            &format!("Failed to list packages: {}", e),
        )
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AppError::new(
            "LIST_PACKAGES_FAILED",
            &format!("List packages failed: {}", stderr),
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let packages: Vec<AppPackage> = stdout
        .lines()
        .filter_map(|line| {
            line.strip_prefix("package:").map(|s| {
                AppPackage {
                    id: s.trim().to_string(),
                    label: None, // Placeholder for future Agent data
                    icon: None,  // Placeholder for future Agent data
                }
            })
        })
        .collect();

    Ok(packages)
}

/// Clear app data and cache
#[tauri::command]
pub fn clear_app_data(device_id: String, package_name: String) -> Result<(), AppError> {
    let executor = AdbExecutor::new();
    let adb_path = executor.get_adb_path();

    let output = hidden_command(adb_path)
        .args(["-s", &device_id, "shell", "pm", "clear", &package_name])
        .output()
        .map_err(|e| AppError::new("CLEAR_DATA_FAILED", &format!("Failed to clear data: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AppError::new(
            "CLEAR_DATA_FAILED",
            &format!("PM clear failed: {}", stderr),
        ));
    }

    Ok(())
}

/// Grant all runtime permissions to an app
#[tauri::command]
pub fn grant_all_permissions(device_id: String, package_name: String) -> Result<(), AppError> {
    let executor = AdbExecutor::new();
    let adb_path = executor.get_adb_path();

    // Logic: Get all requested "dangerous" permissions and grant them
    let output = hidden_command(adb_path)
        .args([
            "-s",
            &device_id,
            "shell",
            "dumpsys",
            "package",
            &package_name,
        ])
        .output()
        .map_err(|e| {
            AppError::new(
                "GRANT_FAILED",
                &format!("Failed to get package info: {}", e),
            )
        })?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut permissions = Vec::new();
    let mut in_requested = false;

    for line in stdout.lines() {
        let line = line.trim();
        if line.contains("requested permissions:") {
            in_requested = true;
            continue;
        }
        if in_requested && line.contains(":") {
            break;
        }
        if in_requested && !line.is_empty() {
            permissions.push(line.to_string());
        }
    }

    for perm in permissions {
        // Grant permission (ignore errors as some permissions might not be grantable via ADB)
        let _ = hidden_command(adb_path)
            .args([
                "-s",
                &device_id,
                "shell",
                "pm",
                "grant",
                &package_name,
                &perm,
            ])
            .output();
    }

    Ok(())
}

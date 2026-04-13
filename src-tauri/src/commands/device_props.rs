// Device Properties - Query device hardware and software information
// Provides device model, battery, storage, RAM, and CPU details

use crate::adb::AdbExecutor;
use crate::command_utils::hidden_command;
use crate::error::AppError;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct DeviceProps {
    pub model: String,
    pub android_version: String,
    pub sdk_version: String,
    pub battery_level: Option<u8>,
    pub is_charging: bool,
    pub screen_resolution: Option<String>,
    pub storage_total: Option<String>,
    pub storage_free: Option<String>,
    pub ram_total: Option<String>,
    pub manufacturer: Option<String>,
    pub cpu: Option<String>,
    pub build_number: Option<String>,
    pub security_patch: Option<String>,
}

/// Get device properties (model, version, battery)
#[tauri::command]
pub fn get_device_props(device_id: String) -> Result<DeviceProps, AppError> {
    let executor = AdbExecutor::new();
    let adb_path = executor.get_adb_path();

    // Get properties
    let props_output = hidden_command(adb_path)
        .args(["-s", &device_id, "shell", "getprop"])
        .output()
        .map_err(|e| AppError::new("GET_PROPS_FAILED", &format!("Failed to get props: {}", e)))?;

    let props_str = String::from_utf8_lossy(&props_output.stdout);

    let model =
        extract_prop(&props_str, "ro.product.model").unwrap_or_else(|| "Unknown".to_string());
    let android_version = extract_prop(&props_str, "ro.build.version.release")
        .unwrap_or_else(|| "Unknown".to_string());
    let sdk_version =
        extract_prop(&props_str, "ro.build.version.sdk").unwrap_or_else(|| "Unknown".to_string());

    // Additional props
    let manufacturer = extract_prop(&props_str, "ro.product.brand");

    // CPU: Try ro.soc.model first (has friendly name on some devices), then board/hardware
    let cpu_raw = extract_prop(&props_str, "ro.soc.model")
        .or_else(|| extract_prop(&props_str, "ro.hardware.chipname"))
        .or_else(|| extract_prop(&props_str, "ro.product.board"))
        .or_else(|| extract_prop(&props_str, "ro.hardware"));

    // Map common codenames to marketing names
    let cpu = cpu_raw.map(|raw| map_soc_codename(&raw));

    let build_number = extract_prop(&props_str, "ro.build.display.id");
    let security_patch = extract_prop(&props_str, "ro.build.version.security_patch");

    // Get battery info
    let battery_output = hidden_command(&adb_path)
        .args(["-s", &device_id, "shell", "dumpsys", "battery"])
        .output()
        .ok();

    let (battery_level, is_charging) = if let Some(output) = battery_output {
        let battery_str = String::from_utf8_lossy(&output.stdout);

        let level = battery_str
            .lines()
            .find(|l| l.trim().starts_with("level:"))
            .and_then(|l| l.split(':').nth(1))
            .and_then(|v| v.trim().parse::<u8>().ok());

        let charging = battery_str.contains("USB powered: true")
            || battery_str.contains("AC powered: true")
            || battery_str.contains("Wireless powered: true");

        (level, charging)
    } else {
        (None, false)
    };

    // Get screen resolution
    let screen_output = hidden_command(&adb_path)
        .args(["-s", &device_id, "shell", "wm", "size"])
        .output()
        .ok();

    let screen_resolution = screen_output.and_then(|output| {
        let s = String::from_utf8_lossy(&output.stdout);
        s.lines()
            .find(|l| l.contains("Physical size:") || l.contains("Override size:"))
            .and_then(|l| l.split(':').nth(1))
            .map(|v| v.trim().to_string())
    });

    // Get storage info using df
    let storage_output = hidden_command(&adb_path)
        .args(["-s", &device_id, "shell", "df", "/data"])
        .output()
        .ok();

    let (storage_total, storage_free) = if let Some(output) = storage_output {
        let s = String::from_utf8_lossy(&output.stdout);
        let values: Option<(String, String)> = s.lines().nth(1).and_then(|line| {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 4 {
                let total = format_storage_size(parts[1]);
                let free = format_storage_size(parts[3]);
                Some((total, free))
            } else {
                None
            }
        });
        (values.as_ref().map(|v| v.0.clone()), values.map(|v| v.1))
    } else {
        (None, None)
    };

    // Get RAM info
    let ram_output = hidden_command(&adb_path)
        .args(["-s", &device_id, "shell", "cat", "/proc/meminfo"])
        .output()
        .ok();

    let ram_total = ram_output.and_then(|output| {
        let s = String::from_utf8_lossy(&output.stdout);
        s.lines()
            .find(|l| l.starts_with("MemTotal:"))
            .and_then(|l| {
                let kb: u64 = l.split_whitespace().nth(1)?.parse().ok()?;
                let gb = kb as f64 / 1024.0 / 1024.0;
                Some(format!("{:.1} GB", gb))
            })
    });

    Ok(DeviceProps {
        model,
        android_version,
        sdk_version,
        battery_level,
        is_charging,
        screen_resolution,
        storage_total,
        storage_free,
        ram_total,
        manufacturer,
        cpu,
        build_number,
        security_patch,
    })
}

fn format_storage_size(s: &str) -> String {
    if let Ok(kb) = s.parse::<u64>() {
        if kb >= 1024 * 1024 {
            format!("{:.1} GB", kb as f64 / 1024.0 / 1024.0)
        } else if kb >= 1024 {
            format!("{:.1} MB", kb as f64 / 1024.0)
        } else {
            format!("{} KB", kb)
        }
    } else {
        s.to_string()
    }
}

fn extract_prop(output: &str, key: &str) -> Option<String> {
    // Format: [key]: [value]
    let pattern = format!("[{}]:", key);
    output
        .lines()
        .find(|line| line.contains(&pattern))
        .and_then(|line| {
            // Find the value between last [ and ]
            let value_start = line.rfind('[')? + 1;
            let value_end = line.rfind(']')?;
            if value_start < value_end {
                Some(line[value_start..value_end].to_string())
            } else {
                None
            }
        })
}

/// Map SoC codenames to marketing names
fn map_soc_codename(codename: &str) -> String {
    let lower = codename.to_lowercase();
    match lower.as_str() {
        // Qualcomm Snapdragon 8 series (Flagship)
        "taro" => "Snapdragon 8 Gen 1".to_string(),
        "kalama" | "kailua" => "Snapdragon 8 Gen 2".to_string(),
        "pineapple" => "Snapdragon 8 Gen 3".to_string(),
        "lahaina" => "Snapdragon 888".to_string(),
        "kona" => "Snapdragon 865".to_string(),
        "msmnile" | "sm8150" => "Snapdragon 855".to_string(),
        "sdm845" => "Snapdragon 845".to_string(),

        // Qualcomm Snapdragon 7 series
        "crow" => "Snapdragon 7 Gen 3".to_string(),
        "cape" => "Snapdragon 7+ Gen 2".to_string(),
        "kodiak" => "Snapdragon 7 Gen 1".to_string(),
        "sm7250" | "lito" => "Snapdragon 765G".to_string(),

        // Qualcomm Snapdragon 6 series
        "parrot" => "Snapdragon 6 Gen 1".to_string(),
        "bengal" => "Snapdragon 685/680".to_string(),
        "holi" => "Snapdragon 695".to_string(),
        "sm6150" => "Snapdragon 675".to_string(),
        "trinket" => "Snapdragon 665".to_string(),

        // Qualcomm Snapdragon 4 series
        "khaje" => "Snapdragon 4 Gen 1".to_string(),
        "scuba" => "Snapdragon 460".to_string(),

        // MediaTek Dimensity
        "mt6893" | "mt6891" => "Dimensity 1200".to_string(),
        "mt6885" | "mt6889" => "Dimensity 1000".to_string(),
        "mt6877" => "Dimensity 900".to_string(),
        "mt6873" | "mt6875" => "Dimensity 800".to_string(),
        "mt6853" => "Dimensity 720".to_string(),
        "mt6833" => "Dimensity 700".to_string(),

        // MediaTek Helio
        "mt6769" => "Helio G85".to_string(),
        "mt6768" => "Helio G80".to_string(),
        "mt6765" | "mt6762" => "Helio P35".to_string(),
        "mt6785" => "Helio G95".to_string(),

        // Samsung Exynos
        "exynos2200" => "Exynos 2200".to_string(),
        "exynos2100" => "Exynos 2100".to_string(),
        "exynos990" => "Exynos 990".to_string(),
        "exynos9820" | "exynos9825" => "Exynos 9825".to_string(),
        "exynos9810" => "Exynos 9810".to_string(),
        "exynos1280" => "Exynos 1280".to_string(),
        "s5e8825" => "Exynos 1280".to_string(),

        // Google Tensor
        "oriole" | "raven" => "Google Tensor".to_string(),
        "cloudripper" | "bluejay" => "Google Tensor G2".to_string(),
        "zuma" => "Google Tensor G3".to_string(),

        // If already looks like a proper name or not found, return as-is with title-case
        _ => {
            // Check if it looks like a marketing name already (contains letters and numbers properly formatted)
            if codename.contains("Snapdragon")
                || codename.contains("Dimensity")
                || codename.contains("Exynos")
                || codename.contains("Helio")
                || codename.contains("Tensor")
            {
                codename.to_string()
            } else {
                // Return original, capitalized
                let mut chars = codename.chars();
                match chars.next() {
                    None => String::new(),
                    Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_prop_returns_value_for_known_key() {
        let output = "[ro.product.model]: [Pixel 7]\n[ro.build.version.release]: [14]\n";
        assert_eq!(
            extract_prop(output, "ro.product.model"),
            Some("Pixel 7".to_string())
        );
    }

    #[test]
    fn extract_prop_returns_none_for_missing_key() {
        let output = "[ro.product.model]: [Pixel 7]\n";
        assert_eq!(extract_prop(output, "ro.build.version.release"), None);
    }

    #[test]
    fn extract_prop_returns_none_for_empty_value() {
        let output = "[ro.product.model]: []\n";
        assert_eq!(extract_prop(output, "ro.product.model"), None);
    }

    #[test]
    fn format_storage_size_gigabytes() {
        // 2 GB in KB = 2 * 1024 * 1024 = 2097152
        assert_eq!(format_storage_size("2097152"), "2.0 GB");
    }

    #[test]
    fn format_storage_size_megabytes() {
        // 512 MB in KB = 512 * 1024 = 524288
        assert_eq!(format_storage_size("524288"), "512.0 MB");
    }

    #[test]
    fn format_storage_size_kilobytes() {
        assert_eq!(format_storage_size("512"), "512 KB");
    }

    #[test]
    fn format_storage_size_non_numeric() {
        assert_eq!(format_storage_size("N/A"), "N/A");
    }

    #[test]
    fn map_soc_codename_known_qualcomm() {
        assert_eq!(map_soc_codename("taro"), "Snapdragon 8 Gen 1");
        assert_eq!(map_soc_codename("kalama"), "Snapdragon 8 Gen 2");
        assert_eq!(map_soc_codename("pineapple"), "Snapdragon 8 Gen 3");
    }

    #[test]
    fn map_soc_codename_known_mediatek() {
        assert_eq!(map_soc_codename("mt6893"), "Dimensity 1200");
        assert_eq!(map_soc_codename("mt6833"), "Dimensity 700");
    }

    #[test]
    fn map_soc_codename_known_tensor() {
        assert_eq!(map_soc_codename("zuma"), "Google Tensor G3");
        assert_eq!(map_soc_codename("oriole"), "Google Tensor");
    }

    #[test]
    fn map_soc_codename_already_marketing_name() {
        assert_eq!(
            map_soc_codename("Snapdragon 8 Gen 1"),
            "Snapdragon 8 Gen 1"
        );
        assert_eq!(map_soc_codename("Dimensity 9000"), "Dimensity 9000");
    }

    #[test]
    fn map_soc_codename_unknown_capitalizes() {
        assert_eq!(map_soc_codename("somechip"), "Somechip");
    }

    #[test]
    fn map_soc_codename_case_insensitive() {
        assert_eq!(map_soc_codename("TARO"), "Snapdragon 8 Gen 1");
        assert_eq!(map_soc_codename("Kalama"), "Snapdragon 8 Gen 2");
    }

    #[test]
    fn map_soc_codename_empty_string() {
        assert_eq!(map_soc_codename(""), "");
    }
}

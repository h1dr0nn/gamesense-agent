/// UIAutomator accessibility tree integration.
///
/// Strategy: dump the Android accessibility tree via `adb shell uiautomator dump`,
/// parse the XML to extract clickable element bounds, then let the vision model
/// identify *which* element to tap — we provide the precise pixel coordinates from
/// the tree instead of relying on the model's spatial guessing.
///
/// Many game engines (Unity, Unreal) do NOT expose accessibility trees, in which
/// case the dump succeeds but returns an empty or skeleton XML.  The caller must
/// check `UiTree::is_useful()` before trusting the result.
use crate::adb::AdbExecutor;
use crate::command_utils::hidden_command;
use crate::error::AppError;

/// A single interactive element found in the accessibility tree.
#[derive(Debug, Clone, PartialEq)]
pub struct UiElement {
    /// Human-readable text from `text` or `content-desc` attribute.
    pub label: String,
    /// `resource-id` attribute (e.g. `com.example:id/button_ok`).
    pub resource_id: String,
    /// Class name (e.g. `android.widget.Button`).
    pub class: String,
    /// Center X as percentage of screen width (0–100).
    pub center_x_pct: f64,
    /// Center Y as percentage of screen height (0–100).
    pub center_y_pct: f64,
    /// Raw pixel bounds `[x1, y1, x2, y2]`.
    pub bounds: [i32; 4],
    pub clickable: bool,
}

impl UiElement {
    /// Absolute pixel tap coordinates (center of the element).
    pub fn tap_coords(&self) -> (i32, i32) {
        let cx = (self.bounds[0] + self.bounds[2]) / 2;
        let cy = (self.bounds[1] + self.bounds[3]) / 2;
        (cx, cy)
    }

    /// `tap:X%:Y%` action string this element maps to.
    pub fn tap_action(&self) -> String {
        format!("tap:{:.0}%:{:.0}%", self.center_x_pct, self.center_y_pct)
    }
}

/// Parsed accessibility tree from one uiautomator dump.
#[derive(Debug, Clone)]
pub struct UiTree {
    pub elements: Vec<UiElement>,
    pub screen_width: i32,
    pub screen_height: i32,
}

impl UiTree {
    /// Returns `true` if the tree contains at least one clickable element,
    /// meaning the accessibility layer is populated for this app.
    pub fn is_useful(&self) -> bool {
        self.elements.iter().any(|e| e.clickable)
    }

    /// Find the first clickable element whose label or resource_id contains
    /// the given hint (case-insensitive substring match).
    pub fn find_by_hint(&self, hint: &str) -> Option<&UiElement> {
        let lower = hint.to_lowercase();
        self.elements.iter().find(|e| {
            e.clickable
                && (e.label.to_lowercase().contains(&lower)
                    || e.resource_id.to_lowercase().contains(&lower))
        })
    }

    /// Return all clickable elements as a compact description list
    /// suitable for injection into a prompt.
    pub fn describe_clickable(&self) -> Vec<String> {
        self.elements
            .iter()
            .filter(|e| e.clickable)
            .map(|e| {
                let label = if e.label.is_empty() {
                    e.resource_id.split('/').last().unwrap_or("?").to_string()
                } else {
                    e.label.clone()
                };
                format!(
                    "\"{}\" at {:.0}%:{:.0}% ({})",
                    label, e.center_x_pct, e.center_y_pct, e.class
                )
            })
            .collect()
    }
}

/// Dump the accessibility tree from the device, parse, and return a `UiTree`.
///
/// Returns `Err` only on ADB failures. A successful dump with no elements
/// (e.g. Unity game) returns `Ok(UiTree { elements: [] })`.
pub fn dump_ui_tree(device_id: &str, screen_w: u32, screen_h: u32) -> Result<UiTree, AppError> {
    let executor = AdbExecutor::new();
    let adb_path = executor.get_adb_path();

    // Dump to device temp file then pull via stdout for speed
    let dump_output = hidden_command(adb_path)
        .args([
            "-s",
            device_id,
            "shell",
            "uiautomator",
            "dump",
            "/sdcard/ui_dump.xml",
            "&&",
            "cat",
            "/sdcard/ui_dump.xml",
        ])
        .output()
        .map_err(|e| {
            AppError::new(
                "UIAUTOMATOR_FAILED",
                &format!("uiautomator dump failed: {}", e),
            )
        })?;

    // uiautomator may succeed with a non-zero exit code on some devices — treat output as best-effort
    let xml = String::from_utf8_lossy(&dump_output.stdout);

    if xml.trim().is_empty() {
        // Fall back: pull the file directly
        let pull_output = hidden_command(adb_path)
            .args(["-s", device_id, "shell", "cat", "/sdcard/ui_dump.xml"])
            .output()
            .map_err(|e| {
                AppError::new(
                    "UIAUTOMATOR_FAILED",
                    &format!("Failed to read ui_dump.xml: {}", e),
                )
            })?;
        let xml2 = String::from_utf8_lossy(&pull_output.stdout);
        return Ok(parse_ui_xml(&xml2, screen_w, screen_h));
    }

    Ok(parse_ui_xml(&xml, screen_w, screen_h))
}

/// Parse the uiautomator XML string into a `UiTree`.
///
/// The XML format is:
/// ```xml
/// <hierarchy rotation="0">
///   <node ... bounds="[x1,y1][x2,y2]" clickable="true" text="OK" content-desc="" class="..." resource-id="..." />
///   ...
/// </hierarchy>
/// ```
///
/// We use simple string parsing instead of a full XML library to keep
/// dependencies minimal and avoid panics on malformed output.
pub fn parse_ui_xml(xml: &str, screen_w: u32, screen_h: u32) -> UiTree {
    let mut elements = Vec::new();

    for line in xml.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with("<node") {
            continue;
        }

        let clickable = attr_value(trimmed, "clickable") == "true"
            || attr_value(trimmed, "long-clickable") == "true";

        let bounds_str = attr_value(trimmed, "bounds");
        let Some(bounds) = parse_bounds(&bounds_str) else {
            continue;
        };

        let label = {
            let text = attr_value(trimmed, "text");
            let desc = attr_value(trimmed, "content-desc");
            if !text.is_empty() { text } else { desc }
        };

        let cx = (bounds[0] + bounds[2]) / 2;
        let cy = (bounds[1] + bounds[3]) / 2;
        let center_x_pct = if screen_w > 0 { cx as f64 * 100.0 / screen_w as f64 } else { 0.0 };
        let center_y_pct = if screen_h > 0 { cy as f64 * 100.0 / screen_h as f64 } else { 0.0 };

        elements.push(UiElement {
            label,
            resource_id: attr_value(trimmed, "resource-id"),
            class: attr_value(trimmed, "class"),
            center_x_pct,
            center_y_pct,
            bounds,
            clickable,
        });
    }

    UiTree {
        elements,
        screen_width: screen_w as i32,
        screen_height: screen_h as i32,
    }
}

/// Extract the value of `name="value"` from an XML attribute string.
/// Returns an empty string if the attribute is not found.
fn attr_value(node: &str, name: &str) -> String {
    let needle = format!("{}=\"", name);
    let start = match node.find(&needle) {
        Some(i) => i + needle.len(),
        None => return String::new(),
    };
    let rest = &node[start..];
    let end = rest.find('"').unwrap_or(rest.len());
    rest[..end].to_string()
}

/// Parse `[x1,y1][x2,y2]` bounds string into `[x1, y1, x2, y2]`.
fn parse_bounds(bounds_str: &str) -> Option<[i32; 4]> {
    // Strip surrounding quotes if present
    let s = bounds_str.trim_matches('"');
    // Format: [x1,y1][x2,y2]
    let nums: Vec<i32> = s
        .split(|c: char| !c.is_ascii_digit() && c != '-')
        .filter(|t| !t.is_empty())
        .filter_map(|t| t.parse().ok())
        .collect();
    if nums.len() >= 4 {
        Some([nums[0], nums[1], nums[2], nums[3]])
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<hierarchy rotation="0">
  <node index="0" text="" resource-id="" class="android.widget.FrameLayout" package="com.example" content-desc="" checkable="false" checked="false" clickable="false" enabled="true" focusable="false" focused="false" scrollable="false" long-clickable="false" password="false" selected="false" bounds="[0,0][1080,2340]">
    <node index="0" text="Start Game" resource-id="com.example:id/btn_start" class="android.widget.Button" package="com.example" content-desc="" checkable="false" checked="false" clickable="true" enabled="true" focusable="true" focused="false" scrollable="false" long-clickable="false" password="false" selected="false" bounds="[216,900][864,1080]" />
    <node index="1" text="" resource-id="" class="android.widget.ImageView" package="com.example" content-desc="Coin Stack" checkable="false" checked="false" clickable="true" enabled="true" focusable="true" focused="false" scrollable="false" long-clickable="false" password="false" selected="false" bounds="[0,500][540,800]" />
    <node index="2" text="Score: 100" resource-id="com.example:id/score" class="android.widget.TextView" package="com.example" content-desc="" checkable="false" checked="false" clickable="false" enabled="true" focusable="false" focused="false" scrollable="false" long-clickable="false" password="false" selected="false" bounds="[0,0][540,100]" />
  </node>
</hierarchy>"#;

    #[test]
    fn parses_clickable_button() {
        let tree = parse_ui_xml(SAMPLE_XML, 1080, 2340);
        let btn = tree.find_by_hint("Start Game").unwrap();
        assert_eq!(btn.label, "Start Game");
        assert!(btn.clickable);
        assert_eq!(btn.bounds, [216, 900, 864, 1080]);
    }

    #[test]
    fn center_coords_computed_correctly() {
        let tree = parse_ui_xml(SAMPLE_XML, 1080, 2340);
        let btn = tree.find_by_hint("Start Game").unwrap();
        let (cx, cy) = btn.tap_coords();
        assert_eq!(cx, (216 + 864) / 2); // 540
        assert_eq!(cy, (900 + 1080) / 2); // 990
    }

    #[test]
    fn percentage_coordinates_correct() {
        let tree = parse_ui_xml(SAMPLE_XML, 1080, 2340);
        let btn = tree.find_by_hint("Start Game").unwrap();
        // cx = 540 / 1080 * 100 = 50%
        assert!((btn.center_x_pct - 50.0).abs() < 1.0);
    }

    #[test]
    fn tap_action_string_format() {
        let tree = parse_ui_xml(SAMPLE_XML, 1080, 2340);
        let btn = tree.find_by_hint("Start Game").unwrap();
        let action = btn.tap_action();
        assert!(action.starts_with("tap:"));
        assert!(action.contains('%'));
    }

    #[test]
    fn finds_by_content_desc() {
        let tree = parse_ui_xml(SAMPLE_XML, 1080, 2340);
        let img = tree.find_by_hint("Coin Stack").unwrap();
        assert_eq!(img.label, "Coin Stack");
        assert!(img.clickable);
    }

    #[test]
    fn non_clickable_element_excluded_from_find() {
        let tree = parse_ui_xml(SAMPLE_XML, 1080, 2340);
        // "Score: 100" is not clickable
        assert!(tree.find_by_hint("Score").is_none());
    }

    #[test]
    fn is_useful_true_when_clickable_elements_exist() {
        let tree = parse_ui_xml(SAMPLE_XML, 1080, 2340);
        assert!(tree.is_useful());
    }

    #[test]
    fn is_useful_false_for_empty_tree() {
        let tree = parse_ui_xml("<hierarchy></hierarchy>", 1080, 2340);
        assert!(!tree.is_useful());
    }

    #[test]
    fn describe_clickable_lists_elements() {
        let tree = parse_ui_xml(SAMPLE_XML, 1080, 2340);
        let desc = tree.describe_clickable();
        assert_eq!(desc.len(), 2);
        assert!(desc[0].contains("Start Game"));
        assert!(desc[0].contains('%'));
    }

    #[test]
    fn parse_bounds_valid() {
        assert_eq!(parse_bounds("[216,900][864,1080]"), Some([216, 900, 864, 1080]));
    }

    #[test]
    fn parse_bounds_zero() {
        assert_eq!(parse_bounds("[0,0][1080,2340]"), Some([0, 0, 1080, 2340]));
    }

    #[test]
    fn parse_bounds_empty_returns_none() {
        assert!(parse_bounds("").is_none());
        assert!(parse_bounds("[][]").is_none());
    }

    #[test]
    fn attr_value_extracts_text() {
        let node = r#"<node text="Hello" clickable="true" bounds="[0,0][100,100]" />"#;
        assert_eq!(attr_value(node, "text"), "Hello");
        assert_eq!(attr_value(node, "clickable"), "true");
    }

    #[test]
    fn attr_value_missing_returns_empty() {
        let node = r#"<node text="Hi" />"#;
        assert_eq!(attr_value(node, "missing"), "");
    }
}

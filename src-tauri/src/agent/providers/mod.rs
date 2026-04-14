pub mod gemini;
pub mod openai;

use crate::error::AppError;

/// Common response type for all vision providers.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AgentResponse {
    pub reasoning: String,
    pub action: String,
    pub confidence: f64,
    /// Optional structured screen analysis (AppAgent-style).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub screen_analysis: Option<ScreenAnalysis>,
    /// game_state is optional — new prompt uses screen_analysis.game_state instead.
    #[serde(default)]
    pub game_state: GameStateResponse,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ScreenAnalysis {
    #[serde(default)]
    pub tutorial_indicator: Option<String>,
    #[serde(default)]
    pub interactive_elements: Vec<String>,
    /// "tutorial" | "playing" | "menu" | "game_over" | "won"
    #[serde(default)]
    pub game_state: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct GameStateResponse {
    /// Grid is optional and ignored — models return inconsistent types across games.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grid: Option<serde_json::Value>,
    #[serde(default)]
    pub score: u32,
    #[serde(default)]
    pub status: String,
}

/// Call the appropriate text-only provider for summarization.
///
/// - `base_url = None` → Gemini API
/// - `base_url = Some(url)` → OpenAI-compatible endpoint
pub async fn call_text(
    api_key: &str,
    model: &str,
    prompt: &str,
    base_url: Option<&str>,
) -> Result<String, AppError> {
    match base_url {
        Some(url) if !url.is_empty() => openai::call_text(api_key, model, prompt, url).await,
        _ => gemini::call_text(api_key, model, prompt).await,
    }
}

/// Extract the first complete JSON object `{...}` from a string that may contain
/// markdown code fences, leading/trailing prose, or chain-of-thought text.
/// Uses bracket depth counting so it always returns the first complete object,
/// even when the model appends additional JSON blocks or explanatory text after.
fn extract_json(text: &str) -> &str {
    let bytes = text.as_bytes();
    let mut start = None;
    let mut depth = 0i32;

    for (i, &b) in bytes.iter().enumerate() {
        match b {
            b'{' => {
                if depth == 0 {
                    start = Some(i);
                }
                depth += 1;
            }
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    if let Some(s) = start {
                        return &text[s..=i];
                    }
                }
            }
            _ => {}
        }
    }

    text.trim()
}

/// Call the appropriate vision provider based on `base_url`.
///
/// - `base_url = None` → Gemini API
/// - `base_url = Some(url)` → OpenAI-compatible endpoint
pub async fn call_vision(
    api_key: &str,
    model: &str,
    prompt: &str,
    image_base64: &str,
    base_url: Option<&str>,
) -> Result<AgentResponse, AppError> {
    let text = match base_url {
        Some(url) if !url.is_empty() => {
            openai::call(api_key, model, prompt, image_base64, url).await?
        }
        _ => gemini::call(api_key, model, prompt, image_base64).await?,
    };

    let json_str = extract_json(&text);
    let mut response: AgentResponse = serde_json::from_str(json_str).map_err(|e| {
        AppError::new(
            "API_JSON_PARSE_FAILED",
            &format!("Invalid JSON from API: {}. Raw: {}", e, text),
        )
    })?;

    // Clamp tap coordinates to valid range (model sometimes hallucinates > 100%)
    clamp_tap_coordinates(&mut response.action);

    Ok(response)
}

/// If `action` is a `tap:X%:Y%` string, clamp X and Y to [0, 100].
fn clamp_tap_coordinates(action: &mut String) {
    let Some(s) = action.strip_prefix("tap:") else { return };
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 2 { return }
    let parse = |p: &str| -> Option<f64> { p.trim_end_matches('%').parse().ok() };
    if let (Some(x), Some(y)) = (parse(parts[0]), parse(parts[1])) {
        let x = x.clamp(0.0, 100.0);
        let y = y.clamp(0.0, 100.0);
        *action = format!("tap:{:.0}%:{:.0}%", x, y);
    }
}

use crate::error::AppError;
use serde::{Deserialize, Serialize};

const API_BASE: &str = "https://generativelanguage.googleapis.com/v1beta/models";

#[derive(Serialize)]
struct Request {
    contents: Vec<Content>,
    #[serde(rename = "generationConfig")]
    generation_config: GenerationConfig,
}

#[derive(Serialize)]
struct Content {
    parts: Vec<Part>,
}

#[derive(Serialize)]
#[serde(untagged)]
enum Part {
    Text { text: String },
    Image {
        #[serde(rename = "inlineData")]
        inline_data: InlineData,
    },
}

#[derive(Serialize)]
struct InlineData {
    #[serde(rename = "mimeType")]
    mime_type: String,
    data: String,
}

#[derive(Serialize)]
struct GenerationConfig {
    #[serde(rename = "responseMimeType")]
    response_mime_type: String,
    temperature: f64,
    #[serde(rename = "maxOutputTokens")]
    max_output_tokens: u32,
}

#[derive(Deserialize)]
struct Response {
    candidates: Vec<Candidate>,
}

#[derive(Deserialize)]
struct Candidate {
    content: CandidateContent,
}

#[derive(Deserialize)]
struct CandidateContent {
    parts: Vec<ResponsePart>,
}

#[derive(Deserialize)]
struct ResponsePart {
    text: String,
}

/// Call Gemini with a text-only prompt (no image). Returns raw text response.
pub async fn call_text(api_key: &str, model: &str, prompt: &str) -> Result<String, AppError> {
    let url = format!("{}/{}:generateContent?key={}", API_BASE, model, api_key);

    let request = Request {
        contents: vec![Content {
            parts: vec![Part::Text {
                text: prompt.to_string(),
            }],
        }],
        generation_config: GenerationConfig {
            response_mime_type: "text/plain".to_string(),
            temperature: 0.3,
            max_output_tokens: 1024,
        },
    };

    let client = reqwest::Client::new();
    let response = client
        .post(&url)
        .json(&request)
        .send()
        .await
        .map_err(|e| {
            AppError::new("GEMINI_REQUEST_FAILED", &format!("HTTP request failed: {}", e))
        })?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(match status.as_u16() {
            401 => AppError::new("GEMINI_AUTH_FAILED", "Invalid API key"),
            429 => AppError::new("GEMINI_RATE_LIMITED", "Rate limit exceeded, try again later"),
            _ => AppError::new("GEMINI_API_ERROR", &format!("API error {}: {}", status, body)),
        });
    }

    let parsed: Response = response.json().await.map_err(|e| {
        AppError::new("GEMINI_PARSE_FAILED", &format!("Failed to parse response: {}", e))
    })?;

    parsed
        .candidates
        .first()
        .and_then(|c| c.content.parts.first())
        .map(|p| p.text.clone())
        .ok_or_else(|| AppError::new("GEMINI_EMPTY_RESPONSE", "No response from Gemini"))
}

pub async fn call(
    api_key: &str,
    model: &str,
    prompt: &str,
    image_base64: &str,
) -> Result<String, AppError> {
    let url = format!("{}/{}:generateContent?key={}", API_BASE, model, api_key);

    let request = Request {
        contents: vec![Content {
            parts: vec![
                Part::Text {
                    text: prompt.to_string(),
                },
                Part::Image {
                    inline_data: InlineData {
                        mime_type: "image/png".to_string(),
                        data: image_base64.to_string(),
                    },
                },
            ],
        }],
        generation_config: GenerationConfig {
            response_mime_type: "application/json".to_string(),
            temperature: 0.1,
            max_output_tokens: 1024,
        },
    };

    let client = reqwest::Client::new();
    let response = client
        .post(&url)
        .json(&request)
        .send()
        .await
        .map_err(|e| {
            AppError::new(
                "GEMINI_REQUEST_FAILED",
                &format!("HTTP request failed: {}", e),
            )
        })?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(match status.as_u16() {
            401 => AppError::new("GEMINI_AUTH_FAILED", "Invalid API key"),
            429 => AppError::new("GEMINI_RATE_LIMITED", "Rate limit exceeded, try again later"),
            _ => AppError::new(
                "GEMINI_API_ERROR",
                &format!("API error {}: {}", status, body),
            ),
        });
    }

    let parsed: Response = response.json().await.map_err(|e| {
        AppError::new(
            "GEMINI_PARSE_FAILED",
            &format!("Failed to parse response: {}", e),
        )
    })?;

    parsed
        .candidates
        .first()
        .and_then(|c| c.content.parts.first())
        .map(|p| p.text.clone())
        .ok_or_else(|| AppError::new("GEMINI_EMPTY_RESPONSE", "No response from Gemini"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_serializes_with_generation_config() {
        let req = Request {
            contents: vec![Content {
                parts: vec![Part::Text {
                    text: "test".to_string(),
                }],
            }],
            generation_config: GenerationConfig {
                response_mime_type: "application/json".to_string(),
                temperature: 0.1,
                max_output_tokens: 1024,
            },
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("generationConfig"));
        assert!(json.contains("responseMimeType"));
    }
}

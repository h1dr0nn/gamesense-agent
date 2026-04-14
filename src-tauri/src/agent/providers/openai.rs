use crate::error::AppError;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct Request {
    model: String,
    messages: Vec<Message>,
    #[serde(rename = "max_tokens")]
    max_tokens: u32,
    temperature: f64,
    response_format: ResponseFormat,
    stream: bool,
}

#[derive(Serialize)]
struct Message {
    role: String,
    content: Vec<Content>,
}

#[derive(Serialize)]
#[serde(untagged)]
enum Content {
    Text {
        #[serde(rename = "type")]
        content_type: String,
        text: String,
    },
    Image {
        #[serde(rename = "type")]
        content_type: String,
        image_url: ImageUrl,
    },
}

#[derive(Serialize)]
struct ImageUrl {
    url: String,
}

#[derive(Serialize)]
struct ResponseFormat {
    #[serde(rename = "type")]
    format_type: String,
}

#[derive(Deserialize)]
struct Response {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: ResponseMessage,
}

#[derive(Deserialize)]
struct ResponseMessage {
    content: String,
}

/// Call an OpenAI-compatible text-only endpoint (no image). Returns raw text response.
pub async fn call_text(api_key: &str, model: &str, prompt: &str, base_url: &str) -> Result<String, AppError> {
    let url = format!("{}/chat/completions", base_url.trim_end_matches('/'));

    let request = Request {
        model: model.to_string(),
        messages: vec![Message {
            role: "user".to_string(),
            content: vec![Content::Text {
                content_type: "text".to_string(),
                text: prompt.to_string(),
            }],
        }],
        max_tokens: 1024,
        temperature: 0.3,
        stream: false,
        response_format: ResponseFormat {
            format_type: "text".to_string(),
        },
    };

    let mut req = reqwest::Client::new().post(&url).json(&request);
    if !api_key.is_empty() {
        req = req.header("Authorization", format!("Bearer {}", api_key));
    }

    let response = req.send().await.map_err(|e| {
        AppError::new("OPENAI_REQUEST_FAILED", &format!("HTTP request failed: {}", e))
    })?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(match status.as_u16() {
            401 => AppError::new("OPENAI_AUTH_FAILED", "Invalid API key"),
            429 => AppError::new("OPENAI_RATE_LIMITED", "Rate limit exceeded, try again later"),
            _ => AppError::new("OPENAI_API_ERROR", &format!("API error {}: {}", status, body)),
        });
    }

    let body = response.text().await.map_err(|e| {
        AppError::new("OPENAI_READ_FAILED", &format!("Failed to read response body: {}", e))
    })?;

    let parsed: Response = serde_json::from_str(&body).map_err(|e| {
        AppError::new(
            "OPENAI_PARSE_FAILED",
            &format!("Failed to parse response: {}. Body: {}", e, &body[..body.len().min(500)]),
        )
    })?;

    parsed
        .choices
        .first()
        .map(|c| c.message.content.clone())
        .ok_or_else(|| AppError::new("OPENAI_EMPTY_RESPONSE", "No response from API"))
}

/// Call an OpenAI-compatible vision endpoint.
///
/// `base_url` should be the base URL without trailing slash, e.g.:
/// - `https://api.openai.com/v1`
/// - `http://localhost:11434/v1` (Ollama)
/// - A custom proxy URL
pub async fn call(
    api_key: &str,
    model: &str,
    prompt: &str,
    image_base64: &str,
    base_url: &str,
) -> Result<String, AppError> {
    let url = format!("{}/chat/completions", base_url.trim_end_matches('/'));
    let image_data_url = format!("data:image/png;base64,{}", image_base64);

    let request = Request {
        model: model.to_string(),
        messages: vec![Message {
            role: "user".to_string(),
            content: vec![
                Content::Text {
                    content_type: "text".to_string(),
                    text: prompt.to_string(),
                },
                Content::Image {
                    content_type: "image_url".to_string(),
                    image_url: ImageUrl { url: image_data_url },
                },
            ],
        }],
        max_tokens: 1024,
        temperature: 0.1,
        stream: false,
        response_format: ResponseFormat {
            format_type: "json_object".to_string(),
        },
    };

    let mut req = reqwest::Client::new().post(&url).json(&request);
    if !api_key.is_empty() {
        req = req.header("Authorization", format!("Bearer {}", api_key));
    }

    let response = req.send().await.map_err(|e| {
        AppError::new(
            "OPENAI_REQUEST_FAILED",
            &format!("HTTP request failed: {}", e),
        )
    })?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(match status.as_u16() {
            401 => AppError::new("OPENAI_AUTH_FAILED", "Invalid API key"),
            429 => AppError::new("OPENAI_RATE_LIMITED", "Rate limit exceeded, try again later"),
            _ => AppError::new(
                "OPENAI_API_ERROR",
                &format!("API error {}: {}", status, body),
            ),
        });
    }

    let body = response.text().await.map_err(|e| {
        AppError::new("OPENAI_READ_FAILED", &format!("Failed to read response body: {}", e))
    })?;
    eprintln!("[openai] raw response body: {}", &body[..body.len().min(2000)]);

    let parsed: Response = serde_json::from_str(&body).map_err(|e| {
        AppError::new(
            "OPENAI_PARSE_FAILED",
            &format!("Failed to parse response: {}. Body: {}", e, &body[..body.len().min(500)]),
        )
    })?;

    let content = parsed
        .choices
        .first()
        .map(|c| c.message.content.clone())
        .ok_or_else(|| AppError::new("OPENAI_EMPTY_RESPONSE", "No response from API"))?;

    Ok(extract_json(&content))
}

/// Strip markdown code fences if the model wraps JSON in ```json ... ```
fn extract_json(content: &str) -> String {
    let trimmed = content.trim();
    // Handle ```json\n{...}\n``` or ```\n{...}\n```
    if let Some(inner) = trimmed
        .strip_prefix("```json")
        .or_else(|| trimmed.strip_prefix("```"))
    {
        if let Some(end) = inner.rfind("```") {
            return inner[..end].trim().to_string();
        }
    }
    trimmed.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_serializes_with_response_format() {
        let req = Request {
            model: "gpt-4o".to_string(),
            messages: vec![Message {
                role: "user".to_string(),
                content: vec![Content::Text {
                    content_type: "text".to_string(),
                    text: "test".to_string(),
                }],
            }],
            max_tokens: 1024,
            temperature: 0.1,
            stream: false,
            response_format: ResponseFormat {
                format_type: "json_object".to_string(),
            },
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("response_format"));
        assert!(json.contains("json_object"));
        assert!(json.contains("\"stream\":false"));
    }

    #[test]
    fn extract_json_strips_markdown_fence() {
        let with_fence = "```json\n{\"key\": \"value\"}\n```";
        assert_eq!(extract_json(with_fence), "{\"key\": \"value\"}");
    }

    #[test]
    fn extract_json_strips_bare_fence() {
        let bare = "```\n{\"key\": 1}\n```";
        assert_eq!(extract_json(bare), "{\"key\": 1}");
    }

    #[test]
    fn extract_json_passthrough_plain_json() {
        let plain = "{\"key\": \"value\"}";
        assert_eq!(extract_json(plain), plain);
    }
}

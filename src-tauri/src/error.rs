// GameSense Agent - Error Types
// Centralized error handling for the application

use serde::Serialize;

/// Application-wide error type
#[derive(Debug, Clone, Serialize)]
pub struct AppError {
    pub code: String,
    pub message: String,
    pub details: Option<String>,
}

impl AppError {
    pub fn new(code: &str, message: &str) -> Self {
        Self {
            code: code.to_string(),
            message: message.to_string(),
            details: None,
        }
    }

    pub fn with_details(code: &str, message: &str, details: &str) -> Self {
        Self {
            code: code.to_string(),
            message: message.to_string(),
            details: Some(details.to_string()),
        }
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.code, self.message)
    }
}

impl std::error::Error for AppError {}

/// ADB-specific errors
#[derive(Debug, Clone, Serialize)]
pub enum AdbError {
    NotFound,
    ExecutionFailed(String),
    ParseError(String),
    DeviceNotFound(String),
    Unauthorized(String),
    Timeout,
}

impl From<AdbError> for AppError {
    fn from(err: AdbError) -> Self {
        match err {
            AdbError::NotFound => AppError::new(
                "ADB_NOT_FOUND",
                "ADB executable not found. Please ensure Android platform-tools are installed.",
            ),
            AdbError::ExecutionFailed(msg) => AppError::with_details(
                "ADB_EXECUTION_FAILED",
                "Failed to execute ADB command",
                &msg,
            ),
            AdbError::ParseError(msg) => AppError::with_details(
                "ADB_PARSE_ERROR",
                "Failed to parse ADB output",
                &msg,
            ),
            AdbError::DeviceNotFound(id) => AppError::with_details(
                "DEVICE_NOT_FOUND",
                "Device not found or disconnected",
                &id,
            ),
            AdbError::Unauthorized(id) => AppError::with_details(
                "DEVICE_UNAUTHORIZED",
                "Device requires USB debugging authorization",
                &id,
            ),
            AdbError::Timeout => AppError::new(
                "ADB_TIMEOUT",
                "ADB command timed out",
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_error_new() {
        let err = AppError::new("TEST", "test message");
        assert_eq!(err.code, "TEST");
        assert_eq!(err.message, "test message");
        assert!(err.details.is_none());
    }

    #[test]
    fn app_error_with_details() {
        let err = AppError::with_details("TEST", "msg", "detail");
        assert_eq!(err.details.as_deref(), Some("detail"));
    }

    #[test]
    fn app_error_display() {
        let err = AppError::new("CODE", "message");
        assert_eq!(format!("{}", err), "[CODE] message");
    }

    #[test]
    fn adb_error_not_found_converts() {
        let err: AppError = AdbError::NotFound.into();
        assert_eq!(err.code, "ADB_NOT_FOUND");
    }

    #[test]
    fn adb_error_execution_failed_converts() {
        let err: AppError = AdbError::ExecutionFailed("test".into()).into();
        assert_eq!(err.code, "ADB_EXECUTION_FAILED");
        assert_eq!(err.details.as_deref(), Some("test"));
    }

    #[test]
    fn adb_error_device_not_found_converts() {
        let err: AppError = AdbError::DeviceNotFound("ABC123".into()).into();
        assert_eq!(err.code, "DEVICE_NOT_FOUND");
        assert_eq!(err.details.as_deref(), Some("ABC123"));
    }

    #[test]
    fn adb_error_unauthorized_converts() {
        let err: AppError = AdbError::Unauthorized("ABC123".into()).into();
        assert_eq!(err.code, "DEVICE_UNAUTHORIZED");
    }

    #[test]
    fn adb_error_timeout_converts() {
        let err: AppError = AdbError::Timeout.into();
        assert_eq!(err.code, "ADB_TIMEOUT");
    }

    #[test]
    fn adb_error_parse_error_converts() {
        let err: AppError = AdbError::ParseError("bad output".into()).into();
        assert_eq!(err.code, "ADB_PARSE_ERROR");
        assert_eq!(err.details.as_deref(), Some("bad output"));
    }
}

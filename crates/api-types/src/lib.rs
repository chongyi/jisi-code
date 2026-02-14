//! Shared request/response types used by API-facing crates.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HealthCheckResponse {
    pub status: String,
}

impl HealthCheckResponse {
    #[must_use]
    pub fn ok() -> Self {
        Self {
            status: "ok".to_string(),
        }
    }
}

/// Convenience alias for handlers that prefer a shorter type name.
pub type HealthResponse = HealthCheckResponse;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub code: String,
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn health_check_ok_payload() {
        let response = HealthCheckResponse::ok();
        assert_eq!(response.status, "ok");
    }

    #[test]
    fn error_response_round_trip_json() {
        let response = ErrorResponse {
            code: "not_found".to_string(),
            message: "resource missing".to_string(),
        };

        let json = serde_json::to_string(&response).expect("serialize error response");
        let decoded: ErrorResponse =
            serde_json::from_str(&json).expect("deserialize error response");

        assert_eq!(decoded, response);
    }
}

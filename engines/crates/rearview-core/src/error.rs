use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;

pub type RearviewResult<T> = Result<T, RearviewError>;

#[derive(Debug, thiserror::Error)]
pub enum RearviewError {
    #[error("configuration error: {0}")]
    Config(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("gone: {0}")]
    Gone(String),
    #[error("conflict: {0}")]
    Conflict(String),
    #[error("portfolio pending first run: {0}")]
    PortfolioPendingFirstRun(String),
    #[error("validation error: {0}")]
    Validation(String),
    #[error("metric catalog error: {0}")]
    MetricCatalog(String),
    #[error("query planning error: {0}")]
    Planner(String),
    #[error("postgres error: {0}")]
    Postgres(#[from] sqlx::Error),
    #[error("clickhouse error: {0}")]
    ClickHouse(String),
    #[error("nats error: {0}")]
    Nats(String),
    #[error("http client error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("yaml error: {0}")]
    Yaml(#[from] serde_yaml::Error),
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    error_type: &'static str,
    message: String,
}

impl RearviewError {
    pub fn status_code(&self) -> StatusCode {
        match self {
            Self::NotFound(_) => StatusCode::NOT_FOUND,
            Self::Gone(_) => StatusCode::GONE,
            Self::Conflict(_) | Self::PortfolioPendingFirstRun(_) => StatusCode::CONFLICT,
            Self::Config(_)
            | Self::Postgres(_)
            | Self::ClickHouse(_)
            | Self::Nats(_)
            | Self::Http(_)
            | Self::Io(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::Validation(_) | Self::MetricCatalog(_) | Self::Planner(_) => {
                StatusCode::BAD_REQUEST
            }
            Self::Json(_) | Self::Yaml(_) => StatusCode::BAD_REQUEST,
        }
    }

    pub fn error_type(&self) -> &'static str {
        match self {
            Self::Config(_) => "config",
            Self::NotFound(_) => "not_found",
            Self::Gone(_) => "gone",
            Self::Conflict(_) => "conflict",
            Self::PortfolioPendingFirstRun(_) => "portfolio_pending_first_run",
            Self::Validation(_) => "validation",
            Self::MetricCatalog(_) => "metric_catalog",
            Self::Planner(_) => "planner",
            Self::Postgres(_) => "postgres",
            Self::ClickHouse(_) => "clickhouse",
            Self::Nats(_) => "nats",
            Self::Http(_) => "http",
            Self::Io(_) => "io",
            Self::Json(_) => "json",
            Self::Yaml(_) => "yaml",
        }
    }
}

impl IntoResponse for RearviewError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        let body = ErrorResponse {
            error_type: self.error_type(),
            message: self.to_string(),
        };
        (status, Json(body)).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gone_should_map_to_http_410_and_gone_error_type() {
        let error = RearviewError::Gone("strategy portfolio archived".to_string());

        assert_eq!(error.status_code(), StatusCode::GONE);
        assert_eq!(error.error_type(), "gone");
    }
}

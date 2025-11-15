// Health check endpoint
use actix_web::{get, HttpResponse, Result};
use crate::types::HealthResponse;
use chrono::Utc;

#[get("/healthz")]
pub async fn healthz() -> Result<HttpResponse> {
    let response = HealthResponse {
        status: "ok".to_string(),
        time: Utc::now().to_rfc3339(),
        version: option_env!("CARGO_PKG_VERSION").map(|s| s.to_string()),
    };
    Ok(HttpResponse::Ok().json(response))
}

#[get("/health")]
pub async fn health() -> Result<HttpResponse> {
    // Alias for compatibility
    let response = HealthResponse {
        status: "ok".to_string(),
        time: Utc::now().to_rfc3339(),
        version: option_env!("CARGO_PKG_VERSION").map(|s| s.to_string()),
    };
    Ok(HttpResponse::Ok().json(response))
}

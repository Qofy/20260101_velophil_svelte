// Shared type definitions
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

// ============================================================================
// Response wrappers
// ============================================================================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ItemMetadata {
    pub name: String,
    #[serde(rename = "updatedAt")]
    pub updated_at: u64,
    pub version: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checksum: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ListResponse<T> {
    pub items: Vec<T>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ItemResponse<T> {
    pub name: String,
    pub data: T,
    #[serde(rename = "updatedAt")]
    pub updated_at: u64,
    pub version: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UpsertRequest<T> {
    pub data: T,
    #[serde(rename = "baseVersion")]
    pub base_version: Option<u64>,
    #[serde(rename = "updatedAtClient")]
    pub updated_at_client: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UpsertResponse {
    pub version: u64,
    #[serde(rename = "updatedAt")]
    pub updated_at: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ConflictResponse {
    #[serde(rename = "serverVersion")]
    pub server_version: u64,
    #[serde(rename = "serverUpdatedAt")]
    pub server_updated_at: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VersionedData<T> {
    pub data: T,
    pub version: u64,
    pub updated_at: u64,
    pub created_at: u64,
}

// ============================================================================
// Custom configuration types
// ============================================================================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Position {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CustomLayout {
    pub positions: HashMap<String, Position>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NameEntry {
    pub name: String,
    pub mass: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CustomNamesConfig {
    pub names: HashMap<String, NameEntry>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CustomZoneConfig {
    pub zone_names: Vec<String>,
    pub assignments: HashMap<String, String>,
    pub colors: HashMap<String, String>,
    pub enabled: HashMap<String, bool>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CustomOrderConfig {
    pub data: Value,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CustomServerConfig {
    pub servers: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CustomClientConfig {
    pub clients: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CustomReservConfig {
    pub data: Value,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CustomPinsConfig {
    pub data: Value,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CustomKitchenConfig {
    pub data: Value,
}

// ============================================================================
// Runtime data types
// ============================================================================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RuntimeOrder {
    pub name: String,
    pub data: Value,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RuntimeReservation {
    pub name: String,
    pub data: Value,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RuntimeEvent {
    pub id: String,
    pub ts: u64,
    #[serde(rename = "type")]
    pub event_type: String,
    pub payload: Value,
    pub cursor: String,
}

// ============================================================================
// Health check response
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub time: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

// ============================================================================
// Error response (per-field errors)
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: ErrorDetail,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorDetail {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<HashMap<String, String>>,
}

impl ErrorResponse {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            error: ErrorDetail {
                code: code.into(),
                message: message.into(),
                details: None,
            },
        }
    }

    pub fn with_details(
        code: impl Into<String>,
        message: impl Into<String>,
        details: HashMap<String, String>,
    ) -> Self {
        Self {
            error: ErrorDetail {
                code: code.into(),
                message: message.into(),
                details: Some(details),
            },
        }
    }
}

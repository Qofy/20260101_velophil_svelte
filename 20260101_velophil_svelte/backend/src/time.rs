// src/time.rs
use chrono::Utc;

pub fn now() -> String {
    Utc::now().to_rfc3339()
}

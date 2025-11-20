// src/models/auth_types.rs
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::Utc;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String,          // user id
    pub email: String,
    pub roles: Vec<String>,   // e.g. ["user"], ["admin"], ["super_admin"]
    pub iss: String,
    pub aud: String,
    pub iat: i64,
    pub exp: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserRecord {
    pub id: String,
    pub email: String,
    pub password_hash: String,
    pub roles: Vec<String>,
    pub created_at: String,
}

impl UserRecord {
    pub fn new_admin(email: &str, password_hash: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            email: email.to_lowercase(),
            password_hash,
            roles: vec!["admin".into()],
            created_at: Utc::now().to_rfc3339(),
        }
    }

    pub fn new_user(email: &str, password_hash: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            email: email.to_lowercase(),
            password_hash,
            roles: vec!["user".into()],
            created_at: Utc::now().to_rfc3339(),
        }
    }
}

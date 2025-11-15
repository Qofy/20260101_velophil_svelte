// User management endpoints (admin only)
use actix_web::{get, put, web, HttpRequest, HttpResponse, Result};
use serde::{Deserialize, Serialize};
use crate::config::AppConfig;
use crate::db::Database;
use crate::handlers::cookies::{extract_token, ACCESS_COOKIE_NAME};
use crate::handlers::auth::validate_token;
use crate::models::auth_types::UserRecord;
use crate::types::ErrorResponse;

#[derive(Debug, Serialize, Deserialize)]
pub struct UserInfo {
    pub id: String,
    pub email: String,
    pub roles: Vec<String>,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UsersListResponse {
    pub users: Vec<UserInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateUserRolesRequest {
    pub roles: Vec<String>,
}

/// List all users (admin only)
#[get("/users")]
pub async fn list_users(
    db: web::Data<Database>,
    cfg: web::Data<AppConfig>,
    req: HttpRequest,
) -> Result<HttpResponse> {
    // Verify admin role
    let token = extract_token(&req, ACCESS_COOKIE_NAME)
        .ok_or_else(|| actix_web::error::ErrorUnauthorized("Missing authentication token"))?;

    let claims = validate_token(&cfg, &token)
        .ok_or_else(|| actix_web::error::ErrorUnauthorized("Invalid token"))?;

    if !claims.roles.contains(&"admin".to_string()) {
        return Ok(HttpResponse::Forbidden().json(ErrorResponse::new(
            "insufficient_permissions",
            "Admin role required"
        )));
    }

    // List all users
    let users: Vec<UserRecord> = db.list("users")
        .map_err(actix_web::error::ErrorInternalServerError)?;

    let user_infos: Vec<UserInfo> = users.into_iter().map(|u| UserInfo {
        id: u.id,
        email: u.email,
        roles: u.roles,
        created_at: u.created_at,
    }).collect();

    Ok(HttpResponse::Ok().json(UsersListResponse {
        users: user_infos,
    }))
}

/// Get specific user by ID (admin only)
#[get("/users/{user_id}")]
pub async fn get_user(
    path: web::Path<String>,
    db: web::Data<Database>,
    cfg: web::Data<AppConfig>,
    req: HttpRequest,
) -> Result<HttpResponse> {
    // Verify admin role
    let token = extract_token(&req, ACCESS_COOKIE_NAME)
        .ok_or_else(|| actix_web::error::ErrorUnauthorized("Missing authentication token"))?;

    let claims = validate_token(&cfg, &token)
        .ok_or_else(|| actix_web::error::ErrorUnauthorized("Invalid token"))?;

    if !claims.roles.contains(&"admin".to_string()) {
        return Ok(HttpResponse::Forbidden().json(ErrorResponse::new(
            "insufficient_permissions",
            "Admin role required"
        )));
    }

    let user_id = path.into_inner();

    // Get user by ID
    let user: UserRecord = db.get("users", &user_id)
        .map_err(actix_web::error::ErrorInternalServerError)?
        .ok_or_else(|| actix_web::error::ErrorNotFound("User not found"))?;

    let user_info = UserInfo {
        id: user.id,
        email: user.email,
        roles: user.roles,
        created_at: user.created_at,
    };

    Ok(HttpResponse::Ok().json(user_info))
}

/// Update user roles (admin only)
#[put("/users/{user_id}/roles")]
pub async fn update_user_roles(
    path: web::Path<String>,
    payload: web::Json<UpdateUserRolesRequest>,
    db: web::Data<Database>,
    cfg: web::Data<AppConfig>,
    req: HttpRequest,
) -> Result<HttpResponse> {
    // Verify admin role
    let token = extract_token(&req, ACCESS_COOKIE_NAME)
        .ok_or_else(|| actix_web::error::ErrorUnauthorized("Missing authentication token"))?;

    let claims = validate_token(&cfg, &token)
        .ok_or_else(|| actix_web::error::ErrorUnauthorized("Invalid token"))?;

    if !claims.roles.contains(&"admin".to_string()) {
        return Ok(HttpResponse::Forbidden().json(ErrorResponse::new(
            "insufficient_permissions",
            "Admin role required"
        )));
    }

    let user_id = path.into_inner();

    // Get existing user
    let mut user: UserRecord = db.get("users", &user_id)
        .map_err(actix_web::error::ErrorInternalServerError)?
        .ok_or_else(|| actix_web::error::ErrorNotFound("User not found"))?;

    // Update roles
    user.roles = payload.roles.clone();

    // Save updated user
    db.update("users", &user_id, &user)
        .map_err(actix_web::error::ErrorInternalServerError)?;

    let user_info = UserInfo {
        id: user.id,
        email: user.email,
        roles: user.roles,
        created_at: user.created_at,
    };

    Ok(HttpResponse::Ok().json(user_info))
}

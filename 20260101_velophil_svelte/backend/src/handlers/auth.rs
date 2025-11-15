// handlers/auth.rs
use actix_web::{post, get, web, HttpResponse, Result, HttpRequest};
use argon2::{Argon2, PasswordHasher, PasswordVerifier};
use argon2::password_hash::{SaltString, PasswordHash};
use rand_core::OsRng;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use pasetors::{local, keys::SymmetricKey, version4::V4, token::UntrustedToken, claims::{Claims as PasetoClaims, ClaimsValidationRules}};
use time::{OffsetDateTime, Duration as TimeDuration, format_description::well_known::Rfc3339};
use chrono::{Utc, Duration};
use serde_json::json;

use crate::{db::Database, models::auth_types::{UserRecord, RegisterRequest, LoginRequest, Claims}};
use crate::config::{AppConfig, TokenMode};
use crate::handlers::cookies::{set_auth_cookies, clear_auth_cookies, extract_token, ACCESS_COOKIE_NAME, REFRESH_COOKIE_NAME};


#[allow(dead_code)]
type HmacSha256 = Hmac<Sha256>;

#[allow(dead_code)]
fn base64url(data: &[u8]) -> String { URL_SAFE_NO_PAD.encode(data) }

#[allow(dead_code)]
fn sign_hs256(secret: &[u8], header_b64: &str, payload_b64: &str) -> String {
    let mut mac = HmacSha256::new_from_slice(secret).expect("HMAC can take key of any size");
    let signing_input = format!("{}.{}", header_b64, payload_b64);
    mac.update(signing_input.as_bytes());
    let sig = mac.finalize().into_bytes();
    base64url(&sig)
}

#[allow(dead_code)]
fn verify_hs256(secret: &[u8], token: &str) -> Option<(serde_json::Value, serde_json::Value)> {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 { return None; }
    let (h, p, s) = (parts[0], parts[1], parts[2]);
    let expected_sig = sign_hs256(secret, h, p);
    if expected_sig != s { return None; }
    let header = serde_json::from_slice(&URL_SAFE_NO_PAD.decode(h).ok()?).ok()?;
    let payload = serde_json::from_slice(&URL_SAFE_NO_PAD.decode(p).ok()?).ok()?;
    Some((header, payload))
}

#[allow(dead_code)]
fn default_secret(cfg: &AppConfig) -> Vec<u8> {
    if !cfg.security.paseto_v4_local_key_hex.is_empty() {
        hex::decode(cfg.security.paseto_v4_local_key_hex.trim()).unwrap_or_else(|_| cfg.security.paseto_v4_local_key_hex.clone().into_bytes())
    } else if !cfg.security.access_token.is_empty() {
        cfg.security.access_token.clone().into_bytes()
    } else {
        // Last resort
        b"quoteflow_default_secret".to_vec()
    }
}

#[allow(dead_code)]
fn make_token_hmac(cfg: &AppConfig, claims: &Claims) -> String {
    let header = json!({"alg":"HS256","typ":"JWT"});
    let header_b64 = base64url(&serde_json::to_vec(&header).unwrap());
    let payload_b64 = base64url(&serde_json::to_vec(claims).unwrap());
    let key = default_secret(cfg);
    let sig = sign_hs256(&key, &header_b64, &payload_b64);
    format!("{}.{}.{}", header_b64, payload_b64, sig)
}

#[allow(dead_code)]
fn validate_token_hmac(cfg: &AppConfig, token: &str) -> Option<Claims> {
    let key = default_secret(cfg);
    if let Some((_h, p)) = verify_hs256(&key, token) {
        let claims: Claims = serde_json::from_value(p).ok()?;
        let now = Utc::now().timestamp();
        if claims.exp < now { return None; }
        if claims.iss != cfg.security.token_iss || claims.aud != cfg.security.token_aud { return None; }
        Some(claims)
    } else if !cfg.security.access_token.is_empty() && cfg.security.access_token == token {
        // Static access token fallback (admin privileges)
        Some(Claims { sub: "access".into(), email: "access@local".into(), roles: vec!["admin".into()], iss: cfg.security.token_iss.clone(), aud: cfg.security.token_aud.clone(), iat: Utc::now().timestamp(), exp: (Utc::now() + Duration::hours(cfg.security.auth_token_expiry_hours as i64)).timestamp() })
    } else {
        None
    }
}

#[allow(dead_code)]
fn extract_bearer_or_query(req: &actix_web::HttpRequest) -> Option<String> {
    if let Some(h) = req.headers().get("authorization") {
        if let Ok(s) = h.to_str() { if let Some(rest) = s.strip_prefix("Bearer ") { return Some(rest.to_string()); } }
    }
    if let Some(q) = req.query_string().split('&').find(|p| p.starts_with("access_token=")) {
        if let Some(val) = q.split('=').nth(1) { return Some(val.to_string()); }
    }
    None
}



#[allow(dead_code)]
fn paseto_key(cfg: &AppConfig) -> Option<SymmetricKey<V4>> {
    let hex = cfg.security.paseto_v4_local_key_hex.trim();
    if hex.len() < 64 { return None; }
    let bytes = hex::decode(hex).ok()?;
    SymmetricKey::<V4>::from(&bytes).ok()
}


#[allow(dead_code)]
fn make_token_paseto(cfg: &AppConfig, claims: &Claims) -> Option<String> {
    let key = paseto_key(cfg)?;
    let mut pclaims = PasetoClaims::new().ok()?;
    pclaims.issuer(&cfg.security.token_iss).ok()?;
    pclaims.audience(&cfg.security.token_aud).ok()?;
    pclaims.subject(&claims.sub).ok()?;
    let now = OffsetDateTime::now_utc();
    let iat_str = now.format(&Rfc3339).ok()?;
    pclaims.issued_at(&iat_str).ok()?;
    let exp = now + TimeDuration::seconds(cfg.security.token_ttl_seconds as i64);
    let exp_str = exp.format(&Rfc3339).ok()?;
    pclaims.expiration(&exp_str).ok()?;
    // Additional claims
    pclaims.add_additional("email", serde_json::Value::String(claims.email.clone())).ok()?;
    pclaims.add_additional("roles", serde_json::to_value(&claims.roles).ok()?).ok()?;
    local::encrypt(&key, &pclaims, None, None).ok()
}

#[allow(dead_code)]
fn validate_token_paseto(cfg: &AppConfig, token: &str) -> Option<Claims> {
    let key = paseto_key(cfg)?;
    let utok = UntrustedToken::try_from(token).ok()?;
    let rules = ClaimsValidationRules::new();
    let trusted = local::decrypt(&key, &utok, &rules, None, None).ok()?;
    let pc = trusted.payload_claims()?;
    let iss = pc.get_claim("iss").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let aud = pc.get_claim("aud").and_then(|v| v.as_str()).unwrap_or("").to_string();
    if iss != cfg.security.token_iss || aud != cfg.security.token_aud { return None; }
    let sub = pc.get_claim("sub").and_then(|v| v.as_str()).unwrap_or("").to_string();
    // Additional
    let email = pc.get_claim("email").and_then(|v| v.as_str().map(|s| s.to_string())).unwrap_or_default();
    let roles = pc.get_claim("roles").and_then(|v| serde_json::from_value::<Vec<String>>(v.clone()).ok()).unwrap_or_default();
    Some(Claims { sub, email, roles, iss, aud, iat: 0, exp: 0 })
}

pub fn make_token(cfg: &AppConfig, claims: &Claims) -> Option<String> {
    match cfg.security.token_mode {
        TokenMode::JwtHmac => Some(make_token_hmac(cfg, claims)),
        TokenMode::PasetoV4Local => make_token_paseto(cfg, claims),
    }
}

pub fn validate_token(cfg: &AppConfig, token: &str) -> Option<Claims> {
    match cfg.security.token_mode {
        TokenMode::JwtHmac => validate_token_hmac(cfg, token),
        TokenMode::PasetoV4Local => validate_token_paseto(cfg, token),
    }
}

#[post("/register")]
pub async fn register(db: web::Data<Database>, cfg: web::Data<AppConfig>, body: web::Json<RegisterRequest>) -> Result<HttpResponse> {
    use crate::validation as v;

    let email = body.email.trim().to_lowercase();

    // Strict email validation
    if let Err(e) = v::validate_email_strict(&email) {
        return Ok(HttpResponse::BadRequest().json(json!({"error": e})));
    }

    // Strong password validation
    if let Err(e) = v::password_strength(&body.password) {
        return Ok(HttpResponse::BadRequest().json(json!({"error": e})));
    }

    // Existing user?
    let users: Vec<UserRecord> = db.list("users").unwrap_or_default();
    if users.iter().any(|u| u.email == email) {
        return Ok(HttpResponse::Conflict().json(json!({"error": "Email already registered"})));
    }

    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default().hash_password(body.password.as_bytes(), &salt).map_err(|_| actix_web::error::ErrorInternalServerError("hash error"))?.to_string();
    let user = if users.is_empty() { UserRecord::new_admin(&email, hash) } else { UserRecord::new_user(&email, hash) };
    db.insert("users", &user.id, &user).map_err(|_| actix_web::error::ErrorInternalServerError("db error"))?;

    let now = Utc::now();

    // Create access token (short-lived)
    let access_claims = Claims {
        sub: user.id.clone(),
        email: user.email.clone(),
        roles: user.roles.clone(),
        iss: cfg.security.token_iss.clone(),
        aud: cfg.security.token_aud.clone(),
        iat: now.timestamp(),
        exp: (now + Duration::seconds(cfg.security.token_ttl_seconds as i64)).timestamp()
    };
    let access_token = make_token(&cfg, &access_claims).ok_or_else(|| actix_web::error::ErrorInternalServerError("token error"))?;

    // Create refresh token (long-lived - 7 days)
    let refresh_claims = Claims {
        sub: user.id.clone(),
        email: user.email.clone(),
        roles: user.roles.clone(),
        iss: cfg.security.token_iss.clone(),
        aud: cfg.security.token_aud.clone(),
        iat: now.timestamp(),
        exp: (now + Duration::days(7)).timestamp(),
    };
    let refresh_token = make_token(&cfg, &refresh_claims).ok_or_else(|| actix_web::error::ErrorInternalServerError("token error"))?;

    // Set HttpOnly cookies
    let access_ttl = cfg.security.token_ttl_seconds as i64;
    let refresh_ttl = 7 * 24 * 60 * 60; // 7 days in seconds
    let cookie_secure = std::env::var("COOKIE_SECURE").unwrap_or_else(|_| "true".to_string()) == "true";
    let cookie_domain = std::env::var("COOKIE_DOMAIN").ok();

    let response = HttpResponse::Created().json(json!({
        "user": {
            "id": user.id,
            "email": user.email,
            "roles": user.roles
        }
    }));

    let response = set_auth_cookies(
        response,
        access_token.clone(),
        refresh_token,
        access_ttl,
        refresh_ttl,
        cookie_secure,
        cookie_domain.as_deref(),
    );

    Ok(response)
}

#[post("/login")]
pub async fn login(db: web::Data<Database>, cfg: web::Data<AppConfig>, body: web::Json<LoginRequest>) -> Result<HttpResponse> {
    use crate::validation as v;

    let email = body.email.trim().to_lowercase();

    // Basic email validation
    if let Err(e) = v::validate_email_strict(&email) {
        return Ok(HttpResponse::BadRequest().json(json!({"error": e})));
    }

    // Check password length (don't validate complexity on login, only on registration)
    if body.password.is_empty() || body.password.len() > 128 {
        return Ok(HttpResponse::BadRequest().json(json!({"error": "Invalid password"})));
    }

    let users: Vec<UserRecord> = db.list("users").unwrap_or_default();
    if let Some(u) = users.iter().find(|u| u.email == email) {
        let parsed = PasswordHash::new(&u.password_hash).map_err(|_| actix_web::error::ErrorInternalServerError("hash read error"))?;
        if Argon2::default().verify_password(body.password.as_bytes(), &parsed).is_ok() {
            let now = Utc::now();

            // Create access token (short-lived)
            let access_claims = Claims {
                sub: u.id.clone(),
                email: u.email.clone(),
                roles: u.roles.clone(),
                iss: cfg.security.token_iss.clone(),
                aud: cfg.security.token_aud.clone(),
                iat: now.timestamp(),
                exp: (now + Duration::seconds(cfg.security.token_ttl_seconds as i64)).timestamp()
            };
            let access_token = make_token(&cfg, &access_claims).ok_or_else(|| actix_web::error::ErrorInternalServerError("token error"))?;

            // Create refresh token (long-lived - 7 days)
            let refresh_claims = Claims {
                sub: u.id.clone(),
                email: u.email.clone(),
                roles: u.roles.clone(),
                iss: cfg.security.token_iss.clone(),
                aud: cfg.security.token_aud.clone(),
                iat: now.timestamp(),
                exp: (now + Duration::days(7)).timestamp(),
            };
            let refresh_token = make_token(&cfg, &refresh_claims).ok_or_else(|| actix_web::error::ErrorInternalServerError("token error"))?;

            // Set HttpOnly cookies
            let access_ttl = cfg.security.token_ttl_seconds as i64;
            let refresh_ttl = 7 * 24 * 60 * 60; // 7 days in seconds
            let cookie_secure = std::env::var("COOKIE_SECURE").unwrap_or_else(|_| "true".to_string()) == "true";
            let cookie_domain = std::env::var("COOKIE_DOMAIN").ok();

            // Return user data as JSON
            let response = HttpResponse::Ok().json(json!({
                "id": u.id,
                "email": u.email,
                "roles": u.roles
            }));

            let response = set_auth_cookies(
                response,
                access_token,
                refresh_token,
                access_ttl,
                refresh_ttl,
                cookie_secure,
                cookie_domain.as_deref(),
            );

            return Ok(response);
        }
    }
    // Return generic error to prevent user enumeration
    Ok(HttpResponse::Unauthorized().json(json!({"error": "Invalid email or password"})))
}

#[post("/logout")]
pub async fn logout(_db: web::Data<Database>) -> Result<HttpResponse> {
    let response = HttpResponse::NoContent().finish();
    let response = clear_auth_cookies(response);
    Ok(response)
}

#[derive(serde::Deserialize)]
pub struct ReconfirmRequest {
    password: String,
}

#[post("/reconfirm")]
pub async fn reconfirm(
    db: web::Data<Database>,
    cfg: web::Data<AppConfig>,
    req: actix_web::HttpRequest,
    body: web::Json<ReconfirmRequest>,
) -> Result<HttpResponse> {
    // Extract token from request (cookie or Bearer token)
    let token = extract_token(&req, ACCESS_COOKIE_NAME).ok_or_else(|| {
        actix_web::error::ErrorUnauthorized("No token provided")
    })?;

    // Validate token - we accept expired tokens to allow reconfirmation
    // Try to decode the token even if expired to get user ID
    let user_id = match cfg.security.token_mode {
        TokenMode::JwtHmac => {
            let key = default_secret(&cfg);
            if let Some((_h, p)) = verify_hs256(&key, &token) {
                let claims: Claims = serde_json::from_value(p).map_err(|_| {
                    actix_web::error::ErrorUnauthorized("Invalid token format")
                })?;
                // Don't check expiration - we want to allow expired tokens
                claims.sub
            } else {
                return Ok(HttpResponse::Unauthorized().json(json!({"error": "Invalid token"})));
            }
        }
        TokenMode::PasetoV4Local => {
            let key = paseto_key(&cfg).ok_or_else(|| {
                actix_web::error::ErrorInternalServerError("Paseto key not configured")
            })?;
            let utok = UntrustedToken::try_from(token.as_str()).map_err(|_| {
                actix_web::error::ErrorUnauthorized("Invalid token format")
            })?;
            // Decrypt without expiration validation
            let trusted = local::decrypt(&key, &utok, &ClaimsValidationRules::new(), None, None)
                .map_err(|_| actix_web::error::ErrorUnauthorized("Invalid token"))?;
            let pc = trusted.payload_claims().ok_or_else(|| {
                actix_web::error::ErrorUnauthorized("Invalid token claims")
            })?;
            pc.get_claim("sub")
                .and_then(|v| v.as_str())
                .ok_or_else(|| actix_web::error::ErrorUnauthorized("Missing user ID in token"))?
                .to_string()
        }
    };

    // Find user in database
    let users: Vec<UserRecord> = db.list("users").unwrap_or_default();
    let user = users.iter().find(|u| u.id == user_id).ok_or_else(|| {
        actix_web::error::ErrorUnauthorized("User not found")
    })?;

    // Verify password
    let parsed = PasswordHash::new(&user.password_hash).map_err(|_| {
        actix_web::error::ErrorInternalServerError("Password hash read error")
    })?;

    if Argon2::default()
        .verify_password(body.password.as_bytes(), &parsed)
        .is_err()
    {
        return Ok(HttpResponse::Unauthorized().json(json!({"error": "Invalid password"})));
    }

    // Password is correct - generate new token
    let now = Utc::now();
    let claims = Claims {
        sub: user.id.clone(),
        email: user.email.clone(),
        roles: user.roles.clone(),
        iss: cfg.security.token_iss.clone(),
        aud: cfg.security.token_aud.clone(),
        iat: now.timestamp(),
        exp: (now + Duration::seconds(cfg.security.token_ttl_seconds as i64)).timestamp(),
    };

    let new_token = make_token(&cfg, &claims).ok_or_else(|| {
        actix_web::error::ErrorInternalServerError("Token generation error")
    })?;

    Ok(HttpResponse::Ok().json(json!({
        "token": new_token,
        "user": {
            "id": user.id,
            "email": user.email,
            "roles": user.roles
        }
    })))
}

#[get("/me")]
pub async fn me(_db: web::Data<Database>, cfg: web::Data<AppConfig>, req: HttpRequest) -> Result<HttpResponse> {
    // Try to extract token from cookie or Authorization header
    if let Some(tok) = extract_token(&req, ACCESS_COOKIE_NAME) {
        if let Some(claims) = validate_token(&cfg, &tok) {
            return Ok(HttpResponse::Ok().json(json!({
                "id": claims.sub,
                "email": claims.email,
                "roles": claims.roles
            })));
        }
    }
    Ok(HttpResponse::Unauthorized().json(json!({"error": "unauthorized"})))
}

#[post("/refresh")]
pub async fn refresh(cfg: web::Data<AppConfig>, req: HttpRequest) -> Result<HttpResponse> {
    // Extract refresh token from cookie
    let refresh_token = extract_token(&req, REFRESH_COOKIE_NAME)
        .ok_or_else(|| actix_web::error::ErrorUnauthorized("No refresh token"))?;

    // Validate refresh token
    let claims = validate_token(&cfg, &refresh_token)
        .ok_or_else(|| actix_web::error::ErrorUnauthorized("Invalid or expired refresh token"))?;

    // Check if token is expired
    let now = Utc::now().timestamp();
    if claims.exp < now {
        return Ok(HttpResponse::Unauthorized().json(json!({"error": "Refresh token expired"})));
    }

    // Issue new access token (short-lived)
    let access_claims = Claims {
        sub: claims.sub.clone(),
        email: claims.email.clone(),
        roles: claims.roles.clone(),
        iss: cfg.security.token_iss.clone(),
        aud: cfg.security.token_aud.clone(),
        iat: now,
        exp: (Utc::now() + Duration::seconds(cfg.security.token_ttl_seconds as i64)).timestamp()
    };
    let new_access_token = make_token(&cfg, &access_claims)
        .ok_or_else(|| actix_web::error::ErrorInternalServerError("Token generation error"))?;

    // Issue new refresh token (rotate refresh token for security)
    let new_refresh_claims = Claims {
        sub: claims.sub,
        email: claims.email,
        roles: claims.roles,
        iss: cfg.security.token_iss.clone(),
        aud: cfg.security.token_aud.clone(),
        iat: now,
        exp: (Utc::now() + Duration::days(7)).timestamp(),
    };
    let new_refresh_token = make_token(&cfg, &new_refresh_claims)
        .ok_or_else(|| actix_web::error::ErrorInternalServerError("Token generation error"))?;

    // Set new cookies
    let access_ttl = cfg.security.token_ttl_seconds as i64;
    let refresh_ttl = 7 * 24 * 60 * 60; // 7 days in seconds
    let cookie_secure = std::env::var("COOKIE_SECURE").unwrap_or_else(|_| "true".to_string()) == "true";
    let cookie_domain = std::env::var("COOKIE_DOMAIN").ok();

    let response = HttpResponse::NoContent().finish();
    let response = set_auth_cookies(
        response,
        new_access_token,
        new_refresh_token,
        access_ttl,
        refresh_ttl,
        cookie_secure,
        cookie_domain.as_deref(),
    );

    Ok(response)
}

use actix_web::dev::{ServiceRequest, ServiceResponse};
use actix_web::body::BoxBody;
use actix_web::HttpMessage;

pub async fn guard_api(
    req: ServiceRequest,
    next: actix_web::middleware::Next<BoxBody>
    ) -> Result<ServiceResponse<BoxBody>, actix_web::Error> {
    // Get config
    let cfg = req.app_data::<web::Data<AppConfig>>().cloned();
    if let Some(cfg) = cfg {
        if let Some(tok) = extract_token(req.request(), ACCESS_COOKIE_NAME) {
            if let Some(claims) = validate_token(&cfg, &tok) {
                req.extensions_mut().insert(claims);
                return next.call(req).await;
            }
        }
    }
    let (req, _pl) = req.into_parts();
    let resp = HttpResponse::Unauthorized().json("unauthorized");
    Ok(ServiceResponse::new(req, resp.map_into_boxed_body()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, App};
    use tempfile::tempdir;
    use crate::config::{AppConfig, SecurityConfig, DatabaseConfig, LoggingConfig, ServerConfig, TokenMode};

    fn make_test_config(mode: TokenMode) -> AppConfig {
        // Create a minimal test config focused on security settings
        AppConfig {
            security: SecurityConfig {
                access_token: "test_access".into(),
                rate_limit_enabled: false,
                rate_limit_rpm: 100,
                auth_token_expiry_hours: 24,
                token_iss: "test_iss".into(),
                token_aud: "test_aud".into(),
                token_ttl_seconds: 3600,
                paseto_v4_local_key_hex: "142f46b1b4acb0946e0d9413f29b331db345cf664b9307165eab7531fa32d8bd".into(),
                token_mode: mode,
                debug_mode: false,
                health_check_enabled: true,
                metrics_enabled: false,
            },
            server: ServerConfig {
                host: "localhost".into(),
                port: 8081,
                name: "test".into()
            },
            database: DatabaseConfig {
                host: "localhost".into(),
                port: 5432,
                database: "test".into(),
                username: "test".into(),
                password: "test".into(),
                max_connections: 10,
                min_connections: 1,
                connection_timeout: 30,
                idle_timeout: 600,
                max_lifetime: 3600,
                ssl_mode: "prefer".into(),
            },
            sled_path: "test.db".into(),
            backup_dir: "backups".into(),
            backup_name_template: "backup_{{timestamp}}".into(),
            backup_interval: None,
            backup_retention: 10,
            pg_conns: vec![],
            cors_rules: vec![],
            logging: LoggingConfig {
                level: "info".into(),
                file_enabled: false,
                file_path: None,
            },
            database_sync_on: false,
        }
    }

    fn make_test_claims() -> Claims {
        Claims {
            sub: "test_user".into(),
            email: "test@example.com".into(),
            roles: vec!["user".into()],
            iss: "test_iss".into(),
            aud: "test_aud".into(),
            iat: chrono::Utc::now().timestamp(),
            exp: (chrono::Utc::now() + chrono::Duration::hours(1)).timestamp(),
        }
    }

    #[test]
    async fn test_token_roundtrip_hmac() {
        let cfg = make_test_config(TokenMode::JwtHmac);
        let claims = make_test_claims();
        let token = make_token(&cfg, &claims).expect("make token");
        let decoded = validate_token(&cfg, &token).expect("validate token");
        assert_eq!(decoded.sub, claims.sub);
        assert_eq!(decoded.email, claims.email);
        assert_eq!(decoded.roles, claims.roles);
    }

    #[test]
    async fn test_token_roundtrip_paseto() {
        let cfg = make_test_config(TokenMode::PasetoV4Local);
        let claims = make_test_claims();
        let token = make_token(&cfg, &claims).expect("make token");
        let decoded = validate_token(&cfg, &token).expect("validate token");
        assert_eq!(decoded.sub, claims.sub);
        assert_eq!(decoded.email, claims.email);
        assert_eq!(decoded.roles, claims.roles);
    }

    #[test]
    async fn test_static_access_token() {
        let cfg = make_test_config(TokenMode::JwtHmac);
        let decoded = validate_token(&cfg, &cfg.security.access_token).expect("validate static token");
        assert_eq!(decoded.sub, "access");
        assert_eq!(decoded.roles, vec!["admin"]);
    }

    #[test]
    async fn test_hmac_token_rejects_tampered() {
        let cfg = make_test_config(TokenMode::JwtHmac);
        let claims = make_test_claims();
        let mut token = make_token(&cfg, &claims).expect("make token");
        token.push('x'); // tamper
        assert!(validate_token(&cfg, &token).is_none());
    }

    #[actix_web::test]
    async fn e2e_register_login_logout_me() {
        let dir = tempdir().unwrap();
        let mut cfg = make_test_config(TokenMode::JwtHmac);
        cfg.sled_path = dir.path().join("sled").to_string_lossy().to_string();
        let db = Database::new(&cfg.sled_path).unwrap();

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(db.clone()))
                .app_data(web::Data::new(cfg.clone()))
                .service(register)
                .service(login)
                .service(logout)
                .service(me)
        ).await;

        let reg = RegisterRequest { email: "user1@test.dev".into(), password: "secret123".into() };
        let req = test::TestRequest::post().uri("/register").set_json(&reg).to_request();
        let reg_resp: serde_json::Value = test::call_and_read_body_json(&app, req).await;
        let token = reg_resp["token"].as_str().unwrap().to_string();

        let me_req = test::TestRequest::get().uri("/me").insert_header(("authorization", format!("Bearer {}", token))).to_request();
        let me_resp: serde_json::Value = test::call_and_read_body_json(&app, me_req).await;
        assert_eq!(me_resp["email"], "user1@test.dev");

        let login_req = LoginRequest { email: "user1@test.dev".into(), password: "secret123".into() };
        let req = test::TestRequest::post().uri("/login").set_json(&login_req).to_request();
        let login_resp: serde_json::Value = test::call_and_read_body_json(&app, req).await;
        assert!(login_resp.get("token").is_some());

        let req = test::TestRequest::post().uri("/logout").to_request();
        let logout_resp = test::call_and_read_body(&app, req).await;
        assert!(std::str::from_utf8(&logout_resp).unwrap().contains("logged out"));
    }

    #[actix_web::test]
    async fn guard_accepts_bearer_and_query() {
        use actix_web::{test, App, HttpResponse, HttpRequest};
        use actix_web::web;

        async fn protected(req: HttpRequest) -> HttpResponse {
            let claims = req.extensions().get::<Claims>().cloned();
            match claims {
                Some(c) => HttpResponse::Ok().json(serde_json::json!({"sub": c.sub})),
                None => HttpResponse::Unauthorized().finish(),
            }
        }

        let  cfg = make_test_config(TokenMode::JwtHmac);
        let claims = make_test_claims();
        let token = make_token(&cfg, &claims).unwrap();

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(cfg.clone()))
                .wrap(actix_web::middleware::from_fn(guard_api))
                .route("/p", web::get().to(protected))
        ).await;

        let req = test::TestRequest::get()
            .uri("/p")
            .insert_header(("authorization", format!("Bearer {}", token)))
            .to_request();
        let resp: serde_json::Value = test::call_and_read_body_json(&app, req).await;
        assert_eq!(resp["sub"], claims.sub);

        let req = test::TestRequest::get()
            .uri(&format!("/p?access_token={}", token))
            .to_request();
        let resp: serde_json::Value = test::call_and_read_body_json(&app, req).await;
        assert_eq!(resp["sub"], claims.sub);
    }

}

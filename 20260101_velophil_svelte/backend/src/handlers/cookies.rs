// Cookie management helpers
use actix_web::cookie::{Cookie, SameSite};
use actix_web::HttpResponse;
use time::Duration;

pub const ACCESS_COOKIE_NAME: &str = "access_token";
pub const REFRESH_COOKIE_NAME: &str = "refresh_token";

/// Create an HttpOnly, Secure, SameSite=Strict cookie
pub fn create_auth_cookie<'a>(
    name: &'a str,
    value: String,
    max_age_seconds: i64,
    secure: bool,
    domain: Option<&'a str>,
) -> Cookie<'a> {
    let mut cookie = Cookie::build(name, value)
        .path("/")
        .max_age(Duration::seconds(max_age_seconds))
        .http_only(true)
        .same_site(SameSite::Strict)
        .finish();

    if secure {
        cookie.set_secure(true);
    }

    if let Some(d) = domain {
        if !d.is_empty() {
            cookie.set_domain(d.to_string());
        }
    }

    cookie
}

/// Clear an auth cookie (set to expired)
pub fn clear_auth_cookie(name: &'static str) -> Cookie<'static> {
    Cookie::build(name, "")
        .path("/")
        .max_age(Duration::seconds(0))
        .http_only(true)
        .same_site(SameSite::Strict)
        .finish()
}

/// Set access and refresh cookies on a response
pub fn set_auth_cookies(
    mut response: HttpResponse,
    access_token: String,
    refresh_token: String,
    access_ttl: i64,
    refresh_ttl: i64,
    secure: bool,
    domain: Option<&str>,
) -> HttpResponse {
    let access_cookie = create_auth_cookie(
        ACCESS_COOKIE_NAME,
        access_token,
        access_ttl,
        secure,
        domain,
    );
    let refresh_cookie = create_auth_cookie(
        REFRESH_COOKIE_NAME,
        refresh_token,
        refresh_ttl,
        secure,
        domain,
    );

    response.add_cookie(&access_cookie).ok();
    response.add_cookie(&refresh_cookie).ok();
    response
}

/// Clear both access and refresh cookies
pub fn clear_auth_cookies(mut response: HttpResponse) -> HttpResponse {
    response.add_cookie(&clear_auth_cookie(ACCESS_COOKIE_NAME)).ok();
    response.add_cookie(&clear_auth_cookie(REFRESH_COOKIE_NAME)).ok();
    response
}

/// Extract token from cookie or Authorization header (fallback for compatibility)
pub fn extract_token(req: &actix_web::HttpRequest, cookie_name: &str) -> Option<String> {
    // Try cookie first
    if let Some(cookie) = req.cookie(cookie_name) {
        let val = cookie.value().trim();
        if !val.is_empty() {
            return Some(val.to_string());
        }
    }

    // Fallback to Authorization header for backward compatibility
    if cookie_name == ACCESS_COOKIE_NAME {
        if let Some(h) = req.headers().get("authorization") {
            if let Ok(s) = h.to_str() {
                if let Some(rest) = s.strip_prefix("Bearer ").or_else(|| s.strip_prefix("bearer ")) {
                    if !rest.trim().is_empty() {
                        return Some(rest.trim().to_string());
                    }
                }
            }
        }
    }

    None
}

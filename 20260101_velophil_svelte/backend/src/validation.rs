use once_cell::sync::Lazy;
use regex::Regex;

static RE_EMAIL: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?i)^[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}$").unwrap());
static RE_PHONE: Lazy<Regex> = Lazy::new(|| Regex::new(r"^[0-9+()\-.\s]{6,32}$").unwrap());
static RE_ALNUM_TEXT: Lazy<Regex> = Lazy::new(|| Regex::new(r"^[A-Za-z0-9\s\-,.&/()'_]{1,128}$").unwrap());
static RE_ZIP: Lazy<Regex> = Lazy::new(|| Regex::new(r"^[A-Za-z0-9\s\-]{2,16}$").unwrap());
static RE_COUNTRY_ISO2: Lazy<Regex> = Lazy::new(|| Regex::new(r"^[A-Z]{2}$").unwrap());
static RE_CURRENCY_ISO3: Lazy<Regex> = Lazy::new(|| Regex::new(r"^[A-Z]{3}$").unwrap());
static RE_TERMS: Lazy<Regex> = Lazy::new(|| Regex::new(r"^(?i)net_?(7|10|14|15|30|45|60)|prepaid|due_on_receipt$").unwrap());
static RE_TAG: Lazy<Regex> = Lazy::new(|| Regex::new(r"^[a-z0-9_\-]{1,24}$").unwrap());
static RE_REGNO: Lazy<Regex> = Lazy::new(|| Regex::new(r"^[A-Za-z0-9\s\-/.]{3,32}$").unwrap());
static RE_VAT_NO_GENERIC: Lazy<Regex> = Lazy::new(|| Regex::new(r"^[A-Za-z0-9]{6,20}$").unwrap());
static RE_SKU: Lazy<Regex> = Lazy::new(|| Regex::new(r"^[A-Za-z0-9_\-\.]{1,32}$").unwrap());
static RE_URL: Lazy<Regex> = Lazy::new(|| Regex::new(r"^https?://[A-Za-z0-9\-._~:/?#\[\]@!$&'()*+,;=%]{1,2048}$").unwrap());
static RE_LANGUAGE_ISO: Lazy<Regex> = Lazy::new(|| Regex::new(r"^[a-z]{2}(-[A-Z]{2})?$").unwrap());
static RE_SAFE_TEXT: Lazy<Regex> = Lazy::new(|| Regex::new(r"^[A-Za-z0-9\s\-,.&/()'_\n]{1,2000}$").unwrap());

pub fn email(s: &str) -> bool { RE_EMAIL.is_match(s) }
pub fn phone(s: &str) -> bool { RE_PHONE.is_match(s) }
pub fn alnum_text(s: &str) -> bool { RE_ALNUM_TEXT.is_match(s) }
pub fn zip(s: &str) -> bool { RE_ZIP.is_match(s) }
pub fn country_iso2(s: &str) -> bool { RE_COUNTRY_ISO2.is_match(s) }
pub fn currency_iso3(s: &str) -> bool { RE_CURRENCY_ISO3.is_match(s) }
pub fn payment_terms(s: &str) -> bool { RE_TERMS.is_match(s) }
pub fn tag(s: &str) -> bool { RE_TAG.is_match(s) }
pub fn company_reg_number(s: &str) -> bool { RE_REGNO.is_match(s) }
pub fn vat_number(country: &str, vat: &str) -> bool {
    match country {
        "NO" => regex::Regex::new(r"^NO\d{9}MVA$").unwrap().is_match(vat),
        "DE" => regex::Regex::new(r"^DE\d{9}$").unwrap().is_match(vat),
        "NL" => regex::Regex::new(r"^NL[A-Z0-9]{12}$").unwrap().is_match(vat),
        _ => RE_VAT_NO_GENERIC.is_match(vat),
    }
}
pub fn sku(s: &str) -> bool { RE_SKU.is_match(s) }

// New validators for enhanced security
pub fn url(s: &str) -> bool {
    RE_URL.is_match(s) && s.len() <= 2048
}

pub fn language_code(s: &str) -> bool {
    RE_LANGUAGE_ISO.is_match(s)
}

pub fn safe_text(s: &str) -> bool {
    // For notes, industry fields - allows newlines but limits length and charset
    RE_SAFE_TEXT.is_match(s) && s.len() <= 2000
}

pub fn credit_limit(amount: f64) -> bool {
    amount >= 0.0 && amount <= 10_000_000.0 // Max 10 million
}

// Authentication validation
pub fn password_strength(password: &str) -> Result<(), String> {
    if password.len() < 12 {
        return Err("Password must be at least 12 characters".into());
    }
    if password.len() > 128 {
        return Err("Password must be less than 128 characters".into());
    }

    let has_lowercase = password.chars().any(|c| c.is_ascii_lowercase());
    let has_uppercase = password.chars().any(|c| c.is_ascii_uppercase());
    let has_digit = password.chars().any(|c| c.is_ascii_digit());
    let has_special = password.chars().any(|c| "!@#$%^&*()_+-=[]{}|;:,.<>?".contains(c));

    if !has_lowercase {
        return Err("Password must contain at least one lowercase letter".into());
    }
    if !has_uppercase {
        return Err("Password must contain at least one uppercase letter".into());
    }
    if !has_digit {
        return Err("Password must contain at least one number".into());
    }
    if !has_special {
        return Err("Password must contain at least one special character (!@#$%^&*...)".into());
    }

    // Check for common patterns
    let lower = password.to_lowercase();
    if lower.contains("password") || lower.contains("123456") || lower.contains("qwerty") {
        return Err("Password contains common patterns".into());
    }

    Ok(())
}

pub fn validate_email_strict(email_str: &str) -> Result<(), String> {
    if !email(email_str) {
        return Err("Invalid email format".into());
    }
    if email_str.len() > 254 {
        return Err("Email too long".into());
    }
    let parts: Vec<&str> = email_str.split('@').collect();
    if parts.len() != 2 {
        return Err("Invalid email format".into());
    }
    let (local, domain) = (parts[0], parts[1]);
    if local.is_empty() || local.len() > 64 {
        return Err("Invalid email local part".into());
    }
    if domain.is_empty() || !domain.contains('.') {
        return Err("Invalid email domain".into());
    }
    Ok(())
}

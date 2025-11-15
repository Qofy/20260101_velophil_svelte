use anyhow::{anyhow, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::Path,
    time::Duration,
    path::PathBuf
};

#[derive(Debug, Clone)]
pub struct PgConnConfig {
    pub conn_string: String,
    pub targets: Option<HashSet<String>>, // table names
}

pub fn is_origin_allowed(rules: &[CorsRule], origin: &str) -> bool {
    for rule in rules {
        if origin_matches(&rule.origin, origin) {
            return rule.action == CorsAction::Allow;
        }
    }
    false
}

fn origin_matches(pattern: &str, origin: &str) -> bool {
    // Convert wildcard pattern to anchored regex
    let mut re_pat = String::new();
    re_pat.push('^');
    for ch in pattern.chars() {
        match ch {
            '*' => re_pat.push_str(".*"),
            '.' | '+' | '?' | '(' | ')' | '[' | ']' | '{' | '}' | '|' | '^' | '$' | '\\' => {
                re_pat.push('\\');
                re_pat.push(ch);
            }
            _ => re_pat.push(ch),
        }
    }
    re_pat.push('$');
    Regex::new(&re_pat)
        .map(|re| re.is_match(origin))
        .unwrap_or(false)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub host: String,
    pub port: u16,
    pub database: String,
    pub username: String,
    pub password: String,
    pub max_connections: u32,
    pub min_connections: u32,
    pub connection_timeout: u64,
    pub idle_timeout: u64,
    pub max_lifetime: u64,
    pub ssl_mode: String,
}

#[derive(Debug, Clone)]
pub struct CorsRule {
    pub origin: String,
    pub action: CorsAction,
    pub methods: Vec<String>,
    pub headers: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CorsAction {
    Allow,
    Deny,
}

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub sled_path: String,
    pub backup_dir: String,
    pub backup_name_template: String,
    pub backup_interval: Option<Duration>,
    pub backup_retention: usize,
    pub pg_conns: Vec<PgConnConfig>,
    pub cors_rules: Vec<CorsRule>,
    pub logging: LoggingConfig,
    pub security: SecurityConfig,
    pub database_sync_on: bool,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub enum TokenMode {
    JwtHmac,
    PasetoV4Local,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct SecurityConfig {
    pub access_token: String,
    pub rate_limit_enabled: bool,
    pub rate_limit_rpm: u32,
    pub auth_token_expiry_hours: u64,
    pub token_iss: String,
    pub token_aud: String,
    pub token_ttl_seconds: u64,
    pub paseto_v4_local_key_hex: String,
    pub token_mode: TokenMode,
    pub debug_mode: bool,
    pub health_check_enabled: bool,
    pub metrics_enabled: bool,
}

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct LoggingConfig {
    pub level: String,
    pub file_enabled: bool,
    pub file_path: Option<String>,
}

fn parse_duration(s: &str) -> Result<Duration> {
    let s = s.trim().to_lowercase();
    if s.ends_with("ms") {
        let n: u64 = s[..s.len() - 2].parse()?;
        return Ok(Duration::from_millis(n));
    }
    if s.ends_with('s') {
        let n: u64 = s[..s.len() - 1].parse()?;
        return Ok(Duration::from_secs(n));
    }
    if s.ends_with('m') {
        let n: u64 = s[..s.len() - 1].parse()?;
        return Ok(Duration::from_secs(n * 60));
    }
    if s.ends_with('h') {
        let n: u64 = s[..s.len() - 1].parse()?;
        return Ok(Duration::from_secs(n * 3600));
    }
    Err(anyhow!("Invalid duration format: {}", s))
}

pub fn load_config_from_file(config_path: &str) -> AppConfig {
    // Load .env file if it exists
    let abs_config_path = Path::new(config_path)
        .canonicalize()
        .unwrap_or_else(|_| PathBuf::from(config_path));

    tracing::info!("About to load .env file from: {}", abs_config_path.display());

    if Path::new(config_path).exists() {
        match dotenvy::from_filename(config_path) {
            Ok(_) => tracing::info!("✓ Loaded .env file from: {}", abs_config_path.display()),
            Err(e) => tracing::warn!("Failed to load .env file from {}: {}", abs_config_path.display(), e),
        }
    } else {
        tracing::warn!(".env file not found at: {} (using defaults)", abs_config_path.display());
    }

    // Server configuration
    let server = ServerConfig {
        host: std::env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string()),
        port: std::env::var("PORT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(8081),
        name: std::env::var("SERVER_NAME").unwrap_or_else(|_| "description_backend".to_string()),
    };

    // Database configuration
    let database = DatabaseConfig {
        host: std::env::var("DB_HOST").unwrap_or_else(|_| "localhost".to_string()),
        port: std::env::var("DB_PORT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(5432),
        database: std::env::var("DB_NAME").unwrap_or_else(|_| "quoteflow".to_string()),
        username: std::env::var("DB_USER").unwrap_or_else(|_| "postgres".to_string()),
        password: std::env::var("DB_PASSWORD").unwrap_or_else(|_| "".to_string()),
        max_connections: std::env::var("DB_MAX_CONNECTIONS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(10),
        min_connections: std::env::var("DB_MIN_CONNECTIONS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(1),
        connection_timeout: std::env::var("DB_CONNECTION_TIMEOUT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(30),
        idle_timeout: std::env::var("DB_IDLE_TIMEOUT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(600),
        max_lifetime: std::env::var("DB_MAX_LIFETIME")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(3600),
        ssl_mode: std::env::var("DB_SSL_MODE").unwrap_or_else(|_| "prefer".to_string()),
    };

    // Logging configuration
    let logging = LoggingConfig {
        level: std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string()),
        file_enabled: std::env::var("LOG_FILE_ENABLED")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(false),
        file_path: std::env::var("LOG_FILE_PATH").ok(),
    };

    // Sled configuration (new vars with fallback to legacy)
    let db_name = std::env::var("DB_NAME").unwrap_or_else(|_| "quoteflow_data".to_string());
    // Use a directory by default to avoid backup errors
    let db_path = std::env::var("DB_PATH").unwrap_or_else(|_| "data".to_string());
    let sled_path = if db_path.ends_with('/') {
        format!("{}{}", db_path, db_name)
    } else {
        format!("{}/{}", db_path, db_name)
    };

    let backup_dir =
        std::env::var("PERIODIC_BACKUP_PATH").unwrap_or_else(|_| "backups".to_string());
    let backup_name_template = std::env::var("PERIODIC_BACKUP_NAME")
        .unwrap_or_else(|_| "quoteflow_data_backup_{{timestamp}}".to_string());
    let backup_interval = std::env::var("PERIODIC_BACKUP_DB")
        .ok()
        .and_then(|v| parse_duration(&v).ok());
    let backup_retention: usize = 10;

    // Parse legacy PostgreSQL connections
    let mut pg_conns: Vec<PgConnConfig> = Vec::new();
    let re = Regex::new(r"^DATABASE_(\d+)_(CONNECTION_PG_STRING|TARGETS)$").unwrap();
    use std::collections::BTreeMap;
    let mut buckets: BTreeMap<String, (Option<String>, Option<String>)> = BTreeMap::new();

    for (k, v) in std::env::vars() {
        if let Some(caps) = re.captures(&k) {
            let idx = caps.get(1).unwrap().as_str().to_string();
            let kind = caps.get(2).unwrap().as_str();
            let entry = buckets.entry(idx).or_default();
            match kind {
                "CONNECTION_PG_STRING" => entry.0 = Some(v),
                "TARGETS" => entry.1 = Some(v),
                _ => {}
            }
        }
    }

    for (_idx, (conn_opt, targets_opt)) in buckets.into_iter() {
        if let Some(conn_string) = conn_opt {
            let targets = targets_opt.map(|s| {
                s.split_whitespace()
                    .map(|t| t.trim().to_string())
                    .filter(|t| !t.is_empty())
                    .collect::<HashSet<_>>()
            });
            pg_conns.push(PgConnConfig {
                conn_string,
                targets,
            });
        }
    }

    // Load CORS rules from .env_cors file
    let cors_rules = load_cors_rules(".env_cors");

    // Security configuration
    let token_mode_env = std::env::var("TOKEN_JWT_HMAC_OR_PURE_PASSETO_NOTJWT")
        .unwrap_or_else(|_| "jwt_hmac".into());
    let token_mode = if token_mode_env.eq_ignore_ascii_case("pure_passeto_not_jwt") {
        TokenMode::PasetoV4Local
    } else {
        TokenMode::JwtHmac
    };

    let security = SecurityConfig {
        access_token: std::env::var("ACCESS_TOKEN").unwrap_or_default(),
        rate_limit_enabled: std::env::var("API_RATE_LIMIT_ENABLED")
            .unwrap_or_else(|_| "false".into())
            .parse()
            .unwrap_or(false),
        rate_limit_rpm: std::env::var("API_RATE_LIMIT_REQUESTS_PER_MINUTE")
            .unwrap_or_else(|_| "100".into())
            .parse()
            .unwrap_or(100),
        auth_token_expiry_hours: std::env::var("AUTH_TOKEN_EXPIRY_HOURS")
            .unwrap_or_else(|_| "24".into())
            .parse()
            .unwrap_or(24),
        token_iss: std::env::var("TOKEN_ISS").unwrap_or_else(|_| "description_backend".into()),
        token_aud: std::env::var("TOKEN_AUD").unwrap_or_else(|_| "quoteflowfrontend".into()),
        token_ttl_seconds: std::env::var("TOKEN_TTL_SECONDS")
            .unwrap_or_else(|_| "43200".into())
            .parse()
            .unwrap_or(43200),
        paseto_v4_local_key_hex: std::env::var("PASETO_V4_LOCAL_KEY_HEX").unwrap_or_default(),
        token_mode,
        debug_mode: std::env::var("DEBUG_MODE")
            .unwrap_or_else(|_| "false".into())
            .parse()
            .unwrap_or(false),
        health_check_enabled: std::env::var("HEALTH_CHECK_ENABLED")
            .unwrap_or_else(|_| "true".into())
            .parse()
            .unwrap_or(true),
        metrics_enabled: std::env::var("METRICS_ENABLED")
            .unwrap_or_else(|_| "false".into())
            .parse()
            .unwrap_or(false),
    };

    // Database sync on/off
    let database_sync_on = std::env::var("DATABASE_SYNC_ON_OFF")
        .unwrap_or_else(|_| "on".into())
        .eq_ignore_ascii_case("on");

    AppConfig {
        server,
        database,
        sled_path,
        backup_dir,
        backup_name_template,
        backup_interval,
        backup_retention,
        pg_conns,
        cors_rules,
        logging,
        security,
        database_sync_on,
    }
}

pub fn build_table_routes(
    pg_conns: &[PgConnConfig],
    tables: &[&str],
) -> HashMap<String, Vec<usize>> {
    // Routing logic per spec
    let mut routes: HashMap<String, Vec<usize>> = HashMap::new();
    if pg_conns.is_empty() {
        return routes; // no replication
    }

    if pg_conns.len() == 1 {
        for t in tables {
            routes.insert((*t).to_string(), vec![0]);
        }
        return routes;
    }

    // If multiple connections
    // Determine which have explicit targets
    let mut any_targets = false;
    for c in pg_conns.iter() {
        if c.targets.is_some() {
            any_targets = true;
            break;
        }
    }

    if !any_targets {
        // replicate all tables to all connections
        for (i, _) in pg_conns.iter().enumerate() {
            for &t in tables {
                routes.entry(t.to_string()).or_default().push(i);
            }
        }
        return routes;
    }

    // If only one connection has targets: that connection gets only those tables;
    // the first connection (index 0) gets all other tables.
    let targeted_indices: Vec<usize> = pg_conns
        .iter()
        .enumerate()
        .filter_map(|(i, c)| c.targets.as_ref().map(|_| i))
        .collect();
    if targeted_indices.len() == 1 {
        let tgt_i = targeted_indices[0];
        let targets = pg_conns[tgt_i].targets.as_ref().unwrap();
        for &t in tables {
            if targets.contains(t) {
                routes.entry(t.to_string()).or_default().push(tgt_i);
            } else {
                routes.entry(t.to_string()).or_default().push(0);
            }
        }
        return routes;
    }

    // Two or more connections with targets: route each table to any connection that specifies it
    for (i, c) in pg_conns.iter().enumerate() {
        if let Some(set) = &c.targets {
            for name in set.iter() {
                routes.entry(name.clone()).or_default().push(i);
            }
        }
    }

    // For tables not explicitly targeted by any, send to first connection by default
    for &t in tables {
        routes.entry(t.to_string()).or_default();
    }
    for (_t, v) in routes.iter_mut() {
        if v.is_empty() {
            v.push(0);
        }
        // ensure unique
        v.sort_unstable();
        v.dedup();
    }

    routes
}

pub fn load_cors_rules(path: &str) -> Vec<CorsRule> {
    let mut rules = Vec::new();

    let abs_cors_path = Path::new(path)
        .canonicalize()
        .unwrap_or_else(|_| PathBuf::from(path));

    tracing::info!("About to load .env_cors file from: {}", abs_cors_path.display());

    if !Path::new(path).exists() {
        tracing::warn!(".env_cors file not found at: {} (CORS will be disabled)", abs_cors_path.display());
        return rules;
    }

    let content = match fs::read_to_string(path) {
        Ok(s) => {
            tracing::info!("✓ Loaded .env_cors file from: {}", abs_cors_path.display());
            s
        },
        Err(e) => {
            tracing::error!("Failed to read .env_cors file from {}: {}", abs_cors_path.display(), e);
            return rules;
        }
    };

    for raw in content.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        // Tokens: origin [ACTION] [METHODS] [HEADERS]
        // Examples:
        // http://localhost:3000
        // http://site ALLOW GET,POST
        // https://x ALLOW ALL content-type,authorization
        let mut parts = line.split_whitespace();
        let origin = match parts.next() {
            Some(o) => o.to_string(),
            None => continue,
        };
        let action_str = parts.next().unwrap_or("ALLOW");
        let action = match action_str.to_uppercase().as_str() {
            "DENY" => CorsAction::Deny,
            _ => CorsAction::Allow,
        };
        let methods = parts.next().unwrap_or("ALL").to_string();
        let headers = parts.next().unwrap_or("ALL").to_string();

        let methods_vec: Vec<String> = if methods.eq_ignore_ascii_case("ALL") {
            vec!["ALL".to_string()]
        } else {
            methods
                .split(',')
                .map(|s| s.trim().to_uppercase())
                .filter(|s| !s.is_empty())
                .collect()
        };
        let headers_vec: Vec<String> = if headers.eq_ignore_ascii_case("ALL") {
            vec!["ALL".to_string()]
        } else {
            headers
                .split(',')
                .map(|s| s.trim().to_lowercase())
                .filter(|s| !s.is_empty())
                .collect()
        };

        rules.push(CorsRule {
            origin,
            action,
            methods: methods_vec,
            headers: headers_vec,
        });
    }
    rules
}

pub fn database_url_from_env_or_config(explicit: Option<&str>, cfg: &AppConfig) -> String {
    if let Some(url) = explicit {
        if !url.is_empty() {
            return url.to_string();
        }
    }
    if let Ok(url) = std::env::var("DATABASE_URL") {
        if !url.is_empty() {
            return url;
        }
    }
    // Build from DatabaseConfig
    let sslmode = cfg.database.ssl_mode.clone();
    format!(
        "postgres://{user}:{pass}@{host}:{port}/{db}?sslmode={sslmode}",
        user = urlencoding::encode(&cfg.database.username),
        pass = urlencoding::encode(&cfg.database.password),
        host = cfg.database.host,
        port = cfg.database.port,
        db = cfg.database.database,
        sslmode = sslmode
    )
}

use actix_cors::Cors;
use actix_web::{
    middleware::Logger,
    App,
    HttpServer,
    HttpResponse,
    Responder,
    web::{
        self,
        JsonConfig,
    },
    error::{
        JsonPayloadError,
    },
};

mod backup;
mod cli;
mod config;
// mod crawler;  // Not implemented yet
mod db;
mod db_manager;
mod handlers;
mod logging;
mod middleware;
// mod model_tests;  // Not implemented yet
mod models;
// mod process_manager;  // Not implemented yet
mod replicate;
mod routes;
mod time;
mod types;
mod validation;

use crate::backup::*;
use crate::cli::*;
use crate::config::*;
use crate::db::*;
use crate::db_manager::*;
use crate::handlers::*;
use crate::logging::*;
use crate::model_tests::*;
use crate::models::*;
use crate::replicate::*;
use crate::time::*;
use crate::validation::*;
use crate::handlers::{customers::*, errors::ApiError, validation::customer_validation_schema};

use argon2::password_hash::SaltString;
use argon2::{Argon2, PasswordHasher};
use backup::BackupManager;
use cli::{AddAdminArgs, Cli, Commands, DbCommands, UserCommands};
use config::{build_table_routes, database_url_from_env_or_config, load_config_from_file};
use db::Database;
use models::{
    Address, CreateCertificateRequest, CreateInvoiceRequest, CreateQuoteRequest, Customer,
    IncomingQuoteItem, InternCertificate, Invoice, Quote,
};
use rand_core::OsRng;
use replicate::Replicator;
use std::sync::Arc;
use tracing::info;

#[cfg(test)]
mod tests;

async fn index() -> impl Responder {
    HttpResponse::Ok().body("Welcome to QuoteFlow API")
}

fn json_error_handler(err: JsonPayloadError, _req: &actix_web::HttpRequest) -> actix_web::Error {
    let body = HttpResponse::BadRequest().json(ApiError::deserialization(err.to_string()));
    actix_web::error::InternalError::from_response(err, body).into()
}

fn seed(db: &Database) {
    // Note: Seeding is disabled due to user scoping requirements
    // Customers, quotes, and invoices now require user_id from authenticated requests
    let _customers: Vec<Customer> = db.list("customers").unwrap_or_default();
    // Seed function temporarily disabled
    // Data should be created through API endpoints after user authentication
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let cli = Cli::parse_args();
    logging::init_logging(cli.verbose).expect("failed to init logging");
    logging::print_build_info();

    let cfg = load_config_from_file(&cli.config);

    if let Some(Commands::User {
        action: UserCommands::AddAdmin(AddAdminArgs { email, password }),
    }) = cli.command.clone()
    {
        let db = Database::new(&cfg.sled_path).expect("db");
        let email_lc = email.trim().to_lowercase();
        let users: Vec<models::UserRecord> = db.list("users").unwrap_or_default();
        if users.iter().any(|u| u.email == email_lc) {
            println!("user exists: {}", email_lc);
            return Ok(());
        }
        let salt = SaltString::generate(&mut OsRng);
        let hash = Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .unwrap()
            .to_string();
        let admin = models::UserRecord::new_admin(&email_lc, hash);
        let _ = db.insert("users", &admin.id, &admin);
        println!("admin added: {}", email_lc);
        return Ok(());
    }

    if let (Some(email), Some(password)) =
        (cli.add_admin_email.clone(), cli.add_admin_password.clone())
    {
        let db = Database::new(&cfg.sled_path).expect("db");
        let email_lc = email.trim().to_lowercase();
        let users: Vec<models::UserRecord> = db.list("users").unwrap_or_default();
        if users.iter().any(|u| u.email == email_lc) {
            println!("user exists: {}", email_lc);
            return Ok(());
        }
        let salt = SaltString::generate(&mut OsRng);
        let hash = Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .unwrap()
            .to_string();
        let admin = models::UserRecord::new_admin(&email_lc, hash);
        let _ = db.insert("users", &admin.id, &admin);
        println!("admin added: {}", email_lc);
        return Ok(());
    }

    info!(
        "QuoteFlow Backend v{} (build {})",
        std::env::var("APP_BUILD_VERSION")
            .unwrap_or_else(|_| env!("CARGO_PKG_VERSION").to_string()),
        std::env::var("VERGEN_BUILD_TIMESTAMP").unwrap_or_else(|_| "now".into())
    );

    let abs_sled_path = std::path::Path::new(&cfg.sled_path)
        .canonicalize()
        .unwrap_or_else(|_| std::path::PathBuf::from(&cfg.sled_path));
    info!("Sled path (relative): {}", cfg.sled_path);
    info!("Sled path (absolute): {}", abs_sled_path.display());
    info!(
        "Server: {}:{} ({})",
        cfg.server.host, cfg.server.port, cfg.server.name
    );
    info!(
        "Logging: level={}, file_enabled={}, file_path={:?}",
        cfg.logging.level, cfg.logging.file_enabled, cfg.logging.file_path
    );

    // Log CORS configuration
    if cfg.cors_rules.is_empty() {
        info!("CORS: No rules configured (all origins blocked)");
    } else {
        info!("CORS: {} rule(s) configured", cfg.cors_rules.len());
        for (idx, rule) in cfg.cors_rules.iter().enumerate() {
            let action_str = match rule.action {
                config::CorsAction::Allow => "ALLOW",
                config::CorsAction::Deny => "DENY",
            };
            let origin_str = if rule.origin.is_empty() {
                "any".to_string()
            } else {
                rule.origin.clone()
            };
            let methods_str = if rule.methods.is_empty() {
                "none".to_string()
            } else {
                rule.methods.join(", ")
            };
            let headers_str = if rule.headers.is_empty() {
                "none".to_string()
            } else {
                rule.headers.join(", ")
            };
            info!(
                "  [{}] {} origin=[{}] methods=[{}] headers=[{}]",
                idx + 1,
                action_str,
                origin_str,
                methods_str,
                headers_str
            );
        }
    }

    // If a DB command was provided, handle it and exit
    if let Some(Commands::Db { action }) = cli.effective_command() {
        let url = database_url_from_env_or_config(cli.database_url.as_deref(), &cfg);
        let mgr = db_manager::DbManager::new(url.clone());
        use std::time::Instant;
        let start = Instant::now();
        match &action {
            DbCommands::Test => {
                logging::log_command_start("db test", "Test database connection");
                match mgr.test_connection().await {
                    Ok(_) => logging::log_command_complete("db test", true, start.elapsed()),
                    Err(e) => {
                        logging::log_error(&format!("DB connection failed: {}", e));
                        logging::log_command_complete("db test", false, start.elapsed());
                        std::process::exit(2);
                    }
                }
            }
            DbCommands::Seed { .. } => {
                logging::log_command_start("db seed", "Create schema and seed sample data");
                let mut ok = true;
                if let Err(e) = mgr.create_schema(true).await {
                    logging::log_error(&format!("schema error: {}", e));
                    ok = false;
                }
                if ok {
                    if let Err(e) = mgr.seed_sample_data().await {
                        logging::log_error(&format!("seed error: {}", e));
                        ok = false;
                    }
                }
                if ok {
                    logging::log_command_complete("db seed", true, start.elapsed());
                } else {
                    logging::log_command_complete("db seed", false, start.elapsed());
                    std::process::exit(2);
                }
            }
            DbCommands::Dump {
                output,
                data,
                schema,
                tables,
            } => {
                logging::log_command_start("db dump", "Export database to SQL file");
                if let Err(e) = mgr
                    .dump(output.clone(), *schema, *data, tables.clone())
                    .await
                {
                    logging::log_error(&format!("dump error: {}", e));
                    logging::log_command_complete("db dump", false, start.elapsed());
                    std::process::exit(2);
                }
                logging::log_command_complete("db dump", true, start.elapsed());
            }
            DbCommands::Import {
                input,
                drop_existing,
            } => {
                logging::log_command_start("db import", "Import SQL file into database");
                if let Err(e) = mgr.import(input.clone(), *drop_existing).await {
                    logging::log_error(&format!("import error: {}", e));
                    logging::log_command_complete("db import", false, start.elapsed());
                    std::process::exit(2);
                }
                logging::log_command_complete("db import", true, start.elapsed());
            }
            DbCommands::GenerateInitialSql {
                output,
                include_sample_data,
            } => {
                logging::log_command_start(
                    "db generate-initial-sql",
                    "Generate initial schema SQL",
                );
                match mgr.generate_initial_sql(*include_sample_data).await {
                    Ok(sql) => {
                        if let Err(e) = std::fs::write(output, &sql) {
                            logging::log_error(&format!("write error: {}", e));
                        }
                        logging::log_command_complete(
                            "db generate-initial-sql",
                            true,
                            start.elapsed(),
                        );
                    }
                    Err(e) => {
                        logging::log_error(&format!("generate sql error: {}", e));
                        logging::log_command_complete(
                            "db generate-initial-sql",
                            false,
                            start.elapsed(),
                        );
                        std::process::exit(2);
                    }
                }
            }
            DbCommands::Migrate { .. } => {
                logging::log_warning("Migrations not yet implemented; skipping");
            }
            DbCommands::Reset { confirm } => {
                logging::log_command_start("db reset", "Drop known tables");
                if !confirm {
                    logging::log_warning("Use --confirm to reset the database");
                    logging::log_command_complete("db reset", false, start.elapsed());
                    std::process::exit(2);
                }
                match mgr.reset().await {
                    Ok(_) => logging::log_command_complete("db reset", true, start.elapsed()),
                    Err(e) => {
                        logging::log_error(&format!("reset error: {}", e));
                        logging::log_command_complete("db reset", false, start.elapsed());
                        std::process::exit(2);
                    }
                }
            }
        }
        return Ok(());
    }

    // Build replicator (if any pg connections)
    let tables = ["customers", "quotes", "invoices", "certificates"];
    // Ensure sled directory exists to avoid backup errors
    let _ = std::fs::create_dir_all(&cfg.sled_path);

    // Process management: Check for conflicts before starting
    let pid_file = format!("{}/quoteflow.pid", cfg.sled_path);
    let process_mgr = process_manager::ProcessManager::new(&pid_file);
    let current_process = process_manager::ProcessInfo::current(cfg.server.port, &cfg.sled_path);

    match process_mgr.check_and_claim(&current_process) {
        Ok(true) => {
            // We can proceed, write our PID file
            if let Err(e) = process_mgr.write(&current_process) {
                tracing::warn!("Failed to write PID file: {}", e);
            }
        }
        Ok(false) => {
            // Same version already running, exit gracefully
            info!("Same version already running, exiting");
            return Ok(());
        }
        Err((exit_code, message)) => {
            // Conflict detected
            match exit_code {
                1 => {
                    logging::log_error(&format!("Newer version is already running"));
                    eprintln!("{}", message);
                    std::process::exit(1);
                }
                2 => {
                    logging::log_error(&format!("Another service is using the port"));
                    eprintln!("{}", message);
                    std::process::exit(2);
                }
                3 => {
                    logging::log_error(&format!("Another service has locked the database file"));
                    eprintln!("{}", message);
                    std::process::exit(3);
                }
                _ => {
                    logging::log_error(&format!("Process conflict: {}", message));
                    eprintln!("{}", message);
                    std::process::exit(exit_code);
                }
            }
        }
    }

    let mut db = Database::new(&cfg.sled_path).expect("Failed to initialize database");

    // Initialize product cache state
    let product_state = web::Data::new(handlers::products::AppState::new());


    let replicator = if cfg.database_sync_on && !cfg.pg_conns.is_empty() {
        let conn_strings: Vec<String> =
            cfg.pg_conns.iter().map(|c| c.conn_string.clone()).collect();
        let routes = build_table_routes(&cfg.pg_conns, &tables);
        info!("Replication routes: {:?}", routes);
        let rep = Replicator::new(&conn_strings, routes).await.ok();
        rep.map(Arc::new)
    } else {
        None
    };

    if let Some(rep) = replicator.clone() {
        db = db.with_replicator(Some(rep));
    }

    // One-time seed (idempotent): only if collections are empty
    // Try to restore from latest backup if database is empty
    let mgr = Arc::new(BackupManager::new(
        &cfg.sled_path,
        &cfg.backup_dir,
        &cfg.backup_name_template,
    ));

    if let Ok(restored) = mgr.restore_from_latest().await {
        if restored {
            info!("Database restored from latest backup");
        }
    }

    if cli.should_seed_on_startup() {
        seed(&db);
    }

    // Start backup task if configured
    if let Some(itv) = cfg.backup_interval {
        info!(
            "Backup enabled every {:?}, keeping {} backups in {}",
            itv, cfg.backup_retention, cfg.backup_dir
        );
        let mgr_clone = mgr.clone();
        tokio::spawn(async move {
            mgr_clone.run(itv, cfg.backup_retention).await;
        });
    } else {
        info!("Backup disabled");
    }

    let host = cfg.server.host.clone();
    let port = cfg.server.port;

    logging::log_server_startup(&cfg.server.host, cfg.server.port);


    // Start HTTP server
    HttpServer::new(move || {
        let cors = {
            use actix_web::http::{header::HeaderName, Method};
            use std::collections::HashSet;
            let rules = cfg.cors_rules.clone();

            // Build union of methods/headers across ALLOW rules
            let mut methods_all = false;
            let mut methods_set: HashSet<Method> = HashSet::new();
            let mut headers_all = false;
            let mut headers_set: HashSet<HeaderName> = HashSet::new();
            for r in &rules {
                if r.action != config::CorsAction::Allow {
                    continue;
                }
                if r.methods.iter().any(|m| m.eq_ignore_ascii_case("ALL")) {
                    methods_all = true;
                } else {
                    for m in &r.methods {
                        if let Ok(mm) = Method::from_bytes(m.as_bytes()) {
                            methods_set.insert(mm);
                        }
                    }
                }
                if r.headers.iter().any(|h| h.eq_ignore_ascii_case("ALL")) {
                    headers_all = true;
                } else {
                    for h in &r.headers {
                        if let Ok(hn) = HeaderName::from_lowercase(h.as_bytes()) {
                            headers_set.insert(hn);
                        }
                    }
                }
            }

            let mut builder = Cors::default().allowed_origin_fn(move |origin, _req| {
                let o = origin.to_str().unwrap_or("");
                config::is_origin_allowed(&rules, o)
            });

            if methods_all {
                builder = builder.allow_any_method();
            } else if !methods_set.is_empty() {
                builder = builder.allowed_methods(methods_set.clone());
            } else {
                builder = builder.allow_any_method();
            }

            if headers_all {
                builder = builder.allow_any_header();
            } else if !headers_set.is_empty() {
                for h in headers_set {
                    builder = builder.allowed_header(h);
                }
            } else {
                builder = builder.allow_any_header();
            }

            builder
        };

        App::new()
            .app_data(JsonConfig::default().limit(32 * 1024).error_handler(json_error_handler))
            .app_data(web::Data::new(db.clone()))
            .app_data(web::Data::new(cfg.clone()))
            .app_data(product_state.clone())
            .wrap(middleware::security::SecurityHeaders) // Add security headers
            .wrap(cors)
            .wrap(Logger::default())
            .route("/", web::get().to(index))
            .service(routes::health::healthz) // Add /healthz endpoint
            .service(routes::health::health)  // Add /health endpoint
            .service(routes::static_files::serve_wasm) // Add WASM serving
            .service(
               web::scope("/api")
                   // Public API routes (no auth required)
                  .service(handlers::version::get_version)
                   // Auth routes under /api/auth
                   .service(
                     web::scope("/auth")
                      .service(handlers::auth::register)
                      .service(handlers::auth::login)
                      .service(handlers::auth::logout)
                      .service(handlers::auth::refresh) // Add refresh endpoint
                      .service(handlers::auth::reconfirm)
                      .service(handlers::auth::me),
                   )
                   // Protected API routes (with auth guard)
                   .service(
                      web::scope("")
                         .wrap(actix_web::middleware::from_fn(handlers::auth::guard_api))
                            // Customer routes
                            .service(handlers::customers::create_customer)
                            .service(handlers::customers::get_customers)
                            .service(handlers::customers::get_customer)
                            .service(handlers::customers::update_customer)
                            .service(handlers::customers::delete_customer)
                            .service(customer_validation_schema)
                            // Quote routes
                            .service(handlers::quotes::get_quotes)
                            .service(handlers::quotes::create_quote)
                            .service(handlers::quotes::get_quote)
                            .service(handlers::quotes::update_quote)
                            .service(handlers::quotes::delete_quote)
                            .service(handlers::quotes::generate_approval_token)
                            .service(handlers::quotes::convert_quote_to_invoice)
                            // Invoice routes
                            .service(handlers::invoices::get_invoices)
                            .service(handlers::invoices::create_invoice)
                            .service(handlers::invoices::get_invoice)
                            .service(handlers::invoices::update_invoice)
                            .service(handlers::invoices::delete_invoice)
                            // Certificate routes
                            .service(handlers::certificates::get_certificates)
                            .service(handlers::certificates::create_certificate)
                            .service(handlers::certificates::get_certificate)
                            .service(handlers::certificates::update_certificate)
                            .service(handlers::certificates::delete_certificate)
                            .configure(handlers::products::configure)
                            .configure(handlers::crawler::configure)

                   )
            )
            .service(
                web::scope("/public")
                    // Public quote routes (no auth required)
                    .service(handlers::quotes::get_quote_by_token)
                    .service(handlers::quotes::approve_quote_public)
                    .service(handlers::quotes::reject_quote_public)
            )
            .service(
                web::scope("/auth")
                    .service(handlers::auth::register)
                    .service(handlers::auth::login)
                    .service(handlers::auth::logout)
                    .service(handlers::auth::reconfirm)
                    .service(handlers::auth::me),
            )
    })
    .bind((host.as_str(), port))?
    .run()
    .await
}




#[derive(Parser)]
#[command(name = "periodicbackend")]
#[command(about = "Periodic Table Backend with PostgreSQL export/import")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Export sled database to PostgreSQL SQL dump
    ExportSql {
        /// Output SQL file path
        #[arg(short, long, default_value = "periodic_export.sql")]
        output: String,
        /// Include DROP TABLE statements
        #[arg(long, default_value = "true")]
        include_drops: bool,
    },
    /// Import PostgreSQL SQL dump into sled database
    ImportSql {
        /// Input SQL file path
        #[arg(short, long)]
        input: String,
        /// Clear existing data before import
        #[arg(long, default_value = "false")]
        clear_existing: bool,
    },
    /// Start the web server (default)
    Serve,
}

// Metadata and versioning structures
#[derive(Debug, Serialize, Deserialize, Clone)]
struct ItemMetadata {
    name: String,
    #[serde(rename = "updatedAt")]
    updated_at: u64,
    version: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    checksum: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct ListResponse<T> {
    items: Vec<T>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct ItemResponse<T> {
    name: String,
    data: T,
    #[serde(rename = "updatedAt")]
    updated_at: u64,
    version: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct UpsertRequest<T> {
    data: T,
    #[serde(rename = "baseVersion")]
    base_version: Option<u64>,
    #[serde(rename = "updatedAtClient")]
    updated_at_client: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct UpsertResponse {
    version: u64,
    #[serde(rename = "updatedAt")]
    updated_at: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct ConflictResponse {
    #[serde(rename = "serverVersion")]
    server_version: u64,
    #[serde(rename = "serverUpdatedAt")]
    server_updated_at: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct VersionedData<T> {
    data: T,
    version: u64,
    updated_at: u64,
    created_at: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Position {
    x: f64,
    y: f64,
    z: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct CustomLayout {
    positions: HashMap<String, Position>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct NameEntry {
    name: String,
    mass: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct CustomNamesConfig {
    names: HashMap<String, NameEntry>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct CustomZoneConfig {
    zone_names: Vec<String>,
    assignments: HashMap<String, String>,
    colors: HashMap<String, String>,
    enabled: HashMap<String, bool>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct CustomOrderConfig {
    data: Value,
}

// Server configurations: simply a list of server names
#[derive(Debug, Serialize, Deserialize, Clone)]
struct CustomServerConfig {
    servers: Vec<String>,
}

// Client configurations: list of client names
#[derive(Debug, Serialize, Deserialize, Clone)]
struct CustomClientConfig {
    clients: Vec<String>,
}

// Reservation configurations: stored as arbitrary JSON
#[derive(Debug, Serialize, Deserialize, Clone)]
struct CustomReservConfig {
    data: Value,
}

// Custom pins configurations: stored as arbitrary JSON
#[derive(Debug, Serialize, Deserialize, Clone)]
struct CustomPinsConfig {
    data: Value,
}

// Runtime orders: temporary runtime data
#[derive(Debug, Serialize, Deserialize, Clone)]
struct RuntimeOrder {
    name: String,
    data: Value,
}

// Kitchen printer configurations: stored as arbitrary JSON
#[derive(Debug, Serialize, Deserialize, Clone)]
struct CustomKitchenConfig {
    data: Value,
}

// Runtime reservations: temporary runtime data
#[derive(Debug, Serialize, Deserialize, Clone)]
struct RuntimeReservation {
    name: String,
    data: Value,
}

// Runtime events for multiplexed streaming
#[derive(Debug, Serialize, Deserialize, Clone)]
struct RuntimeEvent {
    id: String,
    ts: u64,
    #[serde(rename = "type")]
    event_type: String,
    payload: Value,
    cursor: String,
}

// Environment configuration structures
#[derive(Debug, Clone)]
struct Config {
    // Server configuration
    pub port: u16,
    pub host: String,
    pub server_name: String,

    // Database configuration
    pub db_path: String,
    pub backup_path: String,
    pub backup_interval_seconds: u64,

    // Auth configuration
    pub paseto_local_key_b64: Option<String>,
    pub token_issuer: String,
    pub token_audience: String,
    pub token_ttl_seconds: u64,

    // IoT device connections
    pub kitchen_iot_enabled: bool,
    pub kitchen_iot_url: String,
    pub kitchen_iot_timeout_ms: u64,
    pub kitchen_iot_api_key: String,

    pub cashier_enabled: bool,
    pub cashier_url: String,
    pub cashier_timeout_ms: u64,
    pub cashier_api_key: String,
    pub cashier_device_id: String,

    pub display_enabled: bool,
    pub display_url: String,
    pub display_timeout_ms: u64,
    pub display_api_key: String,
    pub display_device_id: String,

    pub pos_system_enabled: bool,
    pub pos_system_url: String,
    pub pos_system_api_key: String,

    pub inventory_system_enabled: bool,
    pub inventory_system_url: String,
    pub inventory_system_api_key: String,

    // Security and debugging
    #[allow(dead_code)]
    pub debug_mode: bool,
    #[allow(dead_code)]
    pub health_check_enabled: bool,
    #[allow(dead_code)]
    pub api_rate_limit_enabled: bool,
    #[allow(dead_code)]
    pub api_rate_limit_requests_per_minute: u32,

    // Access token authentication
    pub access_token: String,
}

#[derive(Debug, Clone, PartialEq)]
enum CorsAction {
    Allow,
    Deny,
}

#[derive(Debug, Clone)]
struct CorsRule {
    pub origin_pattern: String,
    pub action: CorsAction,
    pub methods: Vec<String>,
    pub headers: Vec<String>,
}

#[derive(Debug, Clone)]
struct CorsConfig {
    pub rules: Vec<CorsRule>,
}

/// Parse time string like "5s", "10m", "1h" into seconds
///
/// # Examples
/// - "5s" or "5" → 5 seconds
/// - "30s" → 30 seconds
/// - "1m" → 60 seconds
/// - "5m" → 300 seconds
/// - "1h" → 3600 seconds
/// - "2h" → 7200 seconds
///
/// # Valid Units
/// - s, sec, second, seconds
/// - m, min, minute, minutes
/// - h, hr, hour, hours
/// - d, day, days
///
/// # Valid Range
/// - Minimum: 1 second
/// - Maximum: 24 hours (86400 seconds)
fn parse_time_to_seconds(time_str: &str) -> Result<u64, String> {
    let time_str = time_str.trim();
    if time_str.is_empty() {
        return Err("Empty time string".to_string());
    }

    let (number_part, unit_part) = if let Some(pos) = time_str.find(|c: char| c.is_alphabetic()) {
        (&time_str[..pos], &time_str[pos..])
    } else {
        // If no unit, assume seconds
        (time_str, "s")
    };

    let number: u64 = number_part.parse()
        .map_err(|_| format!("Invalid number: {}", number_part))?;

    let multiplier = match unit_part.to_lowercase().as_str() {
        "s" | "sec" | "second" | "seconds" => 1,
        "m" | "min" | "minute" | "minutes" => 60,
        "h" | "hr" | "hour" | "hours" => 3600,
        "d" | "day" | "days" => 86400,
        _ => return Err(format!("Unknown time unit: {}", unit_part)),
    };

    let total_seconds = number.checked_mul(multiplier)
        .ok_or_else(|| "Time value too large".to_string())?;

    // Validate reasonable range (1 second to 24 hours)
    if total_seconds < 1 {
        return Err("Time must be at least 1 second".to_string());
    }
    if total_seconds > 86400 {
        return Err("Time must be no more than 24 hours (86400 seconds)".to_string());
    }

    Ok(total_seconds)
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, App};
    use actix_web::http::header;

    #[actix_web::test]
    async fn test_parse_time_to_seconds() {
        // Test seconds
        assert_eq!(parse_time_to_seconds("5s"), Ok(5));
        assert_eq!(parse_time_to_seconds("30s"), Ok(30));
        assert_eq!(parse_time_to_seconds("5"), Ok(5)); // Default to seconds

        // Test minutes
        assert_eq!(parse_time_to_seconds("1m"), Ok(60));
        assert_eq!(parse_time_to_seconds("5m"), Ok(300));
        assert_eq!(parse_time_to_seconds("10min"), Ok(600));

        // Test hours
        assert_eq!(parse_time_to_seconds("1h"), Ok(3600));
        assert_eq!(parse_time_to_seconds("2h"), Ok(7200));
        assert_eq!(parse_time_to_seconds("1hr"), Ok(3600));

        // Test days
        assert_eq!(parse_time_to_seconds("1d"), Ok(86400));

        // Test case insensitive
        assert_eq!(parse_time_to_seconds("5S"), Ok(5));
        assert_eq!(parse_time_to_seconds("1M"), Ok(60));
        assert_eq!(parse_time_to_seconds("1H"), Ok(3600));

        // Test whitespace handling
        assert_eq!(parse_time_to_seconds(" 5s "), Ok(5));
        assert_eq!(parse_time_to_seconds("\t1m\n"), Ok(60));

        // Test edge cases
        assert_eq!(parse_time_to_seconds("1s"), Ok(1));   // Minimum
        assert_eq!(parse_time_to_seconds("24h"), Ok(86400)); // Maximum

        // Test error cases
        assert!(parse_time_to_seconds("").is_err());        // Empty
        assert!(parse_time_to_seconds("abc").is_err());     // Invalid number
        assert!(parse_time_to_seconds("5x").is_err());      // Invalid unit
        assert!(parse_time_to_seconds("0s").is_err());      // Below minimum
        assert!(parse_time_to_seconds("25h").is_err());     // Above maximum
    }

    fn build_test_app() -> (actix_web::App<impl actix_web::dev::ServiceFactory<actix_web::dev::ServiceRequest, Config=(), Response=actix_web::dev::ServiceResponse, Error=actix_web::Error, InitError=()>>, web::Data<AppState>) {
        // temporary sled db
        let db = sled::Config::new().temporary(true).open().unwrap();
        // config with stable access token and paseto key
        let mut cfg = Config::from_env();
        cfg.access_token = "test-token".to_string();
        let mut key = [0u8; 32];
        OsRng.try_fill_bytes(&mut key).ok();
        cfg.paseto_local_key_b64 = Some(base64::engine::general_purpose::STANDARD.encode(key));
        let app_state = web::Data::new(AppState::new(db, "test_backup.json".to_string(), &cfg));

        let app = App::new()
            .app_data(app_state.clone())
            .wrap_fn(|req, srv| {
                let path = req.path().to_string();
                let is_api = path.starts_with("/api/");
                let is_public = matches!(path.as_str(), "/api/health" | "/health" | "/api/auth/login" | "/api/auth/register");
                let app_state = req.app_data::<web::Data<AppState>>().cloned();
                if is_api && !is_public {
                    if let Some(app_state) = app_state {
                        if let Err(resp) = validate_access_token(req.request(), &app_state) {
                            let response = req.into_response(resp);
                            return futures_util::future::Either::Left(async { Ok(response) });
                        }
                    }
                }
                let fut = srv.call(req);
futures_util::future::Either::Right(fut)
            })
            // minimal routes to exercise auth + flows
            .route("/api/health", web::get().to(health_check))
            .route("/api/auth/register", web::post().to(auth_register))
            .route("/api/auth/login", web::post().to(auth_login))
            .route("/api/runtime_counts", web::get().to(get_runtime_counts));

        (app, app_state)
    }

    #[actix_web::test]
    async fn e2e_register_login_and_bearer_auth() {
        let (app, _state) = build_test_app();
let app = test::init_service(app).await;

        // health is public
        let health_req = test::TestRequest::get().uri("/api/health").to_request();
let health_resp = test::call_service(&app, health_req).await;
        assert!(health_resp.status().is_success());

        // register
        let reg = RegisterRequest { name: "Test User".into(), email: "user@example.com".into(), password: "secret".into(), note: None };
        let reg_req = test::TestRequest::post().uri("/api/auth/register").set_json(&reg).to_request();
let reg_resp = test::call_service(&app, reg_req).await;
        assert_eq!(reg_resp.status(), actix_web::http::StatusCode::CREATED);

        // login
        let login = LoginRequest { username: "user@example.com".into(), email: "user@example.com".into(), password: "secret".into() };
        let login_req = test::TestRequest::post().uri("/api/auth/login").set_json(&login).to_request();
let login_resp = test::call_service(&app, login_req).await;
        assert!(login_resp.status().is_success());
        let body: LoginResponse = test::read_body_json(login_resp).await;
        assert!(!body.access_token.is_empty());

        // protected route with header token
        let counts_req = test::TestRequest::get()
            .uri("/api/runtime_counts?access_token=test-token")
            .insert_header((header::AUTHORIZATION, format!("Bearer {}", body.access_token)))
            .to_request();
let counts_resp = test::call_service(&app, counts_req).await;
        assert!(counts_resp.status().is_success());
    }

    #[actix_web::test]
async fn e2e_protected_requires_both() {
        let (app, _state) = build_test_app();
let app = test::init_service(app).await;

        // without auth should be 401
        let noauth_req = test::TestRequest::get().uri("/api/runtime_counts").to_request();
let noauth_resp = test::call_service(&app, noauth_req).await;
        assert_eq!(noauth_resp.status(), actix_web::http::StatusCode::UNAUTHORIZED);

        // with only query param should be 401
        let only_qp = test::TestRequest::get().uri("/api/runtime_counts?access_token=test-token").to_request();
let only_qp_resp = test::call_service(&app, only_qp).await;
        assert_eq!(only_qp_resp.status(), actix_web::http::StatusCode::UNAUTHORIZED);

        // register and login to obtain PASETO
        let reg = RegisterRequest { name: "U".into(), email: "u@example.com".into(), password: "pw".into(), note: None };
        let reg_req = test::TestRequest::post().uri("/api/auth/register").set_json(&reg).to_request();
let _ = test::call_service(&app, reg_req).await;
        let login = LoginRequest { username: "u@example.com".into(), email: "u@example.com".into(), password: "pw".into() };
        let login_req = test::TestRequest::post().uri("/api/auth/login").set_json(&login).to_request();
let login_resp = test::call_service(&app, login_req).await;
        let body: LoginResponse = test::read_body_json(login_resp).await;

        // with only header should be 401
        let only_hdr = test::TestRequest::get()
            .uri("/api/runtime_counts")
            .insert_header((header::AUTHORIZATION, format!("Bearer {}", body.access_token)))
            .to_request();
let only_hdr_resp = test::call_service(&app, only_hdr).await;
        assert_eq!(only_hdr_resp.status(), actix_web::http::StatusCode::UNAUTHORIZED);

        // with both should succeed
        let both = test::TestRequest::get()
            .uri("/api/runtime_counts?access_token=test-token")
            .insert_header((header::AUTHORIZATION, format!("Bearer {}", body.access_token)))
            .to_request();
let both_resp = test::call_service(&app, both).await;
        assert!(both_resp.status().is_success());
    }

    #[actix_web::test]
    async fn unit_issue_paseto_token_contains_claims() {
        // Build minimal state with fixed key
        let db = sled::Config::new().temporary(true).open().unwrap();
        let mut cfg = Config::from_env();
        let key_hex = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"; // 32 bytes of 0xaa
        std::env::set_var("PASETO_V4_LOCAL_KEY_HEX", key_hex);
        cfg.paseto_local_key_b64 = None;
        cfg.access_token = "tok".into();
        let state = AppState::new(db, "test.json".into(), &cfg);
        let token = state.issue_paseto_v4_local("user@example.com", Some("User")).expect("token");
        let k = state.paseto_local_key.read().unwrap();
        let backend = paseto::tokens::TimeBackend::Chrono;
        let claims = validate_local_token(&token, None, &k[..], &backend).expect("valid");
        assert_eq!(claims.get("iss").and_then(|v| v.as_str()), Some(&state.token_issuer[..]));
        assert_eq!(claims.get("aud").and_then(|v| v.as_str()), Some(&state.token_audience[..]));
        assert_eq!(claims.get("sub").and_then(|v| v.as_str()), Some("user@example.com"));
    }
}

impl Config {
    /// Load configuration from environment variables with defaults
    fn from_env() -> Self {
        Self {
            // Server configuration
            port: env_var_or("PORT", 8080),
            host: env_var_or("HOST", "127.0.0.1".to_string()),
            server_name: env_var_or("SERVER_NAME", "periodic-backend-server".to_string()),

            // Database configuration
            db_path: env_var_or("DB_PATH", "periodic_data".to_string()),
            backup_path: env_var_or("BACKUP_PATH", "periodic_data_backup.json".to_string()),
            backup_interval_seconds: {
                // Try PERIODIC_BACKUP_DB first (new format), fallback to BACKUP_INTERVAL_SECONDS (old format)
                let backup_time_str = std::env::var("PERIODIC_BACKUP_DB")
                    .or_else(|_| std::env::var("BACKUP_INTERVAL_SECONDS").map(|s| format!("{}s", s)))
                    .unwrap_or_else(|_| "5s".to_string());

                match parse_time_to_seconds(&backup_time_str) {
                    Ok(seconds) => {
                        log::info!("Backup interval set to: {} ({}s)", backup_time_str, seconds);
                        seconds
                    }
                    Err(e) => {
                        log::warn!("Invalid backup interval '{}': {}. Using default 5s", backup_time_str, e);
                        5
                    }
                }
            },

            // Auth configuration
            paseto_local_key_b64: std::env::var("PASETO_V4_LOCAL_KEY_B64").ok(),
            token_issuer: env_var_or("TOKEN_ISS", "periodic-backend".to_string()),
            token_audience: env_var_or("TOKEN_AUD", "periodic-frontend".to_string()),
            token_ttl_seconds: env_var_or("TOKEN_TTL_SECONDS", 60u64 * 60u64 * 12u64), // 12h default

            // Kitchen IoT device
            kitchen_iot_enabled: env_var_or("KITCHEN_IOT_ENABLED", true),
            kitchen_iot_url: env_var_or("KITCHEN_IOT_URL", "http://192.168.1.100:3000".to_string()),
            kitchen_iot_timeout_ms: env_var_or("KITCHEN_IOT_TIMEOUT_MS", 5000),
            kitchen_iot_api_key: env_var_or("KITCHEN_IOT_API_KEY", String::new()),

            // Cashier machine
            cashier_enabled: env_var_or("CASHIER_ENABLED", true),
            cashier_url: env_var_or("CASHIER_URL", "http://192.168.1.101:8081".to_string()),
            cashier_timeout_ms: env_var_or("CASHIER_TIMEOUT_MS", 3000),
            cashier_api_key: env_var_or("CASHIER_API_KEY", String::new()),
            cashier_device_id: env_var_or("CASHIER_DEVICE_ID", "cashier-001".to_string()),

            // Display system
            display_enabled: env_var_or("DISPLAY_ENABLED", true),
            display_url: env_var_or("DISPLAY_URL", "http://192.168.1.102:4000".to_string()),
            display_timeout_ms: env_var_or("DISPLAY_TIMEOUT_MS", 2000),
            display_api_key: env_var_or("DISPLAY_API_KEY", String::new()),
            display_device_id: env_var_or("DISPLAY_DEVICE_ID", "display-main".to_string()),

            // POS system
            pos_system_enabled: env_var_or("POS_SYSTEM_ENABLED", false),
            pos_system_url: env_var_or("POS_SYSTEM_URL", "http://192.168.1.103:9000".to_string()),
            pos_system_api_key: env_var_or("POS_SYSTEM_API_KEY", String::new()),

            // Inventory system
            inventory_system_enabled: env_var_or("INVENTORY_SYSTEM_ENABLED", false),
            inventory_system_url: env_var_or("INVENTORY_SYSTEM_URL", "http://192.168.1.104:7000".to_string()),
            inventory_system_api_key: env_var_or("INVENTORY_SYSTEM_API_KEY", String::new()),

            // Security and debugging
            debug_mode: env_var_or("DEBUG_MODE", false),
            health_check_enabled: env_var_or("HEALTH_CHECK_ENABLED", true),
            api_rate_limit_enabled: env_var_or("API_RATE_LIMIT_ENABLED", true),
            api_rate_limit_requests_per_minute: env_var_or("API_RATE_LIMIT_REQUESTS_PER_MINUTE", 100),

            // Access token authentication
            access_token: env_var_or("ACCESS_TOKEN", "default-insecure-token".to_string()),
        }
    }
}

impl CorsConfig {
    /// Load CORS configuration from .env_cors file
    fn from_file(file_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let content = match fs::read_to_string(file_path) {
            Ok(content) => content,
            Err(_) => {
                log::warn!("CORS configuration file {} not found, using permissive defaults", file_path);
                return Ok(Self::default());
            }
        };

        let mut rules = Vec::new();

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            if let Some(rule) = Self::parse_cors_rule(line) {
                rules.push(rule);
            } else {
                log::warn!("Failed to parse CORS rule: {}", line);
            }
        }

        Ok(Self { rules })
    }

    /// Parse a single CORS rule line
    fn parse_cors_rule(line: &str) -> Option<CorsRule> {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() {
            return None;
        }

        let origin_pattern = parts[0].to_string();
        let action = if parts.len() > 1 {
            match parts[1].to_uppercase().as_str() {
                "DENY" => CorsAction::Deny,
                _ => CorsAction::Allow,
            }
        } else {
            CorsAction::Allow
        };

        let methods = if parts.len() > 2 {
            let methods_str = parts[2];
            if methods_str.to_uppercase() == "ALL" {
                vec!["ALL".to_string()]
            } else {
                methods_str.split(',').map(|m| m.trim().to_uppercase()).collect()
            }
        } else {
            vec!["ALL".to_string()]
        };

        let headers = if parts.len() > 3 {
            let headers_str = parts[3];
            if headers_str.to_uppercase() == "ALL" {
                vec!["ALL".to_string()]
            } else {
                headers_str.split(',').map(|h| h.trim().to_lowercase()).collect()
            }
        } else {
            vec!["ALL".to_string()]
        };

        Some(CorsRule {
            origin_pattern,
            action,
            methods,
            headers,
        })
    }

    /// Create a default CORS configuration for development
    fn default() -> Self {
        Self {
            rules: vec![
                CorsRule {
                    origin_pattern: "http://localhost:*".to_string(),
                    action: CorsAction::Allow,
                    methods: vec!["ALL".to_string()],
                    headers: vec!["ALL".to_string()],
                },
                CorsRule {
                    origin_pattern: "http://127.0.0.1:*".to_string(),
                    action: CorsAction::Allow,
                    methods: vec!["ALL".to_string()],
                    headers: vec!["ALL".to_string()],
                },
            ],
        }
    }

    /// Apply CORS configuration to Actix-web CORS middleware
    fn apply_to_cors(&self) -> Cors {
        let mut cors = Cors::default();

        let mut has_allowed_origins = false;

        for rule in &self.rules {
            match rule.action {
                CorsAction::Allow => {
                    // Handle wildcard patterns
                    if rule.origin_pattern == "*" {
                        cors = cors.allow_any_origin();
                        has_allowed_origins = true;
                    } else if rule.origin_pattern.contains('*') {
                        // For now, treat wildcard patterns as allow any origin
                        // In production, you might want more sophisticated pattern matching
                        cors = cors.allow_any_origin();
                        has_allowed_origins = true;
                    } else {
                        cors = cors.allowed_origin(&rule.origin_pattern);
                        has_allowed_origins = true;
                    }

                    // Apply method restrictions
                    if rule.methods.len() == 1 && rule.methods[0] == "ALL" {
                        cors = cors.allow_any_method();
                    } else {
                        let methods: Vec<Method> = rule.methods
                            .iter()
                            .filter_map(|m| Method::from_str(m).ok())
                            .collect();
                        if !methods.is_empty() {
                            cors = cors.allowed_methods(methods);
                        }
                    }

                    // Apply header restrictions
                    if rule.headers.len() == 1 && rule.headers[0] == "ALL" {
                        cors = cors.allow_any_header();
                    } else {
                        for header in &rule.headers {
                            cors = cors.allowed_header(header.as_str());
                        }
                    }
                }
                CorsAction::Deny => {
                    // CORS middleware doesn't directly support denying specific origins
                    // Log the denial for monitoring
                    log::info!("CORS: Configured to deny origin pattern: {}", rule.origin_pattern);
                }
            }
        }

        // If no allowed origins were set, default to allowing any origin for development
        if !has_allowed_origins {
            log::warn!("No allowed CORS origins configured, defaulting to allow any origin");
            cors = cors.allow_any_origin().allow_any_method().allow_any_header();
        }

        cors
    }
}

/// Helper function to get environment variable or return default value
fn env_var_or<T: FromStr + Clone>(key: &str, default: T) -> T
where
    T::Err: std::fmt::Debug,
{
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse::<T>().ok())
        .unwrap_or(default)
}

// IoT Device Client Implementations
#[derive(Debug, Clone)]
struct IoTDeviceClient {
    client: Client,
    base_url: String,
    api_key: String,
    #[allow(dead_code)]
    timeout_ms: u64,
    enabled: bool,
    device_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct IoTDeviceStatus {
    device_id: Option<String>,
    url: String,
    enabled: bool,
    status: String,
    last_check: u64,
    response_time_ms: Option<u64>,
    error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct IoTHealthCheckResponse {
    kitchen_iot: IoTDeviceStatus,
    cashier: IoTDeviceStatus,
    display: IoTDeviceStatus,
    pos_system: IoTDeviceStatus,
    inventory_system: IoTDeviceStatus,
}

impl IoTDeviceClient {
    fn new(base_url: String, api_key: String, timeout_ms: u64, enabled: bool, device_id: Option<String>) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_millis(timeout_ms))
            .build()
            .unwrap_or_else(|_| Client::new());

        Self {
            client,
            base_url,
            api_key,
            timeout_ms,
            enabled,
            device_id,
        }
    }

    /// Check if the IoT device is reachable
    async fn health_check(&self) -> IoTDeviceStatus {
        let start_time = std::time::Instant::now();
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        if !self.enabled {
            return IoTDeviceStatus {
                device_id: self.device_id.clone(),
                url: self.base_url.clone(),
                enabled: false,
                status: "disabled".to_string(),
                last_check: timestamp,
                response_time_ms: None,
                error: None,
            };
        }

        let health_url = if self.base_url.ends_with('/') {
            format!("{}health", self.base_url)
        } else {
            format!("{}/health", self.base_url)
        };

        let mut request = self.client.get(&health_url);

        if !self.api_key.is_empty() {
            request = request.header("Authorization", format!("Bearer {}", self.api_key));
        }

        if let Some(ref device_id) = self.device_id {
            request = request.header("X-Device-ID", device_id);
        }

        match request.send().await {
            Ok(response) => {
                let response_time = start_time.elapsed().as_millis() as u64;
                let status = if response.status().is_success() {
                    "healthy".to_string()
                } else {
                    format!("unhealthy ({})", response.status())
                };

                IoTDeviceStatus {
                    device_id: self.device_id.clone(),
                    url: self.base_url.clone(),
                    enabled: true,
                    status,
                    last_check: timestamp,
                    response_time_ms: Some(response_time),
                    error: None,
                }
            }
            Err(e) => {
                let response_time = start_time.elapsed().as_millis() as u64;
                IoTDeviceStatus {
                    device_id: self.device_id.clone(),
                    url: self.base_url.clone(),
                    enabled: true,
                    status: "error".to_string(),
                    last_check: timestamp,
                    response_time_ms: Some(response_time),
                    error: Some(e.to_string()),
                }
            }
        }
    }

    /// Send data to the IoT device
    async fn send_data<T: Serialize>(&self, endpoint: &str, data: &T) -> Result<serde_json::Value, String> {
        if !self.enabled {
            return Err("Device is disabled".to_string());
        }

        let url = if self.base_url.ends_with('/') {
            format!("{}{}", self.base_url, endpoint.trim_start_matches('/'))
        } else {
            format!("{}/{}", self.base_url, endpoint.trim_start_matches('/'))
        };

        let mut request = self.client.post(&url).json(data);

        if !self.api_key.is_empty() {
            request = request.header("Authorization", format!("Bearer {}", self.api_key));
        }

        if let Some(ref device_id) = self.device_id {
            request = request.header("X-Device-ID", device_id);
        }

        match request.send().await {
            Ok(response) => {
                if response.status().is_success() {
                    match response.json::<serde_json::Value>().await {
                        Ok(json) => Ok(json),
                        Err(e) => Err(format!("Failed to parse response: {}", e)),
                    }
                } else {
                    Err(format!("HTTP error: {}", response.status()))
                }
            }
            Err(e) => Err(format!("Request failed: {}", e)),
        }
    }

    /// Get data from the IoT device
    async fn get_data(&self, endpoint: &str) -> Result<serde_json::Value, String> {
        if !self.enabled {
            return Err("Device is disabled".to_string());
        }

        let url = if self.base_url.ends_with('/') {
            format!("{}{}", self.base_url, endpoint.trim_start_matches('/'))
        } else {
            format!("{}/{}", self.base_url, endpoint.trim_start_matches('/'))
        };

        let mut request = self.client.get(&url);

        if !self.api_key.is_empty() {
            request = request.header("Authorization", format!("Bearer {}", self.api_key));
        }

        if let Some(ref device_id) = self.device_id {
            request = request.header("X-Device-ID", device_id);
        }

        match request.send().await {
            Ok(response) => {
                if response.status().is_success() {
                    match response.json::<serde_json::Value>().await {
                        Ok(json) => Ok(json),
                        Err(e) => Err(format!("Failed to parse response: {}", e)),
                    }
                } else {
                    Err(format!("HTTP error: {}", response.status()))
                }
            }
            Err(e) => Err(format!("Request failed: {}", e)),
        }
    }
}

// Access token validation function
fn validate_access_token(req: &HttpRequest, app_state: &web::Data<AppState>) -> Result<(), HttpResponse> {
    // Skip authentication for health check endpoints to allow monitoring
    if req.path() == "/api/health" || req.path() == "/health" {
        return Ok(());
    }

    // Require BOTH: 1) access_token query param, 2) Authorization: Bearer PASETO v4.local

    // 1) Validate access_token in query string
    let current_access_token = match app_state.access_token.try_read() {
        Ok(token) => token.clone(),
        Err(_) => {
            log::warn!("Could not acquire access token read lock, denying request");
            return Err(HttpResponse::ServiceUnavailable().json(
                serde_json::json!({
                    "error": "Authentication service temporarily unavailable",
                    "code": "AUTH_UNAVAILABLE"
                })
            ));
        }
    };

    let mut qp_token: Option<String> = None;
    if !req.query_string().is_empty() {
        for param in req.query_string().split('&') {
            if let Some((key, value)) = param.split_once('=') {
                if key == "access_token" {
                    qp_token = Some(value.to_string());
                    break;
                }
            }
        }
    }

    match qp_token {
        Some(ref token) if token == &current_access_token => {}
        Some(_) => {
            return Err(HttpResponse::Unauthorized().json(
                serde_json::json!({
                    "error": "Invalid access_token",
                    "code": "INVALID_TOKEN"
                })
            ));
        }
        None => {
            return Err(HttpResponse::Unauthorized().json(
                serde_json::json!({
                    "error": "access_token query parameter required",
                    "code": "MISSING_TOKEN"
                })
            ));
        }
    }

    // 2) Validate Authorization: Bearer <PASETO>
    let auth_header = req.headers().get("Authorization").and_then(|v| v.to_str().ok());
    let bearer = auth_header.and_then(|s| s.strip_prefix("Bearer "));
    let token = match bearer {
        Some(t) if !t.is_empty() => t,
        _ => {
            return Err(HttpResponse::Unauthorized().json(
                serde_json::json!({
                    "error": "Authorization: Bearer token required",
                    "code": "MISSING_BEARER"
                })
            ));
        }
    };

    let key = app_state.paseto_local_key.read().unwrap();
    let backend = paseto::tokens::TimeBackend::Chrono;
    match validate_local_token(token, None, &key[..], &backend) {
        Ok(claims_json) => {
            let iss_ok = claims_json.get("iss").and_then(|v| v.as_str()) == Some(app_state.token_issuer.as_str());
            let aud_ok = claims_json.get("aud").and_then(|v| v.as_str()) == Some(app_state.token_audience.as_str());
            if !iss_ok || !aud_ok {
                return Err(HttpResponse::Unauthorized().json(
                    serde_json::json!({
                        "error": "PASETO iss/aud mismatch",
                        "code": "INVALID_BEARER"
                    })
                ));
            }
        }
        Err(e) => {
            log::debug!("Invalid PASETO token: {}", e);
            return Err(HttpResponse::Unauthorized().json(
                serde_json::json!({
                    "error": "Invalid PASETO token",
                    "code": "INVALID_BEARER"
                })
            ));
        }
    }

    Ok(())
}

// Access token update structure
#[derive(Debug, Serialize, Deserialize)]
struct AccessTokenUpdateRequest {
    new_token: String,
}

// Auth: Register/Login payloads
#[derive(Debug, Serialize, Deserialize)]
struct RegisterRequest {
    name: String,
    email: String,
    password: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    note: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct LoginRequest {
    username: String,
    email: String,
    password: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct LoginResponse {
    #[serde(rename = "access_token")]
    access_token: String,
    #[serde(rename = "displayName", skip_serializing_if = "Option::is_none")]
    display_name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ClaimsUpdateRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    roles: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    scopes: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct RuntimeEventsResponse {
    items: Vec<RuntimeEvent>,
    #[serde(rename = "nextCursor")]
    next_cursor: String,
    #[serde(rename = "hasMore")]
    has_more: bool,
}

// Payment data structures
#[derive(Debug, Serialize, Deserialize, Clone)]
struct RuntimePayment {
    #[serde(rename = "idempotencyKey")]
    idempotency_key: String,
    index: u64,
    amount: f64,
    currency: String,
    method: String, // "card" | "customer" | "cash" | "wallet"
    source: String, // "table_card" | "mobile" | "web"
    #[serde(skip_serializing_if = "Option::is_none")]
    meta: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct RuntimePaymentResponse {
    id: String,
    ts: u64,
    status: String, // "pending" | "approved" | "declined"
}

// Kitchen job structures
#[derive(Debug, Serialize, Deserialize, Clone)]
struct KitchenPrintRequest {
    order: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    printers: Option<Vec<String>>,
    #[serde(rename = "idempotencyKey", skip_serializing_if = "Option::is_none")]
    idempotency_key: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct KitchenPrinterStatus {
    url: String,
    status: String, // "queued" | "sent" | "failed" | "completed"
    attempts: u32,
    #[serde(rename = "lastError", skip_serializing_if = "Option::is_none")]
    last_error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct KitchenPrintResponse {
    #[serde(rename = "jobId")]
    job_id: String,
    accepted: bool,
    printers: Vec<KitchenPrinterStatus>,
    ts: u64,
}

// Kitchen test structures
#[derive(Debug, Serialize, Deserialize, Clone)]
struct KitchenTestRequest {
    order: Value,
    #[serde(rename = "latencyMs", skip_serializing_if = "Option::is_none")]
    latency_ms: Option<u64>,
    #[serde(rename = "failRate", skip_serializing_if = "Option::is_none")]
    fail_rate: Option<f64>,
    #[serde(rename = "printerName", skip_serializing_if = "Option::is_none")]
    printer_name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct KitchenTestResponse {
    #[serde(rename = "jobId")]
    job_id: String,
    accepted: bool,
    printer: String,
    ts: u64,
}

// Active kitchen config
#[derive(Debug, Serialize, Deserialize, Clone)]
struct ActiveKitchenConfig {
    name: Option<String>,
}

// Runtime counts
#[derive(Debug, Serialize, Deserialize)]
struct RuntimeCounts {
    orders: u64,
    reservations: u64,
    payments: u64,
}

// // Wordle Game API Structures
// #[derive(Debug, Serialize, Deserialize, Clone)]
// struct WordleSession {
//     id: String,
//     #[serde(rename = "cardIndex")]
//     card_index: u32,
//     #[serde(rename = "boardId", skip_serializing_if = "Option::is_none")]
//     board_id: Option<String>,
//     seed: Option<String>,
//     #[serde(rename = "solutionHash")]
//     solution_hash: String,
//     solution: String, // Server-side only, not serialized to clients
//     #[serde(rename = "createdAt")]
//     created_at: u64,
//     participants: Vec<WordleParticipant>,
//     guesses: Vec<WordleGuess>,
//     status: String, // "active" | "completed" | "closed"
//     #[serde(rename = "expiresAt")]
//     expires_at: u64,
// // }
#[derive(Debug, Serialize, Deserialize, Clone)]
struct WordleSession {
    id: String,
    #[serde(rename = "cardIndex")]
    card_index: u32,
    #[serde(rename = "boardId", skip_serializing_if = "Option::is_none")]
    board_id: Option<String>,

    seed: Option<String>,
    #[serde(rename = "solutionHash")]
    solution_hash: String,
    solution: String, // not serialized

    #[serde(rename = "createdAt")]
    created_at: u64,

    // NEW:
    #[serde(rename = "rules")]
    rules: WordleRules,
    #[serde(rename = "wordlist")]
    wordlist: WordListRef,

    participants: Vec<WordleParticipant>,
    guesses: Vec<WordleGuess>,
    status: String, // "active" | "completed" | "closed"
    #[serde(rename = "expiresAt")]
    expires_at: u64,

    // Timer bookkeeping:
    #[serde(rename = "startedAt", skip_serializing_if = "Option::is_none")]
    started_at: Option<u64>,
    #[serde(rename = "endedAt", skip_serializing_if = "Option::is_none")]
    ended_at: Option<u64>,
}
// #[derive(Debug, Serialize, Deserialize, Clone, Default)]
// struct WordleSession {
//     id: String,
//     #[serde(rename = "cardIndex")]
//     card_index: u32,
//     #[serde(rename = "boardId", skip_serializing_if = "Option::is_none")]
//     board_id: Option<String>,
//     seed: Option<String>,
//     #[serde(rename = "solutionHash")]
//     solution_hash: String,
//     solution: String, // server-only
//     #[serde(rename = "createdAt")]
//     created_at: u64,

//     // NEW
//     #[serde(rename = "startedAt", skip_serializing_if = "Option::is_none")]
//     started_at: Option<u64>,
//     #[serde(rename = "endedAt", skip_serializing_if = "Option::is_none")]
//     ended_at: Option<u64>,
//     #[serde(default)]
//     rules: WordleRules,
//     #[serde(skip_serializing_if = "Option::is_none")]
//     owner_id: Option<String>, // or your real field

//     participants: Vec<WordleParticipant>,
//     guesses: Vec<WordleGuess>,
//     status: String,
//     #[serde(rename = "expiresAt")]
//     expires_at: u64,
// }

#[derive(Debug, Serialize, Deserialize, Clone)]
struct WordleParticipant {
    #[serde(rename = "userId")]
    user_id: String,
    #[serde(rename = "displayName", skip_serializing_if = "Option::is_none")]
    display_name: Option<String>,
    #[serde(rename = "joinedAt")]
    joined_at: u64,
    #[serde(rename = "lastSeen")]
    last_seen: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    status: Option<String>, // "active", "away", "left"
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct WordleGuess {
    #[serde(rename = "userId")]
    user_id: String,
    guess: String,
    marks: Vec<char>, // 'c' = correct, 'p' = present, 'a' = absent
    attempt: u32,
    ts: u64,
    won: bool,
    lost: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct WordleSessionCreateRequest {
    #[serde(rename = "cardIndex")]
    card_index: u32,
    #[serde(rename = "boardId", skip_serializing_if = "Option::is_none")]
    board_id: Option<String>,
    seed: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct WordleSessionCreateResponse {
    id: String,
    #[serde(rename = "cardIndex")]
    card_index: u32,
    #[serde(rename = "solutionHash")]
    solution_hash: String,
    #[serde(rename = "createdAt")]
    created_at: u64,
}

#[derive(Debug, Serialize, Deserialize)]
struct WordleJoinRequest {
    #[serde(rename = "userId")]
    user_id: String,
    #[serde(rename = "displayName", skip_serializing_if = "Option::is_none")]
    display_name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct WordleGuessRequest {
    #[serde(rename = "userId")]
    user_id: String,
    guess: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct WordleGuessResponse {
    ok: bool,
    marks: Vec<char>,
    won: bool,
    lost: bool,
    attempt: u32,
    ts: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WordleSessionStateResponse {
    #[serde(rename = "cardIndex")]
    card_index: u32,
    #[serde(rename = "solutionHash")]
    solution_hash: String,
    participants: Vec<WordleParticipant>,
    guesses: Vec<WordleGuess>,
    status: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct WordleLeaveRequest {
    #[serde(rename = "userId")]
    user_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WordleSSEEvent {
    #[serde(rename = "type")]
    event_type: String, // "join" | "leave" | "guess" | "state" | "close"
    #[serde(rename = "sessionId")]
    session_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    participant: Option<WordleParticipant>,
    #[serde(skip_serializing_if = "Option::is_none")]
    guess: Option<WordleGuess>,
    #[serde(skip_serializing_if = "Option::is_none")]
    state: Option<WordleSessionStateResponse>,
    ts: u64,
}

// Rate limiting structure for Wordle
#[derive(Debug, Clone)]
struct WordleRateLimit {
    last_guess: u64,
    #[allow(dead_code)]
    guess_count: u32,
    #[allow(dead_code)]
    window_start: u64,
}

// Wordle WebSocket/SSE Events
#[derive(Debug, Serialize, Deserialize)]
#[allow(dead_code)]
struct WordleEvent {
    #[serde(rename = "type")]
    event_type: String, // "join" | "leave" | "guess-submitted" | "session-closed"
    #[serde(rename = "sessionId")]
    session_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    participant: Option<WordleParticipant>,
    #[serde(skip_serializing_if = "Option::is_none")]
    guess: Option<WordleGuess>,
    ts: u64,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
struct WordListRef {
    name: String,          // config name, eg. "default"
    version: String,       // immutable content hash or semantic version
    lang: String,          // "en"
    word_len: u8,          // 5
    // optionally: source URLs, updated_at, etc.
}
impl Default for WordListRef {
    fn default() -> Self {
        Self {
            name: "default".to_string(),
            version: "1".to_string(),
            lang: "en".to_string(),
            word_len: 5u8,
        }
    }
}

// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub struct WordListRef {
//     pub name: String,
//     pub version: u32,
// }
// #[derive(Debug, Serialize, Deserialize, Clone)]
// struct WordleRules {
//     max_attempts: u8,          // 6
//     max_time_ms: Option<u64>,  // per-game timer (optional)
//     scoring: ScoringRules,     // see below
// }
// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub struct WordleRules {
//     pub max_attempts: u32,
//     pub word_len: u8,
//     pub timed: bool,
//     #[serde(skip_serializing_if = "Option::is_none")]
//     pub total_seconds: Option<u32>,
//     #[serde(skip_serializing_if = "Option::is_none")]
//     pub per_guess_seconds: Option<u32>,
//     pub scoring: WordleScoring,
// // }
// #[derive(Debug, Clone, Serialize, Deserialize)]
// struct WordleRules {
//     max_attempts: u32,
//     word_len: u8,
//     timed: bool,
//     total_seconds: u32,           // 0 means “no total cap”
//     per_guess_seconds: Option<u32>, // optional per-guess cap
//     scoring: ScoringRules,
// }
// #[derive(Debug, Clone, Serialize, Deserialize)]
// struct WordleRules {
//     max_attempts: u32,
//     word_len: u8,
//     timed: bool,
//     total_seconds: u32,            // 0 = none
//     per_guess_seconds: Option<u32>,
//     scoring: ScoringRules,         // <-- this expects ScoringRules
// }
// impl Default for WordleRules {
//     fn default() -> Self {
//         Self {
//             max_attempts: 6,
//             word_len: 5,
//             timed: false,
//             total_seconds: None,
//             per_guess_seconds: None,
//             scoring: WordleScoring::TimeAndRows,
//         }
//     }
// }



#[derive(Debug, Clone, Serialize, Deserialize)]
struct WordleRules {
    max_attempts: u32,
    word_len: u8,
    timed: bool,
    total_seconds: u32,             // 0 = no overall cap
    per_guess_seconds: Option<u32>, // optional per-guess cap
    scoring: ScoringRules,
}
// no  WordleScoring enum

// impl Default for WordleRules {
//     fn default() -> Self {
//         Self {
//             max_attempts: 6,
//             word_len: 5,
//             timed: false,
//             total_seconds: 0,          // <-- not None; it's a u32
//             per_guess_seconds: None,
//             scoring: ScoringRules::default(), // <-- not WordleScoring
//         }
//     }
// }
// Then you can do: SCORING - keep your WordleScoring enum
// Then you can do: SCORING - keep your WordleScoring enum
// impl Default for WordleRules {
//     fn default() -> Self {
//         Self {
//             max_attempts: 6,
//             word_len: 5,
//             timed: false,
//             total_seconds: 0,
//             per_guess_seconds: None,
//             scoring: ScoringRules::from(WordleScoring::TimeAndRows),
//         }
//     }
// }
impl Default for WordleRules {
    fn default() -> Self {
        Self {
            max_attempts: 6,
            word_len: 5,
            timed: false,
            total_seconds: 0,
            per_guess_seconds: None,
            scoring: WordleScoring::TimeAndRows.into(),
        }
    }
}

// // #[derive(Debug, Clone, Serialize, Deserialize)]
// pub enum WordleScoring { TimeOnly, RowsOnly, TimeAndRows }
// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub enum WordleScoring {
//     TimeOnly,
//     RowsOnly,
//     TimeAndRows,
// }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WordleScoring {
    TimeOnly,
    RowsOnly,
    TimeAndRows,
}


// impl From<WordleScoring> for ScoringRules {
//     fn from(s: WordleScoring) -> Self {
//         match s {
//             WordleScoring::TimeAndRows => ScoringRules {
//                 base: 1000,
//                 per_attempt_penalty: 100,
//                 time_penalty_per_sec: 1,
//                 timeout_ms: 300_000,
//             },
//         }
//     }
// }
impl From<WordleScoring> for ScoringRules {
    fn from(s: WordleScoring) -> Self {
        match s {
            WordleScoring::TimeOnly => ScoringRules {
                // Only time matters: no row penalty
                base: 1000,
                per_attempt_penalty: 0,
                time_penalty_per_sec: 2,
                timeout_ms: 300_000, // 5 min (set as you like)
            },
            WordleScoring::RowsOnly => ScoringRules {
                // Only rows/attempts matter: no time penalty and effectively no timeout
                base: 1000,
                per_attempt_penalty: 120,
                time_penalty_per_sec: 0,
                timeout_ms: 0, // 0 = no timeout
            },
            WordleScoring::TimeAndRows => ScoringRules {
                // Both matter
                base: 1000,
                per_attempt_penalty: 100,
                time_penalty_per_sec: 1,
                timeout_ms: 300_000,
            },
        }
    }
}

// #[derive(Debug, Serialize, Deserialize, Clone)]
// struct ScoringRules {
//     // Example: base points for win, penalties per attempt/time.
//     win_base: i32,          // e.g. 1000
//     per_attempt_penalty: i32, // e.g. -100 per extra attempt
//     time_penalty_per_sec: i32, // e.g. -2 per second
//     fail_penalty: i32,      // e.g. -500
// }

// #[derive(Debug, Serialize, Deserialize, Clone)]
// struct WordleScore {
//     #[serde(rename = "userId")]
//     user_id: String,
//     points: i32,
//     attempts: u8,
//     time_ms: u64,
//     won: bool,
//     ts: u64,
// }


// #[derive(Debug, Serialize, Deserialize)]
// struct WordleLeaderboardEntry {
//     #[serde(rename = "userId")]
//     user_id: String,
//     display_name: Option<String>,
//     points: i32,
//     wins: u32,
//     losses: u32,
//     avg_time_ms: Option<u64>,
// }
// #[derive(Debug, Serialize, Deserialize, Clone)]
// struct ScoringRules {
//     base: i32,                // starting points per win
//     per_attempt_penalty: i32, // minus per extra attempt beyond 1
//     time_penalty_per_sec: i32,// minus per elapsed second
//     timeout_ms: u64,          // optional cap
// }
// #[derive(Debug, Clone, Serialize, Deserialize)]
// struct ScoringRules {
//     base: i32,
//     per_attempt_penalty: i32,
//     time_penalty_per_sec: i32,
//     timeout_ms: u64,
// }
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ScoringRules {
    base: i32,
    per_attempt_penalty: i32,
    time_penalty_per_sec: i32,
    /// 0 = no timeout
    timeout_ms: u64,
}
// impl Default for ScoringRules {
//     fn default() -> Self {
//         Self {
//             base: 1000,
//             per_attempt_penalty: 100,
//             time_penalty_per_sec: 1,
//             timeout_ms: 300_000, // 5 minutes
//         }
//     }
// }
impl Default for ScoringRules {
    fn default() -> Self {
        ScoringRules {
            base: 1000,
            per_attempt_penalty: 100,
            time_penalty_per_sec: 1,
            timeout_ms: 300_000, // 5 min
        }
    }
}
#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize, Clone)]
struct WordleScore {
    user_id: String,
    display_name: Option<String>,
    points: i32,
    attempts: u32,
    duration_ms: u64,
    ts: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct WordleLeaderboardEntry {
    user_id: String,
    display_name: Option<String>,
    best_points: i32,
    best_time_ms: u64,
    last_ts: u64,
}
// use once_cell::sync::Lazy;
// use std::{cmp::Ordering, sync::RwLock};

static LEADERBOARD: Lazy<RwLock<Vec<WordleLeaderboardEntry>>> =
    Lazy::new(|| RwLock::new(Vec::new()));


#[derive(Clone)]
struct AppState {
    db: Arc<RwLock<sled::Db>>,
    backup_path: String,
    // Queue of files that couldn't be deleted (e.g., locked) to retry later
    failed_deletions: Arc<RwLock<Vec<String>>>,
    event_sequence: Arc<AtomicU64>,
    // Auth state
    paseto_local_key: Arc<RwLock<[u8; 32]>>, // v4.local symmetric key
    token_issuer: String,
    token_audience: String,
    token_ttl_seconds: u64,
    // IoT Device Clients
    kitchen_iot: IoTDeviceClient,
    cashier: IoTDeviceClient,
    display: IoTDeviceClient,
    pos_system: IoTDeviceClient,
    inventory_system: IoTDeviceClient,
    // Access token for authentication
    access_token: Arc<RwLock<String>>,
    // Wordle game state
    wordle_rate_limits: Arc<RwLock<HashMap<String, WordleRateLimit>>>,
    wordle_sessions: Arc<RwLock<HashMap<String, WordleSession>>>,
    // SSE client tracking: session_id -> list of SSE senders
    wordle_sse_clients: Arc<RwLock<HashMap<String, Vec<tokio::sync::mpsc::UnboundedSender<WordleSSEEvent>>>>>,
}

// Database management functions
impl AppState {
    fn new(db: sled::Db, backup_path: String, config: &Config) -> Self {
        let key_bytes: [u8; 32] = {
            // Try base64 env first
            if let Some(b64) = &config.paseto_local_key_b64 {
                if let Some(arr) = base64::engine::general_purpose::STANDARD
                    .decode(b64)
                    .ok()
                    .and_then(|v| <Vec<u8> as TryInto<[u8;32]>>::try_into(v).ok())
                {
                    arr
                } else {
                    log::warn!("Invalid PASETO_V4_LOCAL_KEY_B64; will try PASETO_V4_LOCAL_KEY_HEX or generate ephemeral key");
                    // fallthrough to HEX or ephemeral
                    let hex_opt = std::env::var("PASETO_V4_LOCAL_KEY_HEX").ok();
                    if let Some(hex_str) = hex_opt {
                        if let Some(arr) = AppState::decode_hex_32(&hex_str) {
                            arr
                        } else {
                            let mut tmp = [0u8; 32];
                            OsRng.try_fill_bytes(&mut tmp).ok();
                            tmp
                        }
                    } else {
                        let mut tmp = [0u8; 32];
                        OsRng.try_fill_bytes(&mut tmp).ok();
                        tmp
                    }
                }
            } else {
                // Try HEX env var
                if let Ok(hex_str) = std::env::var("PASETO_V4_LOCAL_KEY_HEX") {
                    if let Some(arr) = AppState::decode_hex_32(&hex_str) {
                        arr
                    } else {
                        log::warn!("Invalid PASETO_V4_LOCAL_KEY_HEX; generating ephemeral key (tokens will invalidate on restart)");
                        let mut tmp = [0u8; 32];
                        OsRng.try_fill_bytes(&mut tmp).ok();
                        tmp
                    }
                } else {
                    log::warn!("No PASETO key provided, generating ephemeral key (tokens will invalidate on restart)");
                    let mut tmp = [0u8; 32];
                    OsRng.try_fill_bytes(&mut tmp).ok();
                    tmp
                }
            }
        };
        Self {
            db: Arc::new(RwLock::new(db)),
            backup_path,
            failed_deletions: Arc::new(RwLock::new(Vec::new())),
            event_sequence: Arc::new(AtomicU64::new(0)),
            paseto_local_key: Arc::new(RwLock::new(key_bytes)),
            token_issuer: config.token_issuer.clone(),
            token_audience: config.token_audience.clone(),
            token_ttl_seconds: config.token_ttl_seconds,
            kitchen_iot: IoTDeviceClient::new(
                config.kitchen_iot_url.clone(),
                config.kitchen_iot_api_key.clone(),
                config.kitchen_iot_timeout_ms,
                config.kitchen_iot_enabled,
                None,
            ),
            cashier: IoTDeviceClient::new(
                config.cashier_url.clone(),
                config.cashier_api_key.clone(),
                config.cashier_timeout_ms,
                config.cashier_enabled,
                Some(config.cashier_device_id.clone()),
            ),
            display: IoTDeviceClient::new(
                config.display_url.clone(),
                config.display_api_key.clone(),
                config.display_timeout_ms,
                config.display_enabled,
                Some(config.display_device_id.clone()),
            ),
            pos_system: IoTDeviceClient::new(
                config.pos_system_url.clone(),
                config.pos_system_api_key.clone(),
                5000, // Default timeout for POS
                config.pos_system_enabled,
                None,
            ),
            inventory_system: IoTDeviceClient::new(
                config.inventory_system_url.clone(),
                config.inventory_system_api_key.clone(),
                5000, // Default timeout for inventory
                config.inventory_system_enabled,
                None,
            ),
            // Initialize access token from config
            access_token: Arc::new(RwLock::new(config.access_token.clone())),
            // Initialize Wordle game state
            wordle_rate_limits: Arc::new(RwLock::new(HashMap::new())),
            wordle_sessions: Arc::new(RwLock::new(HashMap::new())),
            wordle_sse_clients: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Export database to temporal backup file to avoid lock contention
    fn export_to_backup(&self) -> std::io::Result<()> {
        use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

        let backup_start = Instant::now();
        log::debug!("Starting temporal database backup process...");

        // Best-effort processing of any previously queued deletions
        self.process_failed_deletions();

        // Prepare a fallback temporal path (second-resolution is fine here)
        let fallback_temporal_backup_path = {
            let ts = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            format!("{}.{}", self.backup_path, ts)
        };

        // Try to acquire read lock with shorter timeout to avoid blocking
        let lock_start = Instant::now();
        let timeout = Duration::from_millis(500); // Much shorter timeout

        let db = loop {
            if let Ok(db) = self.db.try_read() {
                break db;
            }
            if lock_start.elapsed() > timeout {
                log::warn!("Backup skipped: could not acquire database read lock within {}ms (database busy with writes)", timeout.as_millis());
                // Instead of failing, create a temporal backup with available data
                return self.create_fallback_temporal_backup(&fallback_temporal_backup_path, backup_start);
            }
            std::thread::sleep(Duration::from_millis(5));
        };
        let lock_duration = lock_start.elapsed();
        log::debug!("Database lock acquired in {}", Self::format_duration(lock_duration));

        let mut backup_data = std::collections::HashMap::new();
        let mut count = 0;
        let mut total_data_size = 0usize;

        // Process database with periodic checks
        let iteration_start = Instant::now();
        for result in db.iter() {
            match result {
                Ok((key, value)) => {
                    let key_str = String::from_utf8(key.to_vec()).unwrap_or_default();
                    let value_str = String::from_utf8(value.to_vec()).unwrap_or_default();

                    // Track data size
                    total_data_size += key_str.len() + value_str.len();

                    backup_data.insert(key_str, value_str);
                    count += 1;

                    // Check if we've been running too long (prevent backup from blocking too long)
                    if count % 1000 == 0 && iteration_start.elapsed() > Duration::from_secs(10) {
                        log::warn!("Backup iteration taking too long, processed {} items in {}",
                            count, Self::format_duration(iteration_start.elapsed()));
                        break; // Exit early to avoid blocking
                    }
                }
                Err(e) => {
                    log::error!("Error reading key during backup: {}", e);
                }
            }
        }

        let iteration_duration = iteration_start.elapsed();
        log::debug!("Database iteration completed in {} ({} items, ~{} bytes)",
            Self::format_duration(iteration_duration), count, total_data_size);

        // Release the database lock before doing file I/O
        drop(db);

        // Validate backup size
        if count < 1 {
            log::error!("Please increase the DB backup time. Value is too small (0 items found)");
            // Don't fail, just create minimal temporal backup
        }

        if total_data_size < 100 && count > 0 {
            log::error!("Please increase the DB backup time. Value is too small ({} bytes total data)", total_data_size);
        }

        // Serialize to JSON
        let serialization_start = Instant::now();
        let backup_json = serde_json::to_string_pretty(&backup_data)
            .map_err(std::io::Error::other)?;
        let serialization_duration = serialization_start.elapsed();

        log::debug!("JSON serialization completed in {} ({} bytes)",
            Self::format_duration(serialization_duration), backup_json.len());

        // Write to temporal file first with up to 3 attempts
        let write_start = Instant::now();
        let mut chosen_temporal: Option<String> = None;
        let mut last_err: Option<std::io::Error> = None;
        for attempt in 1..=3 {
            // Use milliseconds to improve uniqueness and reduce collisions within the same second
            let ts_ms = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;
            let temp_path = format!("{}.{}", self.backup_path, ts_ms);
            match fs::write(&temp_path, &backup_json) {
                Ok(()) => {
                    chosen_temporal = Some(temp_path);
                    break;
                }
                Err(e) => {
                    log::warn!("Attempt {}/3: Failed to write temporal backup {}: {}", attempt, temp_path, e);
                    // Try to remove the partially created temp file; if it fails, queue for later
                    if let Err(remove_err) = fs::remove_file(&temp_path) {
                        log::debug!("Queueing temp file for deferred removal ({}): {}", temp_path, remove_err);
                        self.queue_file_for_removal(temp_path.clone());
                    }
                    last_err = Some(e);
                    // Small backoff before retrying
                    std::thread::sleep(Duration::from_millis(10));
                }
            }
        }
        let temporal_backup_path = match chosen_temporal {
            Some(p) => p,
            None => {
                let err_msg = format!(
                    "Failed to write temporal backup after 3 attempts: {}",
                    last_err.map(|e| e.to_string()).unwrap_or_else(|| "unknown error".to_string())
                );
                log::error!("{}", err_msg);
                return Err(std::io::Error::other(err_msg));
            }
        };
        let write_duration = write_start.elapsed();

        log::debug!("Temporal file write completed in {}", Self::format_duration(write_duration));

        // Try to update main backup file atomically (copy temporal to main)
        let copy_start = Instant::now();
        if let Err(e) = fs::copy(&temporal_backup_path, &self.backup_path) {
            log::warn!("Could not update main backup file: {}. Temporal backup saved as {}", e, temporal_backup_path);
        } else {
            log::debug!("Main backup file updated in {}", Self::format_duration(copy_start.elapsed()));
        }

        let total_duration = backup_start.elapsed();
        let total_millis = total_duration.as_millis();

        // Log comprehensive backup completion info
        log::info!("Database backup saved to {} (temporal: {}) - {} items, {} bytes - took {} ({} milliseconds)",
            self.backup_path,
            temporal_backup_path,
            count,
            backup_json.len(),
            Self::format_duration(total_duration),
            total_millis
        );

        // Log detailed breakdown if backup took longer than 1 second
        if total_duration.as_secs() >= 1 {
            log::info!("Backup timing breakdown: lock={}, iteration={}, serialization={}, write={}",
                Self::format_duration(lock_duration),
                Self::format_duration(iteration_duration),
                Self::format_duration(serialization_duration),
                Self::format_duration(write_duration)
            );
        }

        // Clean up old temporal backups
        self.cleanup_old_temporal_backups();

        Ok(())
    }

    /// Create a fallback temporal backup when main DB is locked
    fn create_fallback_temporal_backup(&self, temporal_path: &str, backup_start: std::time::Instant) -> std::io::Result<()> {
        use std::time::{SystemTime, UNIX_EPOCH};
        log::info!("Creating fallback temporal backup due to database lock contention");

        // Create minimal backup with timestamp and status
        let fallback_data = serde_json::json!({
            "backup_type": "fallback_temporal",
            "timestamp": SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs(),
            "reason": "database_lock_timeout",
            "note": "Main database was locked - this is a minimal temporal backup"
        });

        fs::write(temporal_path, serde_json::to_string_pretty(&fallback_data)?)?;

        let total_duration = backup_start.elapsed();
        log::info!("Fallback temporal backup created at {} in {}",
            temporal_path, Self::format_duration(total_duration));

        Ok(())
    }

    /// Clean up old temporal backup files (keep last 10x backup periods)
    fn cleanup_old_temporal_backups(&self) {
        // First, try to delete any files that were queued for removal earlier
        self.process_failed_deletions();

        let backup_dir = match Path::new(&self.backup_path).parent() {
            Some(p) if !p.as_os_str().is_empty() => p,
            _ => Path::new("."),
        };
        let backup_name_str = Path::new(&self.backup_path)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("backup")
            .to_string();
        let prefix = format!("{}.", backup_name_str);

        // Find all temporal backup files
        if let Ok(entries) = fs::read_dir(backup_dir) {
            let mut temporal_files: Vec<(u64, String)> = Vec::new();

            for entry in entries.flatten() {
                if let Some(file_name) = entry.file_name().to_str() {
                    if let Some(suffix) = file_name.strip_prefix(&prefix) {
                        if let Ok(timestamp) = suffix.parse::<u64>() {
                            temporal_files.push((timestamp, entry.path().to_string_lossy().to_string()));
                        }
                    }
                }
            }

            // Sort by timestamp (newest first) and keep only recent files
            temporal_files.sort_by(|a, b| b.0.cmp(&a.0));

            // Keep only the last 10 temporal backups
            let keep_count = 10;
            if temporal_files.len() > keep_count {
                let mut removed = 0usize;
                let mut failed = 0usize;
                for (_, file_path) in temporal_files.iter().skip(keep_count) {
                    if let Err(e) = fs::remove_file(file_path) {
                        log::debug!("Could not remove old temporal backup {}: {}", file_path, e);
                        // If we still can't remove it (e.g., locked), queue it for later
                        self.queue_file_for_removal(file_path.clone());
                        failed += 1;
                    } else {
                        log::debug!("Cleaned up old temporal backup: {}", file_path);
                        removed += 1;
                    }
                }
                log::info!(
                    "Pruned temporal backups: kept={}, removed={}, failed_removals={}",
                    keep_count,
                    removed,
                    failed
                );
            }
        }
    }

    /// Queue a file path for deferred removal
    fn queue_file_for_removal(&self, path: String) {
        let mut q = self.failed_deletions.write().unwrap();
        q.push(path);
    }

    /// Try to remove any files that were previously queued for deletion
    fn process_failed_deletions(&self) {
        let mut q = self.failed_deletions.write().unwrap();
        if q.is_empty() { return; }
        let mut remaining: Vec<String> = Vec::new();
        for p in q.drain(..) {
            match fs::remove_file(&p) {
                Ok(_) => {
                    log::debug!("Removed previously queued temp file: {}", p);
                }
                Err(e) => {
                    log::debug!("Deferred removal still failing for {}: {}", p, e);
                    remaining.push(p);
                }
            }
        }
        *q = remaining;
    }

    /// Format duration in a human-readable way (e.g., "1m 3s 5μs" or "234234 milliseconds")
    fn format_duration(duration: Duration) -> String {
        let total_micros = duration.as_micros();
        let total_millis = duration.as_millis();
        let total_secs = duration.as_secs();

        if total_secs >= 60 {
            // Format as minutes, seconds, microseconds for long durations
            let minutes = total_secs / 60;
            let remaining_secs = total_secs % 60;
            let remaining_micros = (total_micros % 1_000_000) as u64;

            if remaining_micros > 0 {
                format!("{}m {}s {}μs", minutes, remaining_secs, remaining_micros)
            } else {
                format!("{}m {}s", minutes, remaining_secs)
            }
        } else if total_secs >= 1 {
            // Format as seconds and microseconds
            let remaining_micros = (total_micros % 1_000_000) as u64;
            if remaining_micros > 0 {
                format!("{}s {}μs", total_secs, remaining_micros)
            } else {
                format!("{}s", total_secs)
            }
        } else {
            // Format as milliseconds for sub-second durations
            format!("{} milliseconds", total_millis)
        }
    }

    /// Import database from JSON backup file or latest temporal backup
    fn import_from_backup(&self) -> std::io::Result<()> {
        // Check for the newest temporal backup file
        let best_backup_path = self.find_best_backup_file();

        if !Path::new(&best_backup_path).exists() {
            log::info!("No backup file found at {}, skipping restore", best_backup_path);
            return Ok(());
        }

        log::info!("Restoring database from: {}", best_backup_path);
        let backup_content = fs::read_to_string(&best_backup_path)?;

        // Try to parse as regular backup data
        let backup_data: std::result::Result<std::collections::HashMap<String, String>, _> =
            serde_json::from_str(&backup_content);

        match backup_data {
            Ok(data) => {
                let db = self.db.write().unwrap();
                let mut restored_count = 0;

                for (key, value) in data {
                    // Skip fallback temporal backup metadata
                    if key.starts_with("backup_type") || key.starts_with("timestamp") || key.starts_with("reason") || key.starts_with("note") {
                        continue;
                    }

                    if let Err(e) = db.insert(key.as_bytes(), value.as_bytes()) {
                        log::error!("Failed to restore key '{}': {}", key, e);
                    } else {
                        restored_count += 1;
                    }
                }

                if let Err(e) = db.flush() {
                    log::error!("Failed to flush database after restore: {}", e);
                }

                log::info!("Database restored from backup: {} ({} items)", best_backup_path, restored_count);
            }
            Err(_) => {
                // Check if this is a fallback temporal backup
                if let Ok(fallback_data) = serde_json::from_str::<serde_json::Value>(&backup_content) {
                    if fallback_data.get("backup_type") == Some(&serde_json::Value::String("fallback_temporal".to_string())) {
                        log::warn!("Found fallback temporal backup (database was locked during backup). No data restored.");
                        return Ok(());
                    }
                }
                log::error!("Could not parse backup file as valid JSON data");
            }
        }

        Ok(())
    }

    /// Find the best backup file (main or newest temporal)
    fn find_best_backup_file(&self) -> String {
        use std::time::SystemTime;
        let backup_dir = Path::new(&self.backup_path).parent().unwrap_or(Path::new("."));
        let backup_name = Path::new(&self.backup_path).file_name().unwrap_or(std::ffi::OsStr::new("backup"));

        let main_backup_time = fs::metadata(&self.backup_path)
            .and_then(|m| m.modified())
            .unwrap_or(SystemTime::UNIX_EPOCH);

        let mut newest_temporal: Option<(SystemTime, String)> = None;

        // Look for temporal backup files
        if let Ok(entries) = fs::read_dir(backup_dir) {
            for entry in entries.flatten() {
                if let Some(file_name) = entry.file_name().to_str() {
                    if file_name.starts_with(backup_name.to_str().unwrap_or("")) && file_name.contains('.') && file_name != self.backup_path {
                        if let Ok(metadata) = entry.metadata() {
                            if let Ok(modified) = metadata.modified() {
if newest_temporal.as_ref().is_none_or(|(time, _)| modified > *time) {
                                    newest_temporal = Some((modified, entry.path().to_string_lossy().to_string()));
                                }
                            }
                        }
                    }
                }
            }
        }

        // Use newest temporal backup if it's newer than main backup
        if let Some((temporal_time, temporal_path)) = newest_temporal {
            if temporal_time > main_backup_time {
                log::info!("Using newest temporal backup (more recent than main backup): {}", temporal_path);
                return temporal_path;
            }
        }

        self.backup_path.clone()
    }

    /// Initialize default database entries
    fn initialize_defaults(&self) -> std::io::Result<()> {
        let db = self.db.write().unwrap();

        // Add some default configurations if they don't exist
        let default_layout = CustomLayout { positions: HashMap::new() };
        if db.get("custom_periodic").unwrap_or(None).is_none() {
            let serialized = serde_json::to_vec(&default_layout)
                .map_err(std::io::Error::other)?;
            db.insert("custom_periodic", serialized)
                .map_err(std::io::Error::other)?;
        }

        if let Err(e) = db.flush() {
            log::error!("Failed to flush database after initializing defaults: {}", e);
        }

        log::info!("Database initialized with defaults");
        Ok(())
    }

    /// Export database to PostgreSQL-compatible SQL dump
    fn export_to_postgresql(&self, output_path: &str, include_drops: bool) -> std::io::Result<()> {
        let db = self.db.read().unwrap();
        let mut file = std::fs::File::create(output_path)?;

        // Write SQL header
        writeln!(file, "-- PostgreSQL dump generated by Periodic Backend")?;
        writeln!(file, "-- Generated on: {}", Utc::now().to_rfc3339())?;
        writeln!(file)?;
        writeln!(file, "SET statement_timeout = 0;")?;
        writeln!(file, "SET lock_timeout = 0;")?;
        writeln!(file, "SET client_encoding = 'UTF8';")?;
        writeln!(file, "SET standard_conforming_strings = on;")?;
        writeln!(file)?;

        // Organize data by table type
        let mut tables: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();

        for result in db.iter() {
            match result {
                Ok((key, value)) => {
                    let key_str = String::from_utf8(key.to_vec()).unwrap_or_default();
                    let value_str = String::from_utf8(value.to_vec()).unwrap_or_default();

                    // Determine table name from key prefix
                    let table_name = if key_str.starts_with("versioned_custom_names_") {
                        "versioned_custom_names"
                    } else if key_str.starts_with("versioned_custom_zones_") {
                        "versioned_custom_zones"
                    } else if key_str.starts_with("versioned_custom_orders_") {
                        "versioned_custom_orders"
                    } else if key_str.starts_with("versioned_custom_servers_") {
                        "versioned_custom_servers"
                    } else if key_str.starts_with("versioned_custom_clients_") {
                        "versioned_custom_clients"
                    } else if key_str.starts_with("versioned_custom_reservations_") {
                        "versioned_custom_reservations"
                    } else if key_str.starts_with("versioned_custom_kitchen_") {
                        "versioned_custom_kitchen"
                    } else if key_str.starts_with("versioned_custom_periodic") {
                        "versioned_custom_periodic"
                    } else if key_str.starts_with("custom_names_") {
                        "custom_names"
                    } else if key_str.starts_with("custom_zones_") {
                        "custom_zones"
                    } else if key_str.starts_with("custom_orders_") {
                        "custom_orders"
                    } else if key_str.starts_with("custom_servers_") {
                        "custom_servers"
                    } else if key_str.starts_with("custom_clients_") {
                        "custom_clients"
                    } else if key_str.starts_with("custom_reservations_") {
                        "custom_reservations"
                    } else if key_str.starts_with("custom_kitchen_") {
                        "custom_kitchen"
                    } else if key_str == "custom_periodic" {
                        "custom_periodic"
                    } else if key_str.starts_with("runtime_reservation_") {
                        "runtime_reservations"
                    } else {
                        "misc_data"
                    };

                    tables.entry(table_name.to_string()).or_default().push((key_str, value_str));
                }
                Err(e) => {
                    log::error!("Error reading key during PostgreSQL export: {}", e);
                }
            }
        }

        // Generate SQL for each table
        for (table_name, records) in tables {
            if include_drops {
                writeln!(file, "DROP TABLE IF EXISTS {} CASCADE;", table_name)?;
            }

            // Create table with appropriate schema
            if table_name.starts_with("versioned_") {
                // Versioned tables have enhanced schema with metadata
                writeln!(file, "CREATE TABLE IF NOT EXISTS {} (", table_name)?;
                writeln!(file, "    id SERIAL PRIMARY KEY,")?;
                writeln!(file, "    key_name VARCHAR(255) UNIQUE NOT NULL,")?;
                writeln!(file, "    value_data TEXT NOT NULL,")?;
                writeln!(file, "    version_number BIGINT NOT NULL,")?;
                writeln!(file, "    updated_at BIGINT NOT NULL,")?;
                writeln!(file, "    created_at_epoch BIGINT NOT NULL,")?;
                writeln!(file, "    checksum VARCHAR(64),")?;
                writeln!(file, "    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP")?;
                writeln!(file, ");")?;
                writeln!(file, "CREATE INDEX IF NOT EXISTS idx_{}_version ON {} (version_number);", table_name, table_name)?;
                writeln!(file, "CREATE INDEX IF NOT EXISTS idx_{}_updated ON {} (updated_at);", table_name, table_name)?;
            } else {
                // Legacy tables use simple schema
                writeln!(file, "CREATE TABLE IF NOT EXISTS {} (", table_name)?;
                writeln!(file, "    id SERIAL PRIMARY KEY,")?;
                writeln!(file, "    key_name VARCHAR(255) UNIQUE NOT NULL,")?;
                writeln!(file, "    value_data TEXT NOT NULL,")?;
                writeln!(file, "    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP")?;
                writeln!(file, ");")?;
            }
            writeln!(file)?;

            // Insert data
            if !records.is_empty() {
                writeln!(file, "-- Data for table {}", table_name)?;
                for (key, value) in records {
                    let escaped_key = key.replace("'", "''");
                    let escaped_value = value.replace("'", "''");

                    if table_name.starts_with("versioned_") {
                        // Parse versioned data to extract metadata
                        if let Ok(versioned_data) = serde_json::from_str::<serde_json::Value>(&value) {
                            let data_json = versioned_data["data"].to_string();
                            let version = versioned_data["version"].as_u64().unwrap_or(0);
                            let updated_at = versioned_data["updated_at"].as_u64().unwrap_or(0);
                            let created_at = versioned_data["created_at"].as_u64().unwrap_or(0);

                            // Calculate checksum for the data portion
                            let mut hasher = Sha256::new();
                            hasher.update(data_json.as_bytes());
                            let checksum = format!("{:x}", hasher.finalize());

                            let escaped_data = data_json.replace("'", "''");

                            writeln!(file, "INSERT INTO {} (key_name, value_data, version_number, updated_at, created_at_epoch, checksum) VALUES ('{}', '{}', {}, {}, {}, '{}') ON CONFLICT (key_name) DO UPDATE SET value_data = EXCLUDED.value_data, version_number = EXCLUDED.version_number, updated_at = EXCLUDED.updated_at, checksum = EXCLUDED.checksum;",
                                    table_name, escaped_key, escaped_data, version, updated_at, created_at, checksum)?;
                        } else {
                            // Fallback for malformed versioned data
                            writeln!(file, "INSERT INTO {} (key_name, value_data, version_number, updated_at, created_at_epoch) VALUES ('{}', '{}', 0, 0, 0) ON CONFLICT (key_name) DO UPDATE SET value_data = EXCLUDED.value_data;",
                                    table_name, escaped_key, escaped_value)?;
                        }
                    } else {
                        // Regular non-versioned data
                        writeln!(file, "INSERT INTO {} (key_name, value_data) VALUES ('{}', '{}') ON CONFLICT (key_name) DO UPDATE SET value_data = EXCLUDED.value_data;",
                                table_name, escaped_key, escaped_value)?;
                    }
                }
                writeln!(file)?;
            }
        }

        writeln!(file, "-- End of dump")?;
        log::info!("PostgreSQL export completed: {}", output_path);
        Ok(())
    }

    /// Import PostgreSQL SQL dump into sled database
    fn import_from_postgresql(&self, input_path: &str, clear_existing: bool) -> std::io::Result<()> {
        if !Path::new(input_path).exists() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("SQL file not found: {}", input_path)
            ));
        }

        let db = self.db.write().unwrap();

        if clear_existing {
            log::info!("Clearing existing database before import...");
            db.clear().map_err(std::io::Error::other)?;
        }

        let file = std::fs::File::open(input_path)?;
        let reader = BufReader::new(file);

        // Regex patterns for parsing INSERT statements
        let insert_regex = Regex::new(r"INSERT INTO\s+(\w+)\s*\([^)]*\)\s*VALUES\s*\('([^']*(?:''[^']*)*)',\s*'([^']*(?:''[^']*)*)'\)").unwrap();

        let mut imported_count = 0;

        for line in reader.lines() {
            let line = line?;
            let line = line.trim();

            // Skip comments and empty lines
            if line.starts_with("--") || line.is_empty() {
                continue;
            }

            // Parse INSERT statements
            if let Some(captures) = insert_regex.captures(line) {
                let table_name = captures.get(1).unwrap().as_str();
                let key_name = captures.get(2).unwrap().as_str().replace("''", "'");
                let value_data = captures.get(3).unwrap().as_str().replace("''", "'");

                // Store in sled database
                let sled_key = match table_name {
                    "custom_periodic" => key_name.to_string(),
                    _ => {
                        // For prefixed tables, reconstruct the original key
                        if key_name.starts_with(&format!("{}_", table_name.trim_end_matches('s'))) {
                            key_name.to_string()
                        } else {
                            format!("{}_{}", table_name.trim_end_matches('s'), key_name)
                        }
                    }
                };

                if let Err(e) = db.insert(sled_key.as_bytes(), value_data.as_bytes()) {
                    log::error!("Failed to import key '{}': {}", sled_key, e);
                } else {
                    imported_count += 1;
                }
            }
        }

        if let Err(e) = db.flush() {
            log::error!("Failed to flush database after PostgreSQL import: {}", e);
        }

        log::info!("PostgreSQL import completed: {} records imported from {}", imported_count, input_path);
        Ok(())
    }

    /// Get current timestamp in milliseconds since epoch
    fn current_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }

    /// Calculate checksum for data
    fn calculate_checksum<T: serde::Serialize>(data: &T) -> String {
        let json = serde_json::to_string(data).unwrap_or_default();
        let mut hasher = Sha256::new();
        hasher.update(json.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Get versioned data from database
    fn get_versioned_data<T>(&self, key: &str) -> Result<Option<VersionedData<T>>, std::io::Error>
    where
        T: serde::de::DeserializeOwned + serde::Serialize + Clone,
    {
        let versioned_key = format!("versioned_{}", key);

        // First, check if versioned data exists under a read lock
        {
            let db = self.db.read().unwrap();
            match db.get(&versioned_key) {
                Ok(Some(value)) => {
                    let versioned: VersionedData<T> = serde_json::from_slice(&value)
                        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
                    return Ok(Some(versioned));
                }
                Ok(None) => {
                    // fall through to migration path below (after dropping read lock)
                }
                Err(e) => return Err(std::io::Error::other(e)),
            }
            // Also check for legacy key presence while holding the read lock
            if let Ok(Some(old_value)) = db.get(key) {
                // Deserialize outside the lock scope after cloning the bytes
                let old_bytes: Vec<u8> = old_value.to_vec();
                drop(db);
                let data: T = serde_json::from_slice(&old_bytes)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
                // Perform migration with a write (no read lock held now)
                let versioned = self.set_versioned_data(key, &data, None, None)?;
                return Ok(Some(versioned));
            }
        }

        Ok(None)
    }

    /// Set versioned data in database
    fn set_versioned_data<T>(
        &self,
        key: &str,
        data: &T,
        base_version: Option<u64>,
        _updated_at_client: Option<u64>,
    ) -> Result<VersionedData<T>, std::io::Error>
    where
        T: serde::Serialize + serde::de::DeserializeOwned + Clone,
    {
        let db = self.db.write().unwrap();
        let versioned_key = format!("versioned_{}", key);
        let timestamp = Self::current_timestamp();

        // Check for conflicts if base_version is provided
        let (new_version, created_at) = if let Some(base_ver) = base_version {
            if let Ok(Some(existing_value)) = db.get(&versioned_key) {
                let existing: VersionedData<T> = serde_json::from_slice(&existing_value)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

                if existing.version != base_ver {
                    return Err(std::io::Error::other(
                        format!("Version conflict: base={}, current={}", base_ver, existing.version)
                    ));
                }
                (existing.version + 1, existing.created_at)
            } else {
                (1, timestamp)
            }
        } else {
            // No base version check - increment existing or start at 1
            if let Ok(Some(existing_value)) = db.get(&versioned_key) {
                let existing: VersionedData<T> = serde_json::from_slice(&existing_value)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
                (existing.version + 1, existing.created_at)
            } else {
                (1, timestamp)
            }
        };

        let versioned = VersionedData {
            data: data.clone(),
            version: new_version,
            updated_at: timestamp,
            created_at,
        };

        let serialized = serde_json::to_vec(&versioned)
            .map_err(std::io::Error::other)?;

        db.insert(&versioned_key, serialized)
            .map_err(std::io::Error::other)?;

        // Also update the legacy key for backward compatibility
        let legacy_data = serde_json::to_vec(&data)
            .map_err(std::io::Error::other)?;
        db.insert(key, legacy_data)
            .map_err(std::io::Error::other)?;

        Ok(versioned)
    }

    /// Generate a cursor from timestamp and sequence
    fn generate_cursor(timestamp: u64, sequence: u64) -> String {
        let cursor_data = format!("{}:{}", timestamp, sequence);
        general_purpose::STANDARD.encode(cursor_data.as_bytes())
    }

    /// Parse cursor to get timestamp and sequence
    fn parse_cursor(cursor: &str) -> Option<(u64, u64)> {
        if let Ok(decoded) = general_purpose::STANDARD.decode(cursor) {
            if let Ok(cursor_str) = String::from_utf8(decoded) {
                let parts: Vec<&str> = cursor_str.split(':').collect();
                if parts.len() == 2 {
                    if let (Ok(ts), Ok(seq)) = (parts[0].parse::<u64>(), parts[1].parse::<u64>()) {
                        return Some((ts, seq));
                    }
                }
            }
        }
        None
    }

    /// Emit a runtime event to the event stream
    fn emit_event(&self, event_type: &str, payload: Value) -> std::io::Result<String> {
        let timestamp = Self::current_timestamp();
        let sequence = self.event_sequence.fetch_add(1, Ordering::SeqCst);
        let event_id = Uuid::new_v4().to_string();
        let cursor = Self::generate_cursor(timestamp, sequence);

        let event = RuntimeEvent {
            id: event_id.clone(),
            ts: timestamp,
            event_type: event_type.to_string(),
            payload,
            cursor: cursor.clone(),
        };

        // Store event in database with cursor as key for ordering
        let db = self.db.write().unwrap();
        let event_key = format!("runtime_event_{}_{}", timestamp, sequence);
        let serialized = serde_json::to_vec(&event)
            .map_err(std::io::Error::other)?;

        db.insert(&event_key, serialized)
            .map_err(std::io::Error::other)?;

        Ok(event_id)
    }

    /// Get runtime events since cursor
    fn get_runtime_events(
        &self,
        since_cursor: Option<&str>,
        limit: usize,
        types_filter: Option<Vec<String>>,
    ) -> std::io::Result<RuntimeEventsResponse> {
        let db = self.db.read().unwrap();
        let mut events = Vec::new();

        // Parse the since cursor if provided
        let (since_ts, since_seq) = if let Some(cursor) = since_cursor {
            Self::parse_cursor(cursor).unwrap_or((0, 0))
        } else {
            (0, 0)
        };

        // Scan events in chronological order
        for result in db.scan_prefix("runtime_event_") {
            let (_key, value) = result
                .map_err(std::io::Error::other)?;

            if let Ok(event) = serde_json::from_slice::<RuntimeEvent>(&value) {
                // Check if event is after the cursor
                if event.ts > since_ts || (event.ts == since_ts &&
                    Self::parse_cursor(&event.cursor).map(|(_, seq)| seq > since_seq).unwrap_or(false)) {

                    // Apply type filter if specified
                    if let Some(ref types) = types_filter {
                        if !types.contains(&event.event_type) {
                            continue;
                        }
                    }

                    events.push(event);

                    if events.len() >= limit {
                        break;
                    }
                }
            }
        }

        // Sort by timestamp and sequence
        events.sort_by(|a, b| {
            let a_cursor = Self::parse_cursor(&a.cursor).unwrap_or((0, 0));
            let b_cursor = Self::parse_cursor(&b.cursor).unwrap_or((0, 0));
            a_cursor.cmp(&b_cursor)
        });

        let has_more = events.len() == limit;
        let next_cursor = if let Some(last_event) = events.last() {
            // Generate cursor pointing beyond the last event
            if let Some((ts, seq)) = Self::parse_cursor(&last_event.cursor) {
                Self::generate_cursor(ts, seq + 1)
            } else {
                last_event.cursor.clone()
            }
        } else {
            // No events, return current cursor or generate new one
            since_cursor.unwrap_or(&Self::generate_cursor(Self::current_timestamp(), 0)).to_string()
        };

        Ok(RuntimeEventsResponse {
            items: events,
            next_cursor,
            has_more,
        })
    }
}

async fn get_custom_periodic(data: web::Data<AppState>) -> Result<HttpResponse> {
    match data.get_versioned_data::<CustomLayout>("custom_periodic") {
        Ok(Some(versioned)) => {
            let response = serde_json::json!({
                "positions": versioned.data.positions,
                "version": versioned.version,
                "updatedAt": versioned.updated_at
            });
            Ok(HttpResponse::Ok().json(response))
        }
        Ok(None) => {
            let default_layout = CustomLayout { positions: HashMap::new() };
            let response = serde_json::json!({
                "positions": default_layout.positions,
                "version": 0,
                "updatedAt": 0
            });
            Ok(HttpResponse::Ok().json(response))
        }
        Err(e) => {
            log::error!("Error getting periodic layout: {}", e);
            Ok(HttpResponse::InternalServerError().json(
                serde_json::json!({"error": "Internal server error"})
            ))
        }
    }
}

async fn save_custom_periodic(request: web::Json<serde_json::Value>, data: web::Data<AppState>) -> Result<HttpResponse> {
    let json_value = request.into_inner();

    // Handle both {positions: ...} and CustomLayout formats
    let layout = if let Some(positions) = json_value.get("positions") {
        CustomLayout {
            positions: serde_json::from_value(positions.clone())
                .unwrap_or_else(|_| HashMap::new())
        }
    } else {
        serde_json::from_value(json_value)
            .unwrap_or_else(|_| CustomLayout { positions: HashMap::new() })
    };

    match data.set_versioned_data("custom_periodic", &layout, None, None) {
        Ok(versioned) => {
            log::info!("Custom periodic layout saved successfully");
            Ok(HttpResponse::Ok().json(UpsertResponse {
                version: versioned.version,
                updated_at: versioned.updated_at,
            }))
        }
        Err(e) => {
            log::error!("Failed to save layout: {}", e);
            Ok(HttpResponse::InternalServerError().json(
                serde_json::json!({"error": "Failed to save"})
            ))
        }
    }
}

// Endpoint to list all saved names configurations
async fn list_custom_names(data: web::Data<AppState>) -> Result<HttpResponse> {
    let mut items: Vec<ItemMetadata> = vec![];
    // First collect names while holding a read lock
    let mut names: Vec<String> = vec![];
    {
        let db = data.db.read().unwrap();
        for result in db.iter() {
            let (key, _value) = result.map_err(|e| {
                log::error!("Iteration error: {}", e);
                actix_web::error::ErrorInternalServerError("DB iteration error")
            })?;
            let kstr = String::from_utf8(key.to_vec()).unwrap_or_default();
            if kstr.starts_with("custom_names_") {
                let name = kstr.trim_start_matches("custom_names_").to_string();
                names.push(name);
            }
        }
    }
    // Now, for each name, query versioned data without holding the DB lock
    for name in names {
        let full_key = format!("custom_names_{}", name);
        if let Ok(Some(versioned)) = data.get_versioned_data::<CustomNamesConfig>(&full_key) {
            let checksum = Some(AppState::calculate_checksum(&versioned.data));
            items.push(ItemMetadata {
                name,
                updated_at: versioned.updated_at,
                version: versioned.version,
                checksum,
            });
        } else {
            items.push(ItemMetadata {
                name,
                updated_at: 0,
                version: 0,
                checksum: None,
            });
        }
    }
    Ok(HttpResponse::Ok().json(ListResponse { items }))
}

// Save a named configuration of custom names (legacy POST endpoint)
async fn save_custom_names(cfg: web::Json<(String, CustomNamesConfig)>, data: web::Data<AppState>) -> Result<HttpResponse> {
    let (name, config) = cfg.into_inner();
    let key = format!("custom_names_{}", name);

    match data.set_versioned_data(&key, &config, None, None) {
        Ok(versioned) => {
            log::info!("Custom names config saved: {}", name);
            Ok(HttpResponse::Ok().json(UpsertResponse {
                version: versioned.version,
                updated_at: versioned.updated_at,
            }))
        }
        Err(e) => {
            log::error!("Failed to save names config: {}", e);
            Ok(HttpResponse::InternalServerError().json(
                serde_json::json!({"error": "Failed to save names"})
            ))
        }
    }
}

// PUT endpoint for conditional upserts of custom names
async fn upsert_custom_names(name: web::Path<String>, req: web::Json<UpsertRequest<CustomNamesConfig>>, data: web::Data<AppState>) -> Result<HttpResponse> {
    let name_str = name.into_inner();
    let key = format!("custom_names_{}", name_str);
    let request = req.into_inner();

    match data.set_versioned_data(&key, &request.data, request.base_version, request.updated_at_client) {
        Ok(versioned) => {
            log::info!("Custom names config upserted: {}", name_str);
            Ok(HttpResponse::Ok().json(UpsertResponse {
                version: versioned.version,
                updated_at: versioned.updated_at,
            }))
        }
        Err(e) if e.to_string().contains("Version conflict") => {
            // Parse the current version from error or get it from database
            if let Ok(Some(current)) = data.get_versioned_data::<CustomNamesConfig>(&key) {
                Ok(HttpResponse::Conflict().json(ConflictResponse {
                    server_version: current.version,
                    server_updated_at: current.updated_at,
                }))
            } else {
                Ok(HttpResponse::Conflict().json(serde_json::json!({
                    "error": "Version conflict"
                })))
            }
        }
        Err(e) => {
            log::error!("Failed to upsert names config: {}", e);
            Ok(HttpResponse::InternalServerError().json(
                serde_json::json!({"error": "Failed to save names"})
            ))
        }
    }
}

// PUT endpoint for conditional upserts of custom zones
async fn upsert_custom_zones(name: web::Path<String>, req: web::Json<UpsertRequest<CustomZoneConfig>>, data: web::Data<AppState>) -> Result<HttpResponse> {
    let name_str = name.into_inner();
    let key = format!("custom_zones_{}", name_str);
    let request = req.into_inner();

    match data.set_versioned_data(&key, &request.data, request.base_version, request.updated_at_client) {
        Ok(versioned) => {
            log::info!("Custom zones config upserted: {}", name_str);
            Ok(HttpResponse::Ok().json(UpsertResponse {
                version: versioned.version,
                updated_at: versioned.updated_at,
            }))
        }
        Err(e) if e.to_string().contains("Version conflict") => {
            if let Ok(Some(current)) = data.get_versioned_data::<CustomZoneConfig>(&key) {
                Ok(HttpResponse::Conflict().json(ConflictResponse {
                    server_version: current.version,
                    server_updated_at: current.updated_at,
                }))
            } else {
                Ok(HttpResponse::Conflict().json(serde_json::json!({
                    "error": "Version conflict"
                })))
            }
        }
        Err(e) => {
            log::error!("Failed to upsert zones config: {}", e);
            Ok(HttpResponse::InternalServerError().json(
                serde_json::json!({"error": "Failed to save zones"})
            ))
        }
    }
}

// PUT endpoint for conditional upserts of custom orders
async fn upsert_custom_orders(name: web::Path<String>, req: web::Json<UpsertRequest<CustomOrderConfig>>, data: web::Data<AppState>) -> Result<HttpResponse> {
    let name_str = name.into_inner();
    let key = format!("custom_orders_{}", name_str);
    let request = req.into_inner();

    match data.set_versioned_data(&key, &request.data, request.base_version, request.updated_at_client) {
        Ok(versioned) => {
            log::info!("Custom orders config upserted: {}", name_str);
            Ok(HttpResponse::Ok().json(UpsertResponse {
                version: versioned.version,
                updated_at: versioned.updated_at,
            }))
        }
        Err(e) if e.to_string().contains("Version conflict") => {
            if let Ok(Some(current)) = data.get_versioned_data::<CustomOrderConfig>(&key) {
                Ok(HttpResponse::Conflict().json(ConflictResponse {
                    server_version: current.version,
                    server_updated_at: current.updated_at,
                }))
            } else {
                Ok(HttpResponse::Conflict().json(serde_json::json!({
                    "error": "Version conflict"
                })))
            }
        }
        Err(e) => {
            log::error!("Failed to upsert orders config: {}", e);
            Ok(HttpResponse::InternalServerError().json(
                serde_json::json!({"error": "Failed to save orders"})
            ))
        }
    }
}

// PUT endpoint for conditional upserts of custom servers
async fn upsert_custom_servers(name: web::Path<String>, req: web::Json<UpsertRequest<CustomServerConfig>>, data: web::Data<AppState>) -> Result<HttpResponse> {
    let name_str = name.into_inner();
    let key = format!("custom_servers_{}", name_str);
    let request = req.into_inner();

    match data.set_versioned_data(&key, &request.data, request.base_version, request.updated_at_client) {
        Ok(versioned) => {
            log::info!("Custom servers config upserted: {}", name_str);
            Ok(HttpResponse::Ok().json(UpsertResponse {
                version: versioned.version,
                updated_at: versioned.updated_at,
            }))
        }
        Err(e) if e.to_string().contains("Version conflict") => {
            if let Ok(Some(current)) = data.get_versioned_data::<CustomServerConfig>(&key) {
                Ok(HttpResponse::Conflict().json(ConflictResponse {
                    server_version: current.version,
                    server_updated_at: current.updated_at,
                }))
            } else {
                Ok(HttpResponse::Conflict().json(serde_json::json!({
                    "error": "Version conflict"
                })))
            }
        }
        Err(e) => {
            log::error!("Failed to upsert servers config: {}", e);
            Ok(HttpResponse::InternalServerError().json(
                serde_json::json!({"error": "Failed to save servers"})
            ))
        }
    }
}

// PUT endpoint for conditional upserts of custom clients
async fn upsert_custom_clients(name: web::Path<String>, req: web::Json<UpsertRequest<CustomClientConfig>>, data: web::Data<AppState>) -> Result<HttpResponse> {
    let name_str = name.into_inner();
    let key = format!("custom_clients_{}", name_str);
    let request = req.into_inner();

    match data.set_versioned_data(&key, &request.data, request.base_version, request.updated_at_client) {
        Ok(versioned) => {
            log::info!("Custom clients config upserted: {}", name_str);
            Ok(HttpResponse::Ok().json(UpsertResponse {
                version: versioned.version,
                updated_at: versioned.updated_at,
            }))
        }
        Err(e) if e.to_string().contains("Version conflict") => {
            if let Ok(Some(current)) = data.get_versioned_data::<CustomClientConfig>(&key) {
                Ok(HttpResponse::Conflict().json(ConflictResponse {
                    server_version: current.version,
                    server_updated_at: current.updated_at,
                }))
            } else {
                Ok(HttpResponse::Conflict().json(serde_json::json!({
                    "error": "Version conflict"
                })))
            }
        }
        Err(e) => {
            log::error!("Failed to upsert clients config: {}", e);
            Ok(HttpResponse::InternalServerError().json(
                serde_json::json!({"error": "Failed to save clients"})
            ))
        }
    }
}

// PUT endpoint for conditional upserts of custom reservations
async fn upsert_custom_reservations(name: web::Path<String>, req: web::Json<UpsertRequest<CustomReservConfig>>, data: web::Data<AppState>) -> Result<HttpResponse> {
    let name_str = name.into_inner();
    let key = format!("custom_reservations_{}", name_str);
    let request = req.into_inner();

    match data.set_versioned_data(&key, &request.data, request.base_version, request.updated_at_client) {
        Ok(versioned) => {
            log::info!("Custom reservations config upserted: {}", name_str);
            Ok(HttpResponse::Ok().json(UpsertResponse {
                version: versioned.version,
                updated_at: versioned.updated_at,
            }))
        }
        Err(e) if e.to_string().contains("Version conflict") => {
            if let Ok(Some(current)) = data.get_versioned_data::<CustomReservConfig>(&key) {
                Ok(HttpResponse::Conflict().json(ConflictResponse {
                    server_version: current.version,
                    server_updated_at: current.updated_at,
                }))
            } else {
                Ok(HttpResponse::Conflict().json(serde_json::json!({
                    "error": "Version conflict"
                })))
            }
        }
        Err(e) => {
            log::error!("Failed to upsert reservations config: {}", e);
            Ok(HttpResponse::InternalServerError().json(
                serde_json::json!({"error": "Failed to save reservations"})
            ))
        }
    }
}

// Get a specific names configuration
async fn get_custom_names(name: web::Path<String>, data: web::Data<AppState>) -> Result<HttpResponse> {
    let name_str = name.into_inner();
    let key = format!("custom_names_{}", name_str);

    match data.get_versioned_data::<CustomNamesConfig>(&key) {
        Ok(Some(versioned)) => {
            let response = ItemResponse {
                name: name_str,
                data: versioned.data,
                updated_at: versioned.updated_at,
                version: versioned.version,
            };
            Ok(HttpResponse::Ok().json(response))
        }
        Ok(None) => Ok(HttpResponse::NotFound().json(
            serde_json::json!({"error": "not found"})
        )),
        Err(e) => {
            log::error!("Error getting versioned data: {}", e);
            Ok(HttpResponse::InternalServerError().json(
                serde_json::json!({"error": "Internal server error"})
            ))
        }
    }
}

// Delete a specific names configuration
async fn delete_custom_names(name: web::Path<String>, data: web::Data<AppState>) -> Result<HttpResponse> {
    let name_str = name.into_inner();
    let key = format!("custom_names_{}", name_str);
    let versioned_key = format!("versioned_{}", key);

    let db = data.db.write().unwrap();

    // Remove both versioned and legacy keys
    let mut deleted = false;
    if db.remove(&versioned_key).is_ok() {
        deleted = true;
    }
    if db.remove(&key).is_ok() {
        deleted = true;
    }

    if deleted {
        log::info!("Custom names config deleted: {}", name_str);
        Ok(HttpResponse::Ok().json(serde_json::json!({"status": "deleted"})))
    } else {
        Ok(HttpResponse::NotFound().json(serde_json::json!({"error": "not found"})))
    }
}

// ---- Zone configuration endpoints ----
// List all saved zone configuration names
async fn list_custom_zones(data: web::Data<AppState>) -> Result<HttpResponse> {
    let mut names: Vec<String> = vec![];
    for result in data.db.read().unwrap().iter() {
        let (key, _value) = result.map_err(|e| {
            log::error!("Iteration error: {}", e);
            actix_web::error::ErrorInternalServerError("DB iteration error")
        })?;
        let kstr = String::from_utf8(key.to_vec()).unwrap_or_default();
        if kstr.starts_with("custom_zones_") {
            names.push(kstr.trim_start_matches("custom_zones_").to_string());
        }
    }
    Ok(HttpResponse::Ok().json(serde_json::json!({"configs": names})))
}

// Save a named zone configuration
async fn save_custom_zones(cfg: web::Json<(String, CustomZoneConfig)>, data: web::Data<AppState>) -> Result<HttpResponse> {
    let (name, config) = cfg.into_inner();
    let key = format!("custom_zones_{}", name);
    match serde_json::to_vec(&config) {
        Ok(serialized) => {
            match data.db.write().unwrap().insert(key, serialized) {
                Ok(_) => {
                    log::info!("Custom zones config saved: {}", name);
                    Ok(HttpResponse::Ok().json(serde_json::json!({"status": "saved"})))
                }
                Err(e) => {
                    log::error!("Failed to save zones config: {}", e);
                    Ok(HttpResponse::InternalServerError().json(
                        serde_json::json!({"error": "Failed to save zones"})
                    ))
                }
            }
        }
        Err(e) => {
            log::error!("Failed to serialize zones config: {}", e);
            Ok(HttpResponse::BadRequest().json(
                serde_json::json!({"error": "Invalid data"})
            ))
        }
    }
}

// Get a specific zone configuration
async fn get_custom_zones(name: web::Path<String>, data: web::Data<AppState>) -> Result<HttpResponse> {
    let key = format!("custom_zones_{}", name.into_inner());
    match data.db.read().unwrap().get(key) {
        Ok(Some(value)) => {
            let config: CustomZoneConfig = serde_json::from_slice(&value)
                .unwrap_or(CustomZoneConfig { zone_names: vec![], assignments: HashMap::new(), colors: HashMap::new(), enabled: HashMap::new() });
            Ok(HttpResponse::Ok().json(config))
        }
        _ => Ok(HttpResponse::NotFound().json(
            serde_json::json!({"error": "not found"})
        ))
    }
}

// Delete a specific zone configuration
async fn delete_custom_zones(name: web::Path<String>, data: web::Data<AppState>) -> Result<HttpResponse> {
    let key = format!("custom_zones_{}", name.into_inner());
    match data.db.write().unwrap().remove(key) {
        Ok(_) => Ok(HttpResponse::Ok().json(serde_json::json!({"status": "deleted"}))),
        Err(e) => {
            log::error!("Failed to delete zones config: {}", e);
            Ok(HttpResponse::InternalServerError().json(
                serde_json::json!({"error": "Failed to delete"})
            ))
        }
    }
}

// ---- Order configuration endpoints ----
// List all saved order configuration names
async fn list_custom_orders(data: web::Data<AppState>) -> Result<HttpResponse> {
    let mut names: Vec<String> = vec![];
    for result in data.db.read().unwrap().iter() {
        let (key, _value) = result.map_err(|e| {
            log::error!("Iteration error: {}", e);
            actix_web::error::ErrorInternalServerError("DB iteration error")
        })?;
        let kstr = String::from_utf8(key.to_vec()).unwrap_or_default();
        if kstr.starts_with("custom_orders_") {
            names.push(kstr.trim_start_matches("custom_orders_").to_string());
        }
    }
    Ok(HttpResponse::Ok().json(serde_json::json!({"configs": names})))
}

// Save a named order configuration
async fn save_custom_orders(cfg: web::Json<(String, CustomOrderConfig)>, data: web::Data<AppState>) -> Result<HttpResponse> {
    let (name, config) = cfg.into_inner();
    let key = format!("custom_orders_{}", name);
    match serde_json::to_vec(&config) {
        Ok(serialized) => {
            match data.db.write().unwrap().insert(key, serialized) {
                Ok(_) => {
                    log::info!("Custom orders config saved: {}", name);
                    Ok(HttpResponse::Ok().json(serde_json::json!({"status": "saved"})))
                }
                Err(e) => {
                    log::error!("Failed to save orders config: {}", e);
                    Ok(HttpResponse::InternalServerError().json(serde_json::json!({"error": "Failed to save orders"})))
                }
            }
        }
        Err(e) => {
            log::error!("Failed to serialize orders config: {}", e);
            Ok(HttpResponse::BadRequest().json(serde_json::json!({"error": "Invalid data"})))
        }
    }
}

// Get a specific order configuration
async fn get_custom_orders(name: web::Path<String>, data: web::Data<AppState>) -> Result<HttpResponse> {
    let key = format!("custom_orders_{}", name.into_inner());
    match data.db.read().unwrap().get(key) {
        Ok(Some(value)) => {
            let config: CustomOrderConfig = serde_json::from_slice(&value)
                .unwrap_or(CustomOrderConfig { data: serde_json::json!({}) });
            Ok(HttpResponse::Ok().json(config))
        }
        _ => Ok(HttpResponse::NotFound().json(serde_json::json!({"error": "not found"})))
    }
}

// Delete a specific order configuration
async fn delete_custom_orders(name: web::Path<String>, data: web::Data<AppState>) -> Result<HttpResponse> {
    let key = format!("custom_orders_{}", name.into_inner());
    match data.db.write().unwrap().remove(key) {
        Ok(_) => Ok(HttpResponse::Ok().json(serde_json::json!({"status": "deleted"}))),
        Err(e) => {
            log::error!("Failed to delete orders config: {}", e);
            Ok(HttpResponse::InternalServerError().json(serde_json::json!({"error": "Failed to delete"})))
        }
    }
}

// ---- Server, client, and reservation configuration endpoints ----

/// List all saved server configuration names
async fn list_custom_servers(data: web::Data<AppState>) -> Result<HttpResponse> {
    let mut names: Vec<String> = vec![];
    for result in data.db.read().unwrap().iter() {
        let (key, _value) = result.map_err(|e| {
            log::error!("Iteration error: {}", e);
            actix_web::error::ErrorInternalServerError("DB iteration error")
        })?;
        let kstr = String::from_utf8(key.to_vec()).unwrap_or_default();
        if kstr.starts_with("custom_servers_") {
            names.push(kstr.trim_start_matches("custom_servers_").to_string());
        }
    }
    Ok(HttpResponse::Ok().json(serde_json::json!({"configs": names})))
}

/// Save a named server configuration
async fn save_custom_servers(cfg: web::Json<(String, CustomServerConfig)>, data: web::Data<AppState>) -> Result<HttpResponse> {
    let (name, config) = cfg.into_inner();
    let key = format!("custom_servers_{}", name);
    match serde_json::to_vec(&config) {
        Ok(serialized) => {
            match data.db.write().unwrap().insert(key, serialized) {
                Ok(_) => {
                    log::info!("Custom servers config saved: {}", name);
                    Ok(HttpResponse::Ok().json(serde_json::json!({"status": "saved"})))
                }
                Err(e) => {
                    log::error!("Failed to save servers config: {}", e);
                    Ok(HttpResponse::InternalServerError().json(serde_json::json!({"error": "Failed to save servers"})))
                }
            }
        }
        Err(e) => {
            log::error!("Failed to serialize servers config: {}", e);
            Ok(HttpResponse::BadRequest().json(serde_json::json!({"error": "Invalid data"})))
        }
    }
}

/// Get a specific server configuration
async fn get_custom_servers(name: web::Path<String>, data: web::Data<AppState>) -> Result<HttpResponse> {
    let key = format!("custom_servers_{}", name.into_inner());
    match data.db.read().unwrap().get(key) {
        Ok(Some(value)) => {
            let config: CustomServerConfig = serde_json::from_slice(&value)
                .unwrap_or(CustomServerConfig { servers: vec![] });
            Ok(HttpResponse::Ok().json(config))
        }
        _ => Ok(HttpResponse::NotFound().json(serde_json::json!({"error": "not found"})))
    }
}

/// Delete a specific server configuration
async fn delete_custom_servers(name: web::Path<String>, data: web::Data<AppState>) -> Result<HttpResponse> {
    let key = format!("custom_servers_{}", name.into_inner());
    match data.db.write().unwrap().remove(key) {
        Ok(_) => Ok(HttpResponse::Ok().json(serde_json::json!({"status": "deleted"}))),
        Err(e) => {
            log::error!("Failed to delete servers config: {}", e);
            Ok(HttpResponse::InternalServerError().json(serde_json::json!({"error": "Failed to delete servers"})))
        }
    }
}

/// List all saved client configuration names
async fn list_custom_clients(data: web::Data<AppState>) -> Result<HttpResponse> {
    let mut names: Vec<String> = vec![];
    for result in data.db.read().unwrap().iter() {
        let (key, _value) = result.map_err(|e| {
            log::error!("Iteration error: {}", e);
            actix_web::error::ErrorInternalServerError("DB iteration error")
        })?;
        let kstr = String::from_utf8(key.to_vec()).unwrap_or_default();
        if kstr.starts_with("custom_clients_") {
            names.push(kstr.trim_start_matches("custom_clients_").to_string());
        }
    }
    Ok(HttpResponse::Ok().json(serde_json::json!({"configs": names})))
}

/// Save a named client configuration
async fn save_custom_clients(cfg: web::Json<(String, CustomClientConfig)>, data: web::Data<AppState>) -> Result<HttpResponse> {
    let (name, config) = cfg.into_inner();
    let key = format!("custom_clients_{}", name);
    match serde_json::to_vec(&config) {
        Ok(serialized) => {
            match data.db.write().unwrap().insert(key, serialized) {
                Ok(_) => {
                    log::info!("Custom clients config saved: {}", name);
                    Ok(HttpResponse::Ok().json(serde_json::json!({"status": "saved"})))
                }
                Err(e) => {
                    log::error!("Failed to save clients config: {}", e);
                    Ok(HttpResponse::InternalServerError().json(serde_json::json!({"error": "Failed to save clients"})))
                }
            }
        }
        Err(e) => {
            log::error!("Failed to serialize clients config: {}", e);
            Ok(HttpResponse::BadRequest().json(serde_json::json!({"error": "Invalid data"})))
        }
    }
}

/// Get a specific client configuration
async fn get_custom_clients(name: web::Path<String>, data: web::Data<AppState>) -> Result<HttpResponse> {
    let key = format!("custom_clients_{}", name.into_inner());
    match data.db.read().unwrap().get(key) {
        Ok(Some(value)) => {
            let config: CustomClientConfig = serde_json::from_slice(&value)
                .unwrap_or(CustomClientConfig { clients: vec![] });
            Ok(HttpResponse::Ok().json(config))
        }
        _ => Ok(HttpResponse::NotFound().json(serde_json::json!({"error": "not found"})))
    }
}

/// Delete a specific client configuration
async fn delete_custom_clients(name: web::Path<String>, data: web::Data<AppState>) -> Result<HttpResponse> {
    let key = format!("custom_clients_{}", name.into_inner());
    match data.db.write().unwrap().remove(key) {
        Ok(_) => Ok(HttpResponse::Ok().json(serde_json::json!({"status": "deleted"}))),
        Err(e) => {
            log::error!("Failed to delete clients config: {}", e);
            Ok(HttpResponse::InternalServerError().json(serde_json::json!({"error": "Failed to delete clients"})))
        }
    }
}

/// List all saved reservation configuration names
async fn list_custom_reservations(data: web::Data<AppState>) -> Result<HttpResponse> {
    let mut names: Vec<String> = vec![];
    for result in data.db.read().unwrap().iter() {
        let (key, _value) = result.map_err(|e| {
            log::error!("Iteration error: {}", e);
            actix_web::error::ErrorInternalServerError("DB iteration error")
        })?;
        let kstr = String::from_utf8(key.to_vec()).unwrap_or_default();
        if kstr.starts_with("custom_reservations_") {
            names.push(kstr.trim_start_matches("custom_reservations_").to_string());
        }
    }
    Ok(HttpResponse::Ok().json(serde_json::json!({"configs": names})))
}

/// Save a named reservation configuration
async fn save_custom_reservations(cfg: web::Json<(String, CustomReservConfig)>, data: web::Data<AppState>) -> Result<HttpResponse> {
    let (name, config) = cfg.into_inner();
    let key = format!("custom_reservations_{}", name);
    match serde_json::to_vec(&config) {
        Ok(serialized) => {
            match data.db.write().unwrap().insert(key, serialized) {
                Ok(_) => {
                    log::info!("Custom reservations config saved: {}", name);
                    Ok(HttpResponse::Ok().json(serde_json::json!({"status": "saved"})))
                }
                Err(e) => {
                    log::error!("Failed to save reservations config: {}", e);
                    Ok(HttpResponse::InternalServerError().json(serde_json::json!({"error": "Failed to save reservations"})))
                }
            }
        }
        Err(e) => {
            log::error!("Failed to serialize reservations config: {}", e);
            Ok(HttpResponse::BadRequest().json(serde_json::json!({"error": "Invalid data"})))
        }
    }
}

/// Get a specific reservation configuration
async fn get_custom_reservations(name: web::Path<String>, data: web::Data<AppState>) -> Result<HttpResponse> {
    let key = format!("custom_reservations_{}", name.into_inner());
    match data.db.read().unwrap().get(key) {
        Ok(Some(value)) => {
            let config: CustomReservConfig = serde_json::from_slice(&value)
                .unwrap_or(CustomReservConfig { data: Value::Null });
            Ok(HttpResponse::Ok().json(config))
        }
        _ => Ok(HttpResponse::NotFound().json(serde_json::json!({"error": "not found"})))
    }
}

/// Delete a specific reservation configuration
async fn delete_custom_reservations(name: web::Path<String>, data: web::Data<AppState>) -> Result<HttpResponse> {
    let key = format!("custom_reservations_{}", name.into_inner());
    match data.db.write().unwrap().remove(key) {
        Ok(_) => Ok(HttpResponse::Ok().json(serde_json::json!({"status": "deleted"}))),
        Err(e) => {
            log::error!("Failed to delete reservations config: {}", e);
            Ok(HttpResponse::InternalServerError().json(serde_json::json!({"error": "Failed to delete reservations"})))
        }
    }
}

// ---- Kitchen printer configuration endpoints ----

/// List all saved kitchen printer configuration names
async fn list_custom_kitchen(data: web::Data<AppState>) -> Result<HttpResponse> {
    let mut items: Vec<ItemMetadata> = vec![];
    // First collect names while holding a read lock
    let mut names: Vec<String> = vec![];
    {
        let db = data.db.read().unwrap();
        for result in db.iter() {
            let (key, _value) = result.map_err(|e| {
                log::error!("Iteration error: {}", e);
                actix_web::error::ErrorInternalServerError("DB iteration error")
            })?;
            let kstr = String::from_utf8(key.to_vec()).unwrap_or_default();
            if kstr.starts_with("custom_kitchen_") {
                let name = kstr.trim_start_matches("custom_kitchen_").to_string();
                names.push(name);
            }
        }
    }
    // Now, for each name, query versioned data without holding the DB lock
    for name in names {
        let full_key = format!("custom_kitchen_{}", name);
        if let Ok(Some(versioned)) = data.get_versioned_data::<CustomKitchenConfig>(&full_key) {
            let checksum = Some(AppState::calculate_checksum(&versioned.data));
            items.push(ItemMetadata {
                name,
                updated_at: versioned.updated_at,
                version: versioned.version,
                checksum,
            });
        } else {
            items.push(ItemMetadata {
                name,
                updated_at: 0,
                version: 0,
                checksum: None,
            });
        }
    }
    Ok(HttpResponse::Ok().json(ListResponse { items }))
}

/// Save a named kitchen configuration (legacy POST endpoint)
async fn save_custom_kitchen(cfg: web::Json<(String, CustomKitchenConfig)>, data: web::Data<AppState>) -> Result<HttpResponse> {
    let (name, config) = cfg.into_inner();
    let key = format!("custom_kitchen_{}", name);

    match data.set_versioned_data(&key, &config, None, None) {
        Ok(versioned) => {
            log::info!("Custom kitchen config saved: {}", name);
            Ok(HttpResponse::Ok().json(UpsertResponse {
                version: versioned.version,
                updated_at: versioned.updated_at,
            }))
        }
        Err(e) => {
            log::error!("Failed to save kitchen config: {}", e);
            Ok(HttpResponse::InternalServerError().json(
                serde_json::json!({"error": "Failed to save kitchen config"})
            ))
        }
    }
}

/// PUT endpoint for conditional upserts of kitchen configs
async fn upsert_custom_kitchen(name: web::Path<String>, req: web::Json<UpsertRequest<CustomKitchenConfig>>, data: web::Data<AppState>) -> Result<HttpResponse> {
    let name_str = name.into_inner();
    let key = format!("custom_kitchen_{}", name_str);
    let request = req.into_inner();

    match data.set_versioned_data(&key, &request.data, request.base_version, request.updated_at_client) {
        Ok(versioned) => {
            log::info!("Custom kitchen config upserted: {}", name_str);
            Ok(HttpResponse::Ok().json(UpsertResponse {
                version: versioned.version,
                updated_at: versioned.updated_at,
            }))
        }
        Err(e) if e.to_string().contains("Version conflict") => {
            if let Ok(Some(current)) = data.get_versioned_data::<CustomKitchenConfig>(&key) {
                Ok(HttpResponse::Conflict().json(ConflictResponse {
                    server_version: current.version,
                    server_updated_at: current.updated_at,
                }))
            } else {
                Ok(HttpResponse::Conflict().json(serde_json::json!({
                    "error": "Version conflict"
                })))
            }
        }
        Err(e) => {
            log::error!("Failed to upsert kitchen config: {}", e);
            Ok(HttpResponse::InternalServerError().json(
                serde_json::json!({"error": "Failed to save kitchen config"})
            ))
        }
    }
}

/// Get a specific kitchen configuration
async fn get_custom_kitchen(name: web::Path<String>, data: web::Data<AppState>) -> Result<HttpResponse> {
    let name_str = name.into_inner();
    let key = format!("custom_kitchen_{}", name_str);

    match data.get_versioned_data::<CustomKitchenConfig>(&key) {
        Ok(Some(versioned)) => {
            let response = ItemResponse {
                name: name_str,
                data: versioned.data,
                updated_at: versioned.updated_at,
                version: versioned.version,
            };
            Ok(HttpResponse::Ok().json(response))
        }
        Ok(None) => Ok(HttpResponse::NotFound().json(
            serde_json::json!({"error": "not found"})
        )),
        Err(e) => {
            log::error!("Error getting versioned kitchen config: {}", e);
            Ok(HttpResponse::InternalServerError().json(
                serde_json::json!({"error": "Internal server error"})
            ))
        }
    }
}

/// Delete a specific kitchen configuration
async fn delete_custom_kitchen(name: web::Path<String>, data: web::Data<AppState>) -> Result<HttpResponse> {
    let name_str = name.into_inner();
    let key = format!("custom_kitchen_{}", name_str);
    let versioned_key = format!("versioned_{}", key);

    let db = data.db.write().unwrap();

    // Remove both versioned and legacy keys
    let mut deleted = false;
    if db.remove(&versioned_key).is_ok() {
        deleted = true;
    }
    if db.remove(&key).is_ok() {
        deleted = true;
    }

    if deleted {
        log::info!("Custom kitchen config deleted: {}", name_str);
        Ok(HttpResponse::NoContent().finish()) // 204 No Content as specified
    } else {
        Ok(HttpResponse::NotFound().json(serde_json::json!({"error": "not found"})))
    }
}

/// Health check endpoint
async fn health_check(data: web::Data<AppState>) -> Result<HttpResponse> {
    // Basic health check - don't wait for IoT devices to avoid slowdown
    let response = serde_json::json!({
        "status": "healthy",
        "timestamp": Utc::now().to_rfc3339(),
        "iot_devices_configured": {
            "kitchen_iot": data.kitchen_iot.enabled,
            "cashier": data.cashier.enabled,
            "display": data.display.enabled,
            "pos_system": data.pos_system.enabled,
            "inventory_system": data.inventory_system.enabled,
        }
    });

    Ok(HttpResponse::Ok().json(response))
}

/// Get runtime orders
async fn get_runtime_orders(req: HttpRequest, data: web::Data<AppState>) -> Result<HttpResponse> {
    if let Err(response) = validate_access_token(&req, &data) { return Ok(response); }
    if let Err(resp) = require_scope(&req, &data, "runtime:read") { return Ok(resp); }
    let mut orders: Vec<RuntimeOrder> = vec![];
    let db = data.db.read().unwrap();

    for result in db.iter() {
        let (key, value) = result.map_err(|e| {
            log::error!("Iteration error: {}", e);
            actix_web::error::ErrorInternalServerError("DB iteration error")
        })?;
        let kstr = String::from_utf8(key.to_vec()).unwrap_or_default();
        if kstr.starts_with("runtime_order_") {
            let name = kstr.trim_start_matches("runtime_order_").to_string();
            if let Ok(data_value) = serde_json::from_slice::<Value>(&value) {
                orders.push(RuntimeOrder {
                    name,
                    data: data_value,
                });
            }
        }
    }

    Ok(HttpResponse::Ok().json(orders))
}

/// Save runtime order
async fn save_runtime_order(req: HttpRequest, order: web::Json<RuntimeOrder>, data: web::Data<AppState>) -> Result<HttpResponse> {
    if let Err(response) = validate_access_token(&req, &data) { return Ok(response); }
    if let Err(resp) = require_scope(&req, &data, "orders:write") { return Ok(resp); }
    let order = order.into_inner();
    let key = format!("runtime_order_{}", order.name);
    let order_id = Uuid::new_v4().to_string();
    let timestamp = AppState::current_timestamp();

    match serde_json::to_vec(&order.data) {
        Ok(serialized) => {
            let db = data.db.write().unwrap();
            match db.insert(key, serialized) {
                Ok(_) => {
                    drop(db);

                    // Emit order event
                    let payload = serde_json::json!({
                        "index": 0, // Could be extracted from order.data if needed
                        "symbol": order.name,
                        "items": order.data.get("items"),
                        "source": "api",
                        "ts": timestamp,
                        "meta": order.data
                    });

                    if let Err(e) = data.emit_event("order", payload) {
                        log::error!("Failed to emit order event: {}", e);
                    }

                    log::info!("Runtime order saved: {}", order.name);
                    Ok(HttpResponse::Ok().json(serde_json::json!({
                        "status": "saved",
                        "id": order_id,
                        "ts": timestamp
                    })))
                }
                Err(e) => {
                    log::error!("Failed to save runtime order: {}", e);
                    Ok(HttpResponse::InternalServerError().json(
                        serde_json::json!({"error": "Failed to save runtime order"})
                    ))
                }
            }
        }
        Err(e) => {
            log::error!("Failed to serialize runtime order: {}", e);
            Ok(HttpResponse::BadRequest().json(
                serde_json::json!({"error": "Invalid data"})
            ))
        }
    }
}

/// Get runtime reservations
async fn get_runtime_reservations(req: HttpRequest, data: web::Data<AppState>) -> Result<HttpResponse> {
    if let Err(response) = validate_access_token(&req, &data) { return Ok(response); }
    if let Err(resp) = require_scope(&req, &data, "runtime:read") { return Ok(resp); }
    let mut reservations: Vec<RuntimeReservation> = vec![];
    let db = data.db.read().unwrap();

    for result in db.iter() {
        let (key, value) = result.map_err(|e| {
            log::error!("Iteration error: {}", e);
            actix_web::error::ErrorInternalServerError("DB iteration error")
        })?;
        let kstr = String::from_utf8(key.to_vec()).unwrap_or_default();
        if kstr.starts_with("runtime_reservation_") {
            let name = kstr.trim_start_matches("runtime_reservation_").to_string();
            if let Ok(data_value) = serde_json::from_slice::<Value>(&value) {
                reservations.push(RuntimeReservation {
                    name,
                    data: data_value,
                });
            }
        }
    }

    Ok(HttpResponse::Ok().json(reservations))
}

/// Save runtime reservation
async fn save_runtime_reservation(req: HttpRequest, reservation: web::Json<RuntimeReservation>, data: web::Data<AppState>) -> Result<HttpResponse> {
    if let Err(response) = validate_access_token(&req, &data) { return Ok(response); }
    if let Err(resp) = require_scope(&req, &data, "reservations:write") { return Ok(resp); }
    let reservation = reservation.into_inner();
    let key = format!("runtime_reservation_{}", reservation.name);
    let timestamp = AppState::current_timestamp();

    match serde_json::to_vec(&reservation.data) {
        Ok(serialized) => {
            let db = data.db.write().unwrap();
            match db.insert(key, serialized) {
                Ok(_) => {
                    drop(db);

                    // Emit reservation event
                    let payload = serde_json::json!({
                        "name": reservation.name,
                        "client": reservation.data.get("client"),
                        "start": reservation.data.get("start_time").or(reservation.data.get("time")),
                        "end": reservation.data.get("end_time"),
                        "notes": reservation.data.get("notes"),
                        "ts": timestamp
                    });

                    if let Err(e) = data.emit_event("reservation", payload) {
                        log::error!("Failed to emit reservation event: {}", e);
                    }

                    log::info!("Runtime reservation saved: {}", reservation.name);
                    Ok(HttpResponse::Ok().json(serde_json::json!({"status": "saved"})))
                }
                Err(e) => {
                    log::error!("Failed to save runtime reservation: {}", e);
                    Ok(HttpResponse::InternalServerError().json(
                        serde_json::json!({"error": "Failed to save runtime reservation"})
                    ))
                }
            }
        }
        Err(e) => {
            log::error!("Failed to serialize runtime reservation: {}", e);
            Ok(HttpResponse::BadRequest().json(
                serde_json::json!({"error": "Invalid data"})
            ))
        }
    }
}

/// Get runtime events with cursor-based pagination
async fn get_runtime_events(
    req: HttpRequest,
    query: web::Query<std::collections::HashMap<String, String>>,
    data: web::Data<AppState>
) -> Result<HttpResponse> {
    // Validate access + Bearer (middleware) and require scope
    if let Err(response) = validate_access_token(&req, &data) { return Ok(response); }
    if let Err(resp) = require_scope(&req, &data, "runtime:read") { return Ok(resp); }
    let since_cursor = query.get("sinceCursor").map(|s| s.as_str());
    let limit = query.get("limit")
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(100)
        .min(1000); // Cap at 1000 for safety

    let types_filter = query.get("types")
        .map(|s| s.split(',').map(|t| t.trim().to_string()).collect::<Vec<_>>());

    match data.get_runtime_events(since_cursor, limit, types_filter) {
        Ok(response) => Ok(HttpResponse::Ok().json(response)),
        Err(e) => {
            log::error!("Failed to get runtime events: {}", e);
            Ok(HttpResponse::InternalServerError().json(
                serde_json::json!({"error": "Failed to fetch events"})
            ))
        }
    }
}

/// Process runtime payment
async fn save_runtime_payment(
    req: HttpRequest,
    payment: web::Json<RuntimePayment>,
    data: web::Data<AppState>
) -> Result<HttpResponse> {
    if let Err(response) = validate_access_token(&req, &data) { return Ok(response); }
    if let Err(resp) = require_scope(&req, &data, "payments:write") { return Ok(resp); }
    let payment = payment.into_inner();
    let payment_id = Uuid::new_v4().to_string();
    let timestamp = AppState::current_timestamp();

    // Check for idempotency
    let db = data.db.read().unwrap();
    let idempotency_key = format!("payment_idempotency_{}", payment.idempotency_key);

    if let Ok(Some(existing)) = db.get(&idempotency_key) {
        // Return existing payment response
        if let Ok(existing_response) = serde_json::from_slice::<RuntimePaymentResponse>(&existing) {
            return Ok(HttpResponse::Ok().json(existing_response));
        }
    }
    drop(db);

    // Create payment response
    let response = RuntimePaymentResponse {
        id: payment_id.clone(),
        ts: timestamp,
        status: "pending".to_string(), // Could be "approved" or "declined" based on business logic
    };

    // Store payment and idempotency key
    let db = data.db.write().unwrap();
    let payment_key = format!("runtime_payment_{}", payment_id);
    let payment_data = serde_json::json!({
        "id": payment_id,
        "idempotencyKey": payment.idempotency_key,
        "index": payment.index,
        "amount": payment.amount,
        "currency": payment.currency,
        "method": payment.method,
        "source": payment.source,
        "meta": payment.meta,
        "status": "pending",
        "ts": timestamp
    });

    if let Ok(serialized) = serde_json::to_vec(&payment_data) {
        let _ = db.insert(&payment_key, serialized);
    }

    // Store idempotency response
    if let Ok(response_data) = serde_json::to_vec(&response) {
        let _ = db.insert(&idempotency_key, response_data);
    }
    drop(db);

    // Emit payment event
    let payload = serde_json::json!({
        "id": payment_id,
        "index": payment.index,
        "symbol": payment.meta.as_ref().and_then(|m| m.get("symbol")).and_then(|s| s.as_str()),
        "amount": payment.amount,
        "currency": payment.currency,
        "method": payment.method,
        "source": payment.source,
        "status": "pending",
        "meta": payment.meta
    });

    if let Err(e) = data.emit_event("payment", payload) {
        log::error!("Failed to emit payment event: {}", e);
    }

    log::info!("Runtime payment processed: {} ({})", payment_id, payment.idempotency_key);
    Ok(HttpResponse::Ok().json(response))
}

/// Print to kitchen printers
async fn kitchen_print(
    req: HttpRequest,
    request: web::Json<KitchenPrintRequest>,
    data: web::Data<AppState>
) -> Result<HttpResponse> {
    if let Err(response) = validate_access_token(&req, &data) { return Ok(response); }
    if let Err(resp) = require_scope(&req, &data, "kitchen:print") { return Ok(resp); }
    let request = request.into_inner();
    let job_id = Uuid::new_v4().to_string();
    let timestamp = AppState::current_timestamp();

    // Get printers from request or active config
    let printers = if let Some(printers) = request.printers {
        printers
    } else {
        // Get active kitchen config
        if let Ok(Some(active_config)) = data.get_versioned_data::<ActiveKitchenConfig>("custom_kitchen_active") {
            if let Some(config_name) = active_config.data.name {
                let config_key = format!("custom_kitchen_{}", config_name);
                if let Ok(Some(kitchen_config)) = data.get_versioned_data::<CustomKitchenConfig>(&config_key) {
                    // Extract printer URLs from config
                    if let Some(printers_array) = kitchen_config.data.data.get("printers").and_then(|p| p.as_array()) {
                        printers_array.iter()
                            .filter_map(|p| p.as_str().map(|s| s.to_string()))
                            .collect()
                    } else {
                        vec![]
                    }
                } else {
                    vec![]
                }
            } else {
                vec![]
            }
        } else {
            vec![]
        }
    };

    if printers.is_empty() {
        return Ok(HttpResponse::BadRequest().json(
            serde_json::json!({"error": "No printers configured"})
        ));
    }

    // Create printer status entries
    let printer_statuses: Vec<KitchenPrinterStatus> = printers.iter().map(|url| {
        KitchenPrinterStatus {
            url: url.clone(),
            status: "queued".to_string(),
            attempts: 0,
            last_error: None,
        }
    }).collect();

    let response = KitchenPrintResponse {
        job_id: job_id.clone(),
        accepted: true,
        printers: printer_statuses.clone(),
        ts: timestamp,
    };

    // Emit kitchen_status event
    let kitchen_payload = serde_json::json!({
        "jobId": job_id,
        "orderRef": request.idempotency_key,
        "status": "queued",
        "printers": printer_statuses,
        "ts": timestamp
    });

    if let Err(e) = data.emit_event("kitchen_status", kitchen_payload) {
        log::error!("Failed to emit kitchen_status event: {}", e);
    }

    log::info!("Kitchen print job created: {} for {} printers", job_id, printers.len());
    Ok(HttpResponse::Ok().json(response))
}

/// Set new access token (protected endpoint)
async fn set_access_token(
    request: web::Json<AccessTokenUpdateRequest>,
    data: web::Data<AppState>,
) -> Result<HttpResponse> {
    let request = request.into_inner();

    // Validate new token (basic validation)
    if request.new_token.is_empty() {
        return Ok(HttpResponse::BadRequest().json(
            serde_json::json!({"error": "New token cannot be empty"})
        ));
    }

    if request.new_token.len() < 8 {
        return Ok(HttpResponse::BadRequest().json(
            serde_json::json!({"error": "Token must be at least 8 characters long"})
        ));
    }

    // Update the access token
    {
        let mut token = data.access_token.write().unwrap();
        *token = request.new_token.clone();
    }

    log::info!("Access token updated successfully");

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "status": "success",
        "message": "Access token updated successfully",
        "timestamp": Utc::now().to_rfc3339()
    })))
}

/// Periodic backup task that runs at configurable intervals
async fn get_active_kitchen_config(data: web::Data<AppState>) -> Result<HttpResponse> {
    match data.get_versioned_data::<ActiveKitchenConfig>("custom_kitchen_active") {
        Ok(Some(config)) => Ok(HttpResponse::Ok().json(config.data)),
        Ok(None) => Ok(HttpResponse::Ok().json(ActiveKitchenConfig { name: None })),
        Err(e) => {
            log::error!("Failed to get active kitchen config: {}", e);
            Ok(HttpResponse::InternalServerError().json(
                serde_json::json!({"error": "Failed to get active config"})
            ))
        }
    }
}

/// Set active kitchen configuration
async fn set_active_kitchen_config(
    req: HttpRequest,
    config: web::Json<ActiveKitchenConfig>,
    data: web::Data<AppState>
) -> Result<HttpResponse> {
    if let Err(response) = validate_access_token(&req, &data) { return Ok(response); }
    if let Err(resp) = require_scope(&req, &data, "kitchen:config") { return Ok(resp); }
    let config = config.into_inner();

    match data.set_versioned_data("custom_kitchen_active", &config, None, None) {
        Ok(_) => {
            log::info!("Active kitchen config set to: {:?}", config.name);
            Ok(HttpResponse::Ok().json(&config))
        }
        Err(e) => {
            log::error!("Failed to set active kitchen config: {}", e);
            Ok(HttpResponse::InternalServerError().json(
                serde_json::json!({"error": "Failed to set active config"})
            ))
        }
    }
}

/// Get runtime counts for lightweight badges
async fn get_runtime_counts(
    query: web::Query<std::collections::HashMap<String, String>>,
    data: web::Data<AppState>,
    req: HttpRequest,
) -> Result<HttpResponse> {
    // Require scope to read runtime counts
    if let Err(resp) = require_scope(&req, &data, "runtime:read") { return Ok(resp); }
    let since_ts = query.get("since")
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);

    let db = data.db.read().unwrap();
    let mut counts = RuntimeCounts {
        orders: 0,
        reservations: 0,
        payments: 0,
    };

    // Count events since timestamp
    for (_, value) in db.scan_prefix("runtime_event_").flatten() {
        if let Ok(event) = serde_json::from_slice::<RuntimeEvent>(&value) {
            if event.ts >= since_ts {
                match event.event_type.as_str() {
                    "order" => counts.orders += 1,
                    "reservation" => counts.reservations += 1,
                    "payment" => counts.payments += 1,
                    _ => {}
                }
            }
        }
    }

    Ok(HttpResponse::Ok().json(counts))
}

// ----------------------
// Auth: register / login & authorization helpers
// ----------------------

async fn list_users_admin(data: web::Data<AppState>, req: HttpRequest) -> Result<HttpResponse> {
    if let Err(resp) = require_role(&req, &data, "admin") { return Ok(resp); }
    let mut items: Vec<serde_json::Value> = Vec::new();
    let db = data.db.read().unwrap();
    for (key, value) in db.scan_prefix("user:").flatten() {
        let key_str = String::from_utf8(key.to_vec()).unwrap_or_default();
        let email = key_str.trim_start_matches("user:").to_string();
        if let Ok(v) = serde_json::from_slice::<serde_json::Value>(&value) {
            let roles = v.get("roles").cloned().unwrap_or(serde_json::json!([]));
            let scopes = v.get("scopes").cloned().unwrap_or(serde_json::json!([]));
            items.push(serde_json::json!({"email": email, "roles": roles, "scopes": scopes}));
        }
    }
    Ok(HttpResponse::Ok().json(serde_json::json!({"users": items})))
}

async fn get_user_claims(path: web::Path<String>, data: web::Data<AppState>, req: HttpRequest) -> Result<HttpResponse> {
    // admin-only
    if let Err(resp) = require_role(&req, &data, "admin") { return Ok(resp); }
    let email = normalize_email(&path.into_inner());
    let key = format!("user:{}", email);
    let db = data.db.read().unwrap();
let Some(raw) = db.get(&key).map_err(actix_web::error::ErrorInternalServerError)? else {
        return Ok(HttpResponse::NotFound().json(serde_json::json!({"error":"not found"})));
    };
    let val: serde_json::Value = serde_json::from_slice(&raw).unwrap_or(serde_json::json!({}));
    let roles = val.get("roles").cloned().unwrap_or(serde_json::json!([]));
    let scopes = val.get("scopes").cloned().unwrap_or(serde_json::json!([]));
    Ok(HttpResponse::Ok().json(serde_json::json!({"email": email, "roles": roles, "scopes": scopes})))
}

async fn update_user_claims(path: web::Path<String>, req_json: web::Json<ClaimsUpdateRequest>, data: web::Data<AppState>, req: HttpRequest) -> Result<HttpResponse> {
    // admin-only
    if let Err(resp) = require_role(&req, &data, "admin") { return Ok(resp); }
    let email = normalize_email(&path.into_inner());
    let key = format!("user:{}", email);
let db = data.db.write().unwrap();
    let Some(mut raw) = db.get(&key).ok().flatten() else {
        return Ok(HttpResponse::NotFound().json(serde_json::json!({"error":"not found"})));
    };
    let mut val: serde_json::Value = serde_json::from_slice(&raw).unwrap_or(serde_json::json!({}));
    let update = req_json.into_inner();
    if let Some(roles) = update.roles { val["roles"] = serde_json::Value::Array(roles.into_iter().map(|s| serde_json::json!(s)).collect()); }
    if let Some(scopes) = update.scopes { val["scopes"] = serde_json::Value::Array(scopes.into_iter().map(|s| serde_json::json!(s)).collect()); }
    raw = serde_json::to_vec(&val).unwrap().into();
db.insert(key.as_bytes(), raw).map_err(actix_web::error::ErrorInternalServerError)?;
    Ok(HttpResponse::Ok().json(serde_json::json!({"email": email, "roles": val["roles"], "scopes": val["scopes"]})))
}

fn extract_paseto_claims(req: &HttpRequest, app_state: &web::Data<AppState>) -> Result<serde_json::Value, HttpResponse> {
    let auth_header = req.headers().get("Authorization").and_then(|v| v.to_str().ok());
    let bearer = auth_header.and_then(|s| s.strip_prefix("Bearer "));
    let token = match bearer {
        Some(t) if !t.is_empty() => t,
        _ => return Err(HttpResponse::Unauthorized().json(serde_json::json!({"error":"Authorization: Bearer token required","code":"MISSING_BEARER"})))
    };
    let key = app_state.paseto_local_key.read().unwrap();
    let backend = paseto::tokens::TimeBackend::Chrono;
    match validate_local_token(token, None, &key[..], &backend) {
        Ok(claims) => Ok(claims),
        Err(_) => Err(HttpResponse::Unauthorized().json(serde_json::json!({"error":"Invalid PASETO token","code":"INVALID_BEARER"})))
    }
}

fn require_scope(req: &HttpRequest, app_state: &web::Data<AppState>, scope: &str) -> Result<(), HttpResponse> {
    let claims = extract_paseto_claims(req, app_state)?;
    let scopes = claims.get("scp").and_then(|v| v.as_array()).cloned().unwrap_or_default();
    let ok = scopes.iter().any(|s| s.as_str() == Some(scope));
    if ok { Ok(()) } else { Err(HttpResponse::Forbidden().json(serde_json::json!({"error":"Insufficient scope","required": scope}))) }
}

fn require_role(req: &HttpRequest, app_state: &web::Data<AppState>, role: &str) -> Result<(), HttpResponse> {
    let claims = extract_paseto_claims(req, app_state)?;
    let roles = claims.get("roles").and_then(|v| v.as_array()).cloned().unwrap_or_default();
    let ok = roles.iter().any(|r| r.as_str() == Some(role));
    if ok { Ok(()) } else { Err(HttpResponse::Forbidden().json(serde_json::json!({"error":"Insufficient role","required": role}))) }
}

fn normalize_email(e: &str) -> String { e.trim().to_lowercase() }

fn hash_password_argon2(plain: &str) -> Result<String, String> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let hash = argon2.hash_password(plain.as_bytes(), &salt)
        .map_err(|e| e.to_string())?
        .to_string();
    Ok(hash)
}

fn verify_password_argon2(plain: &str, hashed: &str) -> bool {
    if let Ok(parsed) = PasswordHash::new(hashed) {
        let argon2 = Argon2::default();
        return argon2.verify_password(plain.as_bytes(), &parsed).is_ok();
    }
    false
}

impl AppState {
fn decode_hex_32(s: &str) -> Option<[u8; 32]> {
        let hex = s.trim();
        if hex.len() != 64 { return None; }
        let mut out = [0u8; 32];
        let bytes = hex.as_bytes();
        for i in 0..32 {
            let hi = bytes[2*i] as char;
            let lo = bytes[2*i+1] as char;
            let hv = hi.to_digit(16)? as u8;
            let lv = lo.to_digit(16)? as u8;
            out[i] = (hv << 4) | lv;
        }
        Some(out)
    }

#[cfg_attr(not(test), allow(dead_code))]
fn issue_paseto_v4_local(&self, subject: &str, display_name: Option<&str>) -> Result<String, String> {
        self.issue_paseto_with_claims(subject, display_name, vec![], vec![])
    }

    fn issue_paseto_with_claims(
        &self,
        subject: &str,
        display_name: Option<&str>,
        roles: Vec<serde_json::Value>,
        scopes: Vec<serde_json::Value>,
    ) -> Result<String, String> {
        let key = self.paseto_local_key.read().unwrap();
        let now = chrono::Utc::now();
        let exp = now + chrono::Duration::seconds(self.token_ttl_seconds as i64);
        let token = PasetoBuilder::new()
            .set_encryption_key(&key[..])
            .set_issued_at(None)
            .set_expiration(&exp)
            .set_not_before(&now)
            .set_issuer(&self.token_issuer)
            .set_audience(&self.token_audience)
            .set_subject(subject)
            .set_claim(
                "displayName",
                match display_name { Some(n) => serde_json::json!(n), None => serde_json::Value::Null }
            )
            .set_claim("roles", serde_json::Value::Array(roles))
            .set_claim("scp", serde_json::Value::Array(scopes))
            .build()
            .map_err(|e| e.to_string())?;
        Ok(token)
    }
}

async fn auth_register(req: web::Json<RegisterRequest>, data: web::Data<AppState>) -> Result<HttpResponse> {
    let payload = req.into_inner();
    let email = normalize_email(&payload.email);
    let name = payload.name.trim().to_string();

    if name.is_empty() || email.is_empty() || payload.password.is_empty() {
        return Ok(HttpResponse::BadRequest().json(serde_json::json!({"error":"Missing required fields","code":"INVALID_INPUT"})));
    }

    let user_key = format!("user:{}", email);
    let db = data.db.read().unwrap();
if db.get(&user_key).map_err(actix_web::error::ErrorInternalServerError)?.is_some() {
        return Ok(HttpResponse::Conflict().json(serde_json::json!({"error":"Email already registered","code":"EMAIL_EXISTS"})));
    }
    drop(db);

    let hashed = match hash_password_argon2(&payload.password) {
        Ok(h) => h,
        Err(e) => return Ok(HttpResponse::InternalServerError().json(serde_json::json!({"error":e,"code":"HASH_ERROR"}))),
    };

    // Bootstrap: first registered user becomes admin with iam:write
    let is_first_user = {
        let db = data.db.read().unwrap();
        db.scan_prefix("user:").next().is_none()
    };
    // Default roles/scopes for new users
    let mut default_roles = vec!["user".to_string()];
    let mut default_scopes = vec!["runtime:read".to_string()]; // read-only scopes by default
    if is_first_user {
        default_roles.push("admin".to_string());
        default_scopes.push("iam:write".to_string());
    }

    let record = serde_json::json!({
        "name": name,
        "email": email,
        "password_hash": hashed,
        "note": payload.note,
        "roles": default_roles,
        "scopes": default_scopes
    });
    let db = data.db.write().unwrap();
db.insert(user_key.as_bytes(), serde_json::to_vec(&record).unwrap()).map_err(actix_web::error::ErrorInternalServerError)?;
    drop(db);

    Ok(HttpResponse::Created().json(serde_json::json!({"status":"registered"})))
}

async fn auth_login(req: web::Json<LoginRequest>, data: web::Data<AppState>) -> Result<HttpResponse> {
    let payload = req.into_inner();
    let email_raw = if !payload.email.trim().is_empty() { payload.email.trim() } else { payload.username.trim() };
    let email = normalize_email(email_raw);
    if email.is_empty() || payload.password.is_empty() {
        return Ok(HttpResponse::BadRequest().json(serde_json::json!({"error":"Missing email/username or password","code":"INVALID_INPUT"})));
    }

    let user_key = format!("user:{}", email);
    let db = data.db.read().unwrap();
let Some(user_raw) = db.get(&user_key).map_err(actix_web::error::ErrorInternalServerError)? else {
        return Ok(HttpResponse::Unauthorized().json(serde_json::json!({"error":"Invalid credentials","code":"INVALID_CREDENTIALS"})));
    };
    let user_val: serde_json::Value = serde_json::from_slice(&user_raw).unwrap_or(serde_json::json!({}));
    let stored_hash = user_val.get("password_hash").and_then(|v| v.as_str()).unwrap_or("");
    let display_name = user_val.get("name").and_then(|v| v.as_str()).map(|s| s.to_string());

    if !verify_password_argon2(&payload.password, stored_hash) {
        return Ok(HttpResponse::Unauthorized().json(serde_json::json!({"error":"Invalid credentials","code":"INVALID_CREDENTIALS"})));
    }
    drop(db);

    // Extract roles/scopes from user record (defaults on missing)
    let roles = user_val.get("roles").and_then(|v| v.as_array()).cloned().unwrap_or_else(|| vec![serde_json::json!("user")]);
    let scopes = user_val.get("scopes").and_then(|v| v.as_array()).cloned().unwrap_or_else(|| vec![serde_json::json!("runtime:read")]);

    let token = match data.issue_paseto_with_claims(&email, display_name.as_deref(), roles, scopes) {
        Ok(t) => t,
        Err(e) => return Ok(HttpResponse::InternalServerError().json(serde_json::json!({"error":e,"code":"TOKEN_ISSUE_ERROR"}))),
    };

Ok(HttpResponse::Ok().json(LoginResponse { access_token: token, display_name }))
}

/// Kitchen test print endpoint
async fn kitchen_test_print(
    request: web::Json<KitchenTestRequest>,
    data: web::Data<AppState>
) -> Result<HttpResponse> {
    let request = request.into_inner();
    let job_id = Uuid::new_v4().to_string();
    let timestamp = AppState::current_timestamp();
    let printer_name = request.printer_name.unwrap_or_else(|| "test_printer".to_string());

    let response = KitchenTestResponse {
        job_id: job_id.clone(),
        accepted: true,
        printer: printer_name.clone(),
        ts: timestamp,
    };

    // Emit kitchen_status event for test job
    let kitchen_payload = serde_json::json!({
        "jobId": job_id,
        "status": "queued",
        "printers": [{
            "url": format!("test://{}", printer_name),
            "status": "queued",
            "attempts": 0
        }],
        "ts": timestamp
    });

    if let Err(e) = data.emit_event("kitchen_status", kitchen_payload) {
        log::error!("Failed to emit kitchen_status event: {}", e);
    }

    // TODO: Implement test latency and failure simulation
    log::info!("Kitchen test print job created: {} for {}", job_id, printer_name);
    Ok(HttpResponse::Ok().json(response))
}

/// Get test printers list
async fn get_kitchen_test_printers() -> Result<HttpResponse> {
    let test_printers = vec![
        serde_json::json!({ "name": "test_printer_1", "url": "test://printer1" }),
        serde_json::json!({ "name": "test_printer_2", "url": "test://printer2" }),
        serde_json::json!({ "name": "test_printer_3", "url": "test://printer3" }),
    ];

    Ok(HttpResponse::Ok().json(test_printers))
}

/// Kitchen test health check
async fn get_kitchen_test_health() -> Result<HttpResponse> {
    let health = serde_json::json!({
        "ok": true,
        "ts": AppState::current_timestamp()
    });

    Ok(HttpResponse::Ok().json(health))
}

/// IoT devices health check endpoint
async fn get_iot_health(data: web::Data<AppState>) -> Result<HttpResponse> {
    let kitchen_status = data.kitchen_iot.health_check().await;
    let cashier_status = data.cashier.health_check().await;
    let display_status = data.display.health_check().await;
    let pos_status = data.pos_system.health_check().await;
    let inventory_status = data.inventory_system.health_check().await;

    let health_response = IoTHealthCheckResponse {
        kitchen_iot: kitchen_status,
        cashier: cashier_status,
        display: display_status,
        pos_system: pos_status,
        inventory_system: inventory_status,
    };

    Ok(HttpResponse::Ok().json(health_response))
}

/// Test connectivity to a specific IoT device
async fn test_iot_device(path: web::Path<String>, data: web::Data<AppState>) -> Result<HttpResponse> {
    let device_name = path.into_inner();

    let status = match device_name.as_str() {
        "kitchen" => data.kitchen_iot.health_check().await,
        "cashier" => data.cashier.health_check().await,
        "display" => data.display.health_check().await,
        "pos" => data.pos_system.health_check().await,
        "inventory" => data.inventory_system.health_check().await,
        _ => {
            return Ok(HttpResponse::BadRequest().json(
                serde_json::json!({"error": "Unknown device type"})
            ));
        }
    };

    Ok(HttpResponse::Ok().json(status))
}

/// Send data to IoT device
async fn send_to_iot_device(
    path: web::Path<String>,
    request: web::Json<serde_json::Value>,
    data: web::Data<AppState>,
) -> Result<HttpResponse> {
    let device_name = path.into_inner();
    let payload = request.into_inner();

    // Extract endpoint from payload, default to "data"
    let endpoint = payload.get("endpoint")
        .and_then(|v| v.as_str())
        .unwrap_or("data");

    let result = match device_name.as_str() {
        "kitchen" => data.kitchen_iot.send_data(endpoint, &payload).await,
        "cashier" => data.cashier.send_data(endpoint, &payload).await,
        "display" => data.display.send_data(endpoint, &payload).await,
        "pos" => data.pos_system.send_data(endpoint, &payload).await,
        "inventory" => data.inventory_system.send_data(endpoint, &payload).await,
        _ => {
            return Ok(HttpResponse::BadRequest().json(
                serde_json::json!({"error": "Unknown device type"})
            ));
        }
    };

    match result {
        Ok(response) => Ok(HttpResponse::Ok().json(response)),
        Err(error) => Ok(HttpResponse::BadGateway().json(
            serde_json::json!({"error": error})
        )),
    }
}

/// Get data from IoT device
async fn get_from_iot_device(
    path: web::Path<(String, String)>,
    data: web::Data<AppState>,
) -> Result<HttpResponse> {
    let (device_name, endpoint) = path.into_inner();

    let result = match device_name.as_str() {
        "kitchen" => data.kitchen_iot.get_data(&endpoint).await,
        "cashier" => data.cashier.get_data(&endpoint).await,
        "display" => data.display.get_data(&endpoint).await,
        "pos" => data.pos_system.get_data(&endpoint).await,
        "inventory" => data.inventory_system.get_data(&endpoint).await,
        _ => {
            return Ok(HttpResponse::BadRequest().json(
                serde_json::json!({"error": "Unknown device type"})
            ));
        }
    };

    match result {
        Ok(response) => Ok(HttpResponse::Ok().json(response)),
        Err(error) => Ok(HttpResponse::BadGateway().json(
            serde_json::json!({"error": error})
        )),
    }
}

// ---- Custom Pins Configuration Endpoints ----
/// List all saved pins configuration names
async fn list_custom_pins(data: web::Data<AppState>) -> Result<HttpResponse> {
    let mut items: Vec<ItemMetadata> = vec![];
    for result in data.db.read().unwrap().iter() {
        let (key, value) = result.map_err(|e| {
            log::error!("Iteration error: {}", e);
            actix_web::error::ErrorInternalServerError("DB iteration error")
        })?;
        let kstr = String::from_utf8(key.to_vec()).unwrap_or_default();
        if kstr.starts_with("versioned_custom_pins_") {
            let name = kstr.trim_start_matches("versioned_custom_pins_").to_string();
            if let Ok(versioned_data) = serde_json::from_slice::<VersionedData<CustomPinsConfig>>(&value) {
                items.push(ItemMetadata {
                    name,
                    updated_at: versioned_data.updated_at,
                    version: versioned_data.version,
                    checksum: Some(AppState::calculate_checksum(&versioned_data.data)),
                });
            }
        }
    }
    Ok(HttpResponse::Ok().json(ListResponse { items }))
}

/// Save a named pins configuration (legacy POST endpoint)
async fn save_custom_pins(cfg: web::Json<(String, CustomPinsConfig)>, data: web::Data<AppState>) -> Result<HttpResponse> {
    let (name, config) = cfg.into_inner();
    let key = format!("custom_pins_{}", name);

    match data.set_versioned_data(&key, &config, None, None) {
        Ok(versioned) => {
            log::info!("Custom pins config saved: {}", name);
            Ok(HttpResponse::Ok().json(UpsertResponse {
                version: versioned.version,
                updated_at: versioned.updated_at,
            }))
        }
        Err(e) => {
            log::error!("Failed to save pins config: {}", e);
            Ok(HttpResponse::InternalServerError().json(
                serde_json::json!({"error": "Failed to save pins config"})
            ))
        }
    }
}

/// PUT endpoint for conditional upserts of pins configs
async fn upsert_custom_pins(name: web::Path<String>, req: web::Json<UpsertRequest<CustomPinsConfig>>, data: web::Data<AppState>) -> Result<HttpResponse> {
    let name_str = name.into_inner();
    let key = format!("custom_pins_{}", name_str);
    let request = req.into_inner();

    match data.set_versioned_data(&key, &request.data, request.base_version, request.updated_at_client) {
        Ok(versioned) => {
            log::info!("Custom pins config upserted: {}", name_str);
            Ok(HttpResponse::Ok().json(UpsertResponse {
                version: versioned.version,
                updated_at: versioned.updated_at,
            }))
        }
        Err(e) if e.to_string().contains("Version conflict") => {
            if let Ok(Some(current)) = data.get_versioned_data::<CustomPinsConfig>(&key) {
                Ok(HttpResponse::Conflict().json(ConflictResponse {
                    server_version: current.version,
                    server_updated_at: current.updated_at,
                }))
            } else {
                Ok(HttpResponse::Conflict().json(serde_json::json!({
                    "error": "Version conflict"
                })))
            }
        }
        Err(e) => {
            log::error!("Failed to upsert pins config: {}", e);
            Ok(HttpResponse::InternalServerError().json(
                serde_json::json!({"error": "Failed to save pins config"})
            ))
        }
    }
}

/// Get a specific pins configuration
async fn get_custom_pins(name: web::Path<String>, data: web::Data<AppState>) -> Result<HttpResponse> {
    let name_str = name.into_inner();
    let key = format!("custom_pins_{}", name_str);

    match data.get_versioned_data::<CustomPinsConfig>(&key) {
        Ok(Some(versioned)) => {
            let response = ItemResponse {
                name: name_str,
                data: versioned.data,
                updated_at: versioned.updated_at,
                version: versioned.version,
            };
            Ok(HttpResponse::Ok().json(response))
        }
        Ok(None) => Ok(HttpResponse::NotFound().json(
            serde_json::json!({"error": "not found"})
        )),
        Err(e) => {
            log::error!("Error getting versioned pins config: {}", e);
            Ok(HttpResponse::InternalServerError().json(
                serde_json::json!({"error": "Internal server error"})
            ))
        }
    }
}

/// Delete a specific pins configuration
async fn delete_custom_pins(name: web::Path<String>, data: web::Data<AppState>) -> Result<HttpResponse> {
    let name_str = name.into_inner();
    let key = format!("custom_pins_{}", name_str);
    let versioned_key = format!("versioned_{}", key);

    let db = data.db.write().unwrap();

    // Remove both versioned and legacy keys
    let mut deleted = false;
    if db.remove(&versioned_key).is_ok() {
        deleted = true;
    }
    if db.remove(&key).is_ok() {
        deleted = true;
    }

    if deleted {
        log::info!("Custom pins config deleted: {}", name_str);
        Ok(HttpResponse::NoContent().finish()) // 204 No Content
    } else {
        Ok(HttpResponse::NotFound().json(serde_json::json!({"error": "not found"})))
    }
}
use once_cell::sync::Lazy;
// use std::sync::RwLock;



static ACTIVE_WORDLIST: Lazy<RwLock<WordListRef>> = Lazy::new(|| {
    RwLock::new(WordListRef {
        name: "default".to_string(),
        version: "1".to_string(),   // <- String, not integer
        lang: "en".to_string(),
        word_len: 5u8,              // <- u8 literal
    })
});

fn get_active_wordlist() -> WordListRef {
    ACTIVE_WORDLIST.read().unwrap().clone()
}


// ---- Basic Wordle Game Endpoints ----
/// Create a new Wordle session (idempotent based on cardIndex+boardId+seed)
async fn create_wordle_session(req: web::Json<WordleSessionCreateRequest>, data: web::Data<AppState>) -> Result<HttpResponse> {
    let request = req.into_inner();
    let timestamp = AppState::current_timestamp();

    // Check for existing session with same cardIndex+boardId+seed combination
    {
        let sessions = data.wordle_sessions.read().unwrap();
        for session in sessions.values() {
            if session.card_index == request.card_index &&
               session.board_id == request.board_id &&
               session.seed == request.seed &&
               session.status == "active" {
                // Return existing session (idempotent)
                let response = WordleSessionCreateResponse {
                    id: session.id.clone(),
                    card_index: session.card_index,
                    solution_hash: session.solution_hash.clone(),
                    created_at: session.created_at,
                };
                log::info!("Returning existing Wordle session: {} for card {} (idempotent)", response.id, request.card_index);
                return Ok(HttpResponse::Ok().json(response));
            }
        }
    }

    // Create new session if no match found
    let session_id = Uuid::new_v4().to_string();

    // Generate a simple solution for demo (in production, use a word list)
    let solution = format!("WORD{}", request.card_index % 10); // Demo solution
    let solution_hash = format!("{:x}", sha2::Sha256::digest(solution.as_bytes()));
    // let rules = WordleRules {
    //     max_attempts: 6,
    //     word_len: 5,
    //     timed: false,
    //     total_seconds: None,
    //     per_guess_seconds: None,
    //     scoring: WordleScoring::TimeAndRows,
    // };
let rules = WordleRules {
    max_attempts: 6,
    word_len: 5,
    timed: true,
    total_seconds: 180,            // 3 minutes total; 0 = none
    per_guess_seconds: None,       // or Some(30)
    scoring: ScoringRules {
        base: 1000,
        per_attempt_penalty: 100,
        time_penalty_per_sec: 1,
        timeout_ms: 5 * 60 * 1000, // 5 min hard cap
    },
};
    let session = WordleSession {
        id: session_id.clone(),
        card_index: request.card_index,
        board_id: request.board_id,
        seed: request.seed,
        solution_hash: solution_hash.clone(),
        solution,
        created_at: timestamp,
        participants: Vec::new(),
        guesses: Vec::new(),
        status: "active".to_string(),
        expires_at: timestamp + (24 * 60 * 60 * 1000), // 24 hours
        // missing structure fields:
         // ✅ fill these in with real expressions:
        rules,                     // or: rules: WordleRules::default(),
        // rules: WordleRules::default(),
        wordlist: get_active_wordlist(),   // <<-- concrete WordListRef, not Option
            // wordlist: Some(WordListRef {
            //     name: "default".into(),
            //     version: "1".to_string(), // or a real hash/semver
            //     lang: "en".into(),
            //     word_len: 5,
            // }),
        started_at: Some(timestamp), // if this is Option<u64>; otherwise just `timestamp`
        ended_at: None,            // if this is Option<u64>; otherwise use 0
    };

    // Store session
    {
        let mut sessions = data.wordle_sessions.write().unwrap();
        sessions.insert(session_id.clone(), session);
    }

    let response = WordleSessionCreateResponse {
        id: session_id,
        card_index: request.card_index,
        solution_hash,
        created_at: timestamp,
    };

    log::info!("Created new Wordle session: {} for card {}", response.id, request.card_index);
    Ok(HttpResponse::Ok().json(response))
}

/// List active Wordle sessions
async fn list_wordle_sessions(query: web::Query<HashMap<String, String>>, data: web::Data<AppState>) -> Result<HttpResponse> {
    let card_index_filter = query.get("cardIndex").and_then(|s| s.parse::<u32>().ok());
    let board_id_filter = query.get("boardId");

    let sessions = data.wordle_sessions.read().unwrap();
    let mut filtered_sessions: Vec<WordleSessionCreateResponse> = Vec::new();

    for session in sessions.values() {
        // Apply filters
        if let Some(card_index) = card_index_filter {
            if session.card_index != card_index {
                continue;
            }
        }
        if let Some(board_id) = board_id_filter {
if session.board_id.as_deref() != Some(board_id) {
                continue;
            }
        }

        filtered_sessions.push(WordleSessionCreateResponse {
            id: session.id.clone(),
            card_index: session.card_index,
            solution_hash: session.solution_hash.clone(),
            created_at: session.created_at,
        });
    }

    Ok(HttpResponse::Ok().json(filtered_sessions))
}

/// Join a Wordle session
async fn join_wordle_session(path: web::Path<String>, req: web::Json<WordleJoinRequest>, data: web::Data<AppState>) -> Result<HttpResponse> {
    let session_id = path.into_inner();
    let request = req.into_inner();

    let mut sessions = data.wordle_sessions.write().unwrap();
    if let Some(session) = sessions.get_mut(&session_id) {
        let current_time = AppState::current_timestamp();
        let participant = WordleParticipant {
            user_id: request.user_id.clone(),
            display_name: request.display_name,
            joined_at: current_time,
            last_seen: current_time,
            status: Some("active".to_string()),
        };

        // Check if user already joined
        if !session.participants.iter().any(|p| p.user_id == request.user_id) {
            session.participants.push(participant.clone());

            // Broadcast join event via SSE
            let join_event = WordleSSEEvent {
                event_type: "join".to_string(),
                session_id: session_id.clone(),
                participant: Some(participant),
                guess: None,
                state: None,
                ts: AppState::current_timestamp(),
            };

            // Send to SSE clients
            if let Some(clients) = data.wordle_sse_clients.read().unwrap().get(&session_id) {
                for client in clients {
                    let _ = client.send(join_event.clone());
                }
            }

            log::info!("User {} joined Wordle session {}", request.user_id, session_id);
        }

        Ok(HttpResponse::Ok().json(serde_json::json!({"status": "joined"})))
    } else {
        Ok(HttpResponse::NotFound().json(serde_json::json!({"error": "Session not found"})))
    }
}

/// Close a Wordle session
async fn close_wordle_session(path: web::Path<String>, data: web::Data<AppState>) -> Result<HttpResponse> {
    let session_id = path.into_inner();

    let mut sessions = data.wordle_sessions.write().unwrap();
    if let Some(session) = sessions.get_mut(&session_id) {
        session.status = "closed".to_string();

        // Broadcast close event via SSE
        let close_event = WordleSSEEvent {
            event_type: "close".to_string(),
            session_id: session_id.clone(),
            participant: None,
            guess: None,
            state: None,
            ts: AppState::current_timestamp(),
        };

        // Send to SSE clients
        if let Some(clients) = data.wordle_sse_clients.read().unwrap().get(&session_id) {
            for client in clients {
                let _ = client.send(close_event.clone());
            }
        }

        log::info!("Wordle session closed: {}", session_id);
        Ok(HttpResponse::Ok().json(serde_json::json!({"status": "closed"})))
    } else {
        Ok(HttpResponse::NotFound().json(serde_json::json!({"error": "Session not found"})))
    }
}
/// wordle
// fn compute_points(r: &WordleRules, attempts: u32, duration_ms: u64) -> i32 {
// // fn compute_points(r: &ScoringRules, attempts: u32, duration_ms: u64) -> i32 {
//     let secs = (duration_ms / 1000) as i32;
//     let mut pts = r.base
//         - r.per_attempt_penalty * ((attempts.saturating_sub(1)) as i32)
//         - r.time_penalty_per_sec * secs;
//     if duration_ms > r.timeout_ms { pts = 0; }
//     pts.max(0)
// }
// fn compute_points(r: &WordleRules, attempts: u32, duration_ms: u64) -> i32 {
//     let s = &r.scoring; // <— read scoring fields from here
//     let secs = (duration_ms / 1000) as i32;

//     let mut pts = s.base
//         - s.per_attempt_penalty * (attempts.saturating_sub(1) as i32)
//         - s.time_penalty_per_sec * secs;

//     // hard timeout from scoring rules
//     if duration_ms > s.timeout_ms {
//         pts = 0;
//     }

//     // optional “game-mode” time caps (if you want them)
//     if r.timed {
//         if let Some(pg) = r.per_guess_seconds {
//             let cap = (r.max_attempts as u64) * (pg as u64) * 1000;
//             if cap > 0 && duration_ms > cap { pts = 0; }
//         }
//         if r.total_seconds > 0 {
//             let cap = (r.total_seconds as u64) * 1000;
//             if duration_ms > cap { pts = 0; }
//         }
//     }

//     pts.max(0)
// }
fn compute_points(r: &ScoringRules, attempts: u32, duration_ms: u64) -> i32 {
    let secs = (duration_ms / 1000) as i32;
    let mut pts = r.base
        - r.per_attempt_penalty * (attempts.saturating_sub(1) as i32)
        - r.time_penalty_per_sec * secs;

    if r.timeout_ms > 0 && duration_ms > r.timeout_ms {
        pts = 0;
    }
    pts.max(0)
}
// fn update_leaderboard(user_id: &str, display_name: Option<String>, points: i32, duration_ms: u64, ts: u64) {
//     let mut lb = LEADERBOARD.write().unwrap();
//     if let Some(entry) = lb.iter_mut().find(|e| e.user_id == user_id) {
//         // improve if higher points, or equal points but faster time
//         match points.cmp(&entry.best_points) {
//             Ordering::Greater => {
//                 entry.best_points = points;
//                 entry.best_time_ms = duration_ms;
//                 entry.last_ts = ts;
//                 entry.display_name = display_name.clone();
//             }
//             Ordering::Equal if duration_ms < entry.best_time_ms => {
//                 entry.best_time_ms = duration_ms;
//                 entry.last_ts = ts;
//                 entry.display_name = display_name.clone();
//             }
//             _ => {}
//         }
//     } else {
//         lb.push(WordleLeaderboardEntry {
//             user_id: user_id.to_string(),
//             display_name,
//             best_points: points,
//             best_time_ms: duration_ms,
//             last_ts: ts,
//         });
//     }
//     // keep it tidy (optional)
//     lb.sort_by(|a, b| b.best_points.cmp(&a.best_points).then(a.best_time_ms.cmp(&b.best_time_ms)));
//     if lb.len() > 1000 { lb.truncate(1000); } // cap
// }

// Fix B: Avoid Ordering entirely
fn update_leaderboard(
    user_id: &str,
    display_name: Option<String>,
    points: i32,
    duration_ms: u64,
    ts: u64,
) {
    let mut lb = LEADERBOARD.write().unwrap();
    if let Some(entry) = lb.iter_mut().find(|e| e.user_id == user_id) {
        if points > entry.best_points
            || (points == entry.best_points && duration_ms < entry.best_time_ms)
        {
            entry.best_points = points;
            entry.best_time_ms = duration_ms;
            entry.last_ts = ts;
            entry.display_name = display_name.clone();
        }
    } else {
        lb.push(WordleLeaderboardEntry {
            user_id: user_id.to_string(),
            display_name,
            best_points: points,
            best_time_ms: duration_ms,
            last_ts: ts,
        });
    }

    lb.sort_by(|a, b| {
        b.best_points
            .cmp(&a.best_points)
            .then(a.best_time_ms.cmp(&b.best_time_ms))
    });
    if lb.len() > 1000 {
        lb.truncate(1000);
    }
}

// /// Submit a guess for a Wordle session
// async fn submit_wordle_guess(path: web::Path<String>, req: web::Json<WordleGuessRequest>, data: web::Data<AppState>) -> Result<HttpResponse> {
//     let session_id = path.into_inner();
//     let request = req.into_inner();

//     // Basic rate limiting check
//     {
//         let mut rate_limits = data.wordle_rate_limits.write().unwrap();
//         let key = format!("{}:{}", session_id, request.user_id);
//         let current_time = AppState::current_timestamp();

//         if let Some(limit) = rate_limits.get_mut(&key) {
//             if current_time - limit.last_guess < 1000 { // 1 second rate limit
//                 return Ok(HttpResponse::TooManyRequests().json(
//                     serde_json::json!({"error": "Rate limited: 1 guess per second"})
//                 ));
//             }
//             limit.last_guess = current_time;
//         } else {
//             rate_limits.insert(key, WordleRateLimit {
//                 last_guess: current_time,
//                 guess_count: 1,
//                 window_start: current_time,
//             });
//         }
//     }

//     let mut sessions = data.wordle_sessions.write().unwrap();
//     if let Some(session) = sessions.get_mut(&session_id) {
//         // Validate guess (basic A-Z only validation)
//         let guess = request.guess.to_uppercase();
//         if guess.len() != 5 || !guess.chars().all(|c| c.is_ascii_alphabetic()) {
//             return Ok(HttpResponse::BadRequest().json(
//                 serde_json::json!({"error": "Guess must be 5 letters A-Z only"})
//             ));
//         }

//         // Generate marks (simplified Wordle logic)
//         let mut marks = Vec::new();
//         let solution_chars: Vec<char> = session.solution.chars().collect();
//         let guess_chars: Vec<char> = guess.chars().collect();

//         for (i, &guess_char) in guess_chars.iter().enumerate() {
//             if i < solution_chars.len() && guess_char == solution_chars[i] {
//                 marks.push('c'); // correct position
//             } else if solution_chars.contains(&guess_char) {
//                 marks.push('p'); // present but wrong position
//             } else {
//                 marks.push('a'); // absent
//             }
//         }

//         let won = marks.iter().all(|&m| m == 'c');
//         let attempt = session.guesses.iter().filter(|g| g.user_id == request.user_id).count() as u32 + 1;
//         let lost = attempt >= 6 && !won;

//         let wordle_guess = WordleGuess {
//             user_id: request.user_id.clone(),
//             guess: guess.clone(),
//             marks: marks.clone(),
//             attempt,
//             ts: AppState::current_timestamp(),
//             won,
//             lost,
//         };

//         session.guesses.push(wordle_guess.clone());

//         // Broadcast guess event via SSE
//         let guess_event = WordleSSEEvent {
//             event_type: "guess".to_string(),
//             session_id: session_id.clone(),
//             participant: None,
//             guess: Some(wordle_guess),
//             state: None,
//             ts: AppState::current_timestamp(),
//         };

//         // Send to SSE clients
//         if let Some(clients) = data.wordle_sse_clients.read().unwrap().get(&session_id) {
//             for client in clients {
//                 let _ = client.send(guess_event.clone());
//             }
//         }

//         if won || lost {
//             session.status = "completed".to_string();

//             // Broadcast state change event
//             let state_event = WordleSSEEvent {
//                 event_type: "state".to_string(),
//                 session_id: session_id.clone(),
//                 participant: None,
//                 guess: None,
//                 state: Some(WordleSessionStateResponse {
//                     card_index: session.card_index,
//                     solution_hash: session.solution_hash.clone(),
//                     participants: session.participants.clone(),
//                     guesses: session.guesses.clone(),
//                     status: session.status.clone(),
//                 }),
//                 ts: AppState::current_timestamp(),
//             };

//             if let Some(clients) = data.wordle_sse_clients.read().unwrap().get(&session_id) {
//                 for client in clients {
//                     let _ = client.send(state_event.clone());
//                 }
//             }
//         }

//         let response = WordleGuessResponse {
//             ok: true,
//             marks,
//             won,
//             lost,
//             attempt,
//             ts: AppState::current_timestamp(),
//         };

//         log::info!("Wordle guess submitted: {} by {} in session {}", guess, request.user_id, session_id);
//         Ok(HttpResponse::Ok().json(response))
//     } else {
//         Ok(HttpResponse::NotFound().json(serde_json::json!({"error": "Session not found"})))
//     }
// }

/// Submit a guess for a Wordle session
async fn submit_wordle_guess(
    path: web::Path<String>,
    req: web::Json<WordleGuessRequest>,
    data: web::Data<AppState>,
) -> Result<HttpResponse> {
    let session_id = path.into_inner();
    let request = req.into_inner();

    // Basic rate limiting (1 guess/sec per user per session)
    {
        let mut rate_limits = data.wordle_rate_limits.write().unwrap();
        let key = format!("{}:{}", &session_id, &request.user_id);
        let now = AppState::current_timestamp();

        if let Some(limit) = rate_limits.get_mut(&key) {
            if now - limit.last_guess < 1000 {
                return Ok(HttpResponse::TooManyRequests().json(
                    serde_json::json!({"error": "Rate limited: 1 guess per second"})
                ));
            }
            limit.last_guess = now;
            limit.guess_count = limit.guess_count.saturating_add(1);
        } else {
            rate_limits.insert(key, WordleRateLimit {
                last_guess: now,
                guess_count: 1,
                window_start: now,
            });
        }
    }

    let mut sessions = data.wordle_sessions.write().unwrap();
    if let Some(session) = sessions.get_mut(&session_id) {
        // Use one timestamp for this request
        let ts = AppState::current_timestamp();

        // Validate guess
        let guess = request.guess.to_uppercase();
        // If you have per-session length:
        let expected_len: usize = 5; // fallback
        let expected_len = expected_len; // replace with session.wordlist.word_len as usize if present
        if guess.len() != expected_len || !guess.chars().all(|c| c.is_ascii_alphabetic()) {
            return Ok(HttpResponse::BadRequest().json(
                serde_json::json!({"error": format!("Guess must be {} letters A-Z only", expected_len)})
            ));
        }

        // Mark letters (simple logic; refine for duplicate handling if needed)
        let mut marks = Vec::with_capacity(expected_len);
        let solution_chars: Vec<char> = session.solution.chars().collect();
        let guess_chars: Vec<char> = guess.chars().collect();

        for (i, &g) in guess_chars.iter().enumerate() {
            if i < solution_chars.len() && g == solution_chars[i] {
                marks.push('c'); // correct
            } else if solution_chars.contains(&g) {
                marks.push('p'); // present
            } else {
                marks.push('a'); // absent
            }
        }

        let won = marks.iter().all(|&m| m == 'c');
        // user's Nth attempt (count existing guesses by this user)
        let attempt = session.guesses.iter()
            .filter(|g| g.user_id == request.user_id)
            .count() as u32 + 1;
        let lost = attempt >= 6 && !won;

        let wordle_guess = WordleGuess {
            user_id: request.user_id.clone(),
            guess: guess.clone(),
            marks: marks.clone(),
            attempt,
            ts,
            won,
            lost,
        };

        session.guesses.push(wordle_guess.clone());

        // Broadcast guess via SSE
        let guess_event = WordleSSEEvent {
            event_type: "guess".to_string(),
            session_id: session_id.clone(),
            participant: None,
            guess: Some(wordle_guess.clone()),
            state: None,
            ts,
        };
        if let Some(clients) = data.wordle_sse_clients.read().unwrap().get(&session_id) {
            for client in clients {
                let _ = client.send(guess_event.clone());
            }
        }

     let ts = AppState::current_timestamp();

if won {
    let ts = AppState::current_timestamp();

    // ensure started_at is set (e.g., on the first guess)
    if session.started_at.is_none() {
        session.started_at = Some(ts);
    }

    let duration_ms = session.started_at
        .map(|st| ts.saturating_sub(st))
        .unwrap_or(0);

    // attempts for this user
    let attempts = session.guesses.iter()
        .filter(|g| g.user_id == request.user_id)
        .count() as u32 + 1;

    // compute with the scoring rules
    let points = compute_points(&session.rules.scoring, attempts, duration_ms);
    // find display name for leaderboard
    let display_name = session
        .participants
        .iter()
        .find(|p| p.user_id == request.user_id)
        .and_then(|p| p.display_name.clone());

    update_leaderboard(&request.user_id, display_name, points, duration_ms, ts);
    session.ended_at = Some(ts);
}

if won || lost {
    session.status = "completed".to_string();
            // Broadcast state change
            let state_event = WordleSSEEvent {
                event_type: "state".to_string(),
                session_id: session_id.clone(),
                participant: None,
                guess: None,
                state: Some(WordleSessionStateResponse {
                    card_index: session.card_index,
                    solution_hash: session.solution_hash.clone(),
                    participants: session.participants.clone(),
                    guesses: session.guesses.clone(),
                    status: session.status.clone(),
                }),
                ts,
            };
            if let Some(clients) = data.wordle_sse_clients.read().unwrap().get(&session_id) {
                for client in clients {
                    let _ = client.send(state_event.clone());
                }
            }
        }

        let response = WordleGuessResponse {
            ok: true,
            marks,
            won,
            lost,
            attempt,
            ts,
        };

        log::info!(
            "Wordle guess submitted: {} by {} in session {}",
            guess, request.user_id, session_id
        );
        Ok(HttpResponse::Ok().json(response))
    } else {
        Ok(HttpResponse::NotFound().json(serde_json::json!({"error": "Session not found"})))
    }
}

/// Get Wordle session state
async fn get_wordle_session_state(path: web::Path<String>, data: web::Data<AppState>) -> Result<HttpResponse> {
    let session_id = path.into_inner();

    let sessions = data.wordle_sessions.read().unwrap();
    if let Some(session) = sessions.get(&session_id) {
        let response = WordleSessionStateResponse {
            card_index: session.card_index,
            solution_hash: session.solution_hash.clone(),
            participants: session.participants.clone(),
            guesses: session.guesses.clone(),
            status: session.status.clone(),
        };
        Ok(HttpResponse::Ok().json(response))
    } else {
        Ok(HttpResponse::NotFound().json(serde_json::json!({"error": "Session not found"})))
    }
}

/// SSE stream for live Wordle session updates
async fn stream_wordle_session(path: web::Path<String>, data: web::Data<AppState>) -> Result<HttpResponse> {
    let session_id = path.into_inner();

    // Verify session exists
    {
        let sessions = data.wordle_sessions.read().unwrap();
        if !sessions.contains_key(&session_id) {
            return Ok(HttpResponse::NotFound().json(serde_json::json!({"error": "Session not found"})));
        }
    }

    let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<WordleSSEEvent>();

    // Add client to SSE client tracking
    {
        let mut clients = data.wordle_sse_clients.write().unwrap();
clients.entry(session_id.clone()).or_default().push(tx);
    }

    // Send initial state event
    if let Some(session) = data.wordle_sessions.read().unwrap().get(&session_id) {
        let initial_state = WordleSessionStateResponse {
            card_index: session.card_index,
            solution_hash: session.solution_hash.clone(),
            participants: session.participants.clone(),
            guesses: session.guesses.clone(),
            status: session.status.clone(),
        };

        let initial_event = WordleSSEEvent {
            event_type: "state".to_string(),
            session_id: session_id.clone(),
            participant: None,
            guess: None,
            state: Some(initial_state),
            ts: AppState::current_timestamp(),
        };

        let _ = data.wordle_sse_clients
            .read().unwrap()
            .get(&session_id)
            .map(|clients| {
                for client in clients {
                    let _ = client.send(initial_event.clone());
                }
            });
    }

    use futures_util::stream::unfold;

    let stream = unfold(rx, |mut rx| async move {
        match rx.recv().await {
            Some(event) => {
                if let Ok(json) = serde_json::to_string(&event) {
                    let sse_data = format!("data: {}\n\n", json);
                    Some((Ok::<_, actix_web::Error>(web::Bytes::from(sse_data)), rx))
                } else {
                    None
                }
            }
            None => None,
        }
    });

    Ok(HttpResponse::Ok()
        .content_type("text/event-stream")
        .insert_header(("Cache-Control", "no-cache"))
        .insert_header(("Connection", "keep-alive"))
        .insert_header(("Access-Control-Allow-Origin", "*"))
        .streaming(stream))
}

/// Leave a Wordle session
async fn leave_wordle_session(path: web::Path<String>, req: web::Json<WordleLeaveRequest>, data: web::Data<AppState>) -> Result<HttpResponse> {
    let session_id = path.into_inner();
    let request = req.into_inner();

    let mut sessions = data.wordle_sessions.write().unwrap();
    if let Some(session) = sessions.get_mut(&session_id) {
        // Update participant status to "left"
        for participant in &mut session.participants {
            if participant.user_id == request.user_id {
                participant.status = Some("left".to_string());
                participant.last_seen = AppState::current_timestamp();
                break;
            }
        }

        // Broadcast leave event via SSE
        if let Some(participant) = session.participants.iter().find(|p| p.user_id == request.user_id) {
            let leave_event = WordleSSEEvent {
                event_type: "leave".to_string(),
                session_id: session_id.clone(),
                participant: Some(participant.clone()),
                guess: None,
                state: None,
                ts: AppState::current_timestamp(),
            };

            // Send to SSE clients
            if let Some(clients) = data.wordle_sse_clients.read().unwrap().get(&session_id) {
                for client in clients {
                    let _ = client.send(leave_event.clone());
                }
            }
        }

        log::info!("User {} left Wordle session {}", request.user_id, session_id);
        Ok(HttpResponse::Ok().json(serde_json::json!({"status": "left"})))
    } else {
        Ok(HttpResponse::NotFound().json(serde_json::json!({"error": "Session not found"})))
    }
}

/// Periodic backup task that runs at configurable intervals with timeout protection
async fn start_periodic_backup(app_state: web::Data<AppState>, interval_seconds: u64) {
    let mut interval = interval(Duration::from_secs(interval_seconds));

    loop {
        interval.tick().await;

        log::debug!("Starting periodic backup...");

        // Run backup in a separate task with timeout to prevent blocking the main thread
        let backup_result = tokio::time::timeout(
            Duration::from_secs(60), // 1 minute timeout for backup
            tokio::task::spawn_blocking({
                let app_state = app_state.clone();
                move || app_state.export_to_backup()
            })
        ).await;

        match backup_result {
            Ok(Ok(Ok(()))) => {
                log::debug!("Periodic backup completed successfully");
            }
            Ok(Ok(Err(e))) => {
                log::error!("Failed to create periodic backup: {}", e);
            }
            Ok(Err(e)) => {
                log::error!("Periodic backup task panicked: {}", e);
            }
            Err(_) => {
                log::error!("Periodic backup timed out (>60s), skipping this cycle");
            }
        }
    }
}

/// Signal handler for graceful shutdown
async fn handle_shutdown_signals(app_state: web::Data<AppState>) {
    let mut sigterm = signal::unix::signal(signal::unix::SignalKind::terminate())
        .expect("Failed to create SIGTERM handler");
    let mut sigint = signal::unix::signal(signal::unix::SignalKind::interrupt())
        .expect("Failed to create SIGINT handler");

    tokio::select! {
        _ = sigterm.recv() => {
            log::info!("Received SIGTERM, attempting graceful shutdown...");
        }
        _ = sigint.recv() => {
            log::info!("Received SIGINT (Ctrl+C), attempting graceful shutdown...");
        }
    }

    // Try to save database with timeout
    log::info!("Attempting to save database before shutdown...");

    // Use tokio task with timeout to prevent hanging
    let backup_result = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        tokio::task::spawn_blocking({
            let app_state = app_state.clone();
            move || app_state.export_to_backup()
        })
    ).await;

    match backup_result {
        Ok(Ok(Ok(()))) => {
            log::info!("Database saved successfully during shutdown");
        }
        Ok(Ok(Err(e))) => {
            log::error!("Failed to save database during shutdown: {}", e);
        }
        Ok(Err(e)) => {
            log::error!("Backup task panicked during shutdown: {}", e);
        }
        Err(_) => {
            log::warn!("Database backup timed out during shutdown (>10s), forcing exit");
        }
    }

    log::info!("Shutdown complete, exiting process...");

    // Flush logs and force exit
    eprintln!("Graceful shutdown complete, forcing exit...");

    // Use process exit instead of unsafe libc call
    std::process::exit(0);
}

/// Initialize database with error recovery
fn initialize_database_with_recovery(db_path: &str, backup_path: &str, config: &Config) -> Result<sled::Db, Box<dyn std::error::Error>> {

    // Try to open the existing database
    match sled::open(db_path) {
        Ok(db) => {
            log::info!("Database opened successfully");
            return Ok(db);
        }
        Err(e) => {
            log::warn!("Failed to open database: {}. Attempting recovery...", e);
        }
    }

    // If database is corrupted, remove it and create a new one
    if Path::new(db_path).exists() {
        log::info!("Removing corrupted database directory...");
        if let Err(e) = fs::remove_dir_all(db_path) {
            log::error!("Failed to remove corrupted database: {}", e);
        }
    }

    // Create a new database
    let db = sled::open(db_path)?;
    log::info!("New database created");

    // Try to restore from backup
    if Path::new(backup_path).exists() {
        log::info!("Backup file found, attempting to restore...");
        let temp_state = AppState::new(db, backup_path.to_string(), config);
        if let Err(e) = temp_state.import_from_backup() {
            log::error!("Failed to restore from backup: {}", e);
            log::info!("Initializing with defaults...");
            temp_state.initialize_defaults()?;
        }
        let db = temp_state.db.read().unwrap().clone();
        Ok(db)
    } else {
        log::info!("No backup found, initializing with defaults...");
        let temp_state = AppState::new(db, backup_path.to_string(), config);
        temp_state.initialize_defaults()?;
        let db = temp_state.db.read().unwrap().clone();
        Ok(db)
    }
}


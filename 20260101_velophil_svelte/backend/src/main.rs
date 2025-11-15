// Clean minimal main.rs - Auth-focused backend
use actix_cors::Cors;
use actix_files::Files;
use actix_web::{
    middleware::Logger,
    web::{self, JsonConfig},
    App, HttpResponse, HttpServer, Responder,
    error::JsonPayloadError,
};
use std::sync::Arc;

// Module declarations
mod backup;
mod cli;
mod config;
mod db;
mod db_manager;
mod handlers;
mod logging;
mod middleware;
mod models;
mod replicate;
mod routes;
mod time;
mod types;
mod validation;

// Imports from our modules
use backup::BackupManager;
use cli::Cli;
// use config::AppConfig;
use db::Database;
use replicate::Replicator;

/// Simple index route
async fn index() -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({
        "name": "Backend API",
        "version": env!("CARGO_PKG_VERSION"),
        "status": "running"
    }))
}

/// JSON error handler for better error messages
fn json_error_handler(err: JsonPayloadError, _req: &actix_web::HttpRequest) -> actix_web::Error {
    use types::ErrorResponse;
    let error_response = ErrorResponse::new(
        "json_parse_error",
        format!("Invalid JSON: {}", err)
    );
    let body = HttpResponse::BadRequest().json(error_response);
    actix_web::error::InternalError::from_response(err, body).into()
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Parse CLI arguments
    let cli = Cli::parse_args();

    // Initialize logging
    logging::init_logging(cli.verbose).expect("Failed to initialize logging");
    logging::print_build_info();

    // Load configuration
    let cfg = config::load_config_from_file(&cli.config);

    // Handle CLI commands (admin user creation, backup, etc.)
    if let Some(command) = &cli.command {
        match command {
            cli::Commands::User { action } => {
                use cli::UserCommands;
                use argon2::{Argon2, password_hash::SaltString, PasswordHasher};
                use rand_core::OsRng;

                match action {
                    UserCommands::AddAdmin(args) => {
                        let db = Database::new(&cfg.sled_path).expect("Failed to open database");

                        let email = args.email.trim().to_lowercase();
                        let users: Vec<models::auth_types::UserRecord> = db.list("users").unwrap_or_default();

                        if users.iter().any(|u| u.email == email) {
                            eprintln!("Error: User with email '{}' already exists", email);
                            std::process::exit(2);
                        }

                        let salt = SaltString::generate(&mut OsRng);
                        let hash = Argon2::default()
                            .hash_password(args.password.as_bytes(), &salt)
                            .expect("Failed to hash password")
                            .to_string();

                        let admin = models::auth_types::UserRecord::new_admin(&email, hash);
                        db.insert("users", &admin.id, &admin).expect("Failed to insert admin user");

                        println!("✓ Admin user created: {}", email);
                        println!("  ID: {}", admin.id);
                        println!("  Roles: {:?}", admin.roles);
                        return Ok(());
                    }
                }
            }
            cli::Commands::Db { action } => {
                use cli::DbCommands;

                match action {
                    DbCommands::Dump { output, .. } => {
                        println!("Database dump functionality coming soon: {}", output);
                        return Ok(());
                    }
                    DbCommands::Test => {
                        println!("Testing database connection...");
                        let _db = Database::new(&cfg.sled_path).expect("Failed to open database");
                        println!("✓ Database connection successful");
                        return Ok(());
                    }
                    _ => {
                        println!("Command not yet implemented");
                        return Ok(());
                    }
                }
            }
            _ => {
                eprintln!("Unknown command");
                std::process::exit(1);
            }
        }
    }

    // Initialize database
    let database = Database::new(&cfg.sled_path).expect("Failed to open database");

    // Setup PostgreSQL replication (optional)
    let replicator = if cfg.database_sync_on && !cfg.pg_conns.is_empty() {
        let conn_strings: Vec<String> = cfg.pg_conns.iter().map(|c| c.conn_string.clone()).collect();
        let routes: std::collections::HashMap<String, Vec<usize>> = std::collections::HashMap::new();

        match Replicator::new(&conn_strings, routes).await {
            Ok(rep) => {
                log::info!("PostgreSQL replication enabled");
                Some(Arc::new(rep))
            }
            Err(e) => {
                log::warn!("Failed to initialize replicator: {}", e);
                None
            }
        }
    } else {
        None
    };

    let database = database.with_replicator(replicator);

    // Setup backup manager
    let backup_manager = Arc::new(BackupManager::new(
        &cfg.sled_path,
        &cfg.backup_dir,
        &cfg.backup_name_template,
    ));

    // Start periodic backup task
    if let Some(interval) = cfg.backup_interval {
        let backup_mgr = backup_manager.clone();
        let retention = cfg.backup_retention;
        tokio::spawn(async move {
            backup_mgr.run(interval, retention).await;
        });
        log::info!("Periodic backups enabled: interval={:?}, retention={}", interval, retention);
    }

    // Prepare server address
    let bind_address = format!("{}:{}", cfg.server.host, cfg.server.port);
    log::info!("Starting server on {}", bind_address);
    log::info!("Database path: {}", cfg.sled_path);
    log::info!("Backup path: {}", cfg.backup_dir);

    // Wrap shared state
    let db_data = web::Data::new(database);
    let cfg_data = web::Data::new(cfg.clone());

    // Clone CORS rules for use in the HttpServer closure
    let cors_rules = cfg.cors_rules.clone();

    // Start HTTP server
    HttpServer::new(move || {
        // Configure CORS
        let rules_clone = cors_rules.clone();
        let cors = Cors::default()
            .allowed_origin_fn(move |origin, _req| {
                let origin_str = origin.to_str().unwrap_or("");
                config::is_origin_allowed(&rules_clone, origin_str)
            })
            .allow_any_method()
            .allow_any_header()
            .supports_credentials()
            .max_age(3600);

        App::new()
            // Configure JSON limits and error handling
            .app_data(JsonConfig::default()
                .limit(1024 * 1024) // 1MB limit
                .error_handler(json_error_handler))

            // Shared application state
            .app_data(db_data.clone())
            .app_data(cfg_data.clone())

            // Middleware
            .wrap(middleware::security::SecurityHeaders)
            .wrap(cors)
            .wrap(Logger::default())

            // Health check endpoints
            .service(routes::health::healthz)
            .service(routes::health::health)

            // WASM file serving
            .service(routes::static_files::serve_wasm)

            // API routes
            .service(
                web::scope("/api")
                    // API info route
                    .route("/", web::get().to(index))
                    // Auth routes (public)
                    .service(
                        web::scope("/auth")
                            .service(handlers::auth::register)
                            .service(handlers::auth::login)
                            .service(handlers::auth::logout)
                            .service(handlers::auth::refresh)
                            .service(handlers::auth::reconfirm)
                            .service(handlers::auth::me)
                    )
                    // Protected routes (require authentication)
                    .service(
                        web::scope("")
                            .wrap(actix_web::middleware::from_fn(handlers::auth::guard_api))
                            // User management (admin only)
                            .service(handlers::users::list_users)
                            .service(handlers::users::get_user)
                            .service(handlers::users::update_user_roles)
                            // Add your business routes here
                    )
            )

            // Static files and SPA fallback (must be last)
            .service(Files::new("/", "./static")
                .index_file("index.html")
                .default_handler(web::to(routes::static_files::spa_fallback)))
    })
    .bind(&bind_address)?
    .run()
    .await
}

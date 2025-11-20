use std::io;
use tracing_subscriber::{
    fmt::{self, time::ChronoUtc},
    prelude::*,
    EnvFilter,
};

/// Initialize the logging system with colors and build information
pub fn init_logging(verbose: bool) -> Result<(), Box<dyn std::error::Error>> {
    let filter = if verbose {
        "debug,quoteflow_backend=trace,actix_web=debug"
    } else {
        "info,quoteflow_backend=info,actix_web=info"
    };

    let env_filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new(filter))
        .unwrap();

    let use_ansi = atty::is(atty::Stream::Stdout);
    let fmt_layer = fmt::layer()
        .with_target(true)
        .with_thread_ids(true)
        .with_thread_names(true)
        .with_timer(ChronoUtc::rfc_3339())
        .with_ansi(use_ansi)
        .with_file(true)
        .with_line_number(true)
        .with_writer(io::stdout);

    // Forward log crate records to tracing
    let _ = tracing_log::LogTracer::init();

    let _ = tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer)
        .try_init();

    // First log line: PID
    tracing::info!("PID={} starting up", std::process::id());

    Ok(())
}

/// Print build and version information with colors
pub fn print_build_info() {
    let name = env!("CARGO_PKG_NAME");
    let pkg_version = env!("CARGO_PKG_VERSION");
    let version = std::env::var("APP_BUILD_VERSION").unwrap_or_else(|_| pkg_version.to_string());
    let build_timestamp =
        std::env::var("VERGEN_BUILD_TIMESTAMP").unwrap_or_else(|_| chrono::Utc::now().to_rfc3339());
    let git_branch = std::env::var("VERGEN_GIT_BRANCH").unwrap_or_else(|_| "no-git".into());
    let git_sha_full = std::env::var("VERGEN_GIT_SHA").unwrap_or_else(|_| "00000000".into());
    let git_commit = git_sha_full.chars().take(8).collect::<String>();
    let rust_version = std::env::var("VERGEN_RUSTC_SEMVER")
        .unwrap_or_else(|_| env!("CARGO_PKG_VERSION").to_string());
    let desc = std::env::var("APP_PKG_DESCRIPTION").unwrap_or_else(|_| String::new());
    let bin_name = std::env::var("APP_BIN_FILENAME").unwrap_or_else(|_| String::new());

    println!("{}", "═".repeat(60));
    println!("{} v{}", name, version);
    if !desc.is_empty() {
        println!("Description: {}", desc);
    }
    if !bin_name.is_empty() {
        println!("Binary: {}", bin_name);
    }
    println!("{}", "─".repeat(60));
    println!("Build: {}", build_timestamp);
    println!("Git: {} ({})", git_branch, git_commit);
    println!("Rust: {}", rust_version);
    println!("{}", "═".repeat(60));
    println!();
}

/// Log database connection status with colors
#[allow(dead_code)]
pub fn log_database_status(connected: bool, db_type: &str, details: &str) {
    if connected {
        tracing::info!("✓ Connected to {} ({})", db_type, details);
    } else {
        tracing::error!("✗ Failed to connect to {} ({})", db_type, details);
    }
}

/// Log server startup information
pub fn log_server_startup(host: &str, port: u16) {
    tracing::info!("🚀 Server starting on http://{}:{}", host, port);
}

/// Log CLI command execution
pub fn log_command_start(command: &str, description: &str) {
    tracing::info!("⚡ Executing: {} ({})", command, description);
}

/// Log command completion
pub fn log_command_complete(command: &str, success: bool, duration: std::time::Duration) {
    let status_icon = if success { "✅" } else { "❌" };
    let status_text = if success { "completed" } else { "failed" };
    let status_color = if success { "green" } else { "red" };

    match status_color {
        "green" => tracing::info!(
            "{} Command '{}' {} in {:.2?}",
            status_icon,
            command,
            status_text,
            duration
        ),
        _ => tracing::error!(
            "{} Command '{}' {} after {:.2?}",
            status_icon,
            command,
            status_text,
            duration
        ),
    }
}

/// Log table operation status
#[allow(dead_code)]
pub fn log_table_operation(operation: &str, table: &str, count: Option<usize>, success: bool) {
    let icon = match operation {
        "create" | "seed" => "📝",
        "drop" => "🗑️",
        "dump" => "💾",
        "import" => "📥",
        _ => "🔄",
    };

    if success {
        let count_text = count
            .map(|c| format!(" ({} records)", c))
            .unwrap_or_default();
        tracing::info!(
            "{} {} table {}{}",
            icon,
            operation.to_uppercase(),
            table,
            count_text
        );
    } else {
        tracing::error!("{} Failed to {} table {}", "❌", operation, table);
    }
}

/// Log configuration loading
#[allow(dead_code)]
pub fn log_config_loaded(config_file: &str, settings_count: usize) {
    tracing::info!(
        "⚙️ Configuration loaded from {} ({} settings)",
        config_file,
        settings_count
    );
}

/// Log warning with icon
pub fn log_warning(message: &str) {
    tracing::warn!("⚠️ {}", message);
}

/// Log error with icon
pub fn log_error(message: &str) {
    tracing::error!("❌ {}", message);
}

/// Log success with icon
#[allow(dead_code)]
pub fn log_success(message: &str) {
    tracing::info!("✅ {}", message);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_logging_initialization() {
        // Test that logging initialization doesn't panic
        let result = init_logging(false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_build_info_display() {
        // This test just ensures the function doesn't panic
        // In a real scenario, we'd capture stdout and verify output
        print_build_info();
    }
}

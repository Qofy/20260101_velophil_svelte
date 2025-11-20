use clap::{Args, Parser, Subcommand};

/// QuoteFlow Backend - A comprehensive quote and invoice management system
#[derive(Parser, Clone)]
#[command(name = "description_backend")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "QuoteFlow Backend Server with Database Management")]
#[command(long_about = None)]
pub struct Cli {
    /// Enable verbose logging
    #[arg(short, long, default_value_t = false)]
    pub verbose: bool,

    /// Configuration file path
    #[arg(short, long, default_value = ".env")]
    pub config: String,

    /// Database connection string override
    #[arg(long, env = "DATABASE_URL")]
    pub database_url: Option<String>,

    /// Server port override
    #[arg(short, long, env = "PORT")]
    pub port: Option<u16>,

    /// Server host override
    #[arg(long, env = "HOST", default_value = "127.0.0.1")]
    pub host: String,

    // Compatibility flags (map to subcommands)
    #[arg(long = "dbtest", hide = true)]
    pub compat_dbtest: bool,
    #[arg(long = "dbseed", hide = true)]
    pub compat_dbseed: bool,
    #[arg(long = "dbdump", hide = true)]
    pub compat_dbdump_output: Option<String>,
    #[arg(long = "dbimport", hide = true)]
    pub compat_dbimport_input: Option<String>,
    #[arg(long = "generate-initial-sql", hide = true)]
    pub compat_generate_sql_output: Option<String>,
    #[arg(long, hide = true)]
    pub include_sample_data: bool,
    #[arg(long = "dbreset", hide = true)]
    pub compat_dbreset: bool,
    #[arg(long, hide = true)]
    pub confirm: bool,

    #[command(subcommand)]
    pub command: Option<Commands>,

    #[arg(long)]
    pub add_admin_email: Option<String>,
    #[arg(long)]
    pub add_admin_password: Option<String>,
}

#[derive(Subcommand, Clone)]
pub enum Commands {
    /// Start the web server (default action)
    Serve {
        /// Force seed database on startup
        #[arg(long)]
        seed: bool,
    },
    /// Database management commands
    Db {
        #[command(subcommand)]
        action: DbCommands,
    },
    User {
        #[command(subcommand)]
        action: UserCommands,
    },
}

#[derive(Subcommand, Clone)]
pub enum DbCommands {
    /// Test database connection
    Test,
    /// Seed database with initial data
    Seed {
        /// Force seed even if data exists
        #[arg(long)]
        force: bool,
        /// Specific tables to seed (comma-separated)
        #[arg(long)]
        tables: Option<String>,
    },
    /// Export database to SQL file
    Dump {
        /// Output file path
        #[arg(short, long, default_value = "quoteflow_dump.sql")]
        output: String,
        /// Specific tables to dump (comma-separated)
        #[arg(long)]
        tables: Option<String>,
        /// Include data in dump
        #[arg(long, default_value = "true")]
        data: bool,
        /// Include schema in dump
        #[arg(long, default_value = "true")]
        schema: bool,
    },
    /// Import database from SQL file
    Import {
        /// Input SQL file path
        #[arg(short, long)]
        input: String,
        /// Drop existing tables before import
        #[arg(long)]
        drop_existing: bool,
    },
    /// Generate initial SQL schema without executing
    GenerateInitialSql {
        /// Output file path
        #[arg(short, long, default_value = "initial_schema.sql")]
        output: String,
        /// Include sample data
        #[arg(long)]
        include_sample_data: bool,
    },
    /// Run database migrations
    Migrate {
        /// Migration direction (up/down)
        #[arg(long, default_value = "up")]
        direction: String,
        /// Specific migration version
        #[arg(long)]
        target: Option<String>,
    },
    /// Reset database (drop all tables)
    Reset {
        /// Confirm reset action
        #[arg(long)]
        confirm: bool,
    },
}

#[derive(Subcommand, Debug, Clone)]
pub enum UserCommands {
    AddAdmin(AddAdminArgs),
}

#[derive(Args, Debug, Clone)]
pub struct AddAdminArgs {
    #[arg(long)]
    pub email: String,
    #[arg(long)]
    pub password: String,
}

impl Cli {
    pub fn parse_args() -> Self {
        Self::parse()
    }

    pub fn effective_command(&self) -> Option<Commands> {
        if let Some(cmd) = &self.command {
            return Some(cmd.clone());
        }
        // Compatibility layer
        if self.compat_dbtest {
            return Some(Commands::Db {
                action: DbCommands::Test,
            });
        }
        if self.compat_dbseed {
            return Some(Commands::Db {
                action: DbCommands::Seed {
                    force: false,
                    tables: None,
                },
            });
        }
        if let Some(out) = &self.compat_dbdump_output {
            return Some(Commands::Db {
                action: DbCommands::Dump {
                    output: out.clone(),
                    tables: None,
                    data: true,
                    schema: true,
                },
            });
        }
        if let Some(inp) = &self.compat_dbimport_input {
            return Some(Commands::Db {
                action: DbCommands::Import {
                    input: inp.clone(),
                    drop_existing: true,
                },
            });
        }
        if let Some(out) = &self.compat_generate_sql_output {
            return Some(Commands::Db {
                action: DbCommands::GenerateInitialSql {
                    output: out.clone(),
                    include_sample_data: self.include_sample_data,
                },
            });
        }
        if self.compat_dbreset {
            return Some(Commands::Db {
                action: DbCommands::Reset {
                    confirm: self.confirm,
                },
            });
        }
        None
    }

    #[allow(dead_code)]
    pub fn is_server_mode(&self) -> bool {
        matches!(self.command, None | Some(Commands::Serve { .. }))
            && self.effective_command().is_none()
            && self.add_admin_email.is_none()
    }

    pub fn should_seed_on_startup(&self) -> bool {
        match &self.command {
            Some(Commands::Serve { seed }) => *seed,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_default_values() {
        let cli = Cli::parse_from(&["description_backend"]);
        assert!(!cli.verbose);
        assert_eq!(cli.config, ".env");
        assert_eq!(cli.host, "127.0.0.1");
        assert!(cli.is_server_mode());
    }

    #[test]
    fn test_db_test_command() {
        let cli = Cli::parse_from(&["description_backend", "db", "test"]);
        assert!(!cli.is_server_mode());
        match cli.command {
            Some(Commands::Db {
                action: DbCommands::Test,
            }) => {}
            _ => panic!("Expected db test command"),
        }
    }

    #[test]
    fn test_db_seed_command() {
        let cli = Cli::parse_from(&["description_backend", "db", "seed", "--force"]);
        match cli.command {
            Some(Commands::Db {
                action: DbCommands::Seed { force, .. },
            }) => {
                assert!(force);
            }
            _ => panic!("Expected db seed command"),
        }
    }
}

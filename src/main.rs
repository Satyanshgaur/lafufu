use lafufu::config::AppConfig;
use lafufu::errors::Result;
use lafufu::observability::init_observability;
use lafufu::storage::sqlite::SqliteStorage;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tracing::{info, error};

#[derive(Parser, Debug)]
#[command(name = "lafufu")]
#[command(about = "Lafufu: Local-first Behavioral Intelligence Platform", long_about = None)]
struct Cli {
    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug, Clone)]
enum Commands {
    /// Show current status and database statistics
    Status,
    
    /// Watch configuration and tail configured log sources (Phase 1)
    Watch,

    /// Generate behavioral explanation report (Phase 4)
    Explain {
        /// Time range for explanation (e.g., 24h, 7d)
        #[arg(short, long, default_value = "24h")]
        since: String,
    },

    /// Replay raw logs to recompute derived baseline metrics (Phase 4)
    Replay {
        /// Path to directory of historical log files
        path: PathBuf,
    },

    /// Export database contents to standard formats (Phase 4)
    Export {
        /// Target table to export (events, entities, edges, baselines)
        #[arg(value_name = "TABLE")]
        table: String,
    },
}

#[tokio::main]
async fn main() {
    // 1. Initialize Logging/Observability
    init_observability();
    info!("Starting Lafufu behavioral intelligence engine...");

    // 2. Load Configuration
    let config = match AppConfig::load_default() {
        Ok(c) => c,
        Err(e) => {
            error!("Initialization failure during configuration load: {}", e);
            std::process::exit(1);
        }
    };

    // 3. Resolve Database Connection
    let db_path = match config.get_db_path() {
        Ok(path) => path,
        Err(e) => {
            error!("Failed to resolve database storage path: {}", e);
            std::process::exit(1);
        }
    };

    let storage = match SqliteStorage::new(db_path.to_str().unwrap()) {
        Ok(s) => s,
        Err(e) => {
            error!("Failed to initialize database connection: {}", e);
            std::process::exit(1);
        }
    };

    // 4. Parse commands
    let cli = Cli::parse();
    match cli.command {
        Commands::Status => {
            if let Err(e) = handle_status(storage).await {
                error!("Command execution failed: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Watch => {
            info!("Continuous tailing and monitoring mode not fully implemented (Phase 1).");
        }
        Commands::Explain { since } => {
            info!("Explanation summaries not fully implemented (Phase 4). Query window: {}", since);
        }
        Commands::Replay { path } => {
            info!("Historical replay not fully implemented (Phase 4). Source: {:?}", path);
        }
        Commands::Export { table } => {
            info!("Data export not fully implemented (Phase 4). Target table: {}", table);
        }
    }
}

async fn handle_status(storage: SqliteStorage) -> Result<()> {
    let conn = storage.conn.lock().unwrap();
    
    let event_count: i64 = conn.query_row("SELECT COUNT(*) FROM events", [], |r| r.get(0))?;
    let entity_count: i64 = conn.query_row("SELECT COUNT(*) FROM entities", [], |r| r.get(0))?;
    let edge_count: i64 = conn.query_row("SELECT COUNT(*) FROM edges", [], |r| r.get(0))?;
    let baseline_count: i64 = conn.query_row("SELECT COUNT(*) FROM baselines", [], |r| r.get(0))?;

    println!("========================================");
    println!(" Lafufu Local Database Status & Primitives");
    println!("========================================");
    println!("  Events:    {}", event_count);
    println!("  Entities:  {}", entity_count);
    println!("  Edges:     {}", edge_count);
    println!("  Baselines: {}", baseline_count);
    println!("========================================");
    println!("Status: Phase 0 DB primitives loaded successfully.");
    Ok(())
}

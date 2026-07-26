use clap::{Parser, Subcommand};
use lafufu::config::AppConfig;
use lafufu::errors::Result;
use lafufu::ingestion::{IngestionPipeline, LogStreamer};
use lafufu::normalization::{IdentityConfig, IdentityResolver};
use lafufu::observability::init_observability;
use lafufu::storage::sqlite::SqliteStorage;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{error, info};

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

    /// Ingest a log file or directory of log files into the behavior graph (Phase 1)
    Ingest {
        /// Path to log file or directory
        path: PathBuf,

        /// Specific log adapter to use (generic_json, syslog_auth, github_events, docker_events)
        #[arg(short, long)]
        adapter: Option<String>,
    },

    /// Tail a target log file continuously and process stream (Phase 1)
    Watch {
        /// File path to tail continuously
        path: Option<PathBuf>,

        /// Specific log adapter to use
        #[arg(short, long)]
        adapter: Option<String>,
    },

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

    let resolver = Arc::new(IdentityResolver::new(IdentityConfig::default()));
    let pipeline = Arc::new(IngestionPipeline::new(storage.clone(), resolver));

    // 4. Parse commands
    let cli = Cli::parse();
    match cli.command {
        Commands::Status => {
            if let Err(e) = handle_status(storage).await {
                error!("Command execution failed: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Ingest { path, adapter } => {
            handle_ingest(pipeline, path, adapter);
        }
        Commands::Watch { path, adapter } => {
            handle_watch(pipeline, path, adapter);
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

fn handle_ingest(pipeline: Arc<IngestionPipeline>, path: PathBuf, adapter: Option<String>) {
    info!("Ingesting logs from path: {:?}", path);
    let result = if path.is_dir() {
        pipeline.process_directory(&path, adapter.as_deref())
    } else {
        pipeline.process_file(&path, adapter.as_deref())
    };

    match result {
        Ok(report) => {
            println!("========================================");
            println!(" Ingestion Completed Successfully");
            println!("========================================");
            println!("  Adapter:   {}", report.adapter_used);
            println!("  Events:    {}", report.events_ingested);
            println!("  Entities:  {}", report.entities_created);
            println!("  Edges:     {}", report.edges_updated);
            println!("========================================");
        }
        Err(e) => {
            error!("Failed to ingest log data: {}", e);
            std::process::exit(1);
        }
    }
}

fn handle_watch(pipeline: Arc<IngestionPipeline>, path: Option<PathBuf>, adapter: Option<String>) {
    let streamer = LogStreamer::new(pipeline);
    if let Some(target_file) = path {
        println!("Tailing log file continuously: {:?}", target_file);
        if let Err(e) = streamer.tail_file(target_file, adapter) {
            error!("Error in stream tailing: {}", e);
            std::process::exit(1);
        }
    } else {
        println!("Watch mode requires a target --path file to tail.");
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
    println!("Status: Behavior graph engine operational.");
    Ok(())
}

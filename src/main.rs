use clap::{Parser, Subcommand};
use lafufu::behavior::BaselineEngine;
use lafufu::config::AppConfig;
use lafufu::detection::DetectionEngine;
use lafufu::errors::Result;
use lafufu::explanation::ExplanationEngine;
use lafufu::ingestion::{IngestionPipeline, LogStreamer};
use lafufu::normalization::{IdentityConfig, IdentityResolver};
use lafufu::observability::init_observability;
use lafufu::repository::EntityRepository;
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

    /// Compute/recompute behavioral baselines across short, medium, and long term temporal layers (Phase 2)
    Baselines,

    /// Detect anomalies, new behaviors, behavior drift, and rank most changed entities (Phase 3)
    Detect {
        /// Time window for detection (e.g. 24h, 7d)
        #[arg(short, long, default_value = "24h")]
        since: String,
    },

    /// Generate natural language briefing report of behavioral changes (Phase 4)
    Explain {
        /// Time range for explanation (e.g., 24h, 7d)
        #[arg(short, long, default_value = "24h")]
        since: String,
    },

    /// Render chronological narrative view of an entity's behavioral history (Phase 4)
    Timeline {
        /// Canonical name of target entity
        entity: String,
    },

    /// Compare an entity's behavioral profile across time horizons (Phase 4)
    Diff {
        /// Canonical name of target entity
        entity: String,
    },

    /// Conversational query interface grounded in behavior graph evidence (Phase 4)
    Ask {
        /// Natural language question
        query: String,
    },

    /// Replay raw logs to recompute derived baseline metrics and detect changes (Phase 4)
    Replay {
        /// Path to directory of historical log files
        path: PathBuf,
    },

    /// Export database contents (entities, events, edges) to standard JSON format (Phase 4)
    Export {
        /// Target table to export (events, entities, edges)
        #[arg(value_name = "TABLE")]
        table: String,
    },

    /// Tail a target log file continuously and process stream (Phase 1)
    Watch {
        /// File path to tail continuously
        path: Option<PathBuf>,

        /// Specific log adapter to use
        #[arg(short, long)]
        adapter: Option<String>,
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
    let engine = Arc::new(BaselineEngine::with_default_windows(storage.clone()));
    let detector = Arc::new(DetectionEngine::new(storage.clone(), engine.clone()));
    let explanation = Arc::new(ExplanationEngine::new(storage.clone(), detector.clone(), engine.clone()));

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
            handle_ingest(pipeline, engine, path, adapter);
        }
        Commands::Baselines => {
            handle_baselines(engine, storage);
        }
        Commands::Detect { since } => {
            handle_detect(detector, since);
        }
        Commands::Explain { since } => {
            match explanation.generate_explain_report(&since) {
                Ok(rep) => print!("{}", rep),
                Err(e) => error!("Failed to generate explain report: {}", e),
            }
        }
        Commands::Timeline { entity } => {
            match explanation.generate_timeline(&entity) {
                Ok(tl) => print!("{}", tl),
                Err(e) => error!("Failed to generate timeline: {}", e),
            }
        }
        Commands::Diff { entity } => {
            match explanation.generate_diff(&entity) {
                Ok(df) => print!("{}", df),
                Err(e) => error!("Failed to generate profile diff: {}", e),
            }
        }
        Commands::Ask { query } => {
            match explanation.process_ask_query(&query) {
                Ok(ans) => print!("{}", ans),
                Err(e) => error!("Failed to process query: {}", e),
            }
        }
        Commands::Replay { path } => {
            handle_replay(pipeline, engine, explanation, path);
        }
        Commands::Export { table } => {
            match explanation.export_table(&table) {
                Ok(json) => println!("{}", json),
                Err(e) => error!("Export failed: {}", e),
            }
        }
        Commands::Watch { path, adapter } => {
            handle_watch(pipeline, path, adapter);
        }
    }
}

fn handle_ingest(
    pipeline: Arc<IngestionPipeline>,
    engine: Arc<BaselineEngine>,
    path: PathBuf,
    adapter: Option<String>,
) {
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

            if report.events_ingested > 0 {
                info!("Updating behavioral baselines post-ingestion...");
                let now = chrono::Utc::now();
                let _ = engine.recompute_all_baselines(now);
            }
        }
        Err(e) => {
            error!("Failed to ingest log data: {}", e);
            std::process::exit(1);
        }
    }
}

fn handle_replay(
    pipeline: Arc<IngestionPipeline>,
    engine: Arc<BaselineEngine>,
    explanation: Arc<ExplanationEngine>,
    path: PathBuf,
) {
    info!("Replaying historical log data from: {:?}", path);
    let res = if path.is_dir() {
        pipeline.process_directory(&path, None)
    } else {
        pipeline.process_file(&path, None)
    };

    if let Ok(rep) = res {
        println!("Replayed {} events successfully.", rep.events_ingested);
        let _ = engine.recompute_all_baselines(chrono::Utc::now());
        if let Ok(summary) = explanation.generate_explain_report("24h") {
            print!("{}", summary);
        }
    } else if let Err(e) = res {
        error!("Replay failed: {}", e);
    }
}

fn handle_baselines(engine: Arc<BaselineEngine>, storage: SqliteStorage) {
    let now = chrono::Utc::now();
    match engine.recompute_all_baselines(now) {
        Ok(count) => {
            println!("========================================");
            println!(" Behavioral Baseline Engine Summary");
            println!("========================================");
            println!("  Entities Profiled: {}", count);
            println!("----------------------------------------");

            if let Ok(entities) = EntityRepository::find_all(&storage) {
                for entity in entities {
                    if let Ok(Some(layers)) = engine.load_temporal_layers(&entity.id) {
                        let drift = layers.calculate_drift();
                        let anomaly = layers.calculate_anomaly();
                        println!(
                            "  - {} ({}) | Drift: {:.4} | Anomaly: {:.4}",
                            entity.canonical_name, entity.entity_type, drift, anomaly
                        );
                    }
                }
            }
            println!("========================================");
        }
        Err(e) => {
            error!("Failed to compute baselines: {}", e);
            std::process::exit(1);
        }
    }
}

fn handle_detect(detector: Arc<DetectionEngine>, since_str: String) {
    let now = chrono::Utc::now();
    let since = if since_str.ends_with('h') {
        let hours: i64 = since_str.trim_end_matches('h').parse().unwrap_or(24);
        now - chrono::Duration::hours(hours)
    } else if since_str.ends_with('d') {
        let days: i64 = since_str.trim_end_matches('d').parse().unwrap_or(7);
        now - chrono::Duration::days(days)
    } else {
        now - chrono::Duration::hours(24)
    };

    println!("========================================");
    println!(" Lafufu Behavioral Detection & Scoring");
    println!(" Window: {}", since_str);
    println!("========================================");

    match detector.detect_observations(since) {
        Ok(obs_list) => {
            println!(" Detected Observations: {}", obs_list.len());
            for obs in &obs_list {
                println!("  [{:?}] {} (Score: {:.2})", obs.category, obs.title, obs.anomaly_score);
                println!("    {}", obs.description);
            }
        }
        Err(e) => error!("Failed to detect observations: {}", e),
    }

    println!("----------------------------------------");
    println!(" Most Changed Entities (Ranked):");
    match detector.get_most_changed_entities(5) {
        Ok(changed) => {
            if changed.is_empty() {
                println!("  (No significant behavioral changes detected)");
            } else {
                for (rank, item) in changed.iter().enumerate() {
                    println!(
                        "  {}. {} ({}) | Change Score: {:.4} (Drift: {:.2}, Anomaly: {:.2})",
                        rank + 1, item.canonical_name, item.entity_type, item.combined_change_score, item.drift_score, item.anomaly_score
                    );
                }
            }
        }
        Err(e) => error!("Failed to get most changed entities: {}", e),
    }
    println!("========================================");
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
    println!("Status: Phase 4 Natural Language Interface & Explanations operational.");
    Ok(())
}

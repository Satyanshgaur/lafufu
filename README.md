# Lafufu 🐾

**Lafufu** is a local-first behavioral intelligence platform built in Rust. It learns how digital systems behave, detects meaningful behavioral changes (anomalies and drift), and explains them in human-readable natural language.

Unlike traditional SIEMs (Security Information and Event Management systems) that focus on collecting raw logs, generating noisy alert rules, and pushing data to cloud databases, Lafufu runs **entirely locally**, remains **offline-capable**, and models every system using a **universal behavior graph**.

---

## 🎯 Vision & Architectural Philosophy

1. **Local-First & Offline-Capable**: Security and audit logs contain your company's most sensitive data—authentication attempts, sudo commands, API calls, and developer activity. Lafufu keeps all data on your local machine in a high-performance SQLite engine. No cloud daemons, no external API dependencies for core analysis.
2. **Universal Behavior Graph**: All incoming logs (Linux auth, macOS syslogs, GitHub webhooks, Docker event streams, CloudTrail) are transformed into a single unified representation:
   $$\text{Entity} \longrightarrow \text{Event} \longrightarrow \text{Context} \longrightarrow \text{Time}$$
3. **Multi-Layer Temporal Baselines**: Instead of static threshold rules, Lafufu learns per-entity statistical distributions across three time horizons:
   - **Short-term** (24–72 hours): Spikes & immediate anomalies.
   - **Medium-term** (7–30 days): Gradual behavior drift.
   - **Long-term** (90–365 days): Stable foundational baselines.
4. **CLI-First Explanations**: The CLI is the primary product interface. Natural language summaries and timeline explanations give immediate clarity into *what* changed and *why*.

---

## 🗺️ Phases of Development

| Phase | Description | Status |
| :--- | :--- | :---: |
| **Phase 0 — Core Data Model & Storage Engine** | Universal graph primitives (`Entity`, `Event`, `Edge`, `Baseline`), SQLite WAL storage engine with auto-indexing, thread-safe repositories, and CLI foundation. | **Completed** |
| **Phase 1 — Ingestion Pipeline & Adapters** | Extensible log adapters (Generic JSON, Syslog Auth, GitHub Events, Docker Events), flexible timestamp normalizer, identity resolver (canonical entity mapping), batch directory ingestion, and streaming file tailer. | **Completed** |
| **Phase 2 — Behavioral Baseline Engine** | Per-entity profile learning, rolling statistical distributions (hours, locations, action frequency, interaction graphs), and multi-layer temporal baseline computation. | *Up Next* |
| **Phase 3 — Detection & Scoring Layer** | Anomaly fusion engine combining sequence anomaly, velocity spike detection, and graph structure changes into an integrated behavioral change score. | *Planned* |
| **Phase 4 — Explainability & CLI Reports** | Natural language explanation engine, interactive timeline navigation (`lafufu explain`), log replay (`lafufu replay`), and data export (`lafufu export`). | *Planned* |

---

## 🚀 Current Status

**Phase 0 and Phase 1 are fully implemented and verified!**

- **Database Engine**: Fully operational SQLite storage backend with WAL mode, automatic migrations, and indexing.
- **Log Adapters**: 4 concrete adapters implemented (`generic_json`, `syslog_auth`, `github_events`, `docker_events`).
- **Identity & Normalization**: Canonical identity resolution (`alice@company.com` $\rightarrow$ `alice`) and multi-format timestamp parser.
- **CLI Commands**: `status`, `ingest`, and `watch` commands working end-to-end.
- **Test Suite**: 100% pass rate across unit and integration tests (`cargo test`).

---

## 📁 Folder Structure

```
lafufu/
├── Cargo.toml                  # Package manifest & dependencies (tokio, rusqlite, serde, clap, tracing)
├── vision.md                   # Full architectural vision & phase specifications
├── README.md                   # Project overview & documentation
├── src/
│   ├── main.rs                 # CLI entry point (clap command router)
│   ├── lib.rs                  # Module exports
│   ├── config.rs               # Application & database storage path configuration
│   ├── errors.rs               # Custom Lafufu error taxonomy
│   ├── domain/                 # Universal behavior graph primitives
│   │   ├── entity.rs           # Entity struct with versioned attributes & UUIDv5 derivation
│   │   ├── event.rs            # Event struct (immutable occurrence)
│   │   └── edge.rs             # Edge struct (relationship between entities)
│   ├── storage/                # Storage engine implementation
│   │   └── sqlite/             # SQLite backend (schema migrations, WAL settings, indexing)
│   ├── repository/             # Repository traits (EntityRepository, EventRepository, etc.)
│   ├── normalization/          # Data normalization pipeline
│   │   ├── timestamp.rs        # Robust timestamp parsing (ISO8601, Syslog, Epoch, etc.)
│   │   └── identity.rs         # Canonical identity map & alias resolution
│   ├── adapters/               # Log format adapters
│   │   ├── json.rs             # Generic JSON & NDJSON adapter
│   │   ├── auth.rs             # Syslog auth log adapter (sshd, sudo, pam)
│   │   ├── github.rs           # GitHub webhook & API events adapter
│   │   └── docker.rs           # Docker container event stream adapter
│   ├── ingestion/              # Ingestion pipeline & streaming engine
│   │   ├── pipeline.rs         # IngestionPipeline (batch directory & file processing)
│   │   └── stream.rs           # LogStreamer (continuous file tailing)
│   ├── behavior/               # Behavioral profile data models
│   └── observability/          # Tracing & logging subscriber setup
└── tests/
    ├── storage_tests.rs        # SQLite repository integration tests
    └── ingestion_tests.rs      # Log adapter & ingestion pipeline integration tests
```

---

## 💻 Quick Start & Usage

### 1. Build the Project
```bash
cargo build --release
```

### 2. Run Tests
```bash
cargo test
```

### 3. Check System & Database Status
```bash
cargo run -- status
```

### 4. Ingest Log Files or Directories
```bash
# Ingest a single log file (auto-detects adapter)
cargo run -- ingest path/to/auth.log

# Specify a specific log adapter
cargo run -- ingest path/to/events.json --adapter generic_json

# Ingest an entire directory of logs recursively
cargo run -- ingest path/to/log_dir/
```

### 5. Tail Log Streams Continuously (Watch Mode)
```bash
cargo run -- watch --path /var/log/auth.log
```

---

## 📄 License

MIT License. Designed and engineered for local-first behavioral intelligence.

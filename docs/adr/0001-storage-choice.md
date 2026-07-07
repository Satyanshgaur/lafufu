# ADR 0001: Selection of SQLite for Local-First Storage

* **Status**: Accepted
* **Date**: 2026-07-07

## Context and Problem Statement

Lafufu is a local-first behavioral intelligence platform. It must run on developer laptops, edge servers, or local environments without requiring internet access or complex infrastructure. The application needs to persist:
1. Immutable facts (events, versioned entities, edges of a behavior graph).
2. Derived behavioral profiles (baselines, statistical metrics).

We need a database that is zero-configuration, has a low memory footprint, runs without a background daemon, survives system sleep/wake cycles, and can efficiently query millions of events.

## Decision Drivers

* **Operational Simplicity**: No external database daemon or setup steps (`docker run`, service start) should be required. The installation must be a single binary.
* **Query Performance**: The ability to perform fast time-range lookups, index scans, and simple join queries (e.g. entity event counts).
* **Data Safety**: ACID transactions to prevent corruption during system crashes or power failures.
* **Embeddability**: High-quality, safe bindings in Rust.

## Considered Options

1. **SQLite**: Relational, single-file, embedded database.
2. **DuckDB**: Embedded analytical database, optimized for column-oriented queries.
3. **sled / RocksDB**: Embedded key-value stores.
4. **Neo4j / Dgraph**: Dedicated graph databases (require a daemon).

## Decision Outcome

**Selected Option: SQLite**

SQLite fits the requirements perfectly. It runs entirely inside the application process, writes to a single file, and has mature Rust bindings (`rusqlite` / `sqlx`).

### Consequences

* **Good**: Zero configuration for the end-user. Extremely low memory overhead. Reliable ACID transaction guarantees. Perfect for storing adjacency lists and standard relational queries.
* **Bad**: Lack of native graph query optimizations (e.g. recursive multi-hop traversals). We must handle graph lookups using standard indexes or recursive Common Table Expressions (CTEs), which is acceptable for the expected local scale (1–10 million nodes).
* **Mitigation**: We isolate all persistence logic behind specialized repository traits. If graph scaling requires it in the future, we can swap the backend for an embedded graph engine without affecting the core domain logic.

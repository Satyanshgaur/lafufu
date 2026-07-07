# ADR 0002: Separation of Immutable Facts and Derived Knowledge

* **Status**: Accepted
* **Date**: 2026-07-07

## Context and Problem Statement

In Lafufu, we parse diverse logs and events, construct a behavior graph, and learn behavioral profiles over time. As we refine our statistical formulas, histograms, and anomaly detection algorithms, we must update these profiles. If we store only aggregated profiles or couple raw events with profile calculations directly during ingestion without preserving historical facts, we lose the ability to refine our models retroactively. 

We need a design that allows the system's analytical capabilities to evolve without forcing the user to lose their historical observations.

## Decision Drivers

* **Recomputability**: The ability to wipe derived intelligence and rebuild it from scratch using historical data.
* **Traceability**: Providing audit trails and timeline explanations back to the raw event level.
* **Decoupling**: Preventing mathematical models and baseline representations from leaking into raw event models.

## Considered Options

1. **Integrated Storage Model**: Update statistical summaries directly on ingestion and discard/compress raw events.
2. **Segregated Fact and Derived Model**: Keep raw observations (Entities, Events, Edges) strictly read-only and immutable. Store behavioral baselines and scores as transient, derived calculations that are always recomputable.

## Decision Outcome

**Selected Option: Segregated Fact and Derived Model**

We will categorize our schemas into:
1. **Facts**: `Events` (immutable timestamped occurrences), `Entities` (observed actors/targets), and `Edges` (observed links).
2. **Derived Knowledge**: `Baselines` and `Profiles` (histograms, averages, sequences, transition matrices).

### Consequences

* **Good**: 
  - Complete algorithm updates without data loss: We can ship new detection logic and execute a `lafufu replay` to rebuild profiles.
  - Easier debugging: Analysts can review the raw event database to confirm why a specific baseline score changed.
  - Transaction safety: Raw facts are written once, eliminating concurrent updates and lock contention on historical records.
* **Bad**:
  - Storage footprint: Storing millions of raw events requires more disk space than storing aggregated statistics alone.
  - Optimization needs: Recomputing baselines over massive histories can be slow, necessitating caching and efficient query patterns.
* **Mitigation**: SQLite compression or WAL-mode options, combined with database indexing on time ranges, will keep query speeds high. The database schema separates baselines so they can be written/read separately from log streams.

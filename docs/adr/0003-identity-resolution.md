# ADR 0003: Pipeline-Level Identity Resolution

* **Status**: Accepted
* **Date**: 2026-07-07

## Context and Problem Statement

Lafufu ingests data from multiple sources (e.g. system authorization logs, GitHub events, Docker containers, Kubernetes audits). A single user or system actor may appear under different identifiers across different adapters (for instance, `alice@company.com` on GitHub, `alice` on SSH, and `1002` in system logs). If these are stored as separate entities, the behavioral engine cannot construct a unified profile of the user's activities.

We need a flexible mechanism to resolve duplicate identifiers into canonical system entities.

## Decision Drivers

* **Adapter Simplicity**: Log parsing adapters should not be responsible for understanding global identity mappings.
* **Flexibility**: Users must be able to define their own aliasing rules (e.g., in a local YAML configuration) without modifying source code.
* **Deterministic ID Generation**: Canonical entities should have stable, deterministic identifiers so that replay runs produce identical graphs.

## Considered Options

1. **Adapter-Level Resolution**: Each log adapter resolves identities during parsing.
2. **Database-Level Resolution**: Resolve identities via database joins/triggers during queries.
3. **Pipeline-Level Identity Resolution Component**: A dedicated module in the ingestion pipeline resolves raw entity properties to canonical IDs prior to storage.

## Decision Outcome

**Selected Option: Pipeline-Level Identity Resolution Component**

We will introduce a dedicated `IdentityResolver` as a distinct stage of the ingestion pipeline. Adapters parse logs into a `RawEvent` containing whatever identifiers are present. The pipeline normalizes the log to a `NormalizedEvent`, then runs `IdentityResolver` to check local configuration mappings and assign a deterministic UUIDv5 (or SHA256 canonical hash) based on the resolved canonical name.

### Consequences

* **Good**:
  - Keep adapters completely isolated and single-purpose (parsing and schema normalization only).
  - Centralized, easy-to-test mapping logic.
  - Supports configurable aliasing (regex rules, dictionary lookup, prefix matches) via `lafufu.yaml`.
  - Replays remain completely deterministic and consistent.
* **Bad**:
  - Requires loading mapping dictionaries into memory during ingestion.
  - Conflicts (e.g. mapping two distinct users to the same alias by accident) must be caught and logged safely.
* **Mitigation**: Config validation at startup will detect cyclic or duplicate mappings, and the pipeline will log warning traces using `tracing` when ambiguous aliases are encountered.

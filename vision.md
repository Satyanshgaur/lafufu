## Foundational Architectural Philosophy

Vision

Lafufu is a local-first behavioral intelligence platform that learns how digital systems behave, detects meaningful behavioral changes, and explains them in natural language.

Unlike traditional SIEMs that focus on logs and alerts, Lafufu models every system using a universal behavior graph:

Entity → Event → Context → Time

Everything—from authentication logs and GitHub activity to Docker events and cloud audit logs—is transformed into this common representation.

Before phases, one decision shapes everything else: the system must be local-first, offline-capable, and own its own storage. This rules out cloud databases, external APIs for core functionality, and anything that requires network access to work. The reason is trust. Developers will only point a tool at their auth logs, API keys, and cloud audit trails if they believe the data never leaves their machine. Local-first is not a constraint — it's the product's core value proposition against every cloud-based SIEM competitor.

The second foundational decision is that the behavior graph is the universal data model. Not logs. Not alerts. Not metrics. Everything that enters the system gets transformed into the same representation: Entity, Event, Context, Time. A GitHub push, a Kubernetes pod restart, a failed login, an outbound HTTP connection — all become nodes and edges in the same graph. This is what makes the same engine work across every adapter you add later without rebuilding detection logic each time.

The third decision is that the CLI is the product, not a wrapper around the product. The commands are the user interface. The quality of the natural language output is what users evaluate. This means investing in the explanation layer as seriously as the detection layer.

---

## Phase 0 — Core Data Model and Storage Engine

Everything else depends on getting this right. No detection logic, no CLI commands, no adapters yet. Just the schema and the storage layer.

### The Behavior Graph Schema

Every piece of information in the system resolves to four primitives.

An **Entity** is anything that acts or is acted upon. A user, a service, an API key, a container, a GitHub repository, an IP address, a database. Entities have a type, a unique identifier, and a set of attributes that describe them. Critically, attributes are versioned — you store not just what an entity looks like now but what it looked like at every point in time. This is how you detect drift: Alice's device fingerprint in January versus Alice's device fingerprint today.

An **Event** is something that happened. A login, an API call, a file read, a network connection, a deployment. Events have a type, a timestamp, a source entity, and optionally a target entity. Events are immutable once written. You never update an event — you only append new ones.

**Context** is the envelope around an event that gives it meaning. The geographic location of a login. The HTTP method of an API call. The exit code of a process. The branch name of a push. Context is structured but flexible — different event types carry different context fields, and the system must handle this gracefully rather than forcing every event into the same rigid schema.

**Time** is treated as a first-class dimension, not a field. This means every query in the system is naturally time-ranged. Behavioral baselines are computed over time windows. Drift is measured as change between time windows. The timeline command is a native primitive, not a query you assemble.

### Storage Architecture

SQLite is the right choice for local-first storage at this stage. It is a single file, requires no daemon, survives a laptop going to sleep, and handles millions of rows efficiently for the query patterns you need. The behavior graph is stored as three tables: entities, events, and edges (relationships between entities, with timestamps). A fourth table stores computed behavioral profiles — the baselines — which are pre-computed periodically and cached rather than recalculated on every query.

The storage layer gets its own internal API from day one, completely separate from the ingestion and detection logic. This matters because you will eventually want to support other storage backends — a team-shared SQLite over a network share, or a lightweight embedded database for higher write throughput. Keeping storage behind an interface makes that migration possible without touching detection code.

### Why Not a Graph Database

The temptation here is to use a purpose-built graph database like Neo4j or DGraph. The problem is operational complexity. A local-first tool cannot require the user to run a separate database daemon. SQLite with a well-designed adjacency list schema handles the graph queries you actually need — finding all events for an entity, finding all entities that interacted with a given entity in a time window, computing fan-in and fan-out counts. A proper graph database becomes worth the complexity only when you're doing multi-hop traversals over millions of nodes, which is a later-phase problem.

---

## Phase 1 — Ingestion Pipeline and Adapters

The system is useless without data. Phase 1 builds the pipeline that transforms raw log data into behavior graph entries.

### The Adapter Architecture

An adapter is a module that knows how to read a specific log format and translate it into the universal event schema. Each adapter is responsible for three things: reading from a source (a file path, a directory, a streaming API, a unix socket), parsing the raw format (JSON, syslog, CSV, structured text), and mapping fields to the behavior graph schema.

The adapter interface must be simple enough that adding a new adapter is a few hours of work, not a few days. This means the adapter's only job is parsing and mapping. All storage, deduplication, and downstream processing happens in the core pipeline, not in adapters.

### Starting Adapters

The first adapters, in priority order, should be: generic JSON log files (because nearly every modern system emits JSON and this gives you broad coverage immediately), auth logs from Linux and macOS (because this is where the most immediately interesting behavioral signals live — logins, sudo, SSH), GitHub webhooks and event exports (because developers understand GitHub activity intuitively and can immediately verify whether the system's behavioral analysis is correct), and Docker and container event streams (because DevOps engineers are a natural early audience and container behavior is surprisingly rich).

AWS CloudTrail, Kubernetes audit logs, and browser history come in the next wave. They follow the same adapter pattern but require more complex parsing and richer context extraction.

### Normalization and Deduplication

Raw logs are messy. The same event can appear multiple times. Timestamps can be in any format or timezone. Entity identifiers can be inconsistent — the same user might appear as "alice," "alice@company.com," and "uid=1001" in different log sources. The normalization layer handles all of this before anything hits the behavior graph.

Entity resolution — recognizing that "alice" and "alice@company.com" are the same entity — is done through a configurable identity map that the user can teach over time. The system makes reasonable guesses (email prefix matching, consistent IP-to-hostname resolution) and surfaces conflicts for the user to resolve rather than silently merging entities that might be distinct.

### Streaming Versus Batch

The ingestion pipeline needs to support both modes. Batch mode is for initial setup — pointing immune at six months of existing logs and letting it build baselines. Streaming mode is for ongoing monitoring — tailing a log file or subscribing to an event stream and processing events as they arrive. The behavior graph schema is the same in both cases. The difference is only in how events are delivered to the storage layer.

---

## Phase 2 — Behavioral Baseline Engine

This is the intellectual core of the system. Everything visible to the user depends on this layer working well.

### What a Behavioral Profile Is

A behavioral profile for an entity is a set of learned distributions over that entity's observed behavior. Not rules. Distributions. For a user entity, a profile might capture: typical login hours (a histogram over hours of the day), typical login locations (a set of known cities or IP ranges), typical session duration (a mean and variance), typical sequence of actions within a session (the most common action sequences and their frequencies), and typical interaction partners (which other entities this entity interacts with and how often).

The profile is not a static snapshot. It is a rolling model that weights recent observations more heavily than older ones. This is how you capture drift — a user who used to work 9-to-5 and now works at midnight has a profile that gradually shifts, rather than immediately triggering an alert because they logged in outside their historical window.

### The Three Temporal Layers

Behavioral profiles are maintained at three time scales simultaneously, and this is what makes the detection nuanced rather than noisy.

The short-term layer covers the last 24-72 hours. This is where you catch sudden spikes and immediate anomalies. A service that made zero outbound connections yesterday and is making 10,000 today.

The medium-term layer covers the last 7-30 days. This is where you catch gradual drift that wouldn't register as a spike but represents meaningful change. A user who used to log in from one city and now consistently logs in from another.

The long-term layer covers the last 90-365 days. This is where you establish the stable baseline — what this entity fundamentally looks like. New behaviors are evaluated against this layer to determine whether they're genuinely new or just seasonal variation.

The comparison between layers is where the meaningful signals emerge. Something that's anomalous against the long-term baseline but consistent with the medium-term baseline is drift — the entity has changed but gradually. Something anomalous against all three layers simultaneously is a sudden change and warrants higher attention.

### Anomaly Scoring Without Labels

The fundamental challenge of behavioral analysis without labeled training data is that you don't know in advance what counts as malicious. The system cannot be trained on fraud examples because most users' logs don't contain known-malicious events.

The solution is to score anomalies relative to the entity's own history rather than relative to a global threshold. An outbound connection to a new IP address is anomalous for a service that has only ever talked to three endpoints and completely normal for a service that regularly discovers new endpoints. The same event gets a different score depending on who performed it and what their history shows.

This is the insight from the paper applied to the general case. The LSTM sequence model in the paper learns per-account behavioral baselines. Here you're doing the same thing but for any entity type and without requiring labeled examples.

### What Gets Profiled

Every entity type gets a profile schema tailored to the signals that are meaningful for that type. A user entity's profile emphasizes time-of-day patterns, location patterns, and action sequences. A service entity's profile emphasizes network connection patterns, resource consumption patterns, and inter-service communication patterns. An API key entity's profile emphasizes call frequency, endpoint distribution, and error rate patterns. A container entity's profile emphasizes process tree patterns, network egress patterns, and resource usage patterns.

The schema is extensible. New adapters can register new entity types with their own profile schemas. The core baseline engine doesn't need to know about GitHub-specific behavior — the GitHub adapter defines what a repository entity's profile looks like.

---

## Phase 3 — Detection and Scoring Layer

With behavioral profiles built, the detection layer computes anomaly scores for new events and identifies meaningful behavioral changes.

### The Scoring Architecture

Every new event gets scored against three signals, corresponding to the paper's fusion architecture but generalized.

The sequence signal asks: given this entity's recent history, how expected is this event right now? A login at 3am from a user who has never logged in outside 8am-6pm scores high on sequence anomaly. The same login from a user who regularly works night shifts scores low.

The velocity signal asks: is the rate of this event type unusual compared to this entity's baseline? Fifty failed logins in two minutes is anomalous regardless of what time of day it is or where it comes from. This catches the burst patterns that sequence modeling misses.

The graph signal asks: has this entity's relationship with other entities changed? A service that starts communicating with a new entity it has never interacted with, or a user who suddenly appears in a new cluster of account interactions. This is the fan-in, fan-out, and new-edge detection from the paper applied generally.

The three scores are fused into a single behavioral change score for the event. The fusion weights are learned per entity type based on which signals are historically most predictive for that type. A user entity's fusion weights will be different from a service entity's because users move around (reducing the weight of location-based signals) while services don't (making location or endpoint anomalies very high weight).

### Categories of Output

The system produces four categories of behavioral observation, which map directly to the profile display you described.

**Stable Behaviors** are patterns that have been consistent across the long-term, medium-term, and short-term baselines. These are displayed to give the user confidence that the system has learned the entity correctly, and to establish a reference point for understanding what drift means.

**New Behaviors** are patterns that appear in recent data but have no precedent in the entity's history. A service making a type of API call it has never made before. A user accessing a system they have never accessed before. New behaviors are not inherently malicious — they might be a new feature rollout or a legitimate change in workflow. But they warrant attention because they represent unknown unknowns.

**Behavior Drift** is gradual change across the medium-term and long-term baselines. The entity is still behaving consistently but its center of gravity has moved. This is the hardest category to surface clearly because the individual observations are all within normal range — only the trend is anomalous.

**Most Changed Entity** is a ranked list of entities whose behavioral profiles have changed most significantly in a given time window. This is the entry point for investigations — when you don't know where to look, you start with the entities that have changed the most.

---

## Phase 4 — The Natural Language Interface

This is what the user sees. It is not a frontend bolted on top of the system — it is designed as a first-class component from the beginning.

### Command Architecture

The CLI commands map directly to the system's capabilities and are designed to be composable.

`immune watch` starts the system in monitoring mode. It tails configured log sources and updates behavioral profiles in real time. The output to the terminal is minimal — a quiet indicator that events are being processed — because the valuable output comes from the query commands, not from a scrolling event stream.

`immune explain --since 24h` is the primary daily-use command. It produces a narrative summary of behavioral changes in the last 24 hours, organized by significance. The output is structured as a short briefing: what changed, which entities changed most, and which changes warrant attention. The language is specific and grounded in evidence — not "anomaly detected" but "the checkout service started making connections to a new external endpoint (api.newvendor.com) at 14:23. This endpoint has not appeared in this service's connection history over the past 90 days."

`immune ask "What changed today?"` is the conversational interface. This routes to a language model (running locally via Ollama, or optionally via API) with the behavioral profile data as context. The user can ask questions in natural language and get answers grounded in the actual behavioral data. "Has Alice's behavior changed recently?" "What services talk to the payments database?" "When did the deploy service start making S3 calls?" The language model does not have access to raw logs — it has access to the behavioral profile summaries. This is important both for privacy and for keeping the context window manageable.

`immune timeline <entity>` renders a chronological view of a specific entity's behavioral history. Not a stream of log lines — a narrative of behavioral phases. "January-March: consistent login pattern from London, 9am-6pm. April 2: first login from Berlin. April 2-present: mixed London and Berlin logins." This is the behavior graph's temporal dimension made legible.

`immune diff <entity> --from 30d --to now` compares an entity's behavioral profile at two points in time. This is the power-user command for investigations. "Show me how this service's behavior has changed since the last deployment."

`immune status` gives a system overview — how many entities are being tracked, how many events processed, which adapters are active, when baselines were last updated.

### The Explanation Engine

The quality of natural language output is what separates this from a log viewer with grep. The explanation engine is a template-and-reasoning system, not a pure LLM call for every output.

For each anomaly category and entity type, there are explanation templates that know how to turn structured anomaly data into coherent sentences. "Entity X, which has never performed action Y, performed action Y at time Z" is a template. The values are filled in from the behavior graph. The result is deterministic, fast, and doesn't require a language model for basic output.

The language model is reserved for the `immune ask` interface, where the user is asking open-ended questions that require reasoning across multiple entities and time periods. Even here, the LLM is not generating explanations from scratch — it is reasoning over structured summaries that the explanation engine has already produced. This keeps outputs grounded and reduces hallucination risk.

### Local LLM vs. API

The system should support both. By default, it uses Ollama with a locally running model so that no data ever leaves the machine. Users can opt into using an external API (Claude, OpenAI) for better reasoning quality on complex questions. This opt-in framing is important — the default must be local and private, with the cloud API as a conscious choice.

---

## Phase 5 — Packaging, Distribution, and Developer Experience

A technically excellent tool that's hard to install and configure will not get adopted. This phase is about making the first hour of use feel effortless.

### Installation

A single command install via a package manager. Homebrew on macOS, apt/snap on Linux, winget on Windows. No dependencies beyond what the installer brings. No Docker required. No configuration before first run.

The first-run experience should produce value within five minutes. `immune init` walks through selecting log sources, runs an initial batch ingestion, and produces a first behavioral summary before the user has had to write a single configuration line.

### Configuration

A single YAML file for persistent configuration. Log source paths, adapter settings, entity identity maps, and LLM preferences. The configuration file is human-readable and well-documented inline. Advanced users can hand-edit it; normal users never need to.

### The Watch Directory Pattern

A particularly elegant distribution mechanism is to support a watch directory — a folder where you drop log files or symlink log sources, and immune automatically detects and processes them. This removes the need to configure sources explicitly for common cases. Drop your nginx access log into the watch directory and it starts being analyzed immediately.

---

## Phase 6 — The Behavior Graph as a Platform

Once the core system has users and feedback, the architecture opens up in two directions.

### Adapter Ecosystem

The adapter interface from Phase 1 becomes a public extension point. Third-party adapters for specific platforms — Vercel logs, Railway, Fly.io, Supabase, PlanetScale, GitHub Actions — can be published as packages and installed with a single command. The community builds coverage faster than you could alone.

### The SDK Arrives Here

This is where the Java SDK (or more likely a Python package and a REST API) becomes appropriate. Once developers are using immune and saying "I want to embed this behavioral analysis in my own application," you expose the core engine as a library. By this point, you have real usage data telling you which parts of the API developers actually need. The SDK is shaped by observed use rather than anticipated use.

### The Sharing Layer

This is where Lafufu's network effect begins. Behavioral profiles contain no raw log data — just statistical summaries of entity behavior. An opt-in mechanism lets users share anomaly patterns (not the underlying data) with a central service. A new attack pattern detected in one installation can propagate as a detection heuristic to all others. This is the digital vaccination idea from the original vision, built on top of the local-first CLI foundation rather than requiring a cloud-first architecture from the start.

---

## The Architectural Thread That Runs Through All Phases

Every phase is structured around the same core insight: the behavior graph is the universal abstraction. Raw logs are input. Behavior graphs are the intermediate representation. Natural language summaries are the output. The detection logic, the storage schema, the explanation templates, and the adapter interface are all designed around this representation.

This means that adding a new adapter in Phase 6 doesn't require touching the detection logic. Improving the explanation engine in Phase 4 doesn't require changing the storage schema. Replacing SQLite with a different backend in Phase 3 doesn't require rewriting the baseline engine. The architecture is layered, and each layer has a single responsibility. That's what makes it maintainable as the system grows from a CLI tool into a platform.

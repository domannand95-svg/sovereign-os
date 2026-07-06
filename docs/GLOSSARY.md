---
Document: Engineering Glossary
ID: DOC-GLOSSARY-001
Version: 1.0
Status: Stable
Owner: Repository Maintainer
Reviewers:
  - Core Reviewers
Last Updated: 2026-07-07
Depends On:
  - GOV-CORE-001
Supersedes: None
---

# Engineering Glossary

This glossary defines common terminology used throughout the Sovereign OS / GUSA engineering handbook.

## Core Architecture & State Entities

### Active Memory

A planned runtime memory boundary for high-frequency temporary state, telemetry buffers, and future volatile execution contexts.

### Determinism

The property that identical inputs and valid historical state transitions produce reproducible system state.

### Event Log

The append-only record of ordered state-transition events used to reconstruct system state.

### LSN

Log Sequence Number. A monotonically increasing index used to order event-log entries and identify replay or snapshot boundaries.

### Registry

The structured service responsible for validated system records and baseline state data.

### Snapshot

A compact representation of validated state used to accelerate recovery while preserving event-log fallback.

## Governance & Configuration Management

### ADR

Architecture Decision Record. A document that records an architectural decision, its context, alternatives, and consequences.

### Capability Firewall

The governance boundary that prevents research concepts or unverified capabilities from being treated as production functionality.

### Discovery Layer

A research-facing layer for exploratory ideas, hypotheses, and candidate models that are not production features.

### Epistemic Hierarchy

The validation structure that separates concepts, research claims, specifications, implementations, tests, and production approvals.

### RFC

Request for Comments. A structured proposal for a future technical or architectural change.

## Documentation Classifications

### Production Document

A document describing implemented or approved architecture. Production documents use present tense and avoid speculative claims.

### Research Document

A document describing exploratory work, hypotheses, candidate models, or future investigations. Research documents do not imply implementation.

## Changelog

- **v1.0 (2026-07-07):** Initialized engineering glossary.

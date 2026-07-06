---
Document: Top-Level Architecture Mapping
ID: ARCH-INDEX-001
Version: 1.0
Status: Design Specification
Owner: Repository Maintainer
Reviewers:
  - Core Reviewers
Last Updated: 2026-07-07
Evidence:
  Specifications:
    - GUSA-MRD-100-V1.0
  PRs:
    - PR-070
    - PR-071
    - PR-072
Depends On:
  - GOV-CORE-001
Supersedes: None
---

# GUSA Technical Architecture Mapping

This document maps the active production substrate, planned architecture layers, and segregated research areas.

## 1. Active Production Substrates

- **Registry Service:** Manages primary structural records and baseline state validation.
- **Event Log:** Maintains append-only governance logging and chronological state-transition records.
- **Snapshot System:** Provides snapshot integrity, versioning, checksum validation, and fallback recovery.

## 2. Planned Architecture Layers

- **Active Memory Layer:** Planned volatile state and telemetry buffer layer.
- **Governance Wrapper:** Planned promotion-boundary validation layer.
- **Runtime Orchestrator:** Planned deterministic execution coordination layer.

## 3. Research Extensions

The research documents describe objectives, candidate models, and future investigation areas. They are not implemented production claims.

- **TCF-01A:** Future ternary/memristive-photonic research specification.
- **Indra's Net:** Future distributed sensing and mesh-network research concept.
- **Sovereign Stack Matrix:** Exploratory research area outside the current production baseline.

## Changelog

- **v1.0 (2026-07-07):** Initialized architecture map under PR #73.

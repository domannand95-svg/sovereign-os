# Sovereign OS — Core Architecture Blueprint

## 1. Multi-Crate Workspace Layout

The workspace uses a strict one-way dependency graph:

governance-wrapper
  -> registry-service
  -> event-log
  -> active-memory

## 2. Event Flow

State-changing commands move through the system in this order:

1. governance-wrapper validates the command.
2. registry-service converts it into a typed RegistryEvent.
3. event-log records the transition.
4. active-memory persists the event as JSONL.

## 3. Event Sourcing

The JSONL ledger is the source of truth.

When Registry::open() runs, it replays the event log from disk and rebuilds the in-memory node map.

## 4. Governance Invariants

The governance layer enforces:

- duplicate node rejection
- node existence checks
- lifecycle transition checks
- resource capacity checks

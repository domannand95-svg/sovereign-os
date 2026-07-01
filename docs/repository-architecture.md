# Repository Architecture

## Overview

This repository is organized as a modular Rust workspace. Each crate is responsible for a focused system boundary, allowing the project to evolve through isolated services while still sharing a common architecture.

## Goals

The repository layout is intended to support:

- clear module ownership
- isolated testing
- incremental feature development
- reusable consensus components
- readable documentation
- predictable contributor workflow

## Workspace Structure

The primary workspace areas are:

- `crates/`
- `docs/`
- `target/`
- root configuration files

## Crates

The `crates/` directory contains the main implementation units.

Each crate should:

- expose a focused public API
- contain its own tests where possible
- avoid unnecessary cross-crate coupling
- document major responsibilities

## Network Service

The `network-service` crate contains the distributed coordination layer.

It includes functionality for:

- peer discovery
- framed messaging
- leader election
- heartbeats
- log replication
- commit index management
- snapshots
- cluster membership

## Documentation

The `docs/` directory contains architecture and protocol documents.

Current documentation covers:

- system architecture
- network service architecture
- Raft consensus
- leader election
- heartbeat protocol
- AppendEntries RPC
- RequestVote RPC
- log replication
- log conflict resolution
- commit index management
- state machine application
- persistent storage
- snapshot installation
- log compaction
- cluster membership

## Testing

Tests should be run from the workspace root.

Common commands:

```bash
cargo test
cargo test --workspace
cargo test -p network-service
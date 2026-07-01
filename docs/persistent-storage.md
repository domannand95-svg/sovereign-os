# Persistent Storage

## Overview

Persistent storage preserves Raft state across process restarts, node crashes, and system failures. The storage layer ensures that consensus metadata, replicated log entries, snapshots, and applied state can be recovered safely.

## Purpose

Persistent storage is responsible for:

- Durable log retention
- Crash-safe consensus recovery
- Snapshot persistence
- Term and vote preservation
- State machine restoration

## Persisted State

A Raft node must persist:

- Current Term
- Voted For
- Log Entries
- Commit Index
- Last Applied Index
- Snapshot Metadata
- State Machine Snapshot

## Durable Log

The durable log stores replicated entries before they are acknowledged.

Each entry includes:

- Index
- Term
- Command
- Payload

Entries must be persisted before success is returned to the leader.

## Recovery Flow

1. Node restarts.
2. Persistent metadata is loaded.
3. Durable log entries are restored.
4. Snapshot metadata is restored.
5. Commit index and applied index are recovered.
6. The node rejoins the cluster as a follower.

## Snapshot Storage

Snapshots compact historical log entries into a durable state image.

Stored snapshot metadata includes:

- Last Included Index
- Last Included Term
- Snapshot Path
- Snapshot Checksum or Hash

## Safety Guarantees

Persistent storage guarantees:

- Committed entries survive restarts.
- Terms never regress.
- Votes are not duplicated within a term.
- Applied state remains consistent.
- Snapshot recovery preserves committed state.

## Failure Handling

The storage layer must tolerate:

- Partial writes
- Process crashes
- Restart recovery
- Corrupt snapshot files
- Missing log segments
- Interrupted compaction

## Future Enhancements

Potential improvements include:

- Write-ahead logging
- Checksummed log segments
- Atomic snapshot replacement
- Log segment rotation
- Merkle-backed state verification
- Encrypted durable storage

## Summary

Persistent storage provides the durability foundation for Raft consensus. By preserving terms, votes, log entries, commit progress, and snapshots, it allows nodes to recover safely without violating the guarantees of the replicated state machine.
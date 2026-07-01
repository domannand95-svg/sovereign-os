# Snapshot Installation

## Overview

Snapshot installation allows a Raft leader to synchronize followers that have fallen too far behind to catch up through normal log replication. Rather than transmitting an unbounded number of log entries, the leader transfers a compact snapshot representing the latest durable state.

## Purpose

Snapshot installation:

- Reduces replay time for recovering followers
- Prevents indefinite log growth
- Enables efficient state synchronization
- Supports log compaction

## Workflow

1. Leader detects follower lag beyond retained log entries.
2. Leader sends an InstallSnapshot RPC.
3. Follower validates the snapshot metadata.
4. Existing state is replaced with the snapshot contents.
5. Snapshot metadata becomes the new log baseline.
6. Normal AppendEntries resumes after the snapshot index.

## Snapshot Metadata

Each snapshot contains:

- Last Included Index
- Last Included Term
- Serialized State Machine
- Configuration Metadata (if applicable)

## Safety

Snapshots never violate Raft safety because:

- Only committed entries are included.
- Followers discard conflicting uncommitted history.
- Log replication resumes from a known committed state.

## Future Enhancements

Potential improvements include:

- Chunked snapshot streaming
- Compression
- Incremental snapshots
- Snapshot integrity verification
- Cryptographic hashing of snapshot contents
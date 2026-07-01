# Log Compaction

## Overview

Log compaction reduces the size of the replicated Raft log by replacing older committed entries with a durable snapshot. This prevents unbounded log growth while preserving the ability for nodes to recover and rejoin the cluster.

## Purpose

Log compaction provides:

- Bounded log growth
- Reduced disk usage
- Faster node recovery
- Improved replication performance
- Efficient snapshot-based synchronization

## Compaction Flow

1. Log entries become committed.
2. Entries are applied to the state machine.
3. A snapshot is created.
4. Snapshot metadata is persisted.
5. Entries included in the snapshot are removed.
6. Replication resumes after the snapshot boundary.

## Snapshot Boundary

Each snapshot records:

- Last Included Index
- Last Included Term
- State Machine Image
- Cluster Configuration (if applicable)

All entries before the snapshot boundary may be safely discarded.

## Recovery

During restart:

1. Load the latest snapshot.
2. Restore state machine contents.
3. Restore snapshot metadata.
4. Replay remaining log entries.
5. Resume normal operation.

## Safety

Log compaction preserves:

- Log Matching
- State Machine Safety
- Leader Completeness
- Deterministic replay

No committed data is discarded before it is represented within a durable snapshot.

## Performance Benefits

Compaction reduces:

- Startup time
- Storage requirements
- Replication overhead
- Snapshot transfer size

## Future Enhancements

Potential improvements include:

- Incremental compaction
- Background compaction workers
- Snapshot compression
- Chunked snapshot generation
- Adaptive compaction thresholds

## Summary

Log compaction maintains the long-term efficiency of the Raft log by replacing historical committed entries with compact snapshots while preserving the protocol's safety guarantees.
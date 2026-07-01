# State Machine Application

## Overview

The state machine is the final destination for committed log entries. Once an entry has been safely committed by the Raft consensus algorithm, it is applied in deterministic order to ensure every node reaches the same system state.

## Purpose

The state machine is responsible for:

- Executing committed commands
- Maintaining deterministic state
- Ensuring identical results across replicas
- Preventing duplicate execution
- Supporting crash recovery

## Apply Flow

1. A log entry becomes committed.
2. The commit index advances.
3. The node compares the commit index with the last applied index.
4. Any unapplied committed entries are executed sequentially.
5. The last applied index is updated.

## Ordering Guarantees

Entries are always applied:

- In ascending log index order
- Exactly once
- Only after commitment
- Never skipped
- Never reordered

These guarantees ensure deterministic execution across the cluster.

## Deterministic Execution

Commands executed by the state machine must produce identical results on every node.

Operations should avoid:

- Local timestamps
- Random number generation
- External network requests
- Machine-specific behavior

All outputs must depend solely on the replicated command stream.

## Recovery

After a restart:

1. Persistent metadata is restored.
2. Snapshots are loaded if available.
3. Remaining committed log entries are replayed.
4. The last applied index is restored.
5. Normal replication resumes.

## Snapshot Interaction

Snapshots reduce replay time by replacing older committed log entries with a compact representation of state.

After loading a snapshot:

- Last Applied Index equals Last Included Index
- Replay resumes from the next log entry
- State consistency is preserved

## Safety Guarantees

The application layer guarantees:

- Exactly-once application
- Ordered execution
- Deterministic state transitions
- Crash-safe recovery
- Consistent replicated state

## Failure Handling

The state machine must tolerate:

- Unexpected shutdowns
- Replay after restart
- Snapshot restoration
- Partial recovery
- Delayed replication

## Future Enhancements

Potential improvements include:

- Parallel read execution
- Incremental snapshots
- State verification hashes
- Pluggable application engines
- Performance metrics

## Summary

The state machine transforms committed Raft log entries into durable system state. By enforcing deterministic, ordered execution, it ensures every replica converges on the same result while preserving the safety guarantees of distributed consensus.
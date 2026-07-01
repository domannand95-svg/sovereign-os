# Log Conflict Resolution

## Overview

The Log Conflict Resolution subsystem preserves log consistency across the Raft cluster by repairing divergent follower logs. When a follower's history differs from the leader's authoritative log, the leader identifies the last matching entry and overwrites conflicting entries before continuing replication.

---

# Goals

- Preserve log consistency
- Repair divergent histories
- Maintain State Machine Safety
- Minimize replication retries
- Support automatic recovery

---

# Conflict Detection

A conflict exists when:

- previousLogIndex does not exist
- previousLogTerm differs
- follower log is shorter
- follower contains uncommitted entries

---

# Resolution Workflow

1. Leader sends AppendEntries.
2. Follower validates previous index and term.
3. Validation fails.
4. Follower rejects the request.
5. Leader decrements nextIndex.
6. Leader retries with an earlier log position.
7. Matching prefix is found.
8. Conflicting entries are deleted.
9. New entries are replicated.
10. Normal replication resumes.

---

# Example

Leader

```
1 2 3 4 5 6 7
```

Follower

```
1 2 3 8 9
```

Repair process

```
Find last matching index = 3

Delete:

8
9

Append:

4
5
6
7
```

Follower becomes

```
1 2 3 4 5 6 7
```

---

# nextIndex Algorithm

For every follower:

```
nextIndex = lastLogIndex + 1
```

If replication fails:

```
nextIndex--
retry
```

Repeat until a matching prefix is discovered.

---

# Safety Guarantees

Conflict resolution preserves:

- Log Matching Property
- Leader Completeness
- State Machine Safety
- Deterministic replay

---

# Failure Scenarios

Supported recovery includes:

- leader failover
- delayed packets
- duplicate messages
- stale followers
- network partitions
- follower restart

---

# Integration

This subsystem interacts with:

- AppendEntriesHandler
- Replicated Log
- Commit Index Manager
- Persistent Storage
- Snapshot Manager

---

# Future Enhancements

- Binary search conflict detection
- Conflict term hints
- Snapshot-assisted repair
- Parallel recovery
- Adaptive retry windows

---

# Summary

Conflict resolution ensures that every follower converges toward the leader's authoritative log. By locating the last shared entry and replacing divergent history, the cluster maintains a single consistent replicated state machine despite failures or partitions.
# Commit Index Management

## Overview

The Commit Index Manager is responsible for determining when replicated log entries become permanently committed within the Raft consensus protocol. A log entry is considered committed only after it has been successfully replicated to a majority (quorum) of cluster members.

The commit index provides a consistent synchronization point across the cluster, ensuring every node applies the same commands in the same order.

---

# Objectives

The Commit Index Manager guarantees:

- Majority-based commitment
- Ordered execution
- Deterministic state transitions
- Crash-safe recovery
- Linearizable writes
- Consistent replicated state

---

# Core Concepts

## commitIndex

The highest log index known to be safely committed.

```
commitIndex = highest majority-replicated index
```

---

## lastApplied

Tracks the highest committed entry already applied to the state machine.

```
lastApplied <= commitIndex
```

---

## matchIndex

Each follower maintains a replication progress indicator.

```
Leader

Follower A -> matchIndex = 125

Follower B -> matchIndex = 124

Follower C -> matchIndex = 125
```

---

## nextIndex

Indicates the next log entry that should be transmitted to each follower.

```
nextIndex = matchIndex + 1
```

---

# Commit Pipeline

```
Client Request
      │
            ▼
            Leader Appends Entry
                  │
                        ▼
                        Persist Locally
                              │
                                    ▼
                                    AppendEntries RPC
                                          │
                                                ▼
                                                Followers Persist Entry
                                                      │
                                                            ▼
                                                            Replication ACK
                                                                  │
                                                                        ▼
                                                                        Majority Quorum
                                                                              │
                                                                                    ▼
                                                                                    commitIndex Advances
                                                                                          │
                                                                                                ▼
                                                                                                Apply to State Machine
                                                                                                ```

                                                                                                ---

                                                                                                # Quorum Rules

                                                                                                Cluster Size | Majority Required
                                                                                                ------------ | -----------------
                                                                                                1 | 1
                                                                                                3 | 2
                                                                                                5 | 3
                                                                                                7 | 4
                                                                                                9 | 5

                                                                                                Formula:

                                                                                                ```
                                                                                                floor(N / 2) + 1
                                                                                                ```

                                                                                                ---

                                                                                                # Leader Responsibilities

                                                                                                The leader is responsible for:

                                                                                                - Tracking follower replication
                                                                                                - Maintaining matchIndex values
                                                                                                - Maintaining nextIndex values
                                                                                                - Detecting quorum completion
                                                                                                - Advancing commitIndex
                                                                                                - Broadcasting updated commit indexes

                                                                                                ---

                                                                                                # Follower Responsibilities

                                                                                                Followers:

                                                                                                - Persist incoming entries
                                                                                                - Acknowledge replication
                                                                                                - Advance local commitIndex
                                                                                                - Apply committed entries
                                                                                                - Preserve ordering guarantees

                                                                                                ---

                                                                                                # State Machine Application

                                                                                                Committed entries are applied sequentially.

                                                                                                ```
                                                                                                while lastApplied < commitIndex

                                                                                                    lastApplied++

                                                                                                        apply(log[lastApplied])
                                                                                                        ```

                                                                                                        No committed entry may be skipped.

                                                                                                        ---

                                                                                                        # Failure Handling

                                                                                                        The Commit Index Manager tolerates:

                                                                                                        - Leader crashes
                                                                                                        - Follower crashes
                                                                                                        - Network delays
                                                                                                        - Packet loss
                                                                                                        - Temporary partitions
                                                                                                        - Duplicate acknowledgements

                                                                                                        If leadership changes, the new leader reconstructs replication progress before committing additional entries.

                                                                                                        ---

                                                                                                        # Safety Guarantees

                                                                                                        The Commit Index Manager preserves:

                                                                                                        - Election Safety
                                                                                                        - Leader Completeness
                                                                                                        - Log Matching
                                                                                                        - State Machine Safety
                                                                                                        - Linearizability

                                                                                                        ---

                                                                                                        # Integration

                                                                                                        This component integrates with:

                                                                                                        - RaftNode
                                                                                                        - RaftService
                                                                                                        - AppendEntriesHandler
                                                                                                        - RequestVoteHandler
                                                                                                        - Durable Log
                                                                                                        - Snapshot Manager
                                                                                                        - Network Messaging
                                                                                                        - State Machine

                                                                                                        ---

                                                                                                        # Performance Considerations

                                                                                                        Future optimizations include:

                                                                                                        - Batched commit advancement
                                                                                                        - Parallel acknowledgement processing
                                                                                                        - Adaptive replication windows
                                                                                                        - Read-index optimization
                                                                                                        - Joint consensus support

                                                                                                        ---

                                                                                                        # Summary

                                                                                                        The Commit Index Manager determines when replicated log entries become permanently committed through majority agreement. By coordinating commit progression across the cluster, it guarantees deterministic execution, durable replication, and consistent distributed state while preserving the safety properties defined by the Raft consensus algorithm.
# Log Replication

## Purpose

The log replication subsystem guarantees that every committed client command is replicated to a quorum of cluster members before being applied to the distributed state machine.

---

# Components

- Leader
- Followers
- Replicated Log
- Commit Index
- nextIndex
- matchIndex

---

# Replication Pipeline

Client Request
        │
                ▼
                Leader Appends Entry
                        │
                                ▼
                                Persistent Storage
                                        │
                                                ▼
                                                AppendEntries RPC
                                                        │
                                                                ▼
                                                                Follower Validation
                                                                        │
                                                                                ▼
                                                                                Follower Persistence
                                                                                        │
                                                                                                ▼
                                                                                                Acknowledgement
                                                                                                        │
                                                                                                                ▼
                                                                                                                Majority Reached
                                                                                                                        │
                                                                                                                                ▼
                                                                                                                                Commit Index Advances
                                                                                                                                        │
                                                                                                                                                ▼
                                                                                                                                                Apply to State Machine

                                                                                                                                                ---

                                                                                                                                                # Entry Structure

                                                                                                                                                Each log entry contains:

                                                                                                                                                - index
                                                                                                                                                - term
                                                                                                                                                - command
                                                                                                                                                - serialized payload

                                                                                                                                                ---

                                                                                                                                                # Leader Responsibilities

                                                                                                                                                - Accept client requests
                                                                                                                                                - Append entries locally
                                                                                                                                                - Replicate entries
                                                                                                                                                - Track replication progress
                                                                                                                                                - Retry failed replication
                                                                                                                                                - Advance commit index

                                                                                                                                                ---

                                                                                                                                                # Follower Responsibilities

                                                                                                                                                - Validate previous index
                                                                                                                                                - Validate previous term
                                                                                                                                                - Append entries
                                                                                                                                                - Persist entries
                                                                                                                                                - Return replication status

                                                                                                                                                ---

                                                                                                                                                # Majority Commit

                                                                                                                                                An entry becomes committed only after:

                                                                                                                                                - Majority acknowledgement
                                                                                                                                                - Entry belongs to the current leader's term

                                                                                                                                                ---

                                                                                                                                                # Failure Recovery

                                                                                                                                                If replication fails:

                                                                                                                                                1. Decrement nextIndex
                                                                                                                                                2. Retry AppendEntries
                                                                                                                                                3. Repair divergence
                                                                                                                                                4. Continue replication

                                                                                                                                                ---

                                                                                                                                                # Performance

                                                                                                                                                Optimizations include:

                                                                                                                                                - Batched AppendEntries
                                                                                                                                                - Heartbeat piggybacking
                                                                                                                                                - Snapshot installation
                                                                                                                                                - Log compaction

                                                                                                                                                ---

                                                                                                                                                # Future Work

                                                                                                                                                - Incremental snapshots
                                                                                                                                                - Compression
                                                                                                                                                - Parallel replication
                                                                                                                                                - Adaptive batching
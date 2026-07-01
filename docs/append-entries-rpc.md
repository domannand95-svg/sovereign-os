# AppendEntries RPC

## Overview

AppendEntries is the primary Remote Procedure Call (RPC) used by the Raft leader to replicate log entries and maintain authority over the cluster. The same RPC is also used as a heartbeat when no log entries need replication.

## Purpose

AppendEntries is responsible for:

- Replicating new log entries
- Maintaining leader authority through heartbeats
- Detecting and repairing follower log divergence
- Advancing the commit index

## Request Fields

Each AppendEntries request contains:

- Leader Term
- Leader ID
- Previous Log Index
- Previous Log Term
- New Log Entries
- Leader Commit Index

## Validation

Upon receiving an AppendEntries request, a follower:

1. Rejects requests from older terms.
2. Confirms the previous log index exists.
3. Confirms the previous log term matches.
4. Removes conflicting uncommitted entries.
5. Appends new entries.
6. Updates its commit index.
7. Returns success or failure.

## Heartbeats

When no new log entries exist, the leader periodically sends an empty AppendEntries request.

Heartbeats:

- Prevent unnecessary elections
- Confirm leader availability
- Propagate the current commit index

## Failure Handling

If replication fails:

- The leader decrements nextIndex.
- Another AppendEntries request is attempted.
- Replication continues until the follower's log matches the leader.

## Safety

AppendEntries guarantees:

- Ordered log replication
- Consistent logs across the cluster
- Committed entries are never overwritten
- Followers eventually converge to the leader's state

## Future Enhancements

Potential improvements include:

- Batch replication
- Pipeline replication
- Adaptive retry intervals
- Compression of large entry batches
- Cryptographic validation of replicated entries
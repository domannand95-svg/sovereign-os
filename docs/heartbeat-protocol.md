# Heartbeat Protocol

## Overview

The Heartbeat Protocol is the mechanism Raft leaders use to maintain authority and prevent unnecessary elections. A heartbeat is an AppendEntries RPC with no log entries.

## Purpose

Heartbeats are responsible for:

- Confirming leader liveness
- Resetting follower election timers
- Preventing split leadership
- Propagating commit index updates
- Detecting stale terms

## Heartbeat Flow

1. Leader becomes active.
2. Leader starts a heartbeat interval.
3. Leader sends empty AppendEntries RPCs to all followers.
4. Followers validate the leader term.
5. Followers reset election timers.
6. Followers update commit index if needed.
7. Followers return success or failure.

## Request Contents

A heartbeat includes:

- Leader term
- Leader ID
- Previous log index
- Previous log term
- Empty entries list
- Leader commit index

## Follower Behavior

When a follower receives a valid heartbeat, it:

- Remains in follower state
- Resets its election timer
- Updates its known leader
- Advances local commit index when safe
- Rejects stale leader terms

## Leader Behavior

The leader sends heartbeats periodically until:

- it steps down after seeing a higher term
- it loses quorum
- the node shuts down
- leadership changes

## Timing

Heartbeat intervals should be shorter than election timeouts.

Example:

- Heartbeat interval: 50 ms
- Election timeout: 150–300 ms

## Failure Handling

If heartbeats stop arriving:

1. Follower election timer expires.
2. Follower becomes candidate.
3. New election begins.
4. Cluster selects a new leader.

## Safety Guarantees

Heartbeats preserve:

- Single leader authority
- Term monotonicity
- Election stability
- Commit index propagation

## Future Enhancements

Potential improvements include:

- Adaptive heartbeat intervals
- Quorum health tracking
- Heartbeat batching
- Failure detector integration
- Peer latency monitoring

## Summary

The Heartbeat Protocol keeps the Raft cluster stable by allowing the leader to continuously assert authority and keep followers synchronized. It is a lightweight but essential part of leader election stability and commit propagation.
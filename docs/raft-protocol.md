# Raft Protocol

## Overview

Brief description of the Raft implementation used by Sovereign OS.

---

## Node Roles

- Follower
- Candidate
- Leader

Describe responsibilities of each role.

---

## Elections

- Election timeout
- Vote requests
- Majority quorum
- Term updates
- Split vote handling

---

## Heartbeats

- Heartbeat interval
- Leader liveness
- Resetting election timers

---

## Log Replication

- AppendEntries RPC
- PrevLogIndex validation
- PrevLogTerm validation
- MatchIndex updates
- CommitIndex advancement

---

## Failure Recovery

- Leader failure
- Follower catch-up
- Network partitions
- Rejoining peers

---

## Safety Guarantees

- Election Safety
- Leader Completeness
- Log Matching
- State Machine Safety

---

## Current Implementation

Describe how these concepts map onto the crates inside this repository.
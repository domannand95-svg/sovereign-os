# Network Consensus Protocol

## 1. Overview
- Consensus goals
- Raft architecture
- Failure assumptions

## 2. Components
- RaftNode
- RaftService
- ElectionTimer
- AppendEntriesHandler
- RequestVoteHandler
- CommitIndexManager
- DurableLog
- SnapshotManager

## 3. Leader Election
- Follower state
- Candidate transition
- Vote requests
- Vote responses
- Leader promotion

## 4. Heartbeats
- Empty AppendEntries
- Election timeout reset
- Liveness guarantees

## 5. Log Replication
- Entry creation
- Replication pipeline
- Follower acknowledgement
- Majority quorum
- Commit progression

## 6. Conflict Resolution
- prevLogIndex
- prevLogTerm
- Divergent log truncation
- Entry replay

## 7. Commit Index
- Majority calculation
- Applied index
- State machine updates

## 8. Snapshot Installation
- Snapshot transfer
- Log compaction
- Recovery process

## 9. Peer Networking
- Discovery
- Length-prefixed messaging
- Request routing

## 10. Persistence
- Durable log
- Persistent node state
- Crash recovery

## 11. Safety Properties
- Election safety
- Leader completeness
- Log matching
- State machine safety

## 12. Future Integration
- Poseidon hashing
- Merkle state proofs
- SMT storage
- Hardware fail-closed integration
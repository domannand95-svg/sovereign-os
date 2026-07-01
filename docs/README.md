# Sovereign OS Documentation

This directory contains the technical documentation for the Sovereign OS distributed consensus subsystem.

---

# Architecture

These documents describe the overall system structure.

- System Architecture
- Repository Architecture
- Network Service Architecture

---

# Consensus Protocol

Core Raft consensus documentation.

- Leader Election
- Heartbeat Protocol
- RequestVote RPC
- AppendEntries RPC
- Network Consensus Protocol
- Cluster Membership

---

# Log Management

Replication and persistence.

- Log Replication
- Log Conflict Resolution
- Log Compaction
- Commit Index Management

---

# State Management

Managing durable state.

- Persistent Storage
- Snapshot Installation
- State Machine Application

---

# Development

Development workflow and repository organization.

- Repository Architecture

---

# Reading Order

New contributors should read the documentation in this order:

1. System Architecture
2. Repository Architecture
3. Network Service Architecture
4. Network Consensus Protocol
5. Leader Election
6. Heartbeat Protocol
7. RequestVote RPC
8. AppendEntries RPC
9. Log Replication
10. Commit Index Management
11. Log Compaction
12. Snapshot Installation
13. Persistent Storage
14. State Machine Application
15. Cluster Membership
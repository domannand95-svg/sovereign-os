# RequestVote RPC

## Overview

RequestVote is the Raft RPC used by candidates to request leadership votes from peers during an election. It ensures that only a node with a sufficiently up-to-date log can become leader.

## Purpose

RequestVote is responsible for:

- Starting leader elections
- Enforcing one vote per term
- Preventing stale candidates from becoming leader
- Preserving committed log entries

## Request Fields

Each RequestVote request contains:

- Candidate Term
- Candidate ID
- Last Log Index
- Last Log Term

## Voting Rules

A follower grants its vote only when:

- the candidate term is not stale
- the follower has not already voted in the current term
- the candidate log is at least as up-to-date as the follower log

## Log Freshness

A candidate log is considered up-to-date when:

1. its last log term is greater than the voter's last log term, or
2. both logs have the same last term and the candidate's last log index is greater than or equal to the voter's last log index

## Election Flow

1. Follower election timeout expires.
2. Follower becomes candidate.
3. Candidate increments term.
4. Candidate votes for itself.
5. Candidate sends RequestVote RPCs.
6. Peers validate the request.
7. Candidate becomes leader after receiving majority approval.

## Rejection Cases

A vote is rejected when:

- candidate term is stale
- voter already voted for another candidate
- candidate log is not up-to-date
- a newer leader or term has already been observed

## Safety

RequestVote preserves:

- Election Safety
- Leader Completeness
- Term monotonicity
- Single-leader-per-term behavior

## Future Enhancements

Potential improvements include:

- Pre-vote optimization
- Leadership transfer support
- Election backoff tuning
- Peer health awareness
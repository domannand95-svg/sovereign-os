# Leader Election

## Overview

Leader election ensures that exactly one node coordinates log replication for the cluster at any given time. Raft uses randomized election timeouts to minimize split votes and guarantee eventual leader selection.

## Roles

Each node operates in one of three states:

- Follower
- Candidate
- Leader

Followers respond to leaders. Candidates request votes during elections. Leaders manage client requests and replicate log entries.

## Election Process

1. Followers wait for heartbeats.
2. If a heartbeat is not received before the election timeout expires, the follower becomes a candidate.
3. The candidate increments its current term.
4. The candidate votes for itself.
5. RequestVote RPCs are sent to every peer.
6. A majority of votes elects the new leader.
7. The new leader immediately begins sending heartbeats.

## Election Timeout

Election timeouts should be randomized to reduce simultaneous elections.

Typical ranges include:

- 150–300 ms
- 300–600 ms (larger deployments)

## Voting Rules

A node grants at most one vote per term.

Votes are granted only if:

- the request term is current
- the candidate's log is at least as up-to-date as the voter's log
- the voter has not already voted

## Split Votes

If no candidate gains a majority:

- election timeouts expire again
- a new election begins
- randomized timers reduce repeated collisions

## Leader Responsibilities

After election the leader:

- Sends periodic heartbeats
- Accepts client commands
- Replicates log entries
- Advances the commit index
- Coordinates snapshot installation when necessary

## Safety

Leader election preserves Raft safety by ensuring:

- one leader per term
- monotonically increasing terms
- committed entries are never overwritten
- stale leaders automatically step down after observing newer terms

## Future Enhancements

Possible improvements include:

- Adaptive election timeout tuning
- Network latency awareness
- Pre-vote optimization
---
Document: Constitutional Authority Separation
ID: ADR-0002
Version: 1.0
Status: Proposed
Owner: domannand95-svg
Reviewers:
- Core Reviewers
Last Updated: 2026-07-31
Evidence: A02 owner-approved planning checkpoint
Depends On:
- SOV-ARCH-GAR-001
- HB-CH04
Supersedes: None
---

# ADR-0002: Constitutional Authority Separation

## Status

Proposed

This decision becomes accepted only after owner-approved merge. It defines
constitutional contracts for later implementation and does not claim that the
runtime currently enforces them.

## Context

Sovereign OS will coordinate agents that can propose research, execute tools,
evaluate results, challenge evidence, allocate authority, promote candidates,
and investigate misconduct. If one subject can control enough of those steps,
apparently valid records can conceal self-certification, evidence suppression,
friendly-reviewer selection, self-issued capabilities, or unilateral changes to
the rules used to judge it.

Existing architecture and handbook documents require separation between
execution, verification, promotion, governance, and revocation. This decision
turns those principles into explicit role boundaries that future schemas,
policies, tests, and institutional profiles must preserve.

An authority role is not a statement that an actor is universally trusted. It
is a versioned, scoped permission to make a particular kind of decision under a
particular policy. An agent, model, service, person, or institution may act only
through authority that can be identified and audited.

## Decision

Sovereign OS shall represent the following logical authorities separately:

| Authority | Permitted responsibility | Prohibited unilateral action |
|---|---|---|
| Proposer | Submit an objective, claim, plan, artifact, policy amendment, model, grant request, or promotion request | Admit, evaluate, approve, promote, or erase its own proposal |
| Executor | Perform an admitted operation using capability-scoped tools and resources | Expand its grant, alter policy, choose its evaluator, or publish authoritative success |
| Evaluator | Apply predeclared criteria, fixtures, tolerances, and budgets and record results | Change criteria after observing results, promote the subject, or suppress failed evaluations |
| Adversarial reviewer | Challenge evidence, methods, citations, security, independence, and alternative explanations | Rewrite source evidence, become sole promotion authority, or conceal unresolved disagreement |
| Policy authority | Decide whether a requested transition is admissible under a specific policy version | Create evidence, execute the requested effect, or silently waive a mandatory rule |
| Promotion authority | Approve, reject, defer, demote, or revoke a candidate's admitted status or scoped authority | Promote itself, its controlled descendant, or a candidate without required independent evidence |
| Incident investigator | Preserve and analyse incident evidence and publish findings | Decide its own incident, alter the subject's historical evidence, or impose an unreviewable final sanction |
| Incident decision authority | Impose proportionate restriction, suspension, revocation, removal, reinstatement, or remediation | Investigate an incident in which it is the subject or erase evidence supporting the outcome |
| Evidence custodian | Admit and preserve immutable evidence and provenance under canonical rules | Decide whether the evidence proves its own proposal or mutate admitted history |
| Human constitutional authority | Approve constitutional amendments and explicitly reserved high-impact decisions | Rewrite history, bypass mandatory evidence, or grant undocumented ambient authority |

These are logical roles. Implementations may use more components or more
specialised reviewers, but must not collapse a prohibited decision cycle into
one interested subject.

### Decision-subject independence

For every governed decision, the system shall identify:

- the decision subject;
- the initiating actor and authority;
- every evaluating and reviewing actor;
- the deciding authority;
- the evidence custodian;
- the controlling policy and criteria versions;
- declared organisational, model-lineage, data, financial, and operational
  conflicts; and
- any human approval required by institutional policy.

An actor is not independent merely because it has a different process ID,
agent name, prompt, account, or model instance. Independence assessment must
consider common control, shared model lineage, shared hidden context, shared
training or evaluation data, delegated authority, and material conflicts.

Policy shall reject a transition when required independence cannot be
established. Uncertainty fails closed; it is not treated as proof of
independence.

### Non-self-certification invariants

No subject may:

1. approve or record its own promotion;
2. issue, widen, renew, delegate, or restore its own capability or resource
   grant;
3. be the sole evaluator or reviewer of its own claim, artifact, conduct, model,
   descendant model, or requested authority;
4. select only favourable evaluators or dismiss an unfavourable result without
   an independently authorised reason;
5. change mandatory criteria, fixtures, policy, baselines, or tolerances after
   observing the evaluated result;
6. suppress, replace, reorder, relabel, or mutate admitted negative evidence;
7. close its own incident or appeal;
8. unilaterally amend the constitutional authority model; or
9. turn successful execution, popularity, provider reputation, local ownership,
   or elapsed service time into additional authority.

Splitting these actions across aliases, descendants, delegated agents, or
services under the same controlling subject does not satisfy separation.

### Evidence custody

Proposers, executors, evaluators, and reviewers may submit evidence. They do not
receive authority to rewrite evidence after canonical admission.

Corrections shall append a new attributable record that links to the corrected
record and preserves the original. Disputes, failed attempts, minority findings,
and later revocations remain discoverable. A projection may hide information
from an unauthorised viewer, but privacy-aware redaction must not alter the
canonical evidence or its integrity commitments.

### Policy and execution separation

Policy authority decides whether an operation is permitted. Executor authority
performs an admitted operation. Evidence custody records the resulting facts.
None of these responsibilities implies either of the others.

A policy decision must bind the exact subject, operation, target, tool, data
scope, budget, expiry, approval set, and policy version. An allow decision is
not a transferable ambient credential and cannot authorise a materially
different effect.

### Promotion and trust separation

Evaluation produces evidence; review challenges it; policy determines
admissibility; promotion authority decides the scoped outcome. A reliability
assessment informs later policy but is not itself a grant.

Promotion authority may require multiple compatible findings but must preserve
disagreement. Quorum or majority agreement does not establish independence when
the participants share a disqualifying controller or correlated evidence
source.

### Incident and appeal separation

The incident subject shall not control evidence preservation, investigation, or
the final decision. The investigator and decision authority must be distinct
for serious sanctions unless an accountable human constitutional authority
records an emergency exception permitted by the governing policy.

An appeal is a new governed evaluation of the existing outcome. It cannot
delete the original incident or restore authority automatically. The appeal
reviewer must not be the subject or sole original decision-maker.

### Constitutional amendments

Changes to role boundaries, canonical identity, evidence integrity, grant
semantics, promotion authority, incident authority, or amendment procedure
require:

1. a versioned amendment proposal;
2. recorded rationale, affected invariants, risks, and migration impact;
3. independent review;
4. explicit human constitutional approval;
5. a prospective activation point; and
6. an immutable link from the superseded version to the replacement.

An amendment shall not retroactively make an unauthorised historical action
authorised or erase the policy that applied when the action occurred.

### Small-deployment composition

A small institution may assign multiple administrative roles to the same
accountable organisation or person when independent staffing is unavailable.
This does not permit self-certification.

At minimum:

- the decision subject cannot be its own evaluator, promotion authority, or
  incident decision authority;
- automated proposer or executor agents cannot hold human constitutional
  authority;
- high-impact grants, promotion, removal, constitutional amendment, and
  emergency exceptions require an accountable human approval when policy marks
  them as reserved;
- role composition and conflicts must be visible in the decision record; and
- if required separation cannot be achieved, the transition remains pending or
  denied rather than being silently weakened.

Institutional profiles may require stricter separation, dual control, external
review, or higher quorums.

### Emergency action

Emergency authority may suspend capabilities or isolate work to prevent
imminent harm before a full investigation completes. It may not promote a
candidate, erase evidence, silently amend policy, or create permanent
unreviewable authority.

Every emergency action must be bounded in scope and time, preserve evidence,
state its triggering policy and accountable actor, and enter independent review
within a policy-defined period. Expiry fails closed.

## Required decision record

Every admitted authority transition shall eventually encode enough information
to reconstruct:

- decision and subject identifiers;
- subject and authority lineage;
- requested and previous state;
- decided state and exact scope;
- initiating, evaluating, reviewing, policy, deciding, and evidence authorities;
- policy, criteria, tool, dataset, and environment versions;
- evidence references and unresolved findings;
- conflicts, independence result, and required approvals;
- reason, time, expiry, and activation point; and
- appeal, supersession, suspension, or revocation links when applicable.

The future canonical encoding and identifier types are delegated to A03-A07.
This ADR fixes the authority relationships those types must be capable of
representing and rejecting.

## Required negative cases

Later schema and policy work shall include deterministic rejection cases for:

| Case | Required result |
|---|---|
| Proposer approves its own artifact | Reject |
| Executor widens or renews its own grant | Reject |
| Candidate evaluates or promotes itself through an alias or descendant | Reject |
| Proposer replaces an unfavourable reviewer without independent authority | Reject and preserve the original assignment and finding |
| Evaluation criteria change after results are visible | Reject or restart under a new attributable evaluation version |
| Negative or minority evidence is omitted from a promotion request | Reject as incomplete evidence |
| Reviewer independence is unknown or materially conflicted | Fail closed pending independent review |
| Incident subject closes its own investigation or appeal | Reject |
| Policy authority both waives a mandatory control and executes the effect | Reject unless a documented constitutional emergency rule explicitly permits the bounded action |
| Constitutional change is activated without human approval | Reject |
| New policy attempts to authorise a previously unauthorised historical act | Reject retroactive authorisation |
| Emergency suspension lacks expiry or review requirement | Reject |

## Alternatives Considered

### One trusted governor

Rejected because compromise, error, capture, or hidden conflict in one actor
would control evidence, policy, and outcomes without an independent check.

### A single universal trust score

Rejected because competence and reliability are scoped. A global score would
allow success in one domain to leak into unrelated tool, data, or governance
authority.

### Majority vote alone

Rejected because correlated agents, shared model lineage, common prompts, or a
single controller can create false consensus. Voting may contribute evidence
but does not replace independence and policy requirements.

### Provider or model identity as authority

Rejected because provider reputation, model scale, and local ownership are not
evidence that a particular version is reliable for a particular governed scope.

### Fully separate infrastructure for every role

Not constitutionally required at all scales. Logical separation and
decision-subject independence are mandatory; physical and organisational
separation can increase with risk and institutional policy.

## Consequences

- Later schemas require explicit authority, subject, scope, policy, conflict,
  evidence, and lineage identifiers.
- Policy evaluation must reason about role incompatibility and independence
  before effects occur.
- Some decisions will remain pending when a deployment cannot supply an
  independent authority.
- Small deployments remain possible, but cannot claim that aliases or multiple
  processes provide independence.
- Emergency containment is available without creating a promotion or amendment
  bypass.
- Audit records become larger and institutional configuration more explicit.
- A03 through A07 must preserve these invariants in types, schemas, and
  deterministic negative tests.

## Verification

A02 is accepted when:

- every role named in the A02 backlog item has a defined responsibility and
  prohibited unilateral action;
- self-promotion, self-review, silent evidence mutation, self-grant, reviewer
  substitution, unilateral incident closure, and unilateral constitutional
  amendment are explicitly prohibited;
- small-deployment composition does not weaken decision-subject independence;
- required negative cases have named expected outcomes;
- this ADR is linked from the governing architecture and documentation index;
  and
- documentation distinguishes this proposed contract from implemented runtime
  enforcement.

## Changelog

- **v1.0 (2026-07-31):** Proposed the constitutional authority-separation
  contract for A02.

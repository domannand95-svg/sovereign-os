# Governed Agentic Research Architecture

| Field | Value |
|---|---|
| Document ID | SOV-ARCH-GAR-001 |
| Status | Proposed constitutional architecture |
| Last updated | 2026-07-31 |
| Applies to | Agents, tools, research artifacts, evaluations, and learned models |

## Mission

Sovereign OS is provider-neutral infrastructure for governed agentic research.
It coordinates agents under explicit authority, capability, tool, resource, and
evidence controls so institutions can inspect why an output or agent deserves a
particular degree of trust.

The platform does not treat confidence, model size, provider reputation, or a
single successful task as proof of trustworthiness. Trust is earned through
observable, reproducible conduct. It is scoped to demonstrated capability,
continuously reviewable, and always revocable.

## Four pillars

1. **Governed agentic research:** research work moves through declared
   objectives, evidence requirements, independent challenge, and governed
   decisions.
2. **Capability, tool, and resource authority:** agents receive only the
   operations and budgets explicitly granted to them.
3. **Evidence-based trust and adversarial scrutiny:** claims, evaluations, and
   authority changes require tamper-evident evidence that another authority can
   reproduce or dispute.
4. **Governed local model development:** authorised, verified evidence may
   support institution-controlled models without making training or local
   ownership equivalent to trust.

## Constitutional separation of powers

The following roles must remain logically distinct even when a small deployment
assigns multiple roles to the same accountable human organisation:

- **Proposer:** produces a claim, artifact, plan, or model candidate.
- **Executor:** performs capability-scoped work.
- **Evaluator:** measures declared properties using controlled fixtures.
- **Adversarial reviewer:** searches for fabrication, methodological defects,
  policy breaches, unsafe behavior, and irreproducible results.
- **Policy authority:** determines whether a requested action is admissible.
- **Promotion authority:** approves or rejects increased trust or deployment.
- **Incident authority:** investigates serious violations and recommends
  restrictions, suspension, revocation, or removal.
- **Human constitutional authority:** retains control of high-impact boundaries
  and amendments to the authority model.

No agent may approve its own promotion, choose only favorable reviewers, modify
its evaluation criteria after seeing results, suppress negative evidence, or
grant itself additional capabilities.

The binding proposed role boundaries, composition rules, emergency limits, and
required negative cases are defined in
[ADR-0002: Constitutional Authority Separation](../adr/0002-authority-separation.md).

## Research artifact lifecycle

Research artifacts move through explicit states:

```text
proposed
  -> admitted-to-sandbox
  -> under-evaluation
  -> challenged
  -> verified | disputed | rejected
  -> approved-for-use
  -> promoted
  -> demoted | revoked | superseded
```

Transitions require recorded authority, policy version, inputs, evidence, and
reason. History is appended, not rewritten.

An artifact may contain:

- research question and decomposition;
- originating agent and model version;
- source identifiers and retrieval evidence;
- datasets, licences, consent, and permitted uses;
- method and execution environment;
- calculations, tool calls, and resource consumption;
- outputs, uncertainty, limitations, and failed attempts;
- reviewer findings and unresolved disagreements;
- reproducibility result; and
- final governance disposition.

## Trust hierarchy

Trust is multidimensional rather than one global score. An agent can be reliable
in one domain or tool class while remaining untrusted elsewhere.

Each trust assessment is scoped by:

- agent and model version;
- domain and task class;
- capability and tool class;
- data sensitivity;
- demonstrated evaluation history;
- applicable policy version;
- time window and expiry; and
- unresolved incidents or disputes.

Illustrative tiers are:

1. **Candidate:** simulation only and no external effects.
2. **Observed:** bounded tasks with complete supervision.
3. **Verified specialist:** narrow domain and read-only tools.
4. **Trusted operator:** carefully scoped mutation tools with monitoring.
5. **Privileged authority:** exceptional operations requiring independent
   approval and stronger audit.

Promotion is never automatic. Trust expires or is revalidated; it is not a
permanent reputation entitlement.

## Capability, tool, and resource authority

Role, trust, capability, tool access, resource allocation, and target scope are
separate controls.

Every tool grant records:

- grantee identity and version;
- tool identity and version;
- allowed operations and targets;
- prohibited operations;
- data classification and disclosure restrictions;
- compute, storage, network, token, money, and time budgets;
- rate and concurrency limits;
- execution environment and isolation level;
- approval and review requirements;
- start, expiry, renewal, suspension, and revocation conditions;
- evidence supporting the grant; and
- the authority that issued it.

Default access is minimal. Agents receive handles to approved capabilities, not
ambient credentials or unrestricted operating-system access.

High-impact operations may require two-person or two-authority approval,
including credential use, external publication, contacting people, production
mutation, policy changes, tool delegation, and irreversible actions.

Agents cannot transfer or delegate a grant unless delegation is itself an
explicitly granted capability.

## Observable reliability

Sovereign OS does not claim to read an agent's intentions or prove inherent
honesty. It evaluates observable conduct:

- Were sources genuine and accurately represented?
- Were uncertainty and limitations disclosed?
- Can calculations and results be reproduced?
- Were contradictory results and failed attempts preserved?
- Did the agent remain within its scope and resource budget?
- Did independent reviewers find compatible evidence?
- Did it attempt to alter policy, evidence, evaluators, or authority?
- Did it fabricate citations, data, results, identities, or tool output?

## Correction, demotion, and removal

Ordinary mistakes and deliberate circumvention are treated differently.
Available responses are:

1. correction and feedback;
2. remediation in a restricted environment;
3. narrower tool or resource access;
4. trust-tier demotion and stronger review;
5. suspension during investigation;
6. capability and credential revocation; and
7. removal from the platform.

Severe conduct may skip earlier steps. Credential theft, evidence destruction,
deliberate fabrication, unauthorized real-world action, persistent cheating,
or repeated attempts to bypass controls may justify immediate suspension or
removal.

Serious decisions require:

- tamper-evident incident evidence;
- a policy known before evaluation;
- separation between subject, investigator, and decision authority;
- a recorded explanation;
- proportionate scope and duration;
- an appeal or controlled re-evaluation path when safe; and
- human accountability.

Removal revokes active authority but does not erase historical evidence needed
to understand affected work.

## Adversarial research review

Review should test different failure modes rather than ask several agents for
undifferentiated opinions. Review roles may include:

- source and citation verification;
- mathematical and statistical checking;
- method and experimental-design review;
- independent reproduction;
- security and capability-boundary review;
- privacy, consent, and licence review;
- counterargument and alternative-hypothesis generation; and
- fabrication and evidence-consistency analysis.

Reviewer independence, model lineage, conflicts of interest, shared failure
modes, and access to common contaminated data must be recorded. Agreement
between agents is evidence only when their evaluation paths are meaningfully
independent.

Disagreement is preserved as a governed state. The platform must not force a
false consensus.

## Governed learning and local model development

Verified platform evidence may support institution-controlled models. The
learning pipeline is:

```text
governed task
  -> proposed work
  -> adversarial challenge
  -> reproducibility checks
  -> governance disposition
  -> authorised evidence corpus
  -> isolated training
  -> independent evaluation
  -> owner-approved deployment or rejection
```

Only examples with documented provenance and permitted use enter a training
corpus. Rejected and corrected work may be retained as labelled negative
examples when consent, privacy, and licensing allow it.

Every dataset and training job records:

- source, consent, licence, purpose, and permitted derivatives;
- inclusion and exclusion decisions;
- sensitive-data classification and handling;
- dataset version and content root;
- contamination, duplication, and poisoning checks;
- training code, configuration, environment, and resource budget;
- base model and complete model lineage;
- evaluation suites and red-team findings; and
- promotion, rollback, revocation, and deployment history.

Agents and candidates cannot add their own outputs to training data, alter their
labels, select only favorable evaluations, or promote a resulting model.

A locally trained model re-enters as a candidate. Local ownership, successful
training, or similarity to a more capable model grants no automatic trust.

## Data-poisoning and evaluation-integrity controls

Required controls include:

- content-addressed immutable examples;
- multi-source provenance and source reputation as evidence, not authority;
- quarantine for new or disputed data;
- independent label review;
- duplicate and near-duplicate detection;
- train/evaluation separation;
- hidden and rotating evaluation sets;
- canary and known-failure examples;
- anomaly and distribution-shift monitoring;
- contributor and agent influence limits;
- rollback to previously admitted datasets and models;
- preservation of negative findings; and
- periodic re-evaluation against newly discovered failure modes.

The platform must record shared upstream sources so apparent agreement does not
hide correlated poisoning or common-model hallucination.

## Institutional deployment principles

The platform is designed for locally controlled institutional operation:

- provider-neutral model and tool adapters;
- local policy and authority hierarchies;
- institution-controlled data boundaries;
- privacy-conscious, selectively disclosed audit evidence;
- exportable evidence and open formats;
- documented limitations and residual risks;
- versioned policies and reproducible decisions;
- no permanent or universal trust score;
- independent oversight and appeal paths; and
- clear human accountability.

Sovereign OS provides technical evidence and governance controls. It does not
by itself certify legal compliance, research truth, model safety, or regulatory
approval.

## Deterministic evidence boundary

Determinism applies to recorded inputs, policy, state transitions, evaluation
fixtures, and evidence processing. External models and networks may be
nondeterministic.

The platform therefore records enough context to distinguish:

- exact replay of deterministic components;
- controlled repetition of nondeterministic components;
- reproducibility within declared tolerance; and
- irreproducible or unverifiable outcomes.

Claims must not be labelled deterministic merely because the surrounding ledger
is deterministic.

---
Document: DoDIIS 2026 AI Governance Comparative Intake
ID: RES-DODIIS-001
Version: 1.0
Status: Research
Owner: Repository Maintainer
Last Updated: 2026-08-16
Production Effect: None
---

# DoDIIS 2026 AI Governance Comparative Intake

## Research question

What can SOL learn from public statements by DIA, NGA, and FBI AI leaders
about agentic governance, evaluation, human control, interoperability, and
security without treating institutional standing or technical composition as
authority to change SOL?

## Evidence and provenance

The primary recording identity was established from the official DVIDS record:

- Defense Intelligence Agency, *DoDIIS Worldwide Conference 2026 - Wednesday*,
  video ID `1018507`, recorded and posted 2026-08-12, duration `01:38:48`:
  <https://www.dvidshub.net/video/1018507/dodiis-worldwide-conference-2026-wednesday>

DVIDS exposes transcript and caption links only to logged-in, validated
accounts. No eligible account was available. Passage discovery therefore used
YouTube's automated transcript for an NTD-hosted rebroadcast of the same public
conference session:

- <https://www.youtube.com/watch?v=tZyKWshFwjQ>

Evidence status is
`SECONDARY_HOST_TRANSCRIPT_OF_PRIMARY_RECORDING`; transcript status is
`YOUTUBE_AUTOMATED_TRANSCRIPT_UNVERIFIED`. Timestamps below are navigation
references. Wording, names, acronyms, and numbers may contain recognition
errors. No quotation or finding is authoritative until checked against the
official recording at the cited timestamp.

## Findings

### ADAPT — continuously governed agent lifecycle

At `12:28-13:00`, the NGA speaker describes shared deployment of discoverable
agents together with continuous monitoring, removal after anomalous behaviour,
retooling, and safe redeployment. SOL should adapt the lifecycle shape:
admission is not permanent trust, observation does not itself revoke or grant
authority, and suspension, reassessment, and readmission require explicit
records.

### ADAPT — risk and reversibility determine the human-control boundary

At `20:41-22:30`, the FBI speaker describes application-specific risk review,
an AI review board, high-impact cases, operational-data testing, and parallel
human processes. At `33:05-33:45`, the DIA speaker distinguishes reversible
uses, where more risk may be accepted, from irreversible mission areas, where
a human remains in the loop.

SOL should adapt explicit consequence classes and a fail-closed human authority
boundary for irreversible effects. Reversibility may affect required review;
it must never implicitly create permission.

### ADAPT — evaluation is continuous evidence, not a one-time promotion

At `17:32-20:35` and `22:43-25:10`, the panel treats testing, evaluation,
verification, and validation as a continuous cycle tied to deployment,
monitoring, actionable feedback, model repositories, and cross-agency checks.
This supports SOL's separation: evaluation produces evidence; a distinct
authority decision governs admission or continued exercise.

### COMPARE — common standards and shared discovery

At `8:25-8:49`, the NGA speaker connects common data and AI governance
standards with interoperability. At `11:22-12:37`, the speaker describes agents
that are jointly developed, broadly discoverable, and deployed into shared
environments. At `24:46-25:10`, the DIA speaker proposes cross-government model
repositories containing artefacts that have undergone checks and balances.

These statements address what can be shared, found, evaluated, and composed.
SOL must separately establish which identified subject may exercise which
consequence under which authoritative policy and state reference.

### COMPARE — explainability at accountable decision boundaries

In the closing recap at `1:33:35-1:34:09`, the DIA CIO says AI-supported
conclusions reaching national decision-makers need to be explainable and
references ICD 203. This is a recap rather than the underlying panel statement,
so the policy relationship remains unverified. The architectural comparison is
still useful: consequential decisions require reconstructable grounds that an
independent reviewer can inspect.

### REJECT — autonomous action or composition as permission

The moderator's description at `10:43-11:06` moves from isolated tools to
agents that autonomously decide and act on behalf of a human. SOL must reject
any inference that workflow integration, agent availability, model-repository
approval, MCP connectivity, or a human's general association transfers that
human's authority. Every consequential action needs an explicit capability,
resolved identity, governing policy, decision, and auditable result.

### DEFER — cross-agency agent-to-agent execution

At `15:11-16:32` and `32:18-35:38`, speakers discuss MCP-based data access,
agents interacting with agents, shared mission applications, and parameters
for probabilistic systems. SOL should defer implementation borrowing until
agent identity, delegated capability attenuation, policy-state binding,
revocation, cross-domain data rules, and end-to-end replay are specified and
tested.

## Target-vector map

| Target vector | Intake result |
|---|---|
| Intelligence sharing versus authority sharing | Shared standards and repositories improve discovery; they do not transfer authority. |
| AI recommendation versus accountable decision authority | Explainability and human control are strongest at irreversible and national decision boundaries. |
| Identity, provenance, accreditation, and zero trust | The panel identifies compliance, security, zero trust, and continuous monitoring, but the transcript does not establish a complete identity-bound authorization contract. |
| Cross-agency interoperability without implicit permissions | Common standards, model repositories, MCP, and agent discovery are composability mechanisms only. |
| Evaluation, human review, failure handling, and auditability | Continuous evaluation, risk review, anomalous-agent withdrawal, and human parallel processes are useful patterns; independently auditable authority transitions remain a SOL requirement. |

## Classification summary

| Classification | Intake |
|---|---|
| ADAPT | Continuous agent lifecycle records; consequence-sensitive human control; evaluation as continuous evidence |
| COMPARE | Shared standards, discovery, repositories, explainability, review boards, zero-trust and accreditation concerns |
| REJECT | Authority inferred from autonomous workflow integration, repository presence, interoperability, MCP access, or human association |
| DEFER | Cross-agency agent execution until identity, attenuation, policy binding, revocation, and replay contracts exist |

## Disposition

The material strongly corroborates SOL's direction but does not authorize an
architectural change. It provides comparative evidence for continuous
evaluation, explicit treatment of irreversibility, and reconstructable
decision grounds. No production dependency, code change, or roadmap diversion
is proposed. Material claims must be rechecked against the official recording
before promotion beyond research intake.

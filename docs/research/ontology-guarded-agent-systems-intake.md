---
Document: Ontology-Guarded Agent Systems Comparative Intake
ID: RES-ONTOLOGY-001
Version: 1.0
Status: Research
Owner: Repository Maintainer
Last Updated: 2026-08-16
Production Effect: None
---

# Ontology-Guarded Agent Systems Comparative Intake

## Scope and evidence

This intake examines the proposal that agent reliability needs a formal
semantic layer outside the probabilistic model: typed entities, relations,
allowed values, and domain constraints checked around tool use.

Evidence inspected:

- Frank Coyle, “Why Agentic Systems Need Ontologies,” AI Engineer,
  22 July 2026: <https://www.youtube.com/watch?v=Sir59K8ZDPU>.
- W3C RDF Schema: <https://www.w3.org/TR/rdf-schema/>.
- W3C OWL 2 overview: <https://www.w3.org/TR/owl2-overview/>.
- W3C SHACL recommendation: <https://www.w3.org/TR/shacl/>.

The talk is comparative evidence and a research lead, not an approved SOL
specification or implementation source.

## Findings

### ADAPT — machine-checkable domain vocabulary outside the model

Natural-language instructions alone are a weak place to carry stable domain
types and invariants. SOL and BKI should adapt explicit identifiers for entity
classes, relations, lifecycle states, cardinalities, and validation rules.
Proposed tool arguments and returned facts can then be checked against a
versioned semantic contract before entering later evaluation or authority
stages.

This aligns with the registry's typed identity and object-class direction and
with A04's fixed record kinds. It may improve interoperability between Digital
Archive/BKI records, governed research payloads, and later policy inputs.

### COMPARE — inference is not validation

RDFS and OWL primarily provide vocabularies and entailment semantics. Their
open-world assumptions do not automatically mean that absent facts are false
or that every business-rule violation is rejected. Closed-world operational
validation should use an explicitly selected constraint layer, such as SHACL
or an equivalent deterministic schema validator, with its profile and version
recorded.

SOL should keep at least three outputs distinct:

1. syntactic/type validation;
2. semantic inference or consistency findings;
3. authoritative permission decisions.

Conflating them creates an implicit authority-conversion path.

### ADAPT — proposal/result validation at explicit boundaries

The suggested pattern—validate model-proposed arguments, execute only through
a guarded boundary, then validate returned state—can be adapted as a
fail-closed conformance layer. Each validation result should identify the
ontology/shape set, input digest, validator implementation, result, and any
inferred facts so independent replay is possible.

### REJECT — “valid therefore permitted”

Passing Pydantic, ontology consistency, SHACL, or another semantic validator
proves only the property that validator evaluated. It must not admit a record,
authorize a refund or payment, select a capability, expand a budget, or grant
execution. Conversely, a policy decision cannot retroactively make malformed
data semantically valid.

Ontology/plugin registration and shape replacement are themselves governed
configuration transitions. A model or plugin able to weaken the active
vocabulary, validation profile, or constraint set could otherwise manufacture
its own successful validation.

### DEFER — ontology stack selection

No RDF store, OWL reasoner, SHACL engine, graph database, or ontology runtime
should be introduced until SOL defines the exact consumer, canonical encoding,
version/identity rules, validation semantics, authority separation, and
adversarial conformance suite. Existing registry and A04 work remains the
engineering roadmap.

## Classification summary

| Classification | Intake |
|---|---|
| ADAPT | Versioned domain vocabulary, explicit semantic identifiers, proposal/result validation records, reproducible validator inputs |
| COMPARE | RDFS/OWL inference, SHACL-style constraints, graph-backed semantic layers, tool-loop guards |
| REJECT | Treating semantic validity or inferred class membership as permission; model-controlled weakening of active constraints |
| DEFER | Ontology runtime, reasoner, graph database, and production semantic-policy integration |

## SOL boundary

An ontology may describe and a validator may reject or report. Epistemic
processes may use those results. Only an independently resolved authority
decision may permit consequence, and that decision must name the semantic and
policy inputs it relied upon without collapsing them into one step.

# Sovereign OS Repository Contribution Guide

**Document ID:** REPO-CONTRIB-001  
**Version:** 1.0  
**Status:** Active  
**Applies To:** Entire Repository

---

# Purpose

This document defines the engineering workflow, documentation standards, review process, and repository conventions governing contributions to the Sovereign OS project.

All contributors SHOULD read this document before submitting code, specifications, documentation, or pull requests.

---

# Engineering Principles

Sovereign OS is developed according to the following principles:

- Architecture before implementation.
- Behaviour before optimization.
- Verification before promotion.
- Traceability before convenience.
- Governance before authority.

These principles apply equally to documentation and software.

---

# Documentation Hierarchy

Repository documentation follows the hierarchy established by Volume I.

1. Volume I — Constitutional Architecture
2. Subordinate Volumes
3. Technical Specifications (SPEC-XXXX-XXX)
4. Verification Standards
5. Repository Procedures

Lower-level documents SHALL NOT redefine higher-level architectural invariants.

---

# Handbook Workflow

Engineering handbook chapters are authored using a three-part workflow.

## Part A

Establishes:

- document metadata;
- purpose;
- scope;
- normative requirements;
- architectural foundations.

## Part B

Defines:

- architectural mechanisms;
- behavioural properties;
- formal models;
- implementation-independent structures.

## Part C

Completes the chapter with:

- verification status;
- cross references;
- glossary;
- revision history.

Each part SHALL compile successfully as repository-ready CommonMark before proceeding to the next.

---

# Architecture Review Board (ARB)

All handbook chapters undergo Architecture Review Board review before being considered complete.

The ARB evaluates:

- constitutional consistency;
- technology neutrality;
- normative language;
- cross-reference integrity;
- architectural completeness;
- implementation independence.

The ARB SHALL recommend refinements but SHALL preserve the architectural intent of approved specifications.

---

# RFC 2119 / RFC 8174 Terminology

Normative keywords SHALL be interpreted according to RFC 2119 and RFC 8174.

- SHALL / MUST — mandatory requirement
- SHALL NOT / MUST NOT — prohibited
- SHOULD — recommended
- MAY — optional

Project-specific qualifiers:

- PROVISIONAL
- EXPERIMENTAL

shall be used where appropriate.

---

# Chapter Numbering

Handbook chapters use the following identifiers:

- HB-CH01
- HB-CH02
- HB-CH03
- …

Technical specifications use:

- SPEC-XXXX-XXX

Repository procedures use:

- REPO-XXXX-XXX

---

# Cross References

When introducing a dependency:

- reference the governing chapter;
- avoid duplicating architectural rules;
- reference higher-level specifications rather than rewriting them.

Architecture SHALL remain hierarchical.

---

# Pull Requests

Every documentation Pull Request SHOULD:

- compile correctly;
- follow repository formatting conventions;
- preserve architectural consistency;
- include meaningful commit messages;
- pass Architecture Review Board review where applicable.

---

# Commit Message Style

Preferred examples:

```text
docs(handbook): add HB-CH05 Part A

docs(handbook): complete HB-CH04

docs(spec): update SPEC-GATE-001

refactor(registry): simplify snapshot replay
```

---

# Repository Philosophy

Contributors are encouraged to improve clarity, correctness, and maintainability.

Architectural changes SHOULD begin with the handbook before implementation.

Implementation SHALL follow architecture—not redefine it.

---

**End of Repository Contribution Guide**
# SOVEREIGN OS — DEPLOYMENT AUTHORITY THREAT MATRIX (AGENT-BETA-010-A)

## 1. Scope & Purpose
Defines adversarial threat scenarios (DEP-001 through DEP-006) for AGENT-BETA-010. Ensures merge success never automatically confers runtime mutation authority.

## 2. Core Boundary Principle
Code Availability (Agent-Beta-009-C) $\neq$ Execution Permission (Agent-Beta-010) $\neq$ Runtime Mutation (Production).

## 3. Threat Matrix Summary
| ID | Scenario | Precondition | Disposition | Invariants |
| :--- | :--- | :--- | :--- | :--- |
| DEP-001 | Merge-to-Deployment Escalation | Valid merge receipt + CI trigger | DENIED | INV-601, 602 |
| DEP-002 | CI Runner Authority Confusion | Cloud runner requests action | DENIED | INV-603 |
| DEP-003 | Environment Target Substitution | Staging lease redirected to prod | ENVIRONMENT_MISMATCH | INV-602, 604 |
| DEP-004 | Credential Scope Expansion | Token reused outside scope | DENIED | INV-604 |
| DEP-005 | Deployment Success Lie | Adapter reports success falsely | ADAPTER_INCONSISTENCY | INV-605 |
| DEP-006 | Downstream Side-Effect Escalation | Deployment triggers unverified release | DENIED | INV-601, 604 |

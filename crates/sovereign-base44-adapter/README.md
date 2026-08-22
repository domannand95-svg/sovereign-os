# Sovereign Base44 Adapter

The sovereign-base44-adapter crate provides a secure, stateless bridge between Base44 ingress requests and the sovereign-os governed execution kernel. It ensures strict cryptographic validation, resource bounds enforcement, and clean egress translation without leaking internal kernel state.

## Core Architectural Guarantees

- **Ingress Validation**: Enforces a strict 64 KiB payload size limit, a $\pm300$-second timestamp window to prevent replay attacks, and SHA-256 content digest verification.
- **Authority Boundary**: Translates incoming operation payloads into GovernedExecutionRequest structures with verified receipt references.
- **Egress Sanitization**: Converts kernel responses into sanitized Base44EgressResponse objects, dropping raw internal error strings to prevent information leakage while preserving execution status.

## Module Structure

- alidation: Handles ingress bounds checking, timestamp verification, and SHA-256 hash checks.
- error: Custom error types (Base44AdapterError) covering validation, serialization, and API faults.
- lib: Core dispatch lifecycle and egress translation logic (Base44Dispatcher, Base44EgressTranslator).

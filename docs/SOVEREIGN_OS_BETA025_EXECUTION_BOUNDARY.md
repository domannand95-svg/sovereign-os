# Sovereign-OS BETA-025 Execution Boundary Specification

## Overview
This document outlines the architecture and execution boundaries for `sovereign-os`, focusing on the runtime hooks, admission validation, and the `sovereign-base44-adapter` stateless translation layer.

## Architectural Layers (Bottom-Up Order)
1. **BKI (Base Kernel Interface)**: Primitives, core security types, and memory-safe boundary checks.
2. **sovereign-base44-adapter**: Stateless translation layer mapping ingress DTOs (`Base44IngressRequest`) to internal execution structures.
3. **sovereign-execution**: Governed runtime environment defining `GovernedExecutor`, `FileCreationAdapter`, and `Base44EngineRuntime`.

## Engineering Constraints
- **Zero-Tolerance Linting**: Must pass `cargo clippy -- -D warnings` with zero warnings.
- **Fail-Fast & Stateless**: No blanket `#[allow(...)]` attributes in production paths.
- **Explicit Error Handling**: No generic or opaque errors; all errors must be typed and auditable.
- **Panic Safety**: Absolutely zero `unwrap()` or `expect()` calls allowed in production modules or test helper production paths.

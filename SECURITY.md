# Security Policy

## Supported Scope

Sovereign OS is currently a private internal-alpha project. Security support
applies only to the authoritative Rust workspace identified in
`PROJECT_STATUS.md`. Preserved prototype crates and research documents are not
production capabilities.

## Reporting a Vulnerability

Do not disclose a suspected vulnerability in a public issue, pull request, or
discussion. Contact the repository owner privately through GitHub and include:

- the affected commit and component;
- reproduction steps or a minimal proof of concept;
- expected and observed behavior;
- likely impact and required preconditions; and
- any suggested containment or remediation.

Do not access data, credentials, devices, or systems beyond what is necessary
to demonstrate the issue. Do not perform denial-of-service, persistence,
social-engineering, or destructive testing.

## Response Process

The maintainer will acknowledge a complete report, preserve the evidence,
classify the affected authority boundary, and coordinate a private remediation
branch where appropriate. Fixes must pass the repository's normal formatting,
strict lint, test, security, coverage, and review gates before disclosure.

Security fixes do not bypass constitutional change control. A remediation that
changes canonical identity, ledger format, state roots, policy authority,
promotion authority, or consensus requires the corresponding governed decision.

## Current Limitations

There is no public release or production support promise. The project does not
yet provide a packaged node, network service, general capability firewall,
audit runtime, discovery runtime, or distributed consensus implementation.

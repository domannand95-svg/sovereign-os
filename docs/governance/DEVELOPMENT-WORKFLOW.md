# Development and Review Workflow

This workflow governs repository changes. It is separate from the runtime
authority model for research agents.

1. Inspect and prepare the smallest useful change.
2. State the outcome, scope, non-goals, risks, and changed files.
3. Prefer local work and transfer only files needed for verification.
4. Never stage build output, credentials, temporary archives, or unrelated
   changes.
5. Run formatting, strict linting, relevant tests, and the full authoritative
   workspace test suite when code changes.
6. Publish a draft pull request and let Linux and Windows checks complete.
7. Merge only after explicit project-owner approval.

Planning approval is not standing merge approval for later pull requests.


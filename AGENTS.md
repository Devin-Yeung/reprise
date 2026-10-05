# AGENTS.md

## Development workflow

- Test-first development: write or update relevant tests before their corresponding implementation.
- Reviewable jj stacks: for feature work with separable concerns, create a dependency-ordered stack of focused commits. Separate design, contract tests, implementation, and integration or fixture changes when independently reviewable. When rewriting a stack, move its feature bookmark to the final tip, verify the aggregate diff equals the pre-rewrite tip, and show the final `jj log`.

## API design and documentation

- Define public APIs and interface contracts in Rust types, signatures, and Rustdoc. Markdown records rationale, constraints, rejected alternatives, and open questions; it does not duplicate code-level API shapes, so the code remains the single source of truth.
- Design predictable APIs and keep documentation high-signal: briefly explain purpose and non-obvious behavior that affects callers' decisions, rather than restating predictable implementation details. Writing, maintaining, and reading documentation all cost effort: every sentence must earn its place. Long documentation goes unread.
- Rustdoc must be self-contained: include the context needed to understand the documented API without relying on local Markdown files. Self-contained means understandable on its own, not exhaustive.

## Pull request descriptions

- Keep PR descriptions scan-first and decision-useful: use short bullets, lead with the change's scope and outcome, and include validation or non-goals only when they materially help reviewers assess the change. Describe the feature slice rather than enumerating implementation details.

## Design docs

- Design docs (`docs/design/`) declare a lifecycle `status` in front matter: `draft` docs are the source of truth for intent and constraints, `implemented` docs defer to merged code, and `retired` docs record past decisions. Read `docs/design/README.md` before creating a design doc, landing one, or changing its status.

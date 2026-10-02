# AGENTS.md

- Rustdoc must be self-contained: include the context needed to understand the documented API without relying on local Markdown files. Self-contained means understandable on its own, not exhaustive.
- Design predictable APIs and keep documentation high-signal. Code is the single source of truth; briefly explain purpose and document non-obvious behavior that affects callers' decisions, rather than restating predictable implementation details. Writing, maintaining, and reading documentation all cost effort: every sentence must earn its place. Long documentation goes unread.

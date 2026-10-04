# AGENTS.md

- Writing test AFTER implementing is NOT ALLOWED. Tests should be written before implementing.
- Rustdoc must be self-contained: include the context needed to understand the documented API without relying on local Markdown files. Self-contained means understandable on its own, not exhaustive.
- Design predictable APIs and keep documentation high-signal. Code is the single source of truth; briefly explain purpose and document non-obvious behavior that affects callers' decisions, rather than restating predictable implementation details. Writing, maintaining, and reading documentation all cost effort: every sentence must earn its place. Long documentation goes unread.
- Design docs (`docs/design/`) declare a lifecycle `status` in front matter: `draft` docs are the source of truth, `implemented` docs defer to merged code, `retired` docs record past decisions. Read `docs/design/README.md` before creating a design doc, landing one, or changing its status.

# Design docs

A design doc captures intent before code exists, then becomes a dated record of
that intent once merged code expresses it precisely. Its front matter states which
role it currently plays, so a reader knows whether to trust the doc or the code.

Every doc in this directory except this README opens with:

```yaml
---
status: draft # draft | implemented | retired
date: 2026-10-04 # when status was last set
superseded-by: other-doc.md # retired only, when a successor exists
---
```

`date` changes only with `status`, so it always answers "intent as of when".

## `draft`

The code has not taken shape; the doc is the source of truth. It carries the
problem, constraints, chosen approach, rejected alternatives with reasons, and
open questions.

When implementation must diverge, update the doc in the same change. Silent
divergence during this stage is what later surfaces as doc-code conflict.

A partially landed design stays `draft` until the whole design lands. Landed
sections point to the code and defer to it; the rest remains authoritative.

## `implemented`

Set in the PR that lands the design. Merged code is now the source of truth; the
doc is an index into it and a record of intent as of `date`.

That PR also trims the doc: replace whatever the code now states precisely
(types, signatures, protocols, configuration) with pointers to the code, and keep
what code cannot carry: why, rejected alternatives, scope boundaries. A loose
description left beside precise code is where conflicts come from.

Afterwards, when doc and code disagree, follow the code and correct or trim the
doc rather than changing code to match it. New design work goes into a new
`draft`; when that draft replaces this design, retire this doc.

## `retired`

Superseded or abandoned. The doc is a historical decision record: consult it for
why a choice was made, not for current behavior. It stays in place so links keep
working, with content frozen; only front matter changes, including
`superseded-by` when a successor exists.

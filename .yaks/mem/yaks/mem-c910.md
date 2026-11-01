---
id: mem-c910
title: 'Signs: render text, edit text'
status: hairy
type: task
priority: 3
created: '2026-11-01T01:16:34Z'
updated: '2026-11-01T01:16:34Z'
parent: mem-f4c5
depends_on:
- mem-12d5
---

Signs keep their four lines in the `Sign` block entity (`Text1`..`Text4`), which the viewer ignores today.

## Scope

- Render each sign's text on its face (standing and wall signs).
- Right click opens a small editor for the four lines; saving updates the block entity (persisted with the world once saving exists).

## Evidence

Screenshot of the sample world's "waterfall" sign (mcpe2java's test finds 24 signs); edit one and screenshot.

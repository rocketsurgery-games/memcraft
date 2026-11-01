---
id: mem-d6f2
title: Game menu
status: hairy
type: task
priority: 3
created: '2026-11-01T01:16:34Z'
updated: '2026-11-01T01:16:34Z'
parent: mem-f5cf
---

A pause menu, the home for Save and later settings.

## Scope

- Esc releases the pointer (the browser does this regardless) and opens a menu overlay: Resume, Save (disabled until saving exists, or hidden in read-only builds), Controls help.
- Resume (or a click on the scene) relocks the pointer.
- Bevy UI, so native and browser match.

## Evidence

drive.js: Esc shows the menu (screenshot), Resume relocks (pointer-lock state logged).

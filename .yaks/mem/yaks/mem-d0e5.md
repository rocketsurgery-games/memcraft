---
id: mem-d0e5
title: 'Block picker: search and pick-block'
status: hairy
type: task
priority: 3
created: '2026-11-01T01:16:13Z'
updated: '2026-11-01T01:16:13Z'
parent: mem-4fcd
depends_on:
- mem-8d69
- mem-f4f7
- mem-6934
---

Choose any catalog block for the hotbar.

## Scope

- E opens a panel (releasing the pointer): a text search box and a grid of catalog icons with names on hover; clicking one puts it in the selected hotbar slot; E or Esc closes and the next click relocks.
- Middle click while locked picks the targeted block into the selected slot (pick-block).

## Evidence

drive.js: open the picker, type a search, screenshot the filtered grid; select one, place it.

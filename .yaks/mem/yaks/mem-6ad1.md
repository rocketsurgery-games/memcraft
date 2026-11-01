---
id: mem-6ad1
title: 'Levers and buttons: flip and press'
status: hairy
type: feature
priority: 3
created: '2026-11-01T03:38:52Z'
updated: '2026-11-01T03:38:52Z'
parent: mem-f4c5
depends_on:
- mem-6934
---

Right-clicking a lever flips its data bit (on/off); a button sets its pressed bit and pops back after ~1s (stone 20 ticks, wood 30). No redstone propagation (out of scope, see simulation yak). Needs lever/button shapes in blocks::shape if they still draw as cubes. Interaction takes precedence over placing when the target is interactable (same rule as mem-1457). Verify: unit test for the data-bit flip; headless drive showing the lever changes on click.

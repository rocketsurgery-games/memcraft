---
id: mem-f13a
title: Add walk mode
status: shaving
type: feature
priority: 3
created: '2026-10-29T12:01:49Z'
updated: '2026-10-29T17:22:17Z'
---

This is a decent sized chunk of work, and the first crack at collision detection. So let's take the time to build it carefully, using existing bevy crates where helpful. Consider carefully whether it's worth doing "real" physics, or hacking something together the way the original MC did.

If one of the bevy physics crates can be used largely OOTB, without being obscenely expensive, perhaps that would be better than playing "physics whack-a-mole" with a more hand-crafted solution.

---
▸ 2026-10-29T17:19:38Z [delta:Set up remote yak workflow]

moved: hairy -> shaving

---
▸ 2026-10-29T17:19:38Z [delta:Set up remote yak workflow]

Claim (yakherd: this thread). Owner: worker `walk-1`.

- Scope: `crates/memcraft-viewer/src/controls.rs` (extend the controller with a walk mode and a fly/walk toggle), a new collision/physics module in the viewer, `main.rs` only to register it, `Cargo.toml` if a physics crate is adopted, README controls line.
- Out of scope: `blocks.rs`, `mesh.rs` rendering (owned by `mem-7aba`), except reading `blocks::look` shapes for solidity.
- Evidence: unit tests for collision (falling onto ground, not walking through a wall, stepping/jumping up one block), headless browser run showing the player standing on terrain and walking, wasm size before/after.
- Judge: the yakherd at landing. The physics-crate-vs-hand-rolled choice is a `Decision:` note with numbers.

---
▸ 2026-10-29T17:22:17Z [walk-1]

Started: 2026-10-29T17:20Z (`date -u`, walk-1).
`yaks path mem-f13a`: /home/user/.local/share/yaks/farms/memcraft/.yaks/mem/yaks/mem-f13a.md

Plan: (1) measure physics-crate cost vs hand-rolled (baseline wasm size/build time first), (2) Decision note, (3) implement in new `physics.rs` + `controls.rs`, tests, browser run.

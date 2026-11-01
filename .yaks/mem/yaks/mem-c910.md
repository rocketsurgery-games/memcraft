---
id: mem-c910
title: 'Signs: render text, edit text'
status: shaving
type: task
priority: 3
created: '2026-11-01T01:16:34Z'
updated: '2026-11-01T03:40:06Z'
parent: mem-f4c5
depends_on:
- mem-12d5
- mem-6934
---

Signs keep their four lines in the `Sign` block entity (`Text1`..`Text4`), which the viewer ignores today.

## Scope

- Render each sign's text on its face (standing and wall signs).
- Right click opens a small editor for the four lines; saving updates the block entity (persisted with the world once saving exists).

## Evidence

Screenshot of the sample world's "waterfall" sign (mcpe2java's test finds 24 signs); edit one and screenshot.

---
▸ 2026-11-01T03:40:06Z [herd]
kind: claim
---

Claim (herd) for worker `sign-1`.

Scope:

- Render each sign's four lines (`Sign` block entity, `Text1`..`Text4`) on its face, standing and wall signs, using the box shapes from mem-12d5. Bevy `Text2d`/world-space text or a baked glyph texture are both fine; record the `Decision:` with the alternative rejected.
- Right click on a sign opens a small editor for the four lines; Enter/Save updates the in-memory block entity and the rendered text; Esc cancels. Pointer lock is released while editing. The block-entity edit should live with the editable world (`world.rs`) so saving (mem-a86b) can pick it up later.
- Another worker (`door-1`, mem-1457) is adding the "right click interacts with interactable blocks instead of placing" rule at the same time. Keep your hook small and clearly separated (one function deciding "sign -> open editor"); the herd will merge the two.
- Out of scope: sign placement with text, colours/formatting.

Judge: herd re-runs the evidence on main.

Evidence:

- Unit test for reading the four lines from block entities.
- `cargo test --workspace`, clippy native and wasm, `sh crates/memcraft-viewer/web/build.sh`.
- Headless screenshot of the sample world's "waterfall" sign (mcpe2java's test finds 24 signs; get its position from the data), then edit one and screenshot.

---
▸ 2026-11-01T03:40:06Z [herd]
kind: move
from: hairy
to: shaving
---

moved: hairy -> shaving

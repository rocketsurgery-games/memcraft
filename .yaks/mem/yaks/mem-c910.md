---
id: mem-c910
title: 'Signs: render text, edit text'
status: shaving
type: task
priority: 3
created: '2026-11-01T01:16:34Z'
updated: '2026-11-01T23:46:39Z'
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

---
▸ 2026-11-01T15:21:48Z [herd]

Runner was wiped again (~11h ago, found Sun Nov  1 15:21:48 UTC 2026); workers door-1/paint-1/sign-1 show no progress since 03:40Z and their runner checkouts are gone. Treating as dead pending Joel's go-ahead to respawn.

---
▸ 2026-11-01T23:40:22Z [sign-2]

Claim (sign-2): taking over from lost sign-1; base a48f18b.

- date: Sun Nov  1 23:40:22 UTC 2026
- farm: /home/user/.local/share/yaks/farms/memcraft/.yaks/mem/yaks/mem-c910.md
- checkout: /home/user/memcraft-5

---
▸ 2026-11-01T23:46:39Z [sign-2]

Decision: sign text is a baked glyph texture on a quad in front of the board, from a small built-in 5x8 ASCII bitmap font (`font.rs`), one entity per sign with text.

- Rejected: Bevy `Text2d`, which renders only through the 2D pipeline (not in this 3D scene), and needs features we cannot add (Cargo.toml is off limits).
- Rejected: Minecraft's own font (not redistributable); drawing text pixels as column-mesh quads (huge meshes, remesh on every edit).
- Cost: monospaced 6 px advance (Minecraft's is proportional), no colours/formatting (`§x` codes are skipped).

Decision: the editor and the menu coexist through a `controls::Modal` flag.

- Opening the editor (`EditSign` message from the right click in `targeting::edit_blocks`) frees the pointer and sets `Modal`; closing locks it again.
- `show_menu` hides the menu while `Modal` is set, `grab_pointer` does not lock on a scene click while it is set.
- The player's input systems already run only while the pointer is locked (`pointer_locked`), so WASD/number keys do nothing while typing.
- Rejected: leaving the menu up behind/over the editor; a separate editor state machine in the menu.

Decision: a block entity is dropped when its cell's block id changes (`WorldMap::set_block`), so a broken sign loses its text and a later save has no orphan. `WorldMap::{block_entity, set_block_entity, take_edited_cells}` are the world-side API (saving reads `col.block_entities`).

Heads-up for the herd: in this checkout `git status` shows `entities.rs`, `item_icons.rs`, `assets/paintings/`, the new textures, and the edits to `main.rs`/`mesh.rs`/`build.rs`/`mcpe-world/src/lib.rs` as uncommitted/untracked on top of a48f18b: the mem-15e6 commits there are empty. My signs code uses `entities::Wall`. I commit only my own files with explicit paths.

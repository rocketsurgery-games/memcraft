---
id: mem-99d0
title: Left-button mouse lock
status: shaving
type: bug
priority: 3
created: '2026-10-29T11:58:44Z'
updated: '2026-10-29T13:59:54Z'
---

The UI displays "click to look around (Esc releases)", but you actually have to right-drag to look around.

---
▸ 2026-10-29T13:59:54Z [delta:Set up remote yak workflow]

moved: hairy -> shaving

---
▸ 2026-10-29T13:59:54Z [delta:Set up remote yak workflow]

Cause: Bevy 0.19's `FreeCamera` grabs the cursor only while `mouse_key_cursor_grab` (default `MouseButton::Right`) is held, or after the `M` toggle, and keeps that grab state in private `Local`s. Changing the button would still give hold-to-look, not click-to-look.

Fix: replace `FreeCamera` with a small controller the viewer owns (`crates/memcraft-viewer/src/controls.rs`). Left click locks the pointer and it stays locked; Esc, focus loss, or the browser dropping pointer lock releases it. Flight and scroll-for-speed work as before. Walk mode (`mem-f13a`) can then extend this controller.

Runner note: the cloud runner was reset mid-build (13:10–13:58 UTC) and lost the unsynced farm edits, so this claim is a re-do.

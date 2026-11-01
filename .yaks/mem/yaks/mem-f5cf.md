---
id: mem-f5cf
title: Map chunk persistence
status: hairy
type: feature
priority: 3
created: '2026-10-31T23:21:32Z'
updated: '2026-11-01T15:33:27Z'
---

Initially, let's just focus on a manual save affordance in the menu (I guess we'll need a menu first!). Presumably we'd need some other serving mechanism to even be _able_ to save anything. We can explore that as well, but also preserve an "ephemeral" mode for read-only deployments with no server.

---
▸ 2026-11-01T01:17:10Z [herd]
kind: ask
needs: human
---

asked: needs human

Persistence needs a format and a home. Options and leans:

1. **Format.** (a) Write back into the PE LevelDB world, so the saved world stays a real PE 0.16 world that `mcpe2java` (and PE itself) can read; (b) our own overlay of changed columns applied on load. Lean: (a). It is more work in `mcpe-world` (yak `mem-a86b`) but there is one world format and no drift.
2. **Read-only (Pages) builds.** Edits live in memory and are lost on reload; Save is replaced by "Download world (.zip)". Optionally, autosave changed columns to the browser's IndexedDB so a reload keeps them. Lean: download first, IndexedDB as a follow-up.
3. **Server** (yak `mem-a1f7`). A small Rust server in this workspace (e.g. axum) that serves the web build and one world dir, with an endpoint the viewer probes to enable Save; no auth, for local/LAN use at first. Lean: yes, after the writer and native save.
4. **Native.** Load a world from a directory given on the command line (the embedded world stays the default), and Save writes it back in place, keeping a backup copy. Lean: yes.

---
▸ 2026-11-01T01:50:58Z [Joel Webber]
kind: answer
needs: agent
---

1. As long as you think the format is reasonable and not too much of a pain to work with, then that's good with me.
2. That's good with me. I'm not worried about the download case -- this read-only mode only needs to exist as a simple fallback.
3. Yup, that works.
4. :thumbs-up:

---
▸ 2026-11-01T15:21:36Z [herd]
kind: pickup
---

picked up

---
▸ 2026-11-01T15:21:48Z [herd]

Answers recorded (herd): (1) write back into the PE LevelDB world (mem-a86b); (2) read-only Pages build: in-memory edits plus a simple download fallback, IndexedDB not needed; (3) axum server mem-a1f7 after writer and native save; (4) native load from a dir arg, save in place with backup. Unblocks mem-a86b.

---
▸ 2026-11-01T15:33:27Z [delta:herd]

delta:writer-1 on mem-a86b, thread ksQQOn_lPggNQrKkBKNvXOzaN5LOADtzhZIBxBBCaHaBp8ZDPqDsZufUZIx8, base 48ed523.

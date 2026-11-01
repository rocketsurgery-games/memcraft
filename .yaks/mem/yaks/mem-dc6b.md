---
id: mem-dc6b
title: 'Native: load a world dir from the command line, save in place with backup'
status: hairy
type: task
priority: 3
created: '2026-11-01T16:25:13Z'
updated: '2026-11-01T16:25:13Z'
parent: mem-f5cf
depends_on:
- mem-a86b
---

Per Joel (mem-f5cf answer 4): the native viewer takes an optional world directory argument (embedded sample world stays the default) and can save edits back in place via mcpe-world's WorldWriter, keeping a backup copy of the previous db/ first. Save trigger can be a key until the menu's Save hook (mem-d6f2) exists; mem-5219 then wires the menu. Track which columns changed in WorldMap. Evidence: edit, save, restart with the dir, edit is visible; backup dir exists; mcpe2java converts the saved world.

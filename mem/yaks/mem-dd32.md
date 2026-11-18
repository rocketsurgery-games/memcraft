---
id: mem-dd32
title: 'Route B: own Rust/WASM PE-faithful explorer (decide whether to build)'
status: shorn
type: task
priority: 3
created: '2026-10-29T02:36:14Z'
updated: '2026-10-29T12:08:48Z'
parent: mem-0908
---

Route A works (mcraft.fun plays the converted world). Remaining reasons for our own client: PE look and feel, no dependence on a third-party site, and a full Rust stack. `mcpe-world` already builds for wasm32 and is the starting point. Needs a decision before work starts: textures (Mojang's can't be redistributed; options are an openly licensed pack or the user supplies one), and scope (fly-through vs walk/edit).

---
▸ 2026-10-29T03:56:32Z [delta:Start investigation for yak mem-0908]

joelgwebber (Delta comment): "let's try it" — look for a low-effort path to a basic fly-through; suggested Bevy (verify its wasm target).

---
▸ 2026-10-29T03:56:32Z [delta:Start investigation for yak mem-0908]

moved: hairy -> shaving

---
▸ 2026-10-29T11:56:33Z [Joel Webber]

Off to a great start! Native and web builds work beautifully out of the box.

![mc-web](mem-dd32/mc-web.png)

---
▸ 2026-10-29T12:08:48Z [delta:Start investigation for yak mem-0908]

Done: decided to build and shipped the first cut, mem-0174 (Bevy 0.19 fly-through, native and web). Landed on origin/main as `499b3d1`.

Follow-ups now live as their own top-level yaks: mem-7aba (textures), mem-99d0 (left-button mouse lock), mem-f13a (walk mode), mem-f434 (Pages hosting).

---
▸ 2026-10-29T12:08:48Z [delta:Start investigation for yak mem-0908]

moved: shaving -> shorn

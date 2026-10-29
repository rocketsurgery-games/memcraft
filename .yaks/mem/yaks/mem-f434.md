---
id: mem-f434
title: Host converted world for sharing (mcraft.fun ?map= link)
status: hairy
type: task
priority: 3
created: '2026-10-29T02:36:14Z'
updated: '2026-10-29T11:54:38Z'
needs: agent
---

`?map=<url>` needs a CORS-enabled zip URL. Options: GitHub release asset on rocketsurgery-games/memcraft (CORS ok via objects.githubusercontent? verify), GitHub Pages, or any static host. Note this makes the world public; decide with Joel. Could also self-host mcraft.fun's build next to the zip for a single stable link.

---
▸ 2026-10-29T03:56:31Z [delta:Start investigation for yak mem-0908]

joelgwebber (Delta comment): public hosting is fine; lock down later if needed.

---
▸ 2026-10-29T04:48:57Z [delta:Start investigation for yak mem-0908]

Proposal (awaiting push authorization): publish both routes from one GitHub Pages site on rocketsurgery-games/memcraft. The repo is currently **private**, so Pages either needs a plan that allows Pages on private repos or a separate public repo.

- `/` serves the memcraft-viewer web build.
- `/my-world.zip` is the converted world, linked as `https://mcraft.fun/?map=<pages url>/my-world.zip` (Pages sends `Access-Control-Allow-Origin: *`).

The deploy would be a `gh-pages` branch or a Pages workflow. Either way it means pushing to origin, which I haven't been authorized to do.

---
▸ 2026-10-29T04:48:58Z [delta:Start investigation for yak mem-0908]

asked: needs human

Where should the public share live, and may I push to it? Lean: GitHub Pages on rocketsurgery-games/memcraft (needs the repo public or a Pages-capable plan), deployed from a gh-pages branch; alternatively a new public repo just for the site.

---
▸ 2026-10-29T11:54:38Z [Joel Webber]

Great idea, that's completely fine with me to host it on github.

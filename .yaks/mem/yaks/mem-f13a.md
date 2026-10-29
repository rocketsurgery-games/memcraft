---
id: mem-f13a
title: Add walk mode
status: hairy
type: feature
priority: 3
created: '2026-10-29T12:01:49Z'
updated: '2026-10-29T12:01:49Z'
---

This is a decent sized chunk of work, and the first crack at collision detection. So let's take the time to build it carefully, using existing bevy crates where helpful. Consider carefully whether it's worth doing "real" physics, or hacking something together the way the original MC did.

If one of the bevy physics crates can be used largely OOTB, without being obscenely expensive, perhaps that would be better than playing "physics whack-a-mole" with a more hand-crafted solution.

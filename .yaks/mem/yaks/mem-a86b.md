---
id: mem-a86b
title: 'World writer: save edited columns as PE world records'
status: hairy
type: task
priority: 3
created: '2026-11-01T01:16:34Z'
updated: '2026-11-01T01:16:34Z'
parent: mem-f5cf
---

`mcpe-world` can only read. Saving needs the reverse: encode a column (blocks, data, light, heights, biomes, block entities, entities) back into the records PE 0.16 reads, and write them into the LevelDB world (rusty-leveldb can write), producing a world dir or a zip.

Format depends on the open question on the parent yak.

## Evidence

Round-trip test on the sample world: load, change blocks in a few columns, save, reload, compare; mcpe2java converts the saved world.

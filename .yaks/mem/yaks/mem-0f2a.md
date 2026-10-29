---
id: mem-0f2a
title: 'mcpe2java: convert MCPE world to Java 1.12 Anvil'
status: shorn
type: task
priority: 3
created: '2026-10-29T01:33:57Z'
updated: '2026-10-29T01:59:03Z'
parent: mem-0908
depends_on:
- mem-0d8e
---

---
▸ 2026-10-29T01:45:30Z [delta:Start investigation for yak mem-0908]

moved: hairy -> shaving

---
▸ 2026-10-29T01:59:02Z [delta:Start investigation for yak mem-0908]

Converter landed in `632d2ff` (local `main`).

```sh
cargo run --release -p mcpe2java -- "worlds/My World" out/my-world.zip
```

Output: 256 chunks and 89 block entities in about 0.2 s, giving a 633 KB zip.

- Block ids pass through, with exceptions for fence wood types, swapped ids (dropper/activator rail, wooden slabs, grass path/end rod), podzol, beetroot, piston and trapdoor facings, and PE-only blocks (nether reactor core → iron block, item frame block → air).
- Light is recomputed with Java's rules (sky column + BFS flood across chunk borders), because PE sub-chunks store no light and unlit chunks render black.
- `level.dat`: Java 1.12.2 (DataVersion 1343), a void flat generator, a world border of 256 centred on (128, 128), and the player's position, rotation and inventory taken from `~local_player`.
- Converted: signs (24, as JSON text), chests, dispensers, beds, flower pots.
- Dropped: NetherReactor (381, pointless), ItemFrame (9; an entity in Java), PistonArm (9), Cauldron (1). Mobs and entities aren't converted.

## Evidence

- `cargo test`: 4 converter tests against My World (one region, 256 chunks; gzipped Java `level.dat`; all 24 signs with text; surface air is sky-lit). Clippy is clean.
- An independent read-back with `prismarine-provider-anvil` 1.12.2 (the loader mcraft.fun's server uses) found 256 chunks with sensible block counts. Air above y=60 had 3,950,281 sky-lit blocks against 890 dark. Every sign's text round-trips, e.g. "i love / this / waterfall".

Next: mem-ef1e, loading it in the actual browser client.

---
▸ 2026-10-29T01:59:03Z [delta:Start investigation for yak mem-0908]

moved: shaving -> shorn

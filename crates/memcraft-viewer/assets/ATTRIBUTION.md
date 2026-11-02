# Texture attribution

The block and item textures in `textures/` and the painting picture in
`paintings/` are from **Pixel Perfection** by Hugh
"XSSheep" Rutland, licensed under the
[Creative Commons Attribution-ShareAlike 4.0 International License](https://creativecommons.org/licenses/by-sa/4.0/)
(CC BY-SA 4.0). The full legal code is in
[`LICENSE-CC-BY-SA-4.0.md`](LICENSE-CC-BY-SA-4.0.md).

- Original pack: <https://www.planetminecraft.com/texture-pack/131pixel-perfection/>
  ("Pixel Perfection by XSSheep is licensed under a Creative Commons
  Attribution-Share Alike 4.0 International License", and "Feel free to use
  these textures in any mixpack or whatever you want (paid or unpaid)").
- Files taken from the Pixel Perfection 1.11 resource pack as redistributed in
  <https://github.com/PrismarineJS/minecraft-assets-pixel-perfection>
  (`data/1.11/assets/minecraft/textures/blocks/`, `.../items/` for the `item_*`
  icons and `.../painting/` for the paintings, commit
  `81e383742b9b68eb26fe13857ca23ac9dd52114a`, data last changed in `b36fb31`; its `pack.txt` reads
  "XSSheep's Pixel Perfection."). Every file was kept byte for byte; only the
  names differ, see [`textures-sources.txt`](textures-sources.txt).

## Changes

`build.rs` adapts the textures when it builds the atlas that is embedded in the
viewer:

- only the first 16x16 frame of animated textures (water, lava, fire) is used;
- the grass, leaf, vine, fern, tall grass and lily pad textures, which the
  pack leaves greyscale for the game to tint by biome, are multiplied by a fixed
  green; the grass side texture gets its (tinted) overlay composited on top;
- all textures are packed into one image;
- the painting picture is embedded as it is, and the viewer cuts each motif out
  of it (the item frame's `itemframe_*` textures and the item icons are tiles of
  the atlas).

The atlas and the tiles in it are therefore an adaptation of Pixel Perfection and
are shared under the same CC BY-SA 4.0 licence. The viewer's source code is not
part of that adaptation and keeps the licence of the rest of this repository.

No texture from Mojang's Minecraft is included.

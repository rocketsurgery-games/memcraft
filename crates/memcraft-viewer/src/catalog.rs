//! Every block the viewer can place: Pocket Edition 0.16 `(id, data)`, a
//! display name, and a category, in roughly creative-inventory order.
//!
//! Only blocks that make sense to hold in a hand are listed: no technical
//! halves (upper doors, double-plant tops, double slabs, piston heads), no
//! powered/lit twins (lit furnace, powered repeater), no portals, and no
//! blocks [`blocks::look`] cannot draw. Orientation data is a plain default
//! (stairs ascend east, doors are the lower half facing the default way,
//! furnaces face +z so the front is the icon); choosing a facing at placement
//! time is up to the caller.

// Nothing uses the catalog yet (the hotbar will).
#![allow(dead_code)]

use std::sync::LazyLock;

use crate::blocks::{self, Look, Tile};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Category {
    Building,
    Decoration,
    Plants,
    Redstone,
    Misc,
}

impl Category {
    /// In creative-inventory (and table) order.
    pub const ALL: [Category; 5] =
        [Category::Building, Category::Decoration, Category::Plants, Category::Redstone, Category::Misc];

    pub fn name(self) -> &'static str {
        match self {
            Category::Building => "Building",
            Category::Decoration => "Decoration",
            Category::Plants => "Plants",
            Category::Redstone => "Redstone",
            Category::Misc => "Misc",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Entry {
    pub id: u8,
    pub data: u8,
    pub name: &'static str,
    pub category: Category,
}

/// Blocks whose icon is the top face rather than a side: the side of grass or
/// a cake says less than what you see from above.
const TOP_ICON: [u8; 7] = [2, 60, 92, 110, 116, 198, 243];

impl Entry {
    /// How the block is drawn in the world.
    pub fn look(&self) -> Look {
        blocks::look(self.id, self.data)
    }

    /// The atlas tile to show as this block's icon: the +z side face, or the
    /// top for the few blocks in [`TOP_ICON`]. Flat-coloured blocks give
    /// `WHITE`; tint it with `look().color`.
    pub fn icon(&self) -> Tile {
        let face = if TOP_ICON.contains(&self.id) { 2 } else { 4 };
        self.look().tiles[face]
    }
}

/// All entries, grouped by category in [`Category::ALL`] order.
pub fn all() -> &'static [Entry] {
    static ALL: LazyLock<Vec<Entry>> = LazyLock::new(build);
    &ALL
}

/// The names of a 16-colour block, in data-value order, e.g. `dyed!("Wool")`.
macro_rules! dyed {
    ($kind:literal) => {
        [
            concat!("White ", $kind),
            concat!("Orange ", $kind),
            concat!("Magenta ", $kind),
            concat!("Light Blue ", $kind),
            concat!("Yellow ", $kind),
            concat!("Lime ", $kind),
            concat!("Pink ", $kind),
            concat!("Gray ", $kind),
            concat!("Light Gray ", $kind),
            concat!("Cyan ", $kind),
            concat!("Purple ", $kind),
            concat!("Blue ", $kind),
            concat!("Brown ", $kind),
            concat!("Green ", $kind),
            concat!("Red ", $kind),
            concat!("Black ", $kind),
        ]
    };
}

struct Builder {
    out: Vec<Entry>,
    category: Category,
}

impl Builder {
    fn one(&mut self, id: u8, data: u8, name: &'static str) {
        self.out.push(Entry { id, data, name, category: self.category });
    }

    /// Data values `0..` in order, one per name.
    fn run(&mut self, id: u8, names: &[&'static str]) {
        for (data, name) in names.iter().enumerate() {
            self.one(id, data as u8, name);
        }
    }
}

const WOODS: [&str; 6] = ["Oak", "Spruce", "Birch", "Jungle", "Acacia", "Dark Oak"];

fn build() -> Vec<Entry> {
    let mut b = Builder { out: Vec::new(), category: Category::Building };

    // Building: terrain, stone, wood, bricks, slabs, stairs.
    b.run(
        1,
        &[
            "Stone",
            "Granite",
            "Polished Granite",
            "Diorite",
            "Polished Diorite",
            "Andesite",
            "Polished Andesite",
        ],
    );
    b.one(2, 0, "Grass Block");
    b.one(3, 0, "Dirt");
    b.one(3, 1, "Coarse Dirt");
    b.one(243, 0, "Podzol");
    b.one(110, 0, "Mycelium");
    b.one(198, 0, "Grass Path");
    b.one(60, 0, "Farmland");
    b.one(4, 0, "Cobblestone");
    b.one(48, 0, "Mossy Cobblestone");
    b.run(5, &["Oak Planks", "Spruce Planks", "Birch Planks", "Jungle Planks", "Acacia Planks", "Dark Oak Planks"]);
    b.one(12, 0, "Sand");
    b.one(12, 1, "Red Sand");
    b.one(13, 0, "Gravel");
    b.one(14, 0, "Gold Ore");
    b.one(15, 0, "Iron Ore");
    b.one(16, 0, "Coal Ore");
    b.one(21, 0, "Lapis Lazuli Ore");
    b.one(56, 0, "Diamond Ore");
    b.one(73, 0, "Redstone Ore");
    b.one(129, 0, "Emerald Ore");
    b.one(153, 0, "Nether Quartz Ore");
    b.one(173, 0, "Block of Coal");
    b.one(42, 0, "Block of Iron");
    b.one(41, 0, "Block of Gold");
    b.one(22, 0, "Lapis Lazuli Block");
    b.one(57, 0, "Block of Diamond");
    b.one(133, 0, "Block of Emerald");
    b.run(17, &["Oak Log", "Spruce Log", "Birch Log", "Jungle Log"]);
    b.run(162, &["Acacia Log", "Dark Oak Log"]);
    b.run(98, &["Stone Bricks", "Mossy Stone Bricks", "Cracked Stone Bricks", "Chiseled Stone Bricks"]);
    b.one(45, 0, "Bricks");
    b.run(24, &["Sandstone", "Chiseled Sandstone", "Smooth Sandstone"]);
    // The red sandstone variants (data 1, 2) share one look, so only one entry.
    b.one(179, 0, "Red Sandstone");
    b.run(155, &["Block of Quartz", "Chiseled Quartz Block", "Pillar Quartz Block"]);
    b.run(168, &["Prismarine", "Prismarine Bricks", "Dark Prismarine"]);
    b.one(201, 0, "Purpur Block");
    b.one(202, 0, "Purpur Pillar");
    b.one(112, 0, "Nether Bricks");
    b.one(87, 0, "Netherrack");
    b.one(88, 0, "Soul Sand");
    b.one(121, 0, "End Stone");
    b.one(49, 0, "Obsidian");
    b.one(172, 0, "Terracotta");
    b.run(159, &dyed!("Terracotta"));
    b.one(44, 0, "Stone Slab");
    b.one(44, 1, "Sandstone Slab");
    b.one(44, 3, "Cobblestone Slab");
    b.one(44, 4, "Brick Slab");
    b.one(44, 5, "Stone Brick Slab");
    b.one(44, 6, "Quartz Slab");
    b.one(44, 7, "Nether Brick Slab");
    b.one(182, 0, "Red Sandstone Slab");
    b.one(205, 0, "Purpur Slab");
    b.run(
        158,
        &["Oak Slab", "Spruce Slab", "Birch Slab", "Jungle Slab", "Acacia Slab", "Dark Oak Slab"],
    );
    b.one(53, 0, "Oak Stairs");
    b.one(134, 0, "Spruce Stairs");
    b.one(135, 0, "Birch Stairs");
    b.one(136, 0, "Jungle Stairs");
    b.one(163, 0, "Acacia Stairs");
    b.one(164, 0, "Dark Oak Stairs");
    b.one(67, 0, "Cobblestone Stairs");
    b.one(108, 0, "Brick Stairs");
    b.one(109, 0, "Stone Brick Stairs");
    b.one(128, 0, "Sandstone Stairs");
    b.one(180, 0, "Red Sandstone Stairs");
    b.one(156, 0, "Quartz Stairs");
    b.one(203, 0, "Purpur Stairs");
    b.one(114, 0, "Nether Brick Stairs");
    b.run(85, &["Oak Fence", "Spruce Fence", "Birch Fence", "Jungle Fence", "Acacia Fence", "Dark Oak Fence"]);
    b.one(113, 0, "Nether Brick Fence");
    b.one(139, 0, "Cobblestone Wall");
    b.one(139, 1, "Mossy Cobblestone Wall");

    // Decoration.
    b.category = Category::Decoration;
    b.run(35, &dyed!("Wool"));
    b.run(171, &dyed!("Carpet"));
    b.one(20, 0, "Glass");
    b.run(241, &dyed!("Stained Glass"));
    b.one(101, 0, "Iron Bars");
    b.one(47, 0, "Bookshelf");
    b.one(58, 0, "Crafting Table");
    b.one(61, 3, "Furnace");
    b.one(54, 0, "Chest");
    b.one(146, 0, "Trapped Chest");
    b.one(116, 0, "Enchanting Table");
    b.one(117, 0, "Brewing Stand");
    b.one(118, 0, "Cauldron");
    b.one(145, 0, "Anvil");
    b.one(84, 0, "Jukebox");
    b.one(26, 0, "Bed");
    b.one(92, 0, "Cake");
    b.one(140, 0, "Flower Pot");
    b.one(65, 2, "Ladder");
    b.one(50, 5, "Torch");
    b.one(89, 0, "Glowstone");
    b.one(169, 0, "Sea Lantern");
    b.one(19, 0, "Sponge");
    b.one(165, 0, "Slime Block");
    b.one(170, 0, "Hay Bale");
    b.one(103, 0, "Melon");
    b.one(86, 0, "Carved Pumpkin");
    b.one(91, 0, "Jack o'Lantern");
    b.one(82, 0, "Clay");
    b.one(80, 0, "Snow");
    b.one(78, 0, "Snow Layer");
    b.one(79, 0, "Ice");
    b.one(174, 0, "Packed Ice");

    // Plants.
    b.category = Category::Plants;
    b.run(18, &["Oak Leaves", "Spruce Leaves", "Birch Leaves", "Jungle Leaves"]);
    b.run(161, &["Acacia Leaves", "Dark Oak Leaves"]);
    b.run(
        6,
        &["Oak Sapling", "Spruce Sapling", "Birch Sapling", "Jungle Sapling", "Acacia Sapling", "Dark Oak Sapling"],
    );
    b.one(31, 1, "Tall Grass");
    b.one(31, 2, "Fern");
    b.one(32, 0, "Dead Bush");
    b.one(37, 0, "Dandelion");
    b.run(
        38,
        &[
            "Poppy",
            "Blue Orchid",
            "Allium",
            "Azure Bluet",
            "Red Tulip",
            "Orange Tulip",
            "White Tulip",
            "Pink Tulip",
            "Oxeye Daisy",
        ],
    );
    // Lower halves; the upper half is the same data with bit 8 set.
    b.run(175, &["Sunflower", "Lilac", "Double Tallgrass", "Large Fern", "Rose Bush", "Peony"]);
    b.one(39, 0, "Brown Mushroom");
    b.one(40, 0, "Red Mushroom");
    b.one(106, 0, "Vines");
    b.one(111, 0, "Lily Pad");
    b.one(81, 0, "Cactus");
    b.one(83, 0, "Sugar Cane");
    // Crops are listed fully grown so the icon is recognisable.
    b.one(59, 7, "Wheat");
    b.one(141, 7, "Carrots");
    b.one(142, 7, "Potatoes");
    b.one(244, 7, "Beetroots");
    b.one(115, 3, "Nether Wart");

    // Redstone and mechanisms.
    b.category = Category::Redstone;
    b.one(55, 0, "Redstone Dust");
    b.one(76, 5, "Redstone Torch");
    b.one(93, 0, "Redstone Repeater");
    b.one(123, 0, "Redstone Lamp");
    b.one(152, 0, "Block of Redstone");
    b.one(46, 0, "TNT");
    b.one(29, 1, "Piston");
    b.one(33, 1, "Sticky Piston");
    b.one(23, 3, "Dispenser");
    b.one(69, 0, "Lever");
    b.one(77, 0, "Stone Button");
    b.one(143, 0, "Wooden Button");
    b.one(70, 0, "Stone Pressure Plate");
    b.one(72, 0, "Wooden Pressure Plate");
    b.one(147, 0, "Light Weighted Pressure Plate");
    b.one(148, 0, "Heavy Weighted Pressure Plate");
    b.one(66, 0, "Rail");
    b.one(27, 0, "Powered Rail");
    b.one(28, 0, "Detector Rail");
    b.one(126, 0, "Activator Rail");
    b.one(64, 0, "Oak Door");
    b.one(193, 0, "Spruce Door");
    b.one(194, 0, "Birch Door");
    b.one(195, 0, "Jungle Door");
    b.one(196, 0, "Acacia Door");
    b.one(197, 0, "Dark Oak Door");
    b.one(71, 0, "Iron Door");
    b.one(96, 0, "Trapdoor");
    b.one(167, 0, "Iron Trapdoor");
    b.one(107, 0, "Oak Fence Gate");
    b.one(183, 0, "Spruce Fence Gate");
    b.one(184, 0, "Birch Fence Gate");
    b.one(185, 0, "Jungle Fence Gate");
    b.one(186, 0, "Dark Oak Fence Gate");
    b.one(187, 0, "Acacia Fence Gate");

    // Misc: blocks a survival player never holds.
    b.category = Category::Misc;
    b.one(7, 0, "Bedrock");
    b.one(9, 0, "Water");
    b.one(11, 0, "Lava");
    b.one(52, 0, "Monster Spawner");
    b.one(99, 14, "Brown Mushroom Block");
    b.one(100, 14, "Red Mushroom Block");
    b.one(99, 10, "Mushroom Stem");
    b.one(245, 0, "Stonecutter");
    b.one(246, 0, "Glowing Obsidian");
    b.one(247, 0, "Nether Reactor Core");

    b.out
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;
    use crate::blocks::Shape;

    #[test]
    fn every_entry_is_drawn() {
        for e in all() {
            assert_ne!(e.look().shape, Shape::Air, "{} ({}:{}) is air", e.name, e.id, e.data);
        }
    }

    #[test]
    fn no_entry_uses_the_unknown_block_colour() {
        for e in all() {
            assert_ne!(e.look().color, [1.0, 0.0, 1.0], "{} ({}:{}) has no look", e.name, e.id, e.data);
        }
    }

    #[test]
    fn names_are_unique() {
        let mut seen = HashSet::new();
        for e in all() {
            assert!(seen.insert(e.name), "duplicate name {:?}", e.name);
        }
    }

    #[test]
    fn blocks_are_unique() {
        let mut seen = HashSet::new();
        for e in all() {
            assert!(seen.insert((e.id, e.data)), "duplicate block {}:{} ({})", e.id, e.data, e.name);
        }
    }

    #[test]
    fn data_fits_four_bits() {
        assert!(all().iter().all(|e| e.data < 16));
    }

    #[test]
    fn categories_are_grouped_in_order() {
        let order: Vec<_> = all().iter().map(|e| e.category).collect();
        let mut sorted = order.clone();
        sorted.sort_by_key(|c| Category::ALL.iter().position(|a| a == c));
        assert_eq!(order, sorted);
    }

    /// Run with `--nocapture` to see the counts.
    #[test]
    fn counts_by_category() {
        let mut total = 0;
        for c in Category::ALL {
            let n = all().iter().filter(|e| e.category == c).count();
            println!("{:<10} {n}", c.name());
            assert!(n > 0, "{} is empty", c.name());
            total += n;
        }
        println!("{:<10} {total}", "total");
        assert_eq!(total, all().len());
    }
}

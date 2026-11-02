//! Which atlas tile shows a Pocket Edition item (not a block), and what the
//! game calls it: the item stacks of containers and the hotbar, and the item
//! in an item frame. The icons are Pixel Perfection's item textures, see
//! `assets/ATTRIBUTION.md`; items without one are left out (and so have no
//! name here).

use crate::blocks::{atlas::tile as t, Tile};

/// The icon of item `id` with damage value `damage`, for item ids of 256 and up.
pub fn icon(id: i16, damage: i16) -> Option<Tile> {
    entry(id, damage).map(|(_, tile)| tile)
}

/// The name of item `id` with damage value `damage`: "Iron Ingot", "Rose Red".
pub fn name(id: i16, damage: i16) -> Option<&'static str> {
    entry(id, damage).map(|(name, _)| name)
}

fn entry(id: i16, damage: i16) -> Option<(&'static str, Tile)> {
    if id == 351 {
        return DYES.get(usize::try_from(damage).ok()?).copied();
    }
    ICONS.binary_search_by_key(&id, |&(i, ..)| i).ok().map(|i| (ICONS[i].1, ICONS[i].2))
}

/// Dye colours by damage value: black, red, green, brown, blue, purple, cyan,
/// light grey, grey, pink, lime, yellow, light blue, magenta, orange, white.
const DYES: [(&str, Tile); 16] = [
    ("Ink Sac", t::ITEM_DYE_POWDER_BLACK),
    ("Rose Red", t::ITEM_DYE_POWDER_RED),
    ("Cactus Green", t::ITEM_DYE_POWDER_GREEN),
    ("Cocoa Beans", t::ITEM_DYE_POWDER_BROWN),
    ("Lapis Lazuli", t::ITEM_DYE_POWDER_BLUE),
    ("Purple Dye", t::ITEM_DYE_POWDER_PURPLE),
    ("Cyan Dye", t::ITEM_DYE_POWDER_CYAN),
    ("Light Gray Dye", t::ITEM_DYE_POWDER_SILVER),
    ("Gray Dye", t::ITEM_DYE_POWDER_GRAY),
    ("Pink Dye", t::ITEM_DYE_POWDER_PINK),
    ("Lime Dye", t::ITEM_DYE_POWDER_LIME),
    ("Dandelion Yellow", t::ITEM_DYE_POWDER_YELLOW),
    ("Light Blue Dye", t::ITEM_DYE_POWDER_LIGHT_BLUE),
    ("Magenta Dye", t::ITEM_DYE_POWDER_MAGENTA),
    ("Orange Dye", t::ITEM_DYE_POWDER_ORANGE),
    ("Bone Meal", t::ITEM_DYE_POWDER_WHITE),
];

/// Sorted by item id.
const ICONS: [(i16, &str, Tile); 135] = [
    (256, "Iron Shovel", t::ITEM_IRON_SHOVEL),
    (257, "Iron Pickaxe", t::ITEM_IRON_PICKAXE),
    (258, "Iron Axe", t::ITEM_IRON_AXE),
    (259, "Flint and Steel", t::ITEM_FLINT_AND_STEEL),
    (260, "Apple", t::ITEM_APPLE),
    (261, "Bow", t::ITEM_BOW_STANDBY),
    (262, "Arrow", t::ITEM_ARROW),
    (263, "Coal", t::ITEM_COAL),
    (264, "Diamond", t::ITEM_DIAMOND),
    (265, "Iron Ingot", t::ITEM_IRON_INGOT),
    (266, "Gold Ingot", t::ITEM_GOLD_INGOT),
    (267, "Iron Sword", t::ITEM_IRON_SWORD),
    (268, "Wooden Sword", t::ITEM_WOOD_SWORD),
    (269, "Wooden Shovel", t::ITEM_WOOD_SHOVEL),
    (270, "Wooden Pickaxe", t::ITEM_WOOD_PICKAXE),
    (271, "Wooden Axe", t::ITEM_WOOD_AXE),
    (272, "Stone Sword", t::ITEM_STONE_SWORD),
    (273, "Stone Shovel", t::ITEM_STONE_SHOVEL),
    (274, "Stone Pickaxe", t::ITEM_STONE_PICKAXE),
    (275, "Stone Axe", t::ITEM_STONE_AXE),
    (276, "Diamond Sword", t::ITEM_DIAMOND_SWORD),
    (277, "Diamond Shovel", t::ITEM_DIAMOND_SHOVEL),
    (278, "Diamond Pickaxe", t::ITEM_DIAMOND_PICKAXE),
    (279, "Diamond Axe", t::ITEM_DIAMOND_AXE),
    (280, "Stick", t::ITEM_STICK),
    (281, "Bowl", t::ITEM_BOWL),
    (282, "Mushroom Stew", t::ITEM_MUSHROOM_STEW),
    (283, "Golden Sword", t::ITEM_GOLD_SWORD),
    (284, "Golden Shovel", t::ITEM_GOLD_SHOVEL),
    (285, "Golden Pickaxe", t::ITEM_GOLD_PICKAXE),
    (286, "Golden Axe", t::ITEM_GOLD_AXE),
    (287, "String", t::ITEM_STRING),
    (288, "Feather", t::ITEM_FEATHER),
    (289, "Gunpowder", t::ITEM_GUNPOWDER),
    (290, "Wooden Hoe", t::ITEM_WOOD_HOE),
    (291, "Stone Hoe", t::ITEM_STONE_HOE),
    (292, "Iron Hoe", t::ITEM_IRON_HOE),
    (293, "Diamond Hoe", t::ITEM_DIAMOND_HOE),
    (294, "Golden Hoe", t::ITEM_GOLD_HOE),
    (295, "Wheat Seeds", t::ITEM_SEEDS_WHEAT),
    (296, "Wheat", t::ITEM_WHEAT),
    (297, "Bread", t::ITEM_BREAD),
    (298, "Leather Cap", t::ITEM_LEATHER_HELMET),
    (299, "Leather Tunic", t::ITEM_LEATHER_CHESTPLATE),
    (300, "Leather Pants", t::ITEM_LEATHER_LEGGINGS),
    (301, "Leather Boots", t::ITEM_LEATHER_BOOTS),
    (302, "Chain Helmet", t::ITEM_CHAINMAIL_HELMET),
    (303, "Chain Chestplate", t::ITEM_CHAINMAIL_CHESTPLATE),
    (304, "Chain Leggings", t::ITEM_CHAINMAIL_LEGGINGS),
    (305, "Chain Boots", t::ITEM_CHAINMAIL_BOOTS),
    (306, "Iron Helmet", t::ITEM_IRON_HELMET),
    (307, "Iron Chestplate", t::ITEM_IRON_CHESTPLATE),
    (308, "Iron Leggings", t::ITEM_IRON_LEGGINGS),
    (309, "Iron Boots", t::ITEM_IRON_BOOTS),
    (310, "Diamond Helmet", t::ITEM_DIAMOND_HELMET),
    (311, "Diamond Chestplate", t::ITEM_DIAMOND_CHESTPLATE),
    (312, "Diamond Leggings", t::ITEM_DIAMOND_LEGGINGS),
    (313, "Diamond Boots", t::ITEM_DIAMOND_BOOTS),
    (314, "Golden Helmet", t::ITEM_GOLD_HELMET),
    (315, "Golden Chestplate", t::ITEM_GOLD_CHESTPLATE),
    (316, "Golden Leggings", t::ITEM_GOLD_LEGGINGS),
    (317, "Golden Boots", t::ITEM_GOLD_BOOTS),
    (318, "Flint", t::ITEM_FLINT),
    (319, "Raw Porkchop", t::ITEM_PORKCHOP_RAW),
    (320, "Cooked Porkchop", t::ITEM_PORKCHOP_COOKED),
    (321, "Painting", t::ITEM_PAINTING),
    (322, "Golden Apple", t::ITEM_APPLE_GOLDEN),
    (323, "Sign", t::ITEM_SIGN),
    (324, "Wooden Door", t::ITEM_DOOR_WOOD),
    (325, "Bucket", t::ITEM_BUCKET_EMPTY),
    (326, "Water Bucket", t::ITEM_BUCKET_WATER),
    (327, "Lava Bucket", t::ITEM_BUCKET_LAVA),
    (328, "Minecart", t::ITEM_MINECART_NORMAL),
    (329, "Saddle", t::ITEM_SADDLE),
    (330, "Iron Door", t::ITEM_DOOR_IRON),
    (331, "Redstone", t::ITEM_REDSTONE_DUST),
    (332, "Snowball", t::ITEM_SNOWBALL),
    (333, "Boat", t::ITEM_BOAT),
    (334, "Leather", t::ITEM_LEATHER),
    (335, "Milk Bucket", t::ITEM_BUCKET_MILK),
    (336, "Brick", t::ITEM_BRICK),
    (337, "Clay", t::ITEM_CLAY_BALL),
    (338, "Sugar Canes", t::ITEM_REEDS),
    (339, "Paper", t::ITEM_PAPER),
    (340, "Book", t::ITEM_BOOK_NORMAL),
    (341, "Slimeball", t::ITEM_SLIMEBALL),
    (344, "Egg", t::ITEM_EGG),
    (345, "Compass", t::ITEM_COMPASS_00),
    (347, "Clock", t::ITEM_CLOCK_00),
    (348, "Glowstone Dust", t::ITEM_GLOWSTONE_DUST),
    (349, "Raw Fish", t::ITEM_FISH_COD_RAW),
    (350, "Cooked Fish", t::ITEM_FISH_COD_COOKED),
    (352, "Bone", t::ITEM_BONE),
    (353, "Sugar", t::ITEM_SUGAR),
    (354, "Cake", t::ITEM_CAKE),
    (355, "Bed", t::ITEM_BED),
    (357, "Cookie", t::ITEM_COOKIE),
    (359, "Shears", t::ITEM_SHEARS),
    (360, "Melon", t::ITEM_MELON),
    (361, "Pumpkin Seeds", t::ITEM_SEEDS_PUMPKIN),
    (362, "Melon Seeds", t::ITEM_SEEDS_MELON),
    (363, "Raw Beef", t::ITEM_BEEF_RAW),
    (364, "Steak", t::ITEM_BEEF_COOKED),
    (365, "Raw Chicken", t::ITEM_CHICKEN_RAW),
    (366, "Cooked Chicken", t::ITEM_CHICKEN_COOKED),
    (367, "Rotten Flesh", t::ITEM_ROTTEN_FLESH),
    (368, "Ender Pearl", t::ITEM_ENDER_PEARL),
    (369, "Blaze Rod", t::ITEM_BLAZE_ROD),
    (370, "Ghast Tear", t::ITEM_GHAST_TEAR),
    (371, "Gold Nugget", t::ITEM_GOLD_NUGGET),
    (374, "Glass Bottle", t::ITEM_POTION_BOTTLE_EMPTY),
    (377, "Blaze Powder", t::ITEM_BLAZE_POWDER),
    (378, "Magma Cream", t::ITEM_MAGMA_CREAM),
    (381, "Eye of Ender", t::ITEM_ENDER_EYE),
    (382, "Glistering Melon", t::ITEM_MELON_SPECKLED),
    (388, "Emerald", t::ITEM_EMERALD),
    (389, "Item Frame", t::ITEM_ITEM_FRAME),
    (390, "Flower Pot", t::ITEM_FLOWER_POT),
    (391, "Carrot", t::ITEM_CARROT),
    (392, "Potato", t::ITEM_POTATO),
    (393, "Baked Potato", t::ITEM_POTATO_BAKED),
    (394, "Poisonous Potato", t::ITEM_POTATO_POISONOUS),
    (396, "Golden Carrot", t::ITEM_CARROT_GOLDEN),
    (400, "Pumpkin Pie", t::ITEM_PUMPKIN_PIE),
    (404, "Redstone Comparator", t::ITEM_COMPARATOR),
    (405, "Nether Brick", t::ITEM_NETHERBRICK),
    (406, "Nether Quartz", t::ITEM_QUARTZ),
    (407, "Minecart with TNT", t::ITEM_MINECART_TNT),
    (408, "Minecart with Hopper", t::ITEM_MINECART_HOPPER),
    (410, "Hopper", t::ITEM_HOPPER),
    (417, "Iron Horse Armor", t::ITEM_IRON_HORSE_ARMOR),
    (418, "Golden Horse Armor", t::ITEM_GOLD_HORSE_ARMOR),
    (419, "Diamond Horse Armor", t::ITEM_DIAMOND_HORSE_ARMOR),
    (420, "Lead", t::ITEM_LEAD),
    (421, "Name Tag", t::ITEM_NAME_TAG),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icons_are_sorted_by_id_and_found() {
        assert!(ICONS.windows(2).all(|w| w[0].0 < w[1].0));
        assert_eq!(name(265, 0), Some("Iron Ingot"));
        assert_eq!(name(351, 15), Some("Bone Meal"));
        assert_eq!(name(342, 0), None);
        assert_eq!(icon(265, 0), Some(t::ITEM_IRON_INGOT));
        assert_eq!(icon(351, 1), Some(t::ITEM_DYE_POWDER_RED));
        assert_eq!(icon(351, 16), None);
        assert_eq!(icon(30000, 0), None);
    }
}

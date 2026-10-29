//! Block translation and light properties.
//!
//! Pocket Edition 0.16 numbered blocks almost exactly like Java 1.12: both
//! descend from the same pre-flattening id space. This module holds only
//! the exceptions, plus Java's light opacity and emission tables (needed
//! because PE sub-chunk records carry no light at all).

/// Translates a Pocket Edition `(id, data)` to Java 1.12 `(id, data)`.
pub fn to_java(id: u8, data: u8) -> (u8, u8) {
    match id {
        // PE folds every wood fence into id 85, keyed by wood type.
        85 => match data {
            1 => (188, 0), // spruce
            2 => (189, 0), // birch
            3 => (190, 0), // jungle
            4 => (192, 0), // acacia
            5 => (191, 0), // dark oak
            _ => (85, 0),
        },
        // Ids that PE and Java assigned to different blocks.
        125 => (158, data), // dropper
        126 => (157, data), // activator rail
        157 => (125, data), // double wooden slab
        158 => (126, data), // wooden slab
        198 => (208, 0),    // grass path
        208 => (198, data), // end rod
        243 => (3, 2),      // podzol
        244 => (207, data), // beetroot
        95 => (166, 0),     // invisible bedrock -> barrier
        // PE-only blocks with no Java counterpart: nearest look-alike.
        245 => (98, 0), // stonecutter -> stone bricks
        246 => (49, 0), // glowing obsidian -> obsidian
        247 => (42, 0), // nether reactor core -> iron block
        // The item frame is a block in PE but an entity in Java (written from
        // its block entity; see `entities::item_frame`).
        199 => (0, 0),
        // Cauldron fill level: PE 0..=6, Java 0..=3.
        118 => (118, data.min(6).div_ceil(2)),
        // Pistons: PE orders horizontal facings south/north/east/west where
        // Java uses north/south/west/east.
        29 | 33 | 34 => {
            let facing = match data & 7 {
                2 => 3,
                3 => 2,
                4 => 5,
                5 => 4,
                f => f,
            };
            (id, facing | (data & 8))
        }
        // Trapdoors: PE direction 0..3 is east/west/south/north and bits
        // 4/8 are upside-down/open; Java is north/south/west/east with
        // 4/8 = open/top.
        96 | 167 => {
            let facing = [3, 2, 1, 0][(data & 3) as usize];
            let open = if data & 8 != 0 { 4 } else { 0 };
            let top = if data & 4 != 0 { 8 } else { 0 };
            (id, facing | open | top)
        }
        _ => (id, data),
    }
}

/// Java light opacity of a block id: how much light it absorbs (0..=15).
pub fn opacity(id: u8) -> u8 {
    match id {
        // Air, glass, plants, rails, torches, signs, doors, fences, panes,
        // carpets and other non-full blocks.
        0 | 6 | 20 | 26 | 27 | 28 | 31 | 32 | 37..=40 | 50 | 51 | 54 | 55 | 59 | 63..=66 | 68..=72
        | 75..=78 | 83 | 85 | 90 | 92..=96 | 101 | 102 | 104..=107 | 111 | 113 | 115 | 117
        | 118 | 119 | 130 | 131 | 132 | 139..=151 | 154 | 160 | 166 | 167 | 171 | 175..=177
        | 183..=198 | 207 => 0,
        18 | 161 | 30 => 1, // leaves, cobweb
        8 | 9 | 79 | 212 => 3, // water, ice, frosted ice
        _ => 15,
    }
}

/// Java block light emitted by a block id (0..=15).
pub fn emission(id: u8) -> u8 {
    match id {
        10 | 11 | 51 | 89 | 91 | 119 | 124 | 138 | 169 => 15, // lava, fire, glowstone, lit pumpkin, end portal, lamp, beacon, sea lantern
        50 | 198 => 14,                                       // torch, end rod
        62 => 13,                                             // lit furnace
        90 => 11,                                             // nether portal
        74 => 9,                                              // lit redstone ore
        76 => 7,                                              // redstone torch
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_ids_pass_through() {
        assert_eq!(to_java(1, 1), (1, 1));
        assert_eq!(to_java(53, 6), (53, 6));
    }

    #[test]
    fn fences_split_by_wood() {
        assert_eq!(to_java(85, 0), (85, 0));
        assert_eq!(to_java(85, 2), (189, 0));
    }

    #[test]
    fn cauldron_level_halves() {
        assert_eq!(to_java(118, 0), (118, 0));
        assert_eq!(to_java(118, 3), (118, 2));
        assert_eq!(to_java(118, 6), (118, 3));
    }

    #[test]
    fn trapdoor_bits_swap() {
        // PE: north (3) + open (8) -> Java: north (0) + open (4).
        assert_eq!(to_java(96, 3 | 8), (96, 4));
        // PE: upside down (4) -> Java: top half (8).
        assert_eq!(to_java(96, 4), (96, 3 | 8));
    }
}

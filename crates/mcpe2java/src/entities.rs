//! Block entity, entity and player translation (PE 0.16 NBT -> Java 1.12 NBT).
//!
//! Java 1.12 stores items with string ids, and PE item numbers match the
//! classic Java numbering, so items are looked up by number in [`item_name`].
//! Unknown items are dropped rather than guessed. Enchantments, which the
//! two editions number differently, are renumbered by [`java_enchantment`].

use nbt::{compound, Tag};

fn pos(be: &Tag) -> Vec<(&'static str, Tag)> {
    ["x", "y", "z"].map(|k| (k, be.get(k).cloned().unwrap_or(Tag::Int(0)))).to_vec()
}

/// Converts one PE block entity, or returns `None` for kinds we drop.
pub fn block_entity(be: &Tag) -> Option<Tag> {
    let id = be.get("id")?.as_str()?;
    let mut out = pos(be);
    match id {
        "Sign" => {
            out.push(("id", Tag::String("minecraft:sign".into())));
            for k in ["Text1", "Text2", "Text3", "Text4"] {
                let text = be.get(k).and_then(Tag::as_str).unwrap_or("");
                out.push((k, Tag::String(json_text(text))));
            }
        }
        "Chest" | "Dispenser" | "Dropper" | "Hopper" | "Furnace" => {
            let java = match id {
                "Chest" => "minecraft:chest",
                "Dispenser" => "minecraft:dispenser",
                "Dropper" => "minecraft:dropper",
                "Hopper" => "minecraft:hopper",
                _ => "minecraft:furnace",
            };
            out.push(("id", Tag::String(java.into())));
            out.push(("Items", Tag::List(10, items(be.get("Items")))));
        }
        "Bed" => {
            out.push(("id", Tag::String("minecraft:bed".into())));
            out.push(("color", Tag::Int(be.get("color").and_then(Tag::as_i64).unwrap_or(14) as i32)));
        }
        "FlowerPot" => {
            out.push(("id", Tag::String("minecraft:flower_pot".into())));
            let item = be.get("item").and_then(Tag::as_i64).unwrap_or(0);
            out.push(("Item", Tag::String(item_name(item as i16).unwrap_or("minecraft:air").into())));
            out.push(("Data", Tag::Int(be.get("mData").and_then(Tag::as_i64).unwrap_or(0) as i32)));
        }
        _ => return None,
    }
    Some(compound(out))
}

/// Java 1.12 sign lines are JSON text components.
fn json_text(s: &str) -> String {
    let mut out = String::from("{\"text\":\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push_str("\"}");
    out
}

/// Converts a PE item list, dropping empty slots and unknown items.
fn items(list: Option<&Tag>) -> Vec<Tag> {
    let Some(list) = list.and_then(Tag::as_list) else { return vec![] };
    list.iter().filter_map(|item| item_stack(item, "Slot", |s| s)).collect()
}

fn item_stack(item: &Tag, slot_key: &str, map_slot: impl Fn(i8) -> i8) -> Option<Tag> {
    let id = item.get("id")?.as_i64()? as i16;
    let count = item.get("Count")?.as_i64()? as i8;
    if id <= 0 || count <= 0 {
        return None;
    }
    let name = item_name(id)?;
    let mut out = vec![
        ("id", Tag::String(name.into())),
        ("Count", Tag::Byte(count)),
        ("Damage", Tag::Short(item.get("Damage").and_then(Tag::as_i64).unwrap_or(0) as i16)),
    ];
    if let Some(slot) = item.get(slot_key).and_then(Tag::as_i64) {
        out.push(("Slot", Tag::Byte(map_slot(slot as i8))));
    }
    if let Some(tag) = item.get("tag") {
        out.push(("tag", item_tag(tag)));
    }
    Some(compound(out))
}

/// An item's `tag` compound, with its enchantments renumbered (unknown ones
/// dropped). Everything else (names, repair cost) has the same shape.
fn item_tag(tag: &Tag) -> Tag {
    let Tag::Compound(entries) = tag else { return tag.clone() };
    let enchantment = |e: &Tag| -> Option<Tag> {
        let id = java_enchantment(e.get("id")?.as_i64()?)?;
        let lvl = e.get("lvl")?.as_i64()? as i16;
        Some(compound([("id", Tag::Short(id)), ("lvl", Tag::Short(lvl))]))
    };
    Tag::Compound(
        entries
            .iter()
            .map(|(k, v)| match (k.as_str(), v) {
                ("ench", Tag::List(_, list)) => (k.clone(), Tag::List(10, list.iter().filter_map(enchantment).collect())),
                _ => (k.clone(), v.clone()),
            })
            .collect(),
    )
}

/// Java 1.12 enchantment id for a PE one. PE numbers them consecutively;
/// Java leaves gaps between armour, weapon, tool and bow enchantments.
pub fn java_enchantment(pe: i64) -> Option<i16> {
    Some(match pe {
        0 => 0,   // protection
        1 => 1,   // fire protection
        2 => 2,   // feather falling
        3 => 3,   // blast protection
        4 => 4,   // projectile protection
        5 => 7,   // thorns
        6 => 5,   // respiration
        7 => 8,   // depth strider
        8 => 6,   // aqua affinity
        9 => 16,  // sharpness
        10 => 17, // smite
        11 => 18, // bane of arthropods
        12 => 19, // knockback
        13 => 20, // fire aspect
        14 => 21, // looting
        15 => 32, // efficiency
        16 => 33, // silk touch
        17 => 34, // unbreaking
        18 => 35, // fortune
        19 => 48, // power
        20 => 49, // punch
        21 => 50, // flame
        22 => 51, // infinity
        23 => 61, // luck of the sea
        24 => 62, // lure
        25 => 9,  // frost walker
        26 => 70, // mending
        _ => return None,
    })
}

fn floats(t: &Tag, k: &str) -> Vec<f64> {
    t.get(k).and_then(Tag::as_list).map(|l| l.iter().filter_map(Tag::as_f64).collect()).unwrap_or_default()
}

/// Fields every Java entity reads.
fn base(java_id: &str, pos: &[f64], motion: &[f64], rotation: &[f64], on_ground: bool) -> Vec<(&'static str, Tag)> {
    let doubles = |v: &[f64]| Tag::List(6, v.iter().map(|&f| Tag::Double(f)).collect());
    vec![
        ("id", Tag::String(java_id.into())),
        ("Pos", doubles(pos)),
        ("Motion", doubles(motion)),
        ("Rotation", Tag::List(5, rotation.iter().map(|&f| Tag::Float(f as f32)).collect())),
        ("OnGround", Tag::Byte(on_ground as i8)),
        ("Air", Tag::Short(300)),
        ("Fire", Tag::Short(-1)),
    ]
}

/// Unit offset `(x, z)` of a Java horizontal facing (0 south, 1 west, 2 north, 3 east).
fn facing_offset(facing: i8) -> (i32, i32) {
    match facing & 3 {
        0 => (0, 1),
        1 => (-1, 0),
        2 => (0, -1),
        _ => (1, 0),
    }
}

/// A Java hanging entity (item frame, painting). `tile` is the block it
/// occupies and `facing` the way it faces, away from its wall. Java
/// recomputes the exact position from these when it loads the entity.
fn hanging(java_id: &str, tile: [i32; 3], facing: i8) -> Vec<(&'static str, Tag)> {
    let (dx, dz) = facing_offset(facing);
    let pos = [
        tile[0] as f64 + 0.5 - dx as f64 * 0.46875,
        tile[1] as f64 + 0.5,
        tile[2] as f64 + 0.5 - dz as f64 * 0.46875,
    ];
    let mut out = base(java_id, &pos, &[0.0; 3], &[facing as f64 * 90.0, 0.0], false);
    out.extend([
        ("TileX", Tag::Int(tile[0])),
        ("TileY", Tag::Int(tile[1])),
        ("TileZ", Tag::Int(tile[2])),
        ("Facing", Tag::Byte(facing)),
    ]);
    out
}

/// The Java item frame entity for a PE item frame block (id 199) with block
/// data `data` and its `ItemFrame` block entity. The frame keeps its item
/// unless the item has no Java 1.12 counterpart.
pub fn item_frame(be: &Tag, data: u8) -> Tag {
    // PE frame data 0..3 faces east/west/south/north.
    let facing = [3, 1, 0, 2][(data & 3) as usize];
    let tile = ["x", "y", "z"].map(|k| be.get(k).and_then(Tag::as_i64).unwrap_or(0) as i32);
    let mut out = hanging("minecraft:item_frame", tile, facing);
    if let Some(item) = be.get("Item").and_then(|i| item_stack(i, "__none", |s| s)) {
        out.push(("Item", item));
        out.push(("ItemRotation", Tag::Byte(be.get("ItemRotation").and_then(Tag::as_i64).unwrap_or(0) as i8 & 7)));
        out.push(("ItemDropChance", Tag::Float(be.get("ItemDropChance").and_then(Tag::as_f64).unwrap_or(1.0) as f32)));
    }
    compound(out)
}

/// Java 1.12 ids of the PE entity types (the low byte of PE's `id`) that we
/// convert. Projectiles, falling blocks, XP orbs and the like are dropped.
pub fn entity_name(kind: i64) -> Option<&'static str> {
    Some(match kind {
        10 => "minecraft:chicken",
        11 => "minecraft:cow",
        12 => "minecraft:pig",
        13 => "minecraft:sheep",
        14 => "minecraft:wolf",
        15 => "minecraft:villager",
        16 => "minecraft:mooshroom",
        17 => "minecraft:squid",
        18 => "minecraft:rabbit",
        19 => "minecraft:bat",
        20 => "minecraft:villager_golem",
        21 => "minecraft:snowman",
        22 => "minecraft:ocelot",
        23 => "minecraft:horse",
        24 => "minecraft:donkey",
        25 => "minecraft:mule",
        26 => "minecraft:skeleton_horse",
        27 => "minecraft:zombie_horse",
        32 => "minecraft:zombie",
        33 => "minecraft:creeper",
        34 => "minecraft:skeleton",
        35 => "minecraft:spider",
        36 => "minecraft:zombie_pigman",
        37 => "minecraft:slime",
        38 => "minecraft:enderman",
        39 => "minecraft:silverfish",
        40 => "minecraft:cave_spider",
        41 => "minecraft:ghast",
        42 => "minecraft:magma_cube",
        43 => "minecraft:blaze",
        44 => "minecraft:zombie_villager",
        45 => "minecraft:witch",
        64 => "minecraft:item",
        83 => "minecraft:painting",
        84 => "minecraft:minecart",
        90 => "minecraft:boat",
        _ => return None,
    })
}

/// The PE entity type: the low byte of `id` (newer saves set flag bits above it).
pub fn entity_kind(e: &Tag) -> Option<i64> {
    Some(e.get("id")?.as_i64()? & 0xff)
}

/// Converts one PE entity, or returns `None` for kinds we drop (and dropped
/// items whose item has no Java counterpart).
pub fn entity(e: &Tag) -> Option<Tag> {
    let kind = entity_kind(e)?;
    let name = entity_name(kind)?;
    let pos = floats(e, "Pos");
    if pos.len() != 3 {
        return None;
    }
    let int = |k: &str| e.get(k).and_then(Tag::as_i64);
    let on_ground = int("OnGround").unwrap_or(0) != 0;
    let mut motion = floats(e, "Motion");
    motion.resize(3, 0.0);
    let mut rotation = floats(e, "Rotation");
    rotation.resize(2, 0.0);

    let out = match kind {
        83 => {
            // PE's tile is the wall block; Java's is the block in front of it.
            let facing = int("Direction")? as i8 & 3;
            let (dx, dz) = facing_offset(facing);
            let tile = [int("TileX")? as i32 + dx, int("TileY")? as i32, int("TileZ")? as i32 + dz];
            let mut out = hanging(name, tile, facing);
            let motive = e.get("Motive").and_then(Tag::as_str).unwrap_or("Kebab");
            out.push(("Motive", Tag::String(motive.into())));
            out
        }
        64 => {
            let mut out = base(name, &pos, &motion, &rotation, on_ground);
            out.push(("Item", item_stack(e.get("Item")?, "__none", |s| s)?));
            out.push(("Age", Tag::Short(int("Age").unwrap_or(0) as i16)));
            out.push(("Health", Tag::Short(int("Health").unwrap_or(5) as i16)));
            out.push(("PickupDelay", Tag::Short(0)));
            out
        }
        84 | 90 => base(name, &pos, &motion, &rotation, on_ground),
        _ => {
            let mut out = base(name, &pos, &motion, &rotation, on_ground);
            // Newer saves keep health among the attributes.
            let health = e
                .get("Attributes")
                .and_then(Tag::as_list)
                .into_iter()
                .flatten()
                .find(|a| a.get("Name").and_then(Tag::as_str).is_some_and(|n| n.ends_with("health")))
                .and_then(|a| a.get("Current")?.as_f64())
                .or_else(|| e.get("Health")?.as_f64());
            if let Some(h) = health {
                out.push(("Health", Tag::Float(h as f32)));
            }
            out.push(("PersistenceRequired", Tag::Byte(1)));
            out.push(("Age", Tag::Int(if int("IsBaby").unwrap_or(0) != 0 { -24000 } else { 0 })));
            if let Some(n) = e.get("CustomName").and_then(Tag::as_str).filter(|n| !n.is_empty()) {
                out.push(("CustomName", Tag::String(n.into())));
            }
            match kind {
                13 => {
                    out.push(("Color", Tag::Byte(int("Color").unwrap_or(0) as i8)));
                    out.push(("Sheared", Tag::Byte(int("Sheared").unwrap_or(0) as i8)));
                }
                22 => out.push(("CatType", Tag::Int(int("Variant").unwrap_or(0) as i32))),
                23 => out.push(("Variant", Tag::Int(int("Variant").unwrap_or(0) as i32))),
                _ => {}
            }
            out
        }
    };
    Some(compound(out))
}

/// The `Player` compound for Java `level.dat`, from PE `~local_player`.
pub fn player(p: &Tag, game_type: i32) -> Tag {
    let floats = |k: &str| -> Vec<f64> {
        p.get(k).and_then(Tag::as_list).map(|l| l.iter().filter_map(Tag::as_f64).collect()).unwrap_or_default()
    };
    let mut pos = floats("Pos");
    if pos.len() == 3 {
        // PE stores the eye position; Java stores the feet.
        pos[1] -= 1.62;
    }
    let rot = floats("Rotation");

    // PE inventory slots 0..8 are links into the creative catalog (id 255);
    // real items follow. Keep real items, mapping into Java's 36 slots.
    let mut inventory = Vec::new();
    if let Some(list) = p.get("Inventory").and_then(Tag::as_list) {
        let mut next = 0i8;
        for item in list {
            if next >= 36 {
                break;
            }
            if let Some(t) = item_stack(item, "__none", |s| s) {
                if let Tag::Compound(mut e) = t {
                    e.push(("Slot".into(), Tag::Byte(next)));
                    inventory.push(Tag::Compound(e));
                }
                next += 1;
            }
        }
    }
    // Armor: PE head, chest, legs, feet -> Java slots 103..100.
    if let Some(list) = p.get("Armor").and_then(Tag::as_list) {
        for (i, item) in list.iter().enumerate().take(4) {
            if let Some(Tag::Compound(mut e)) = item_stack(item, "__none", |s| s) {
                e.push(("Slot".into(), Tag::Byte(103 - i as i8)));
                inventory.push(Tag::Compound(e));
            }
        }
    }

    let mut out = vec![
        ("playerGameType", Tag::Int(game_type)),
        ("Health", Tag::Float(20.0)),
        ("foodLevel", Tag::Int(20)),
        ("Dimension", Tag::Int(0)),
        ("Inventory", Tag::List(10, inventory)),
        (
            "abilities",
            compound([
                ("mayfly", Tag::Byte((game_type == 1) as i8)),
                ("instabuild", Tag::Byte((game_type == 1) as i8)),
                ("invulnerable", Tag::Byte((game_type == 1) as i8)),
                ("mayBuild", Tag::Byte(1)),
                ("flying", Tag::Byte(0)),
                ("flySpeed", Tag::Float(0.05)),
                ("walkSpeed", Tag::Float(0.1)),
            ]),
        ),
    ];
    if pos.len() == 3 {
        out.push(("Pos", Tag::List(6, pos.into_iter().map(Tag::Double).collect())));
    }
    if rot.len() == 2 {
        out.push(("Rotation", Tag::List(5, rot.into_iter().map(|v| Tag::Float(v as f32)).collect())));
    }
    compound(out)
}

/// Java 1.12 item names for the classic numeric ids (blocks 0..=255 share
/// numbering with items). Ids absent here are dropped.
pub fn item_name(id: i16) -> Option<&'static str> {
    Some(match id {
        1 => "minecraft:stone",
        2 => "minecraft:grass",
        3 => "minecraft:dirt",
        4 => "minecraft:cobblestone",
        5 => "minecraft:planks",
        6 => "minecraft:sapling",
        12 => "minecraft:sand",
        13 => "minecraft:gravel",
        17 => "minecraft:log",
        18 => "minecraft:leaves",
        20 => "minecraft:glass",
        24 => "minecraft:sandstone",
        35 => "minecraft:wool",
        37 => "minecraft:yellow_flower",
        38 => "minecraft:red_flower",
        39 => "minecraft:brown_mushroom",
        40 => "minecraft:red_mushroom",
        41 => "minecraft:gold_block",
        42 => "minecraft:iron_block",
        45 => "minecraft:brick_block",
        46 => "minecraft:tnt",
        47 => "minecraft:bookshelf",
        48 => "minecraft:mossy_cobblestone",
        49 => "minecraft:obsidian",
        50 => "minecraft:torch",
        54 => "minecraft:chest",
        57 => "minecraft:diamond_block",
        58 => "minecraft:crafting_table",
        61 => "minecraft:furnace",
        65 => "minecraft:ladder",
        66 => "minecraft:rail",
        81 => "minecraft:cactus",
        82 => "minecraft:clay",
        85 => "minecraft:fence",
        86 => "minecraft:pumpkin",
        89 => "minecraft:glowstone",
        98 => "minecraft:stonebrick",
        102 => "minecraft:glass_pane",
        103 => "minecraft:melon_block",
        152 => "minecraft:redstone_block",
        155 => "minecraft:quartz_block",
        171 => "minecraft:carpet",
        256 => "minecraft:iron_shovel",
        257 => "minecraft:iron_pickaxe",
        258 => "minecraft:iron_axe",
        259 => "minecraft:flint_and_steel",
        260 => "minecraft:apple",
        261 => "minecraft:bow",
        262 => "minecraft:arrow",
        263 => "minecraft:coal",
        264 => "minecraft:diamond",
        265 => "minecraft:iron_ingot",
        266 => "minecraft:gold_ingot",
        267 => "minecraft:iron_sword",
        268 => "minecraft:wooden_sword",
        272 => "minecraft:stone_sword",
        276 => "minecraft:diamond_sword",
        277 => "minecraft:diamond_shovel",
        278 => "minecraft:diamond_pickaxe",
        279 => "minecraft:diamond_axe",
        280 => "minecraft:stick",
        281 => "minecraft:bowl",
        283 => "minecraft:golden_sword",
        287 => "minecraft:string",
        288 => "minecraft:feather",
        289 => "minecraft:gunpowder",
        295 => "minecraft:wheat_seeds",
        296 => "minecraft:wheat",
        297 => "minecraft:bread",
        298 => "minecraft:leather_helmet",
        299 => "minecraft:leather_chestplate",
        300 => "minecraft:leather_leggings",
        301 => "minecraft:leather_boots",
        302 => "minecraft:chainmail_helmet",
        303 => "minecraft:chainmail_chestplate",
        304 => "minecraft:chainmail_leggings",
        305 => "minecraft:chainmail_boots",
        306 => "minecraft:iron_helmet",
        307 => "minecraft:iron_chestplate",
        308 => "minecraft:iron_leggings",
        309 => "minecraft:iron_boots",
        310 => "minecraft:diamond_helmet",
        311 => "minecraft:diamond_chestplate",
        312 => "minecraft:diamond_leggings",
        313 => "minecraft:diamond_boots",
        314 => "minecraft:golden_helmet",
        315 => "minecraft:golden_chestplate",
        316 => "minecraft:golden_leggings",
        317 => "minecraft:golden_boots",
        318 => "minecraft:flint",
        319 => "minecraft:porkchop",
        320 => "minecraft:cooked_porkchop",
        321 => "minecraft:painting",
        322 => "minecraft:golden_apple",
        323 => "minecraft:sign",
        324 => "minecraft:wooden_door",
        325 => "minecraft:bucket",
        326 => "minecraft:water_bucket",
        327 => "minecraft:lava_bucket",
        328 => "minecraft:minecart",
        329 => "minecraft:saddle",
        330 => "minecraft:iron_door",
        331 => "minecraft:redstone",
        332 => "minecraft:snowball",
        333 => "minecraft:boat",
        334 => "minecraft:leather",
        336 => "minecraft:brick",
        337 => "minecraft:clay_ball",
        338 => "minecraft:reeds",
        339 => "minecraft:paper",
        340 => "minecraft:book",
        341 => "minecraft:slime_ball",
        344 => "minecraft:egg",
        345 => "minecraft:compass",
        346 => "minecraft:fishing_rod",
        347 => "minecraft:clock",
        348 => "minecraft:glowstone_dust",
        349 => "minecraft:fish",
        350 => "minecraft:cooked_fish",
        351 => "minecraft:dye",
        352 => "minecraft:bone",
        353 => "minecraft:sugar",
        354 => "minecraft:cake",
        355 => "minecraft:bed",
        357 => "minecraft:cookie",
        359 => "minecraft:shears",
        360 => "minecraft:melon",
        363 => "minecraft:beef",
        364 => "minecraft:cooked_beef",
        365 => "minecraft:chicken",
        366 => "minecraft:cooked_chicken",
        367 => "minecraft:rotten_flesh",
        368 => "minecraft:ender_pearl",
        369 => "minecraft:blaze_rod",
        371 => "minecraft:gold_nugget",
        383 => "minecraft:spawn_egg",
        388 => "minecraft:emerald",
        390 => "minecraft:flower_pot",
        391 => "minecraft:carrot",
        392 => "minecraft:potato",
        393 => "minecraft:baked_potato",
        400 => "minecraft:pumpkin_pie",
        406 => "minecraft:quartz",
        // 416, PE's leather horse armour, only arrived in Java 1.14.
        417 => "minecraft:iron_horse_armor",
        418 => "minecraft:golden_horse_armor",
        419 => "minecraft:diamond_horse_armor",
        420 => "minecraft:lead",
        421 => "minecraft:name_tag",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sign_text_becomes_json() {
        let be = compound([
            ("id", Tag::String("Sign".into())),
            ("Text1", Tag::String("i \"love\"".into())),
            ("x", Tag::Int(1)),
            ("y", Tag::Int(2)),
            ("z", Tag::Int(3)),
        ]);
        let out = block_entity(&be).unwrap();
        assert_eq!(out.get("Text1").unwrap().as_str(), Some(r#"{"text":"i \"love\""}"#));
        assert_eq!(out.get("Text2").unwrap().as_str(), Some(r#"{"text":""}"#));
        assert_eq!(out.get("id").unwrap().as_str(), Some("minecraft:sign"));
    }
}

//! Block entity and player translation (PE 0.16 NBT -> Java 1.12 NBT).
//!
//! Java 1.12 stores items with string ids, and PE item numbers match the
//! classic Java numbering, so items are looked up by number in [`item_name`].
//! Unknown items are dropped rather than guessed.

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
        out.push(("tag", tag.clone()));
    }
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

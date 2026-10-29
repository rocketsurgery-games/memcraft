//! How each Pocket Edition block looks: a flat colour and a coarse shape.
//!
//! No textures yet. Mojang's can't be redistributed, so this first cut uses
//! a hand-picked colour per block, with per-face shading done by the mesher.

/// Coarse geometry used by the mesher.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Shape {
    /// Nothing to draw.
    Air,
    /// A full opaque cube; hides the faces of its neighbours.
    Cube,
    /// A full cube drawn translucent (water, glass, ice). Faces between two
    /// blocks of the same kind are skipped.
    Translucent,
    /// An opaque box from the floor up to the given height (slabs, snow, carpet, rails).
    Partial(f32),
    /// Two crossed quads (flowers, grass, saplings, torches).
    Cross,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Look {
    pub shape: Shape,
    /// Linear-ish sRGB colour, 0..1.
    pub color: [f32; 3],
    /// Alpha for translucent blocks.
    pub alpha: f32,
}

const fn rgb(r: u8, g: u8, b: u8) -> [f32; 3] {
    [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0]
}

const fn cube(c: [f32; 3]) -> Look {
    Look { shape: Shape::Cube, color: c, alpha: 1.0 }
}

const fn partial(h: f32, c: [f32; 3]) -> Look {
    Look { shape: Shape::Partial(h), color: c, alpha: 1.0 }
}

const fn cross(c: [f32; 3]) -> Look {
    Look { shape: Shape::Cross, color: c, alpha: 1.0 }
}

const fn clear(c: [f32; 3], alpha: f32) -> Look {
    Look { shape: Shape::Translucent, color: c, alpha }
}

const AIR: Look = Look { shape: Shape::Air, color: [0.0; 3], alpha: 0.0 };

/// The 16 wool / stained clay / carpet colours, by data value.
const DYE: [[f32; 3]; 16] = [
    rgb(233, 236, 236), // white
    rgb(240, 118, 19),  // orange
    rgb(189, 68, 179),  // magenta
    rgb(58, 175, 217),  // light blue
    rgb(248, 198, 39),  // yellow
    rgb(112, 185, 25),  // lime
    rgb(237, 141, 172), // pink
    rgb(62, 68, 71),    // gray
    rgb(142, 142, 134), // light gray
    rgb(21, 137, 145),  // cyan
    rgb(121, 42, 172),  // purple
    rgb(53, 57, 157),   // blue
    rgb(114, 71, 40),   // brown
    rgb(84, 109, 27),   // green
    rgb(161, 39, 34),   // red
    rgb(20, 21, 25),    // black
];

/// Planks by wood type (oak, spruce, birch, jungle, acacia, dark oak).
const PLANKS: [[f32; 3]; 6] =
    [rgb(162, 130, 78), rgb(115, 85, 49), rgb(196, 179, 123), rgb(160, 115, 81), rgb(168, 90, 50), rgb(67, 43, 20)];

fn tint(c: [f32; 3], k: f32) -> [f32; 3] {
    c.map(|v| v * k)
}

/// Appearance of a Pocket Edition block.
pub fn look(id: u8, data: u8) -> Look {
    let d = data as usize;
    match id {
        0 => AIR,
        1 => cube(match data {
            1 | 2 => rgb(153, 114, 99), // granite
            3 | 4 => rgb(188, 188, 188), // diorite
            5 | 6 => rgb(136, 136, 137), // andesite
            _ => rgb(125, 125, 125),
        }),
        2 => cube(rgb(106, 160, 66)),
        3 => cube(rgb(134, 96, 67)),
        4 => cube(rgb(122, 122, 122)),
        5 | 125 | 157 => cube(PLANKS[d % 6]),
        6 | 31 | 32 | 37 | 38 | 39 | 40 | 59 | 83 | 106 | 141 | 142 | 175 | 244 => cross(match id {
            37 => rgb(240, 220, 40),
            38 => match data {
                0 => rgb(200, 30, 30),
                1 => rgb(60, 160, 230),
                2 => rgb(190, 130, 230),
                3 => rgb(230, 230, 210),
                4..=7 => [rgb(210, 40, 30), rgb(230, 130, 40), rgb(240, 240, 240), rgb(240, 160, 200)][d - 4],
                _ => rgb(240, 240, 220),
            },
            39 => rgb(150, 110, 80),
            40 => rgb(200, 40, 40),
            83 => rgb(150, 200, 100),
            175 => match data & 7 {
                0 => rgb(240, 200, 40),
                1 => rgb(190, 130, 220),
                4 => rgb(220, 40, 40),
                5 => rgb(240, 170, 210),
                _ => rgb(100, 160, 60),
            },
            _ => rgb(90, 150, 50),
        }),
        7 => cube(rgb(60, 60, 60)),
        8 | 9 => clear(rgb(48, 86, 210), 0.65),
        10 | 11 => cube(rgb(230, 100, 20)),
        12 => cube(if data == 1 { rgb(170, 90, 40) } else { rgb(219, 207, 163) }),
        13 => cube(rgb(136, 126, 126)),
        14 => cube(rgb(150, 140, 110)),
        15 => cube(rgb(140, 130, 120)),
        16 => cube(rgb(105, 105, 105)),
        17 | 162 => cube(match (id, data & 3) {
            (17, 1) => rgb(60, 40, 20),
            (17, 2) => rgb(210, 205, 195),
            (17, 3) => rgb(90, 70, 30),
            (162, 1) => rgb(55, 40, 25),
            (162, _) => rgb(100, 95, 85),
            _ => rgb(105, 82, 50),
        }),
        18 | 161 => cube(match data & 3 {
            1 => rgb(50, 90, 50),
            2 => rgb(90, 140, 60),
            _ => rgb(60, 120, 40),
        }),
        19 => cube(rgb(200, 200, 70)),
        20 | 102 => clear(rgb(200, 230, 240), 0.35),
        21 => cube(rgb(100, 110, 140)),
        22 => cube(rgb(30, 70, 160)),
        23 => cube(rgb(110, 110, 110)),                 // dispenser
        29 | 33 => cube(rgb(150, 125, 90)),             // pistons
        34 => partial(1.0, rgb(150, 125, 90)),          // piston head
        55 => partial(0.0625, rgb(170, 20, 10)),        // redstone wire
        93 | 94 => partial(0.125, rgb(160, 150, 150)),  // repeater
        24 | 128 => cube(rgb(216, 203, 155)),
        26 => partial(0.5625, rgb(160, 30, 30)),
        27 | 28 | 66 | 126 => partial(0.0625, match id {
            27 => rgb(200, 170, 50),
            28 => rgb(140, 120, 110),
            _ => rgb(130, 110, 90),
        }),
        35 => cube(DYE[d]),
        41 => cube(rgb(249, 212, 61)),
        42 => cube(rgb(220, 220, 220)),
        43 | 181 => cube(rgb(160, 160, 160)),
        44 | 182 => partial(0.5, match data & 7 {
            1 => rgb(216, 203, 155),
            2 => PLANKS[0],
            3 => rgb(122, 122, 122),
            4 => rgb(150, 70, 60),
            5 => rgb(122, 122, 122),
            6 => rgb(235, 230, 225),
            _ => rgb(160, 160, 160),
        }),
        158 => partial(0.5, PLANKS[(d & 7) % 6]),
        45 => cube(rgb(150, 70, 60)),
        46 => cube(rgb(200, 60, 40)),
        47 => cube(rgb(120, 90, 60)),
        48 => cube(rgb(100, 120, 90)),
        49 => cube(rgb(20, 18, 30)),
        50 => cross(rgb(255, 210, 90)),
        51 => cross(rgb(255, 150, 30)),
        52 => cube(rgb(30, 50, 70)),
        53 | 134 | 135 | 136 | 163 | 164 => partial(0.5, match id {
            134 => PLANKS[1],
            135 => PLANKS[2],
            136 => PLANKS[3],
            163 => PLANKS[4],
            164 => PLANKS[5],
            _ => PLANKS[0],
        }),
        54 | 146 => cube(rgb(160, 115, 45)),
        56 => cube(rgb(130, 170, 170)),
        57 => cube(rgb(100, 220, 215)),
        58 => cube(rgb(140, 100, 60)),
        60 => cube(rgb(110, 75, 45)),
        61 | 62 => cube(rgb(110, 110, 110)),
        63 | 68 => partial(0.5, PLANKS[0]),
        64 | 71 | 193..=197 => cube(if id == 71 { rgb(200, 200, 200) } else { PLANKS[0] }),
        65 => partial(0.125, PLANKS[0]),
        67 => partial(0.5, rgb(122, 122, 122)),
        69 | 70 | 72 | 77 | 143 | 147 | 148 => partial(0.0625, rgb(130, 130, 130)),
        73 | 74 => cube(rgb(140, 105, 105)),
        75 | 76 => cross(rgb(200, 40, 30)),
        78 => partial(0.125 * (d as f32 % 8.0 + 1.0), rgb(245, 250, 250)),
        79 => clear(rgb(160, 190, 250), 0.75),
        80 => cube(rgb(245, 250, 250)),
        81 => cube(rgb(80, 130, 40)),
        82 => cube(rgb(160, 165, 178)),
        84 => cube(rgb(120, 80, 60)),
        85 | 107 | 113 | 139 | 183..=192 => partial(1.0, match id {
            113 => rgb(50, 25, 30),
            139 => rgb(122, 122, 122),
            85 => PLANKS[d % 6],
            _ => PLANKS[0],
        }),
        86 | 91 => cube(rgb(200, 120, 25)),
        87 => cube(rgb(110, 50, 50)),
        88 => cube(rgb(85, 65, 50)),
        89 => cube(rgb(250, 220, 140)),
        92 => partial(0.5, rgb(240, 230, 230)),
        96 | 167 => partial(0.1875, if id == 167 { rgb(200, 200, 200) } else { PLANKS[0] }),
        97 => cube(rgb(125, 125, 125)),
        98 => cube(match data {
            1 => rgb(110, 120, 95),
            2 => rgb(115, 115, 115),
            _ => rgb(122, 122, 122),
        }),
        99 | 100 => cube(if id == 99 { rgb(140, 105, 80) } else { rgb(180, 30, 30) }),
        101 => partial(1.0, rgb(110, 110, 110)),
        103 => cube(rgb(130, 170, 40)),
        104 | 105 => cross(rgb(110, 160, 60)),
        108 => partial(0.5, rgb(150, 70, 60)),
        109 => partial(0.5, rgb(122, 122, 122)),
        110 => cube(rgb(110, 95, 105)),
        111 => partial(0.0625, rgb(40, 110, 30)),
        112 | 114 => partial(if id == 114 { 0.5 } else { 1.0 }, rgb(50, 25, 30)),
        115 => cross(rgb(140, 30, 30)),
        116 => partial(0.75, rgb(100, 40, 40)),
        117 => cross(rgb(130, 120, 100)),
        118 => partial(1.0, rgb(60, 60, 60)),
        120 => partial(0.8125, rgb(70, 110, 90)),
        121 => cube(rgb(220, 220, 160)),
        123 | 124 => cube(if id == 124 { rgb(240, 200, 120) } else { rgb(120, 70, 40) }),
        127 => cross(rgb(180, 100, 40)),
        129 => cube(rgb(115, 135, 120)),
        133 => cube(rgb(80, 210, 110)),
        140 => partial(0.375, rgb(120, 60, 40)),
        145 => partial(1.0, rgb(70, 70, 70)),
        152 => cube(rgb(170, 25, 10)),
        153 => cube(rgb(120, 70, 70)),
        155 => cube(rgb(235, 230, 225)),
        156 => partial(0.5, rgb(235, 230, 225)),
        159 => cube(tint(DYE[d], 0.75)),
        160 => clear(DYE[d], 0.45),
        165 => clear(rgb(110, 190, 90), 0.7),
        168 => cube(rgb(100, 160, 145)),
        169 => cube(rgb(200, 230, 220)),
        170 => cube(rgb(170, 140, 20)),
        171 => partial(0.0625, DYE[d]),
        172 => cube(rgb(150, 92, 66)),
        173 => cube(rgb(20, 20, 20)),
        174 => clear(rgb(150, 180, 230), 0.8),
        179 | 180 => cube(rgb(170, 90, 40)),
        198 => partial(0.9375, rgb(150, 125, 70)),
        199 => AIR, // item frame: drawn as an entity in later versions
        201..=205 => cube(rgb(165, 120, 165)),
        241 => clear(DYE[d], 0.45),
        243 => cube(rgb(90, 65, 30)),
        245 => cube(rgb(115, 115, 115)),
        246 => cube(rgb(80, 20, 20)),
        247 => cube(rgb(210, 210, 210)),
        // Unknown blocks are still drawn, in a loud colour, so gaps are visible.
        _ => cube(rgb(255, 0, 255)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_block_in_the_sample_world_has_a_real_look() {
        let w = mcpe_world::World::open_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/../../worlds/My World")).unwrap();
        let mut unknown = std::collections::BTreeSet::new();
        for c in w.columns.values() {
            for (&id, &data) in c.ids.iter().zip(&c.data) {
                if look(id, data).color == rgb(255, 0, 255) {
                    unknown.insert(id);
                }
            }
        }
        assert!(unknown.is_empty(), "blocks without a look: {unknown:?}");
    }
}

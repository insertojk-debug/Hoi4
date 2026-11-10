use macroquad::prelude::*;

// the map shows longitude -170..180 and latitude 84..-58 (equal scale in both directions)
pub const MW: usize = 3204;
pub const MH: usize = 1296;
pub const COLS: usize = 356; // province grid (about 51,000 provinces)
pub const ROWS: usize = 144;
pub const CELL: f32 = 9.0; // width of one province in map tiles

pub const SPEEDS: [f32; 4] = [1.0, 2.0, 4.0, 8.0];

pub const NEUTRAL: usize = usize::MAX;
pub const DEAD: usize = usize::MAX - 1; // land swallowed by the storm

// ---- economy: stars are the money ----
pub const START_STARS: f32 = 25.0;
pub const INCOME: f32 = 2.0; // stars one civilian factory pays each cycle
pub const CYCLE: u32 = 10; // ticks per income cycle
pub const BUILD_NAMES: [&str; 2] = ["Civilian Factory", "Military Factory"];
pub const BUILD_COST: [f32; 2] = [10.0, 15.0]; // stars
pub const BUILD_TIME: [u32; 2] = [30, 60]; // ticks to finish
pub const MAX_STACK: usize = 3; // soldiers of one player per tile

// ---- soldiers: 0 = Infantry, 1 = Artillery, 2 = Tank ----
pub const KIND_NAMES: [&str; 3] = ["Infantry", "Artillery", "Tank"];
pub const KIND_LETTER: [&str; 3] = ["I", "A", "T"];
pub const STAR_COST: [f32; 3] = [5.0, 8.0, 12.0];
pub const ATK: [f32; 3] = [1.0, 1.6, 1.4];
pub const DEF: [f32; 3] = [1.0, 0.6, 1.3];
pub const SPD: [f32; 3] = [1.0, 0.8, 1.6];

// sea crossings between provinces that are cut apart on the map (lon,lat) <-> (lon,lat)
pub const FERRIES: &[((f32, f32), (f32, f32))] = &[
    ((-4.5, 37.3), (-5.3, 34.3)), // Gibraltar
    ((43.5, 13.6), (43.1, 11.8)), // Bab-el-Mandeb
];

// thin isthmuses that are too narrow for the province grid: link the two provinces by land (lon,lat)
pub const LINKS: &[((f32, f32), (f32, f32))] = &[
    ((-79.3, 9.4), (-78.2, 8.8)), // Panama
];

// narrow straits inside one landmass (lon,lat segments): provinces on opposite sides are not land-linked
pub const STRAITS: &[((f32, f32), (f32, f32))] = &[
    ((-9.0, 35.95), (-2.0, 35.95)), // Gibraltar
    ((42.0, 12.75), (44.5, 12.45)), // Bab-el-Mandeb
];

fn orient(a: Vec2, b: Vec2, c: Vec2) -> f32 {
    (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)
}

pub fn seg_cross(p1: Vec2, p2: Vec2, q1: Vec2, q2: Vec2) -> bool {
    let (d1, d2) = (orient(q1, q2, p1), orient(q1, q2, p2));
    let (d3, d4) = (orient(p1, p2, q1), orient(p1, p2, q2));
    (d1 > 0.0) != (d2 > 0.0) && (d3 > 0.0) != (d4 > 0.0)
}

pub fn proj(lon: f32, lat: f32) -> Vec2 {
    vec2((lon + 170.0) / 350.0 * MW as f32, (84.0 - lat) / 142.0 * MH as f32)
}


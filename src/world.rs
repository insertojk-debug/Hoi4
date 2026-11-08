use macroquad::prelude::*;

pub const MW: usize = 2400;
pub const MH: usize = 1200;
pub const COLS: usize = 280; // province grid (about 39,000 provinces, 12,800 of them land)
pub const ROWS: usize = 140;
pub const CELL: f32 = 2400.0 / 280.0; // width of one province in map tiles

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
];

pub fn proj(lon: f32, lat: f32) -> Vec2 {
    vec2((lon + 170.0) / 350.0 * MW as f32, (75.0 - lat) / 133.0 * MH as f32)
}

pub fn inside(poly: &[Vec2], p: Vec2) -> bool {
    let mut c = false;
    let mut j = poly.len() - 1;
    for i in 0..poly.len() {
        let (a, b) = (poly[i], poly[j]);
        if (a.y > p.y) != (b.y > p.y) && p.x < (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x {
            c = !c;
        }
        j = i;
    }
    c
}

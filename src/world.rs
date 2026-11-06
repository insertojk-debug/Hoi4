use macroquad::prelude::*;

pub const MW: usize = 2400;
pub const MH: usize = 1200;
pub const COLS: usize = 160;
pub const ROWS: usize = 80;
pub const NC: usize = 24;
pub const CELL: f32 = 15.0;

pub const NAMES: [&str; NC] = [
    "United States", "Canada", "Brazil", "Britain", "France", "South Africa", "Australia", "India", "Philippines",
    "Indonesia", "Nigeria", "Poland", "Germany", "Italy", "Russia", "China", "Japan", "Persia", "Egypt", "Mexico",
    "Argentina", "Spain", "Turkey", "Ethiopia",
];

// Countries 0-11 = Allies (player is 0), 12-23 = Axis
pub fn at_war(a: usize, b: usize) -> bool {
    a < NC && b < NC && a / 12 != b / 12
}

pub fn country_color(c: usize) -> Color {
    const P: [(f32, f32, f32); NC] = [
        (0.25, 0.45, 0.95), (0.55, 0.75, 0.95), (0.35, 0.75, 0.40), (0.20, 0.70, 0.75), (0.35, 0.30, 0.75),
        (0.55, 0.40, 0.85), (0.45, 0.85, 0.60), (0.70, 0.60, 0.90), (0.60, 0.85, 0.30), (0.25, 0.55, 0.45),
        (0.20, 0.55, 0.30), (0.75, 0.85, 0.90),
        (0.85, 0.25, 0.25), (0.55, 0.20, 0.45), (0.70, 0.15, 0.20), (0.90, 0.60, 0.20), (0.95, 0.95, 0.95),
        (0.85, 0.45, 0.65), (0.65, 0.35, 0.25), (0.90, 0.80, 0.25), (0.75, 0.55, 0.35), (0.95, 0.50, 0.35),
        (0.80, 0.35, 0.15), (0.60, 0.50, 0.10),
    ];
    let (r, g, b) = P[c.min(NC - 1)];
    Color::new(r, g, b, 1.0)
}

// capital location (longitude, latitude) per country
pub const ANCHORS: [(f32, f32); NC] = [
    (-98.0, 38.0), (-100.0, 58.0), (-52.0, -10.0), (-1.5, 52.5), (2.5, 47.0),
    (24.0, -26.0), (134.0, -25.0), (78.0, 22.0), (122.0, 15.5), (110.0, -7.0),
    (7.0, 9.0), (19.5, 52.0),
    (11.0, 51.0), (12.5, 42.5), (55.0, 58.0), (110.0, 35.0), (138.0, 36.0),
    (52.0, 33.0), (30.0, 27.0), (-102.0, 22.0), (-65.0, -38.0), (-4.0, 40.0),
    (33.0, 39.5), (40.0, 9.0),
];

// extra claim points (country, longitude, latitude) so big countries cover their far regions
pub const EXTRAS: [(usize, f32, f32); 8] = [
    (14, 100.0, 62.0), (14, 140.0, 64.0), (14, 75.0, 66.0), (1, -130.0, 62.0), (1, -42.0, 72.0),
    (0, -152.0, 64.0), (15, 95.0, 36.0), (15, 105.0, 26.0),
];

// territory weight per country (bigger = claims more land)
pub const W: [f32; NC] = [
    1.15, 0.85, 1.0, 1.3, 1.2, 1.0, 1.0, 1.05, 1.2, 1.0, 1.1, 1.2,
    1.2, 1.3, 0.9, 1.0, 1.0, 1.0, 0.9, 1.25, 1.1, 1.3, 1.1, 1.0,
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


// sea crossings between provinces that are cut apart on the map (lon,lat) <-> (lon,lat)
pub const FERRIES: &[((f32, f32), (f32, f32))] = &[
    ((-4.5, 37.3), (-5.3, 34.3)), // Gibraltar
];

// unit types: 0 = Infantry, 1 = Artillery, 2 = Tank
pub const KIND_NAMES: [&str; 3] = ["Infantry", "Artillery", "Tank"];
pub const KIND_LETTER: [&str; 3] = ["I", "A", "T"];
pub const COST: [(f32, f32); 3] = [(10.0, 20.0), (18.0, 15.0), (30.0, 25.0)]; // (industry, manpower)
pub const ATK: [f32; 3] = [1.0, 1.6, 1.4];
pub const DEF: [f32; 3] = [1.0, 0.6, 1.3];
pub const SPD: [f32; 3] = [1.0, 0.8, 1.6];

// Territory Royale
pub const NEUTRAL: usize = usize::MAX;
pub const ATTACK: [f32; 4] = [0.0, 0.25, 0.5, 1.0]; // share of troops you are willing to throw at an enemy

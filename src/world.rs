use macroquad::prelude::*;

pub const MW: usize = 1800;
pub const MH: usize = 900;
pub const COLS: usize = 120;
pub const ROWS: usize = 60;
pub const NC: usize = 20;
pub const CELL: f32 = 15.0;

pub const NAMES: [&str; NC] = [
    "United States", "Canada", "Brazil", "Britain", "France", "South Africa", "Australia", "India", "Philippines",
    "Indonesia", "Germany", "Italy", "Russia", "China", "Japan", "Persia", "Egypt", "Mexico", "Argentina", "Spain",
];

// Countries 0-9 = Allies (player is 0), 10-19 = Axis
pub fn at_war(a: usize, b: usize) -> bool {
    a < NC && b < NC && a / 10 != b / 10
}

pub fn country_color(c: usize) -> Color {
    const P: [(f32, f32, f32); NC] = [
        (0.25, 0.45, 0.95), (0.55, 0.75, 0.95), (0.35, 0.75, 0.40), (0.20, 0.70, 0.75), (0.35, 0.30, 0.75),
        (0.55, 0.40, 0.85), (0.45, 0.85, 0.60), (0.70, 0.60, 0.90), (0.60, 0.85, 0.30), (0.25, 0.55, 0.45),
        (0.85, 0.25, 0.25), (0.55, 0.20, 0.45), (0.70, 0.15, 0.20), (0.90, 0.60, 0.20), (0.95, 0.95, 0.95),
        (0.85, 0.45, 0.65), (0.65, 0.35, 0.25), (0.90, 0.80, 0.25), (0.75, 0.55, 0.35), (0.95, 0.50, 0.35),
    ];
    let (r, g, b) = P[c.min(NC - 1)];
    Color::new(r, g, b, 1.0)
}

// capital location (longitude, latitude) per country
pub const ANCHORS: [(f32, f32); NC] = [
    (-98.0, 38.0), (-100.0, 58.0), (-52.0, -10.0), (-1.5, 52.5), (2.5, 47.0),
    (24.0, -26.0), (134.0, -25.0), (78.0, 22.0), (122.0, 15.5), (110.0, -7.0),
    (11.0, 51.0), (12.5, 42.5), (55.0, 58.0), (110.0, 35.0), (138.0, 36.0),
    (52.0, 33.0), (20.0, 23.0), (-102.0, 22.0), (-65.0, -38.0), (-4.0, 40.0),
];

// extra claim points (country, longitude, latitude) so big countries cover their far regions
pub const EXTRAS: [(usize, f32, f32); 7] = [
    (12, 100.0, 62.0), (12, 140.0, 64.0), (12, 75.0, 66.0), (1, -130.0, 62.0), (1, -42.0, 72.0),
    (13, 95.0, 36.0), (13, 105.0, 26.0),
];

// territory weight per country (bigger = claims more land)
pub const W: [f32; NC] = [
    1.15, 0.85, 1.0, 1.3, 1.2, 1.0, 1.0, 1.05, 1.2, 1.0,
    1.2, 1.3, 0.9, 1.0, 1.0, 1.0, 0.9, 1.25, 1.1, 1.3,
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

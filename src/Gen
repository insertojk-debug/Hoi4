use macroquad::prelude::*;

use crate::coast::{LAND, SEAS};
use crate::game::*;
use crate::world::*;

fn build_adj(provs: &mut Vec<Prov>, tile: &[u16]) {
    for p in provs.iter_mut() {
        p.adj.clear();
    }
    for y in 0..MH {
        for x in 0..MW {
            let a = tile[y * MW + x] as usize;
            for (nx, ny) in [(x + 1, y), (x, y + 1)] {
                if nx < MW && ny < MH {
                    let b = tile[ny * MW + nx] as usize;
                    if a != b && !provs[a].sea && !provs[b].sea && !provs[a].adj.contains(&b) {
                        provs[a].adj.push(b);
                        provs[b].adj.push(a);
                    }
                }
            }
        }
    }
}

// continent id per province (usize::MAX for sea) and number of continents
pub fn components(provs: &[Prov]) -> (Vec<usize>, usize) {
    let mut comp = vec![usize::MAX; provs.len()];
    let mut n = 0;
    for s in 0..provs.len() {
        if provs[s].sea || comp[s] != usize::MAX {
            continue;
        }
        let mut st = vec![s];
        comp[s] = n;
        while let Some(u) = st.pop() {
            for &v in &provs[u].adj {
                if comp[v] == usize::MAX {
                    comp[v] = n;
                    st.push(v);
                }
            }
        }
        n += 1;
    }
    (comp, n)
}

fn nearest_land(provs: &[Prov], t: Vec2, skip: &[usize]) -> usize {
    let mut best = usize::MAX;
    let mut bd = f32::MAX;
    for (i, p) in provs.iter().enumerate() {
        if p.sea || skip.contains(&i) {
            continue;
        }
        let d = p.pos.distance_squared(t);
        if d < bd {
            bd = d;
            best = i;
        }
    }
    best
}

impl Game {
    pub fn new() -> Game {
        rand::srand(11);
        let mut provs: Vec<Prov> = Vec::new();
        let cw = MW as f32 / COLS as f32;
        let ch = MH as f32 / ROWS as f32;
        for r in 0..ROWS {
            for c in 0..COLS {
                let x = (c as f32 + 0.5 + rand::gen_range(-0.3f32, 0.3f32)) * cw;
                let y = (r as f32 + 0.5 + rand::gen_range(-0.3f32, 0.3f32)) * ch;
                provs.push(Prov { pos: vec2(x, y), owner: NC, sea: true, bb: (MW, MH, 0, 0), adj: vec![], sea_adj: vec![] });
            }
        }
        // land = province whose centre (and mostly its surroundings) lies inside a continent outline
        let to_poly = |ps: &[(f32, f32)]| -> Vec<Vec2> { ps.iter().map(|&(a, b)| proj(a, b)).collect() };
        let land_polys: Vec<Vec<Vec2>> = LAND.iter().map(|p| to_poly(p)).collect();
        let sea_polys: Vec<Vec<Vec2>> = SEAS.iter().map(|p| to_poly(p)).collect();
        let offs = [vec2(0.0, 0.0), vec2(-4.5, -4.5), vec2(4.5, -4.5), vec2(-4.5, 4.5), vec2(4.5, 4.5)];
        for p in provs.iter_mut() {
            let mut cnt = 0;
            for o in offs {
                let q = p.pos + o;
                if land_polys.iter().any(|pl| inside(pl, q)) && !sea_polys.iter().any(|pl| inside(pl, q)) {
                    cnt += 1;
                }
            }
            if cnt >= 2 {
                p.sea = false;
            }
        }
        // pixel -> province lookup (seeds are a jittered grid, so only nearby cells matter)
        let mut tile = vec![0u16; MW * MH];
        for y in 0..MH {
            for x in 0..MW {
                let p = vec2(x as f32 + 0.5, y as f32 + 0.5);
                let cx = (x as f32 / cw) as i32;
                let cy = (y as f32 / ch) as i32;
                let mut best = 0;
                let mut bd = f32::MAX;
                for dy in -2..=2i32 {
                    for dx in -2..=2i32 {
                        let (c, r) = (cx + dx, cy + dy);
                        if c < 0 || r < 0 || c >= COLS as i32 || r >= ROWS as i32 {
                            continue;
                        }
                        let i = r as usize * COLS + c as usize;
                        let d = provs[i].pos.distance_squared(p);
                        if d < bd {
                            bd = d;
                            best = i;
                        }
                    }
                }
                tile[y * MW + x] = best as u16;
                let b = &mut provs[best].bb;
                b.0 = b.0.min(x);
                b.1 = b.1.min(y);
                b.2 = b.2.max(x);
                b.3 = b.3.max(y);
            }
        }
        build_adj(&mut provs, &tile);
        // drop single-province islands
        let (comp, nc) = components(&provs);
        let mut sizes = vec![0usize; nc];
        for &c in &comp {
            if c != usize::MAX {
                sizes[c] += 1;
            }
        }
        for i in 0..provs.len() {
            if comp[i] != usize::MAX && sizes[comp[i]] < 2 {
                provs[i].sea = true;
            }
        }
        build_adj(&mut provs, &tile);
        let (comp, nc) = components(&provs);

        // sea lanes: connect continents (minimum spanning tree + close neighbours)
        let mut members: Vec<Vec<usize>> = vec![vec![]; nc];
        for (i, &c) in comp.iter().enumerate() {
            if c != usize::MAX {
                members[c].push(i);
            }
        }
        let mut pairs: Vec<(f32, usize, usize)> = vec![];
        for i in 0..nc {
            for j in (i + 1)..nc {
                let mut best = (f32::MAX, 0, 0);
                for &a in &members[i] {
                    for &b in &members[j] {
                        let d = provs[a].pos.distance(provs[b].pos);
                        if d < best.0 {
                            best = (d, a, b);
                        }
                    }
                }
                pairs.push(best);
            }
        }
        pairs.sort_by(|x, y| x.0.partial_cmp(&y.0).unwrap());
        let mut group: Vec<usize> = (0..nc).collect();
        for (d, a, b) in pairs {
            let (ga, gb) = (group[comp[a]], group[comp[b]]);
            let close = d < 0.05 * MW as f32;
            if ga != gb || close {
                if ga != gb {
                    for g in group.iter_mut() {
                        if *g == gb {
                            *g = ga;
                        }
                    }
                }
                provs[a].sea_adj.push(b);
                provs[b].sea_adj.push(a);
            }
        }

        // capitals, then ownership: nearest seed (capital or claim point) on the same continent
        let mut caps: Vec<usize> = vec![];
        for (lon, lat) in ANCHORS {
            let c = nearest_land(&provs, proj(lon, lat), &caps);
            caps.push(c);
        }
        let mut seeds: Vec<(usize, usize)> = (0..NC).map(|k| (k, caps[k])).collect();
        for (k, lon, lat) in EXTRAS {
            seeds.push((k, nearest_land(&provs, proj(lon, lat), &[])));
        }
        for i in 0..provs.len() {
            if provs[i].sea {
                continue;
            }
            let same: Vec<&(usize, usize)> = seeds.iter().filter(|sd| comp[sd.1] == comp[i]).collect();
            let cand: Vec<&(usize, usize)> = if same.is_empty() { seeds.iter().collect() } else { same };
            let mut best = cand[0].0;
            let mut bd = f32::MAX;
            for sd in cand {
                let d = provs[i].pos.distance(provs[sd.1].pos) / W[sd.0];
                if d < bd {
                    bd = d;
                    best = sd.0;
                }
            }
            provs[i].owner = best;
        }

        let cs = (0..NC).map(|_| Country { mp: 100.0, ic: 60.0 }).collect();
        let img = Image::gen_image_color(MW as u16, MH as u16, BLACK);
        let tex = Texture2D::from_image(&img);
        tex.set_filter(FilterMode::Nearest);
        let mut g = Game {
            provs,
            tile,
            changed: vec![],
            cs,
            armies: vec![],
            next_id: 1,
            day: 0,
            acc: 0.0,
            paused: true,
            speed_i: 0,
            sel_army: None,
            sel_prov: None,
            img,
            tex,
            dirty: true,
            result: None,
            toast: String::new(),
            toast_t: 0.0,
        };
        for c in 0..NC {
            let owned: Vec<usize> = (0..g.provs.len()).filter(|&i| !g.provs[i].sea && g.provs[i].owner == c).collect();
            g.spawn(c, caps[c], 15.0);
            for _ in 0..(owned.len() / 8).max(1) {
                let p = owned[rand::gen_range(0, owned.len())];
                g.spawn(c, p, 10.0);
            }
        }
        g.recolor_all();
        g
    }
}

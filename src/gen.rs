use macroquad::prelude::*;

use crate::geo;
use crate::game::*;
use crate::world::*;

fn build_adj(provs: &mut Vec<Prov>, tile: &[u16]) {
    for p in provs.iter_mut() {
        p.adj.clear();
        p.coast = false;
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
                    if a != b && provs[a].sea != provs[b].sea {
                        provs[a].coast = true;
                        provs[b].coast = true;
                    }
                }
            }
        }
    }
}

// provinces from different landmasses (islands, continents) are never land-linked,
// and a few narrow straits inside one landmass are cut as well
fn cut_regions(provs: &mut Vec<Prov>, region: &[u16]) {
    let segs: Vec<(Vec2, Vec2)> = STRAITS.iter().map(|&(a, b)| (proj(a.0, a.1), proj(b.0, b.1))).collect();
    for a in 0..provs.len() {
        let pa = provs[a].pos;
        let adj = std::mem::take(&mut provs[a].adj);
        let keep: Vec<usize> = adj
            .into_iter()
            .filter(|&b| region[a] == region[b] && !segs.iter().any(|&(s, e)| seg_cross(pa, provs[b].pos, s, e)))
            .collect();
        provs[a].adj = keep;
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
        let (cols, rows) = (COLS, ROWS);
        rand::srand(11);
        let mut provs: Vec<Prov> = Vec::new();
        let cw = MW as f32 / cols as f32;
        let ch = MH as f32 / rows as f32;
        for r in 0..rows {
            for c in 0..cols {
                let x = (c as f32 + 0.5 + rand::gen_range(-0.3f32, 0.3f32)) * cw;
                let y = (r as f32 + 0.5 + rand::gen_range(-0.3f32, 0.3f32)) * ch;
                provs.push(Prov { pos: vec2(x, y), owner: NEUTRAL, sea: true, coast: false, bb: (MW, MH, 0, 0), adj: vec![], sea_adj: vec![] });
            }
        }
        // land = a province whose centre (and mostly its surroundings) is on real land
        let world = geo::build();
        let o = cw * 0.3;
        let offs = [vec2(0.0, 0.0), vec2(-o, -o), vec2(o, -o), vec2(-o, o), vec2(o, o)];
        let mut region = vec![0u16; provs.len()];
        for (idx, p) in provs.iter_mut().enumerate() {
            let mut regs: Vec<u16> = Vec::new();
            for off in offs {
                let (is_land, r) = world.at(p.pos + off);
                if is_land {
                    regs.push(r);
                }
            }
            if regs.len() >= 2 {
                p.sea = false;
                // the landmass most of its samples belong to
                region[idx] = *regs.iter().max_by_key(|&&r| regs.iter().filter(|&&q| q == r).count()).unwrap();
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
                        if c < 0 || r < 0 || c >= cols as i32 || r >= rows as i32 {
                            continue;
                        }
                        let i = r as usize * cols + c as usize;
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
        cut_regions(&mut provs, &region);
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
        cut_regions(&mut provs, &region);
        for &((lo1, la1), (lo2, la2)) in LINKS {
            let a = nearest_land(&provs, proj(lo1, la1), &[]);
            let b = nearest_land(&provs, proj(lo2, la2), &[]);
            if a != usize::MAX && b != usize::MAX && a != b && !provs[a].adj.contains(&b) {
                provs[a].adj.push(b);
                provs[b].adj.push(a);
            }
        }
        let (comp, nc) = components(&provs);

        // sea lanes: connect continents (minimum spanning tree + close neighbours)
        let mut members: Vec<Vec<usize>> = vec![vec![]; nc];
        for (i, &c) in comp.iter().enumerate() {
            if c != usize::MAX && provs[i].coast {
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
        let mut deg = vec![0usize; nc];
        for (d, a, b) in pairs {
            let (ca, cb) = (comp[a], comp[b]);
            let (ga, gb) = (group[ca], group[cb]);
            let tree_edge = ga != gb;
            let extra = d < 0.02 * MW as f32 && deg[ca] < 3 && deg[cb] < 3;
            if tree_edge || extra {
                if tree_edge {
                    for g in group.iter_mut() {
                        if *g == gb {
                            *g = ga;
                        }
                    }
                }
                deg[ca] += 1;
                deg[cb] += 1;
                provs[a].sea_adj.push(b);
                provs[b].sea_adj.push(a);
            }
        }
        // famous crossings between provinces that are on the same landmass
        for &((lo1, la1), (lo2, la2)) in FERRIES {
            let a = nearest_land(&provs, proj(lo1, la1), &[]);
            let b = nearest_land(&provs, proj(lo2, la2), &[]);
            if a != usize::MAX && b != usize::MAX && !provs[a].sea_adj.contains(&b) {
                provs[a].sea_adj.push(b);
                provs[b].sea_adj.push(a);
            }
        }

        let img = Image::gen_image_color(MW as u16, MH as u16, BLACK);
        let tex = Texture2D::from_image(&img);
        tex.set_filter(FilterMode::Nearest);
        let np = provs.len();
        Game {
            provs,
            tile,
            changed: vec![],
            armies: vec![],
            next_id: 1,
            sel_army: None,
            menu: None,
            bld: vec![0u8; np],
            bld_done: vec![0u32; np],
            img,
            tex,
            dirty: false,
            paused: false,
            speed_i: 0,
            acc: 0.0,
            toast: String::new(),
            toast_t: 0.0,
            roy: crate::royale::Royale::new(),
            flush_t: 0.0,
        }
    }
}

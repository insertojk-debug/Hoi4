use macroquad::prelude::*;

use crate::geo;
use crate::game::*;
use crate::world::*;

fn build_adj(provs: &mut Vec<Prov>, tile: &[u16]) {
    for p in provs.iter_mut() {
        p.adj.clear();
        p.nadj.clear();
        p.ladj.clear();
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
                        let (land, sea) = if provs[a].sea { (b, a) } else { (a, b) };
                        if !provs[land].nadj.contains(&sea) {
                            provs[land].nadj.push(sea);
                            provs[sea].ladj.push(land);
                        }
                    }
                    if a != b && provs[a].sea && provs[b].sea && !provs[a].nadj.contains(&b) {
                        provs[a].nadj.push(b);
                        provs[b].nadj.push(a);
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

// islands of the same archipelago group that lie close together are linked by land,
// so the Philippines and Japan play as connected island chains instead of a tangle of sea lanes
fn hop_link(provs: &mut Vec<Prov>, grp: &[u8], region: &[u16]) {
    let hop = HOP_DEG * (MW as f32 / 350.0);
    for g in 1..=3u8 {
        let list: Vec<usize> = (0..provs.len()).filter(|&i| !provs[i].sea && grp[i] == g).collect();
        for x in 0..list.len() {
            for y in (x + 1)..list.len() {
                let (a, b) = (list[x], list[y]);
                if region[a] != region[b] && provs[a].pos.distance(provs[b].pos) < hop && !provs[a].adj.contains(&b) {
                    provs[a].adj.push(b);
                    provs[b].adj.push(a);
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
        let (cw, ch) = (MW as f32 / COLS as f32, MH as f32 / ROWS as f32);
        // detail boxes in map tiles
        let boxes: Vec<(Vec2, Vec2, f32)> = DETAIL
            .iter()
            .map(|&(lo0, la0, lo1, la1, f)| (vec2(proj(lo0, la1).x, proj(lo0, la1).y), vec2(proj(lo1, la0).x, proj(lo1, la0).y), f))
            .collect();
        let new_prov = |pos: Vec2, sp: f32| Prov { pos, owner: NEUTRAL, sea: true, coast: false, sp, bb: (MW, MH, 0, 0), adj: vec![], nadj: vec![], ladj: vec![], sea_adj: vec![] };
        // normal seeds on a jittered grid, except inside detail boxes
        for r in 0..ROWS {
            for c in 0..COLS {
                let x = (c as f32 + 0.5 + rand::gen_range(-0.3f32, 0.3f32)) * cw;
                let y = (r as f32 + 0.5 + rand::gen_range(-0.3f32, 0.3f32)) * ch;
                let p = vec2(x, y);
                if boxes.iter().any(|b| p.x >= b.0.x && p.x <= b.1.x && p.y >= b.0.y && p.y <= b.1.y) {
                    continue;
                }
                provs.push(new_prov(p, cw));
            }
        }
        // finer seeds inside the detail boxes
        for &(lo, hi, f) in &boxes {
            let s = cw * f;
            let (nx, ny) = (((hi.x - lo.x) / s).ceil() as usize, ((hi.y - lo.y) / s).ceil() as usize);
            for r in 0..ny {
                for c in 0..nx {
                    let x = (lo.x + (c as f32 + 0.5 + rand::gen_range(-0.3f32, 0.3f32)) * s).clamp(lo.x, hi.x);
                    let y = (lo.y + (r as f32 + 0.5 + rand::gen_range(-0.3f32, 0.3f32)) * s).clamp(lo.y, hi.y);
                    provs.push(new_prov(vec2(x, y), s));
                }
            }
        }
        assert!(provs.len() < 65000, "too many provinces for u16 tiles");
        // land = a province whose surroundings are mostly real land (nine samples)
        let world = geo::build();
        let mut region = vec![0u16; provs.len()];
        let mut grp = vec![0u8; provs.len()];
        for (idx, p) in provs.iter_mut().enumerate() {
            let o = p.sp * 0.32;
            let mut regs: Vec<u16> = Vec::new();
            for k in 0..9 {
                let off = if k == 0 { vec2(0.0, 0.0) } else { let a = k as f32 * std::f32::consts::FRAC_PI_4; vec2(a.cos() * o, a.sin() * o) };
                let (is_land, r) = world.at(p.pos + off);
                if is_land {
                    regs.push(r);
                }
            }
            if regs.len() >= 4 {
                p.sea = false;
                // the landmass most of its samples belong to
                region[idx] = *regs.iter().max_by_key(|&&r| regs.iter().filter(|&&q| q == r).count()).unwrap();
                grp[idx] = world.group(region[idx]);
            }
        }
        // pixel -> province lookup using buckets of seeds
        const B: f32 = 12.0;
        let (bnx, bny) = ((MW as f32 / B) as usize + 1, (MH as f32 / B) as usize + 1);
        let mut bucket: Vec<Vec<u32>> = vec![vec![]; bnx * bny];
        for (i, p) in provs.iter().enumerate() {
            bucket[(p.pos.y / B) as usize * bnx + (p.pos.x / B) as usize].push(i as u32);
        }
        let mut tile = vec![0u16; MW * MH];
        for y in 0..MH {
            let by = (y as f32 / B) as i32;
            for x in 0..MW {
                let p = vec2(x as f32 + 0.5, y as f32 + 0.5);
                let bx = (x as f32 / B) as i32;
                let mut best = 0usize;
                let mut rad = 1i32;
                loop {
                    let mut bd = f32::MAX;
                    for dy in -rad..=rad {
                        for dx in -rad..=rad {
                            let (cx, cy) = (bx + dx, by + dy);
                            if cx < 0 || cy < 0 || cx >= bnx as i32 || cy >= bny as i32 {
                                continue;
                            }
                            for &i in &bucket[cy as usize * bnx + cx as usize] {
                                let d = provs[i as usize].pos.distance_squared(p);
                                if d < bd {
                                    bd = d;
                                    best = i as usize;
                                }
                            }
                        }
                    }
                    // a nearer seed outside the searched block is impossible when bd is within its reach
                    if bd <= (B * rad as f32) * (B * rad as f32) || rad >= 4 {
                        break;
                    }
                    rad += 1;
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
        hop_link(&mut provs, &grp, &region);
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
        hop_link(&mut provs, &grp, &region);
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
        for (_d, a, b) in pairs {
            let (ca, cb) = (comp[a], comp[b]);
            let (ga, gb) = (group[ca], group[cb]);
            let tree_edge = ga != gb;
            let extra = false; // only the minimum number of lanes that connects the world
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

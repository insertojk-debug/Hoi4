use macroquad::prelude::*;
use std::collections::VecDeque;

const MW: usize = 720;
const MH: usize = 360;
const COLS: usize = 36;
const ROWS: usize = 18;
const NC: usize = 14;
const SPEEDS: [f32; 4] = [1.0, 2.0, 4.0, 8.0];
const NAMES: [&str; NC] = [
    "United States", "Canada", "Brazil", "Albion", "South Africa", "Australia", "India",
    "Germany", "Russia", "China", "Persia", "Egypt", "Mexico", "Argentina",
];

// Countries 0-6 = Allies (player is 0), 7-13 = Axis
fn at_war(a: usize, b: usize) -> bool {
    a < NC && b < NC && a / 7 != b / 7
}

fn country_color(c: usize) -> Color {
    const P: [(f32, f32, f32); NC] = [
        (0.25, 0.45, 0.95), (0.55, 0.75, 0.95), (0.35, 0.75, 0.40), (0.20, 0.70, 0.75),
        (0.55, 0.40, 0.85), (0.45, 0.85, 0.60), (0.30, 0.55, 0.80),
        (0.85, 0.25, 0.25), (0.70, 0.15, 0.20), (0.90, 0.60, 0.20), (0.85, 0.45, 0.65),
        (0.65, 0.35, 0.25), (0.90, 0.80, 0.25), (0.75, 0.55, 0.35),
    ];
    let (r, g, b) = P[c.min(NC - 1)];
    Color::new(r, g, b, 1.0)
}

// --- world outline, (longitude, latitude), very coarse ---
const NAMERICA: &[(f32, f32)] = &[(-168.0,66.0),(-156.0,71.0),(-130.0,70.0),(-100.0,72.0),(-80.0,73.0),(-62.0,60.0),(-56.0,52.0),(-66.0,45.0),(-70.0,42.0),(-76.0,36.0),(-81.0,31.0),(-80.0,25.0),(-83.0,29.0),(-90.0,30.0),(-97.0,26.0),(-97.0,20.0),(-88.0,21.0),(-87.0,16.0),(-83.0,10.0),(-78.0,8.0),(-82.0,7.0),(-86.0,11.0),(-92.0,14.0),(-105.0,20.0),(-110.0,24.0),(-117.0,32.0),(-124.0,40.0),(-124.0,48.0),(-135.0,58.0),(-148.0,61.0),(-158.0,58.0),(-165.0,60.0)];
const GREENLAND: &[(f32, f32)] = &[(-55.0,60.0),(-42.0,60.0),(-22.0,70.0),(-20.0,78.0),(-35.0,83.0),(-60.0,80.0),(-58.0,72.0)];
const SAMERICA: &[(f32, f32)] = &[(-78.0,8.0),(-72.0,12.0),(-62.0,10.0),(-52.0,5.0),(-50.0,0.0),(-35.0,-5.0),(-35.0,-9.0),(-40.0,-20.0),(-48.0,-26.0),(-58.0,-35.0),(-62.0,-40.0),(-66.0,-46.0),(-69.0,-52.0),(-75.0,-48.0),(-73.0,-38.0),(-71.0,-30.0),(-70.0,-18.0),(-76.0,-14.0),(-81.0,-5.0),(-80.0,0.0),(-77.0,4.0)];
const EURASIA: &[(f32, f32)] = &[(-10.0,36.0),(-10.0,44.0),(-2.0,44.0),(-5.0,48.0),(3.0,51.0),(8.0,54.0),(10.0,57.0),(14.0,55.0),(22.0,55.0),(24.0,60.0),(30.0,60.0),(20.0,70.0),(30.0,71.0),(40.0,68.0),(60.0,69.0),(70.0,73.0),(90.0,77.0),(115.0,77.0),(140.0,72.0),(160.0,70.0),(170.0,67.0),(170.0,60.0),(160.0,53.0),(155.0,58.0),(143.0,59.0),(135.0,54.0),(140.0,47.0),(130.0,42.0),(129.0,35.0),(126.0,35.0),(124.0,40.0),(121.0,39.0),(122.0,31.0),(120.0,25.0),(110.0,21.0),(106.0,10.0),(104.0,1.0),(100.0,6.0),(98.0,16.0),(93.0,20.0),(89.0,22.0),(80.0,15.0),(77.0,8.0),(73.0,17.0),(72.0,22.0),(67.0,25.0),(57.0,25.0),(56.0,27.0),(50.0,30.0),(48.0,29.0),(55.0,24.0),(58.0,23.0),(52.0,16.0),(44.0,12.0),(40.0,17.0),(35.0,28.0),(34.0,31.0),(36.0,36.0),(27.0,36.0),(23.0,36.5),(19.0,40.0),(15.0,38.0),(12.0,42.0),(9.0,44.0),(3.0,43.0),(0.0,39.0),(-5.0,36.0)];
const AFRICA: &[(f32, f32)] = &[(-17.0,21.0),(-10.0,30.0),(-6.0,36.0),(10.0,37.0),(11.0,33.0),(20.0,32.0),(32.0,31.5),(34.0,28.0),(37.0,18.0),(43.0,12.0),(51.0,12.0),(46.0,3.0),(40.0,-5.0),(40.0,-15.0),(35.0,-24.0),(32.0,-29.0),(27.0,-34.0),(20.0,-35.0),(18.0,-30.0),(12.0,-17.0),(13.0,-8.0),(9.0,-1.0),(9.0,4.0),(5.0,5.0),(-8.0,4.0),(-13.0,8.0),(-17.0,14.0)];
const AUSTRALIA: &[(f32, f32)] = &[(114.0,-22.0),(122.0,-18.0),(130.0,-12.0),(137.0,-12.0),(142.0,-11.0),(146.0,-19.0),(153.0,-26.0),(150.0,-37.0),(141.0,-38.0),(130.0,-32.0),(116.0,-35.0),(114.0,-26.0)];
const INDONESIA: &[(f32, f32)] = &[(96.0,4.0),(106.0,-6.0),(114.0,-8.0),(130.0,-8.0),(141.0,-3.0),(135.0,-1.0),(120.0,1.0),(110.0,3.0)];
const JAPAN: &[(f32, f32)] = &[(130.0,32.0),(131.0,37.0),(137.0,38.0),(141.0,42.0),(142.0,45.0),(145.0,43.0),(141.0,36.0),(135.0,33.0)];
const BRITAIN: &[(f32, f32)] = &[(-6.0,50.0),(-6.0,57.0),(-3.0,59.0),(0.0,57.0),(2.0,52.0),(1.0,51.0)];
const LAND: &[&[(f32, f32)]] = &[NAMERICA, GREENLAND, SAMERICA, EURASIA, AFRICA, AUSTRALIA, INDONESIA, JAPAN, BRITAIN];
const HUDSON: &[(f32, f32)] = &[(-95.0,58.0),(-95.0,63.0),(-85.0,64.0),(-78.0,62.0),(-78.0,52.0),(-86.0,53.0)];
const BLACKSEA: &[(f32, f32)] = &[(28.0,42.0),(28.0,46.0),(38.0,47.0),(41.0,42.0),(34.0,41.5)];
const CASPIAN: &[(f32, f32)] = &[(47.0,37.0),(47.0,46.0),(54.0,46.0),(54.0,37.0)];
const SEAS: &[&[(f32, f32)]] = &[HUDSON, BLACKSEA, CASPIAN];

fn proj(lon: f32, lat: f32) -> Vec2 {
    vec2((lon + 170.0) / 340.0 * MW as f32, (75.0 - lat) / 133.0 * MH as f32)
}

fn inside(poly: &[Vec2], p: Vec2) -> bool {
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

struct Prov {
    pos: Vec2,
    owner: usize,
    sea: bool,
    adj: Vec<usize>,     // land borders
    sea_adj: Vec<usize>, // sea lanes to other continents (slow)
}

struct Army {
    id: u32,
    owner: usize,
    size: f32,
    prov: usize,
    path: VecDeque<usize>,
    prog: f32,
}

struct Country {
    mp: f32,
    ic: f32,
}

struct Game {
    provs: Vec<Prov>,
    tile: Vec<usize>,
    cs: Vec<Country>,
    armies: Vec<Army>,
    next_id: u32,
    day: u32,
    acc: f32,
    paused: bool,
    speed_i: usize,
    sel_army: Option<u32>,
    sel_prov: Option<usize>,
    img: Image,
    tex: Texture2D,
    dirty: bool,
    result: Option<bool>,
    toast: String,
    toast_t: f32,
}

fn build_adj(provs: &mut Vec<Prov>, tile: &[usize]) {
    for p in provs.iter_mut() {
        p.adj.clear();
    }
    for y in 0..MH {
        for x in 0..MW {
            let a = tile[y * MW + x];
            for (nx, ny) in [(x + 1, y), (x, y + 1)] {
                if nx < MW && ny < MH {
                    let b = tile[ny * MW + nx];
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
fn components(provs: &[Prov]) -> (Vec<usize>, usize) {
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

impl Game {
    fn new() -> Game {
        rand::srand(11);
        let mut provs: Vec<Prov> = Vec::new();
        let cw = MW as f32 / COLS as f32;
        let ch = MH as f32 / ROWS as f32;
        for r in 0..ROWS {
            for c in 0..COLS {
                let x = (c as f32 + 0.5 + rand::gen_range(-0.3f32, 0.3f32)) * cw;
                let y = (r as f32 + 0.5 + rand::gen_range(-0.3f32, 0.3f32)) * ch;
                provs.push(Prov { pos: vec2(x, y), owner: NC, sea: true, adj: vec![], sea_adj: vec![] });
            }
        }
        // land = province whose centre (and mostly its surroundings) lies inside a continent outline
        let to_poly = |ps: &[(f32, f32)]| -> Vec<Vec2> { ps.iter().map(|&(a, b)| proj(a, b)).collect() };
        let land_polys: Vec<Vec<Vec2>> = LAND.iter().map(|p| to_poly(p)).collect();
        let sea_polys: Vec<Vec<Vec2>> = SEAS.iter().map(|p| to_poly(p)).collect();
        let offs = [vec2(0.0, 0.0), vec2(-7.0, -7.0), vec2(7.0, -7.0), vec2(-7.0, 7.0), vec2(7.0, 7.0)];
        for p in provs.iter_mut() {
            let mut cnt = 0;
            for o in offs {
                let q = p.pos + o;
                if land_polys.iter().any(|pl| inside(pl, q)) && !sea_polys.iter().any(|pl| inside(pl, q)) {
                    cnt += 1;
                }
            }
            if cnt >= 3 {
                p.sea = false;
            }
        }
        // pixel -> province lookup (seeds are a jittered grid, so only nearby cells matter)
        let mut tile = vec![0usize; MW * MH];
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
                tile[y * MW + x] = best;
            }
        }
        build_adj(&mut provs, &tile);
        // drop tiny islands (single provinces)
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
        let mut pairs: Vec<(f32, usize, usize)> = vec![];
        for i in 0..nc {
            for j in (i + 1)..nc {
                let mut best = (f32::MAX, 0, 0);
                for a in 0..provs.len() {
                    if comp[a] != i {
                        continue;
                    }
                    for b in 0..provs.len() {
                        if comp[b] != j {
                            continue;
                        }
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
            let close = d < 0.07 * MW as f32;
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

        // capitals: nearest free land province to each anchor
        let anchors = [
            (-98.0, 38.0), (-100.0, 58.0), (-52.0, -10.0), (0.0, 47.0), (24.0, -24.0), (134.0, -25.0), (78.0, 22.0),
            (12.0, 51.0), (55.0, 58.0), (110.0, 35.0), (48.0, 31.0), (15.0, 20.0), (-102.0, 22.0), (-65.0, -38.0),
        ];
        let mut caps: Vec<usize> = vec![];
        for (lon, lat) in anchors {
            let t = proj(lon, lat);
            let mut best = usize::MAX;
            let mut bd = f32::MAX;
            for (i, p) in provs.iter().enumerate() {
                if p.sea || caps.contains(&i) {
                    continue;
                }
                let d = p.pos.distance_squared(t);
                if d < bd {
                    bd = d;
                    best = i;
                }
            }
            caps.push(best);
        }
        // ownership: nearest capital on the same continent (else nearest overall)
        for i in 0..provs.len() {
            if provs[i].sea {
                continue;
            }
            let same: Vec<usize> = (0..NC).filter(|&k| comp[caps[k]] == comp[i]).collect();
            let cand: Vec<usize> = if same.is_empty() { (0..NC).collect() } else { same };
            let mut best = cand[0];
            let mut bd = f32::MAX;
            for &k in &cand {
                let d = provs[i].pos.distance_squared(provs[caps[k]].pos);
                if d < bd {
                    bd = d;
                    best = k;
                }
            }
            provs[i].owner = best;
        }

        let cs = (0..NC).map(|_| Country { mp: 60.0, ic: 30.0 }).collect();
        let img = Image::gen_image_color(MW as u16, MH as u16, BLACK);
        let tex = Texture2D::from_image(&img);
        tex.set_filter(FilterMode::Nearest);
        let mut g = Game {
            provs,
            tile,
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
            let cap = caps[c];
            g.spawn(c, cap, 15.0);
            if let Some(&n) = g.provs[cap].adj.iter().find(|&&n| g.provs[n].owner == c) {
                g.spawn(c, n, 10.0);
            }
        }
        g.recolor();
        g
    }

    fn spawn(&mut self, owner: usize, prov: usize, size: f32) {
        let id = self.next_id;
        self.next_id += 1;
        self.armies.push(Army { id, owner, size, prov, path: VecDeque::new(), prog: 0.0 });
    }

    fn recolor(&mut self) {
        for y in 0..MH {
            for x in 0..MW {
                let p = self.tile[y * MW + x];
                let sea = self.provs[p].sea;
                let mut col = if sea {
                    Color::new(0.08, 0.14, 0.28, 1.0)
                } else {
                    country_color(self.provs[p].owner)
                };
                if !sea {
                    let r = x + 1 < MW && self.tile[y * MW + x + 1] != p;
                    let d = y + 1 < MH && self.tile[(y + 1) * MW + x] != p;
                    if r || d {
                        col = Color::new(col.r * 0.5, col.g * 0.5, col.b * 0.5, 1.0);
                    }
                }
                self.img.set_pixel(x as u32, y as u32, col);
            }
        }
        self.tex.update(&self.img);
        self.dirty = false;
    }

    fn bfs(&self, from: usize, goal: impl Fn(usize) -> bool) -> Option<VecDeque<usize>> {
        let n = self.provs.len();
        let mut prev = vec![usize::MAX; n];
        let mut seen = vec![false; n];
        let mut q = VecDeque::new();
        q.push_back(from);
        seen[from] = true;
        while let Some(u) = q.pop_front() {
            if u != from && goal(u) {
                let mut path = VecDeque::new();
                let mut c = u;
                while c != from {
                    path.push_front(c);
                    c = prev[c];
                }
                return Some(path);
            }
            for &v in self.provs[u].adj.iter().chain(self.provs[u].sea_adj.iter()) {
                if !seen[v] && !self.provs[v].sea {
                    seen[v] = true;
                    prev[v] = u;
                    q.push_back(v);
                }
            }
        }
        None
    }

    fn recruit(&mut self, c: usize, p: usize) -> bool {
        if self.cs[c].ic < 10.0 || self.cs[c].mp < 20.0 {
            return false;
        }
        self.cs[c].ic -= 10.0;
        self.cs[c].mp -= 20.0;
        if let Some(a) = self.armies.iter_mut().find(|a| a.owner == c && a.prov == p && a.path.is_empty()) {
            a.size += 10.0;
        } else {
            self.spawn(c, p, 10.0);
        }
        true
    }

    fn say(&mut self, s: &str) {
        self.toast = s.to_string();
        self.toast_t = 2.0;
    }

    fn tap_map(&mut self, p: usize) {
        let own_here = self.armies.iter().find(|a| a.owner == 0 && a.prov == p).map(|a| a.id);
        if let Some(id) = own_here {
            self.sel_army = if self.sel_army == Some(id) { None } else { Some(id) };
            self.sel_prov = Some(p);
            return;
        }
        if let Some(id) = self.sel_army {
            if self.provs[p].sea {
                self.say("Can't move to sea");
                return;
            }
            let from = self.armies.iter().find(|a| a.id == id).map(|a| a.prov);
            if let Some(f) = from {
                match self.bfs(f, |q| q == p) {
                    Some(path) => {
                        if let Some(a) = self.armies.iter_mut().find(|a| a.id == id) {
                            a.path = path;
                            a.prog = 0.0;
                        }
                    }
                    None => self.say("Unreachable"),
                }
            }
            return;
        }
        self.sel_prov = Some(p);
    }

    fn step_day(&mut self) {
        self.day += 1;
        for p in &self.provs {
            if !p.sea {
                self.cs[p.owner].mp += 0.5;
                self.cs[p.owner].ic += 0.4;
            }
        }
        // AI
        if self.day % 4 == 0 {
            for c in 1..NC {
                if self.cs[c].ic >= 10.0 && self.cs[c].mp >= 20.0 {
                    let owned: Vec<usize> = (0..self.provs.len())
                        .filter(|&i| !self.provs[i].sea && self.provs[i].owner == c)
                        .collect();
                    if !owned.is_empty() {
                        let p = owned[rand::gen_range(0, owned.len())];
                        self.recruit(c, p);
                    }
                }
            }
            for i in 0..self.armies.len() {
                let o = self.armies[i].owner;
                let pv = self.armies[i].prov;
                if o != 0 && self.armies[i].path.is_empty() {
                    let path = self.bfs(pv, |p| at_war(o, self.provs[p].owner));
                    if let Some(pa) = path {
                        self.armies[i].path = pa;
                        self.armies[i].prog = 0.0;
                    }
                }
            }
        }
        // movement: 4 days per land border, 10 days per sea lane, blocked while fighting
        for i in 0..self.armies.len() {
            let o = self.armies[i].owner;
            let pv = self.armies[i].prov;
            let nx = match self.armies[i].path.front() {
                Some(&n) => n,
                None => continue,
            };
            if self.armies.iter().any(|b| b.prov == pv && at_war(o, b.owner)) {
                continue;
            }
            let rate = if self.provs[pv].sea_adj.contains(&nx) { 0.1 } else { 0.25 };
            self.armies[i].prog += rate;
            if self.armies[i].prog >= 0.999 {
                self.armies[i].prog = 0.0;
                self.armies[i].path.pop_front();
                self.armies[i].prov = nx;
            }
        }
        // combat
        let n = self.armies.len();
        let mut dmg = vec![0f32; n];
        for i in 0..n {
            for j in 0..n {
                if i != j
                    && self.armies[i].prov == self.armies[j].prov
                    && at_war(self.armies[i].owner, self.armies[j].owner)
                {
                    let mut d = self.armies[i].size * 0.12 * rand::gen_range(0.7f32, 1.3f32);
                    if self.armies[j].owner == self.provs[self.armies[j].prov].owner {
                        d *= 0.75; // defender bonus
                    }
                    dmg[j] += d;
                }
            }
        }
        for i in 0..n {
            self.armies[i].size -= dmg[i];
        }
        self.armies.retain(|a| a.size > 0.5);
        // capture
        for i in 0..self.armies.len() {
            let o = self.armies[i].owner;
            let pv = self.armies[i].prov;
            let po = self.provs[pv].owner;
            if !self.provs[pv].sea
                && at_war(o, po)
                && !self.armies.iter().any(|b| b.prov == pv && at_war(o, b.owner))
            {
                self.provs[pv].owner = o;
                self.dirty = true;
            }
        }
        // reinforce at home
        for a in self.armies.iter_mut() {
            if self.provs[a.prov].owner == a.owner && a.size < 30.0 {
                a.size += 0.1;
            }
        }
        // merge idle friendly stacks
        let mut i = 0;
        while i < self.armies.len() {
            let mut j = i + 1;
            while j < self.armies.len() {
                let same = self.armies[i].owner == self.armies[j].owner
                    && self.armies[i].prov == self.armies[j].prov
                    && self.armies[i].path.is_empty()
                    && self.armies[j].path.is_empty();
                if same {
                    let s = self.armies[j].size;
                    self.armies[i].size += s;
                    if self.sel_army == Some(self.armies[j].id) {
                        self.sel_army = Some(self.armies[i].id);
                    }
                    self.armies.remove(j);
                } else {
                    j += 1;
                }
            }
            i += 1;
        }
        if let Some(id) = self.sel_army {
            if !self.armies.iter().any(|a| a.id == id) {
                self.sel_army = None;
            }
        }
        let mine = self.provs.iter().filter(|p| !p.sea && p.owner == 0).count();
        let enemy = self.provs.iter().filter(|p| !p.sea && at_war(0, p.owner)).count();
        if mine == 0 {
            self.result = Some(false);
        } else if enemy == 0 {
            self.result = Some(true);
        }
    }
}

struct Lay {
    fs: f32,
    top: f32,
    bar: f32,
    base: f32,
    avail_h: f32,
}

fn layout() -> Lay {
    let sw = screen_width();
    let sh = screen_height();
    let fs = (sh / 24.0).clamp(14.0, 48.0);
    let top = fs * 1.8;
    let bar = fs * 3.2;
    let avail_h = (sh - top - bar).max(1.0);
    let base = (sw / MW as f32).min(avail_h / MH as f32);
    Lay { fs, top, bar, base, avail_h }
}

fn ctext(t: &str, cx: f32, cy: f32, fs: f32, col: Color) {
    let d = measure_text(t, None, fs as u16, 1.0);
    draw_text(t, cx - d.width / 2.0, cy + d.height / 2.0, fs, col);
}

fn date(day: u32) -> String {
    format!("{}-{:02}-{:02}", 1936 + day / 360, (day % 360) / 30 + 1, day % 30 + 1)
}

// bottom bar buttons: PLAY, SPEED, RECRUIT, ZOOM+, ZOOM-
const BW: [f32; 5] = [1.2, 1.2, 2.2, 0.6, 0.6];

fn button_x(sw: f32, i: usize) -> (f32, f32) {
    let total: f32 = BW.iter().sum();
    let x: f32 = BW[..i].iter().sum::<f32>() / total * sw;
    (x, BW[i] / total * sw)
}

fn conf() -> Conf {
    Conf {
        window_title: "HOI Clone".to_string(),
        window_width: 1280,
        window_height: 720,
        ..Default::default()
    }
}

#[macroquad::main(conf)]
async fn main() {
    let mut g = Game::new();
    let mut cam = g.provs[g.armies[0].prov].pos;
    let mut zoom = 3.0f32;
    let mut press: Option<(Vec2, Vec2)> = None;
    let mut moved = false;
    let mut pinch_prev: Option<f32> = None;
    loop {
        let l = layout();
        let sw = screen_width();
        let sh = screen_height();
        let dt = get_frame_time();
        let mid_y = l.top + l.avail_h / 2.0;
        let (mx, my) = mouse_position();

        // ---- input ----
        let (_, wy) = mouse_wheel();
        if wy != 0.0 {
            zoom = (zoom * (1.0 + 0.1 * wy.signum())).clamp(1.0, 10.0);
        }
        let ts = touches();
        if ts.len() >= 2 {
            let d = ts[0].position.distance(ts[1].position);
            if let Some(pd) = pinch_prev {
                if pd > 1.0 {
                    zoom = (zoom * d / pd).clamp(1.0, 10.0);
                }
            }
            pinch_prev = Some(d);
            press = None;
        } else {
            pinch_prev = None;
            if is_mouse_button_pressed(MouseButton::Left) {
                if g.result.is_some() {
                    g = Game::new();
                    cam = g.provs[g.armies[0].prov].pos;
                    zoom = 3.0;
                } else if my > sh - l.bar {
                    let mut bi = 0;
                    for i in 0..5 {
                        let (x, w) = button_x(sw, i);
                        if mx >= x && mx < x + w {
                            bi = i;
                        }
                    }
                    match bi {
                        0 => g.paused = !g.paused,
                        1 => g.speed_i = (g.speed_i + 1) % SPEEDS.len(),
                        2 => {
                            let ok = g.sel_prov.filter(|&p| !g.provs[p].sea && g.provs[p].owner == 0);
                            match ok {
                                Some(p) => {
                                    if !g.recruit(0, p) {
                                        g.say("Need 10 IC + 20 manpower");
                                    }
                                }
                                None => g.say("Select one of your provinces"),
                            }
                        }
                        3 => zoom = (zoom * 1.4).clamp(1.0, 10.0),
                        _ => zoom = (zoom / 1.4).clamp(1.0, 10.0),
                    }
                } else if my > l.top {
                    press = Some((vec2(mx, my), cam));
                    moved = false;
                }
            }
            if let Some((start, cam0)) = press {
                let scale = l.base * zoom;
                if is_mouse_button_down(MouseButton::Left) {
                    let delta = vec2(mx, my) - start;
                    if delta.length() > l.fs * 0.4 {
                        moved = true;
                    }
                    if moved {
                        cam = cam0 - delta / scale;
                    }
                }
                if is_mouse_button_released(MouseButton::Left) {
                    if !moved {
                        let tx = ((mx - sw / 2.0) / scale + cam.x).floor();
                        let ty = ((my - mid_y) / scale + cam.y).floor();
                        if tx >= 0.0 && ty >= 0.0 && (tx as usize) < MW && (ty as usize) < MH {
                            let p = g.tile[ty as usize * MW + tx as usize];
                            g.tap_map(p);
                        }
                    }
                    press = None;
                }
            }
        }
        // keep camera inside the map
        let scale = l.base * zoom;
        let hw = sw / 2.0 / scale;
        let hh = l.avail_h / 2.0 / scale;
        cam.x = if hw >= MW as f32 / 2.0 { MW as f32 / 2.0 } else { cam.x.clamp(hw, MW as f32 - hw) };
        cam.y = if hh >= MH as f32 / 2.0 { MH as f32 / 2.0 } else { cam.y.clamp(hh, MH as f32 - hh) };

        // ---- simulation ----
        if !g.paused && g.result.is_none() {
            g.acc += dt * SPEEDS[g.speed_i];
            while g.acc >= 0.5 {
                g.acc -= 0.5;
                g.step_day();
            }
        }
        if g.dirty {
            g.recolor();
        }
        if g.toast_t > 0.0 {
            g.toast_t -= dt;
        }

        // ---- draw ----
        clear_background(Color::new(0.05, 0.05, 0.08, 1.0));
        draw_texture_ex(
            &g.tex,
            sw / 2.0 - cam.x * scale,
            mid_y - cam.y * scale,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(MW as f32 * scale, MH as f32 * scale)),
                ..Default::default()
            },
        );
        let to_screen = |p: Vec2| vec2(sw / 2.0 + (p.x - cam.x) * scale, mid_y + (p.y - cam.y) * scale);

        // sea lanes
        for (a, pa) in g.provs.iter().enumerate() {
            for &b in &pa.sea_adj {
                if a < b {
                    let s = to_screen(pa.pos);
                    let e = to_screen(g.provs[b].pos);
                    draw_line(s.x, s.y, e.x, e.y, 2.0, Color::new(0.5, 0.8, 1.0, 0.55));
                }
            }
        }
        if let Some(p) = g.sel_prov {
            let c = to_screen(g.provs[p].pos);
            draw_circle_lines(c.x, c.y, l.fs * 1.4, 3.0, WHITE);
        }
        if let Some(id) = g.sel_army {
            if let Some(a) = g.armies.iter().find(|a| a.id == id) {
                let mut prev = to_screen(g.provs[a.prov].pos);
                for &q in &a.path {
                    let nx = to_screen(g.provs[q].pos);
                    draw_line(prev.x, prev.y, nx.x, nx.y, 3.0, YELLOW);
                    prev = nx;
                }
            }
        }
        let bw = (20.0 * scale * 0.95).clamp(l.fs * 1.3, l.fs * 2.2);
        let bh = bw * 0.6;
        for (i, a) in g.armies.iter().enumerate() {
            let k = g.armies[..i].iter().filter(|b| b.prov == a.prov).count() as f32;
            let c = to_screen(g.provs[a.prov].pos);
            let x = c.x - bw / 2.0 + k * (bw + 4.0);
            let y = c.y - bh / 2.0;
            draw_rectangle(x, y, bw, bh, country_color(a.owner));
            let selected = g.sel_army == Some(a.id);
            draw_rectangle_lines(
                x,
                y,
                bw,
                bh,
                if selected { 5.0 } else { 2.0 },
                if selected { YELLOW } else { WHITE },
            );
            ctext(&format!("{}", a.size as i32), x + bw / 2.0, y + bh / 2.0, bh * 0.8, WHITE);
        }

        // top HUD
        draw_rectangle(0.0, 0.0, sw, l.top, Color::new(0.1, 0.1, 0.14, 1.0));
        let info = match g.sel_prov {
            Some(p) if !g.provs[p].sea => format!("Province: {}", NAMES[g.provs[p].owner]),
            Some(_) => "Province: Sea".to_string(),
            None => "Tap a province".to_string(),
        };
        draw_text(
            &format!("{}   MP {:.0}   IC {:.0}   |   {}", date(g.day), g.cs[0].mp, g.cs[0].ic, info),
            l.fs * 0.6,
            l.top * 0.68,
            l.fs,
            WHITE,
        );

        // bottom bar
        draw_rectangle(0.0, sh - l.bar, sw, l.bar, Color::new(0.1, 0.1, 0.14, 1.0));
        let labels = [
            if g.paused { "> PLAY".to_string() } else { "|| PAUSE".to_string() },
            format!("SPEED x{}", SPEEDS[g.speed_i] as i32),
            "RECRUIT 10IC 20MP".to_string(),
            "+".to_string(),
            "-".to_string(),
        ];
        for (i, t) in labels.iter().enumerate() {
            let (x, w) = button_x(sw, i);
            draw_rectangle(x + 3.0, sh - l.bar + 4.0, w - 6.0, l.bar - 8.0, Color::new(0.2, 0.22, 0.3, 1.0));
            ctext(t, x + w / 2.0, sh - l.bar / 2.0, l.fs * 0.85, WHITE);
        }

        if g.toast_t > 0.0 {
            ctext(&g.toast, sw / 2.0, sh - l.bar - l.fs * 1.2, l.fs * 1.1, YELLOW);
        }
        if let Some(win) = g.result {
            draw_rectangle(0.0, 0.0, sw, sh, Color::new(0.0, 0.0, 0.0, 0.7));
            ctext(if win { "VICTORY" } else { "DEFEAT" }, sw / 2.0, sh / 2.0 - l.fs, l.fs * 3.0, WHITE);
            ctext("Tap to restart", sw / 2.0, sh / 2.0 + l.fs * 2.0, l.fs, GRAY);
        }

        next_frame().await;
    }
}

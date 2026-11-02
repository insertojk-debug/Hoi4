use macroquad::prelude::*;
use std::collections::VecDeque;

const MW: usize = 192;
const MH: usize = 108;
const COLS: usize = 9;
const ROWS: usize = 6;
const NC: usize = 6;
const NAMES: [&str; NC] = ["Westmark", "Aurelia", "Valdoria", "Eastgard", "Krovia", "Sundar"];
const SPEEDS: [f32; 4] = [1.0, 2.0, 4.0, 8.0];

// Countries 0-2 = Allies (player is 0), 3-5 = Axis
fn at_war(a: usize, b: usize) -> bool {
    a < NC && b < NC && a / 3 != b / 3
}

fn country_color(c: usize) -> Color {
    match c {
        0 => Color::new(0.25, 0.45, 0.95, 1.0),
        1 => Color::new(0.20, 0.70, 0.75, 1.0),
        2 => Color::new(0.55, 0.40, 0.85, 1.0),
        3 => Color::new(0.85, 0.25, 0.25, 1.0),
        4 => Color::new(0.90, 0.60, 0.20, 1.0),
        _ => Color::new(0.65, 0.35, 0.25, 1.0),
    }
}

struct Prov {
    pos: Vec2,
    owner: usize,
    sea: bool,
    adj: Vec<usize>,
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

impl Game {
    fn new() -> Game {
        rand::srand(7);
        let mut provs: Vec<Prov> = Vec::new();
        let cw = MW as f32 / COLS as f32;
        let ch = MH as f32 / ROWS as f32;
        for r in 0..ROWS {
            for c in 0..COLS {
                let x = (c as f32 + 0.5 + rand::gen_range(-0.3f32, 0.3f32)) * cw;
                let y = (r as f32 + 0.5 + rand::gen_range(-0.3f32, 0.3f32)) * ch;
                provs.push(Prov { pos: vec2(x, y), owner: NC, sea: false, adj: vec![] });
            }
        }
        // capitals: nearest province to each anchor point
        let anchors = [(0.15, 0.3), (0.15, 0.75), (0.38, 0.5), (0.85, 0.3), (0.85, 0.75), (0.62, 0.5)];
        let mut caps: Vec<usize> = vec![];
        for (ax, ay) in anchors {
            let t = vec2(ax * MW as f32, ay * MH as f32);
            let mut best = 0;
            let mut bd = f32::MAX;
            for (i, p) in provs.iter().enumerate() {
                let d = p.pos.distance_squared(t);
                if d < bd {
                    bd = d;
                    best = i;
                }
            }
            caps.push(best);
        }
        for i in 0..provs.len() {
            if !caps.contains(&i) && rand::gen_range(0, 100) < 9 {
                provs[i].sea = true;
            }
        }
        for i in 0..provs.len() {
            if provs[i].sea {
                continue;
            }
            let mut best = 0;
            let mut bd = f32::MAX;
            for (k, &c) in caps.iter().enumerate() {
                let d = provs[i].pos.distance_squared(provs[c].pos);
                if d < bd {
                    bd = d;
                    best = k;
                }
            }
            provs[i].owner = best;
        }
        // pixel -> province lookup
        let mut tile = vec![0usize; MW * MH];
        for y in 0..MH {
            for x in 0..MW {
                let p = vec2(x as f32 + 0.5, y as f32 + 0.5);
                let mut best = 0;
                let mut bd = f32::MAX;
                for (i, pr) in provs.iter().enumerate() {
                    let d = pr.pos.distance_squared(p);
                    if d < bd {
                        bd = d;
                        best = i;
                    }
                }
                tile[y * MW + x] = best;
            }
        }
        // adjacency
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
        // starting armies
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
            for &v in &self.provs[u].adj {
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
        // movement (4 days per province, blocked while fighting)
        for i in 0..self.armies.len() {
            if self.armies[i].path.is_empty() {
                continue;
            }
            let o = self.armies[i].owner;
            let pv = self.armies[i].prov;
            let engaged = self.armies.iter().any(|b| b.prov == pv && at_war(o, b.owner));
            if engaged {
                continue;
            }
            self.armies[i].prog += 0.25;
            if self.armies[i].prog >= 1.0 {
                self.armies[i].prog = 0.0;
                if let Some(nx) = self.armies[i].path.pop_front() {
                    self.armies[i].prov = nx;
                }
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
    scale: f32,
    ox: f32,
    oy: f32,
}

fn layout() -> Lay {
    let sw = screen_width();
    let sh = screen_height();
    let fs = (sh / 24.0).clamp(14.0, 48.0);
    let top = fs * 1.8;
    let bar = fs * 3.2;
    let avail_h = sh - top - bar;
    let scale = (sw / MW as f32).min(avail_h / MH as f32);
    let ox = (sw - MW as f32 * scale) / 2.0;
    let oy = top + (avail_h - MH as f32 * scale) / 2.0;
    Lay { fs, top, bar, scale, ox, oy }
}

fn ctext(t: &str, cx: f32, cy: f32, fs: f32, col: Color) {
    let d = measure_text(t, None, fs as u16, 1.0);
    draw_text(t, cx - d.width / 2.0, cy + d.height / 2.0, fs, col);
}

fn date(day: u32) -> String {
    format!("{}-{:02}-{:02}", 1936 + day / 360, (day % 360) / 30 + 1, day % 30 + 1)
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
    loop {
        let l = layout();
        let sw = screen_width();
        let sh = screen_height();
        let dt = get_frame_time();

        // ---- input ----
        if is_mouse_button_pressed(MouseButton::Left) {
            let (mx, my) = mouse_position();
            if g.result.is_some() {
                g = Game::new();
            } else if my > sh - l.bar {
                let bi = ((mx / (sw / 3.0)) as usize).min(2);
                match bi {
                    0 => g.paused = !g.paused,
                    1 => g.speed_i = (g.speed_i + 1) % SPEEDS.len(),
                    _ => {
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
                }
            } else if my > l.top {
                let tx = ((mx - l.ox) / l.scale).floor();
                let ty = ((my - l.oy) / l.scale).floor();
                if tx >= 0.0 && ty >= 0.0 && (tx as usize) < MW && (ty as usize) < MH {
                    let p = g.tile[ty as usize * MW + tx as usize];
                    g.tap_map(p);
                }
            }
        }

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
            l.ox,
            l.oy,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(MW as f32 * l.scale, MH as f32 * l.scale)),
                ..Default::default()
            },
        );
        let to_screen = |p: Vec2| vec2(l.ox + p.x * l.scale, l.oy + p.y * l.scale);

        // selected province marker
        if let Some(p) = g.sel_prov {
            let c = to_screen(g.provs[p].pos);
            draw_circle_lines(c.x, c.y, l.fs * 1.4, 3.0, WHITE);
        }
        // path of selected army
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
        // armies
        let (bw, bh) = (l.fs * 2.0, l.fs * 1.2);
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
            ctext(&format!("{}", a.size as i32), x + bw / 2.0, y + bh / 2.0, l.fs * 0.9, WHITE);
        }

        // top HUD
        draw_rectangle(0.0, 0.0, sw, l.top, Color::new(0.1, 0.1, 0.14, 1.0));
        let info = match g.sel_prov {
            Some(p) if !g.provs[p].sea => format!("Province: {}", NAMES[g.provs[p].owner]),
            Some(_) => "Province: Sea".to_string(),
            None => "Tap a province".to_string(),
        };
        draw_text(
            &format!(
                "{}   MP {:.0}   IC {:.0}   |   {}",
                date(g.day),
                g.cs[0].mp,
                g.cs[0].ic,
                info
            ),
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
            "RECRUIT (10 IC, 20 MP)".to_string(),
        ];
        for (i, t) in labels.iter().enumerate() {
            let x = i as f32 * sw / 3.0 + 4.0;
            draw_rectangle(x, sh - l.bar + 4.0, sw / 3.0 - 8.0, l.bar - 8.0, Color::new(0.2, 0.22, 0.3, 1.0));
            ctext(t, x + (sw / 3.0 - 8.0) / 2.0, sh - l.bar / 2.0, l.fs, WHITE);
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

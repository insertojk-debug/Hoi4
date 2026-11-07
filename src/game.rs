use macroquad::prelude::*;
use std::collections::VecDeque;

use crate::royale::Royale;
use crate::world::*;

pub struct Prov {
    pub pos: Vec2,
    pub owner: usize,
    pub sea: bool,
    pub coast: bool,
    pub bb: (usize, usize, usize, usize), // bounding box in tiles
    pub adj: Vec<usize>,                  // land borders
    pub sea_adj: Vec<usize>,              // sea lanes to other continents (slow)
}

// soldiers of any player (yours have owner 0)
pub struct Army {
    pub id: u32,
    pub owner: usize,
    pub kind: usize,
    pub size: f32,
    pub prov: usize,
    pub path: VecDeque<usize>,
    pub prog: f32,
}

pub struct Game {
    pub provs: Vec<Prov>,
    pub tile: Vec<u16>,
    pub changed: Vec<usize>,
    pub armies: Vec<Army>,
    pub next_id: u32,
    pub sel_army: Option<u32>,
    pub menu: Option<usize>, // province whose train menu is open
    pub img: Image,
    pub tex: Texture2D,
    pub dirty: bool,
    pub paused: bool,
    pub speed_i: usize,
    pub acc: f32,
    pub toast: String,
    pub toast_t: f32,
    pub roy: Royale,
    pub flush_t: f32,
}

impl Game {
    pub fn spawn(&mut self, owner: usize, prov: usize, size: f32, kind: usize) {
        let id = self.next_id;
        self.next_id += 1;
        self.armies.push(Army { id, owner, kind, size, prov, path: VecDeque::new(), prog: 0.0 });
    }

    fn paint_prov(&mut self, p: usize) {
        let (x0, y0, x1, y1) = self.provs[p].bb;
        let sea = self.provs[p].sea;
        let sea_col = Color::new(0.08, 0.14, 0.28, 1.0);
        let base = if sea { sea_col } else { crate::royale::owner_color(self, self.provs[p].owner) };
        let dark = Color::new(base.r * 0.5, base.g * 0.5, base.b * 0.5, 1.0);
        for y in y0..=y1 {
            for x in x0..=x1 {
                if self.tile[y * MW + x] as usize != p {
                    continue;
                }
                let mut col = base;
                if !sea {
                    let rq = if x + 1 < MW { self.tile[y * MW + x + 1] as usize } else { p };
                    let dq = if y + 1 < MH { self.tile[(y + 1) * MW + x] as usize } else { p };
                    if rq != p || dq != p {
                        // a border; where two land provinces are not linked (a strait) show water instead
                        let strait = |q: usize| q != p && !self.provs[q].sea && !self.provs[p].adj.contains(&q);
                        col = if strait(rq) || strait(dq) { sea_col } else { dark };
                    }
                }
                self.img.set_pixel(x as u32, y as u32, col);
            }
        }
    }

    pub fn recolor_all(&mut self) {
        for p in 0..self.provs.len() {
            self.paint_prov(p);
        }
        self.tex.update(&self.img);
        self.changed.clear();
        self.dirty = false;
    }

    pub fn flush(&mut self) {
        let ch = std::mem::take(&mut self.changed);
        self.dirty = false;
        if ch.is_empty() {
            return;
        }
        let (mut x0, mut y0, mut x1, mut y1) = (MW, MH, 0usize, 0usize);
        for p in ch {
            self.paint_prov(p);
            let b = self.provs[p].bb;
            x0 = x0.min(b.0);
            y0 = y0.min(b.1);
            x1 = x1.max(b.2);
            y1 = y1.max(b.3);
        }
        let (w, h) = (x1 - x0 + 1, y1 - y0 + 1);
        if w * h > MW * MH / 3 {
            self.tex.update(&self.img);
        } else {
            let sub = self.img.sub_image(Rect::new(x0 as f32, y0 as f32, w as f32, h as f32));
            self.tex.update_part(&sub, x0 as i32, y0 as i32, w as i32, h as i32);
        }
    }

    pub fn bfs(&self, from: usize, goal: impl Fn(usize) -> bool) -> Option<VecDeque<usize>> {
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
                if !seen[v] && !self.provs[v].sea && self.provs[v].owner != DEAD {
                    seen[v] = true;
                    prev[v] = u;
                    q.push_back(v);
                }
            }
        }
        None
    }

    pub fn can_afford(&self, owner: usize, kind: usize) -> bool {
        self.roy.players[owner].troops >= TCOST[kind]
    }

    // train a soldier in one of a player's provinces; it costs troops
    pub fn recruit(&mut self, owner: usize, p: usize, kind: usize) -> bool {
        if !self.can_afford(owner, kind) {
            return false;
        }
        self.roy.players[owner].troops -= TCOST[kind];
        if let Some(a) = self.armies.iter_mut().find(|a| a.owner == owner && a.prov == p && a.kind == kind && a.path.is_empty()) {
            a.size += 10.0;
        } else {
            self.spawn(owner, p, 10.0, kind);
        }
        true
    }

    pub fn say(&mut self, s: &str) {
        self.toast = s.to_string();
        self.toast_t = 2.0;
    }

    // returns false when the tap was not about your soldiers or your own land
    pub fn tap_map(&mut self, p: usize) -> bool {
        self.menu = None;
        // one of your soldiers here: select it (tap again to deselect)
        let own_here = self.armies.iter().find(|a| a.owner == 0 && a.prov == p).map(|a| a.id);
        if let Some(id) = own_here {
            self.sel_army = if self.sel_army == Some(id) { None } else { Some(id) };
            return true;
        }
        // a soldier is selected: send it there, then let go of it so you can pick the next one right away
        if let Some(id) = self.sel_army {
            if self.provs[p].sea {
                self.say("Can't move to sea");
                return true;
            }
            let from = self.armies.iter().find(|a| a.id == id).map(|a| a.prov);
            if let Some(f) = from {
                match self.bfs(f, |q| q == p) {
                    Some(path) => {
                        if let Some(a) = self.armies.iter_mut().find(|a| a.id == id) {
                            a.path = path;
                            a.prog = 0.0;
                        }
                        self.sel_army = None;
                    }
                    None => self.say("Unreachable"),
                }
            } else {
                self.sel_army = None;
            }
            return true;
        }
        // one of your own empty provinces: open the train menu
        if !self.provs[p].sea && self.provs[p].owner == 0 {
            self.menu = Some(p);
            return true;
        }
        false
    }
}

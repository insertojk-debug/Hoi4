use macroquad::prelude::*;
use std::collections::{HashMap, VecDeque};

use crate::world::*;

pub struct Prov {
    pub pos: Vec2,
    pub owner: usize,
    pub sea: bool,
    pub bb: (usize, usize, usize, usize), // bounding box in tiles
    pub adj: Vec<usize>,                  // land borders
    pub sea_adj: Vec<usize>,              // sea lanes to other continents (slow)
}

pub struct Army {
    pub id: u32,
    pub owner: usize,
    pub size: f32,
    pub prov: usize,
    pub path: VecDeque<usize>,
    pub prog: f32,
}

pub struct Country {
    pub mp: f32,
    pub ic: f32,
}

pub struct Game {
    pub provs: Vec<Prov>,
    pub tile: Vec<u16>,
    pub changed: Vec<usize>,
    pub cs: Vec<Country>,
    pub armies: Vec<Army>,
    pub next_id: u32,
    pub day: u32,
    pub acc: f32,
    pub paused: bool,
    pub speed_i: usize,
    pub sel_army: Option<u32>,
    pub sel_prov: Option<usize>,
    pub img: Image,
    pub tex: Texture2D,
    pub dirty: bool,
    pub result: Option<bool>,
    pub toast: String,
    pub toast_t: f32,
}

impl Game {
    pub fn spawn(&mut self, owner: usize, prov: usize, size: f32) {
        let id = self.next_id;
        self.next_id += 1;
        self.armies.push(Army { id, owner, size, prov, path: VecDeque::new(), prog: 0.0 });
    }

    fn paint_prov(&mut self, p: usize) {
        let (x0, y0, x1, y1) = self.provs[p].bb;
        let sea = self.provs[p].sea;
        let sea_col = Color::new(0.08, 0.14, 0.28, 1.0);
        let base = if sea { sea_col } else { country_color(self.provs[p].owner) };
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
        for p in ch {
            self.paint_prov(p);
        }
        self.tex.update(&self.img);
        self.dirty = false;
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
                if !seen[v] && !self.provs[v].sea {
                    seen[v] = true;
                    prev[v] = u;
                    q.push_back(v);
                }
            }
        }
        None
    }

    pub fn recruit(&mut self, c: usize, p: usize) -> bool {
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

    pub fn say(&mut self, s: &str) {
        self.toast = s.to_string();
        self.toast_t = 2.0;
    }

    pub fn tap_map(&mut self, p: usize) {
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

    pub fn occupancy(&self) -> HashMap<usize, Vec<usize>> {
        let mut m: HashMap<usize, Vec<usize>> = HashMap::new();
        for (i, a) in self.armies.iter().enumerate() {
            m.entry(a.prov).or_default().push(i);
        }
        m
    }
}

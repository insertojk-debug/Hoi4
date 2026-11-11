use macroquad::prelude::*;
use std::collections::VecDeque;

use crate::diplo::hostile;
use crate::royale::Royale;
use crate::world::*;

pub struct Prov {
    pub pos: Vec2,
    pub owner: usize,
    pub sea: bool,
    pub coast: bool,
    pub bb: (usize, usize, usize, usize), // bounding box in tiles
    pub adj: Vec<usize>,                  // land borders
    pub nadj: Vec<usize>,                 // neighbouring water provinces (for ships and for coastal land)
    pub ladj: Vec<usize>,                 // for water provinces: neighbouring land provinces
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
    pub menu: Option<usize>, // province whose build / train menu is open
    pub bld: Vec<u8>,        // per province: 0 nothing, 1 civilian factory, 2 military factory
    pub bld_done: Vec<u32>,  // tick at which that building is finished
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

    // route over water for ships
    pub fn bfs_sea(&self, from: usize, goal: impl Fn(usize) -> bool) -> Option<VecDeque<usize>> {
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
            for &v in &self.provs[u].nadj {
                if !seen[v] && self.provs[v].sea {
                    seen[v] = true;
                    prev[v] = u;
                    q.push_back(v);
                }
            }
        }
        None
    }

    pub fn say(&mut self, s: &str) {
        self.toast = s.to_string();
        self.toast_t = 2.0;
    }

    // a soldier is selected: send it to the tapped province. Returns false if no soldier is selected.
    pub fn tap_map(&mut self, p: usize) -> bool {
        let id = match self.sel_army {
            Some(id) => id,
            None => return false,
        };
        if self.provs[p].owner == DEAD {
            self.say("Can't go there");
            return true;
        }
        let from = self.armies.iter().find(|a| a.id == id).map(|a| (a.prov, a.size * ATK[a.kind], a.kind));
        let (f, power, kind) = match from {
            Some(x) => x,
            None => {
                self.sel_army = None;
                return true;
            }
        };
        if f == p {
            self.sel_army = None; // tapped its own tile: let go of it
            return true;
        }
        let ship = is_naval(kind);
        if ship && !self.provs[p].sea {
            self.say("Ships can only sail on water");
            return true;
        }
        if !ship && self.provs[p].sea {
            self.say("Soldiers can't walk on water");
            return true;
        }
        let route = if ship { self.bfs_sea(f, |q| q == p) } else { self.bfs(f, |q| q == p) };
        match route {
            Some(path) => {
                let foe: f32 = self.armies.iter().filter(|a| a.prov == p && hostile(self, 0, a.owner)).map(|a| a.size * ATK[a.kind]).sum();
                let owner = self.provs[p].owner;
                if foe > 0.0 {
                    self.say(&format!("Attacking enemy soldiers: strength {:.0} vs your {:.0}", foe, power));
                } else if owner != 0 && owner < crate::royale::PLAYERS && !hostile(self, 0, owner) {
                    self.say(&format!("You are not at war with Bot {} - declare war to take their land", owner));
                }
                if let Some(a) = self.armies.iter_mut().find(|a| a.id == id) {
                    a.path = path;
                    a.prog = 0.0;
                }
                self.sel_army = None; // let go so the next soldier can be picked right away
            }
            None => self.say("Unreachable"),
        }
        true
    }
}

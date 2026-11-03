use macroquad::prelude::*;

use crate::game::*;
use crate::world::*;

impl Game {
    pub fn step_day(&mut self) {
        self.day += 1;
        let mut land = vec![0usize; NC];
        for p in &self.provs {
            if !p.sea {
                self.cs[p.owner].mp += 0.2;
                self.cs[p.owner].ic += 0.15;
                land[p.owner] += 1;
            }
        }
        // AI: recruit and give orders every 4 days
        if self.day % 4 == 0 {
            let mut count = vec![0usize; NC];
            for a in &self.armies {
                count[a.owner] += 1;
            }
            for c in 1..NC {
                for _ in 0..2 {
                    if self.cs[c].ic < 10.0 || self.cs[c].mp < 20.0 || count[c] >= 4 + land[c] / 3 {
                        break;
                    }
                    let owned: Vec<usize> = (0..self.provs.len())
                        .filter(|&i| !self.provs[i].sea && self.provs[i].owner == c)
                        .collect();
                    if !owned.is_empty() {
                        let p = owned[rand::gen_range(0, owned.len())];
                        self.recruit(c, p);
                        count[c] += 1;
                    }
                }
            }
            // only an eighth of the idle AI armies get new orders each time (keeps it fast)
            let turn = self.day / 4;
            for i in 0..self.armies.len() {
                let o = self.armies[i].owner;
                let pv = self.armies[i].prov;
                if o != 0 && self.armies[i].path.is_empty() && (self.armies[i].id + turn) % 8 == 0 {
                    let path = self.bfs(pv, |p| at_war(o, self.provs[p].owner));
                    if let Some(pa) = path {
                        self.armies[i].path = pa;
                        self.armies[i].prog = 0.0;
                    }
                }
            }
        }
        // movement: 4 days per land border, 10 days per sea lane, blocked while fighting
        let occ = self.occupancy();
        for i in 0..self.armies.len() {
            let o = self.armies[i].owner;
            let pv = self.armies[i].prov;
            let nx = match self.armies[i].path.front() {
                Some(&n) => n,
                None => continue,
            };
            let engaged = occ.get(&pv).map_or(false, |v| v.iter().any(|&j| at_war(o, self.armies[j].owner)));
            if engaged {
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
        let occ = self.occupancy();
        let mut dmg = vec![0f32; self.armies.len()];
        for v in occ.values() {
            if v.len() < 2 {
                continue;
            }
            for &i in v {
                for &j in v {
                    if i != j && at_war(self.armies[i].owner, self.armies[j].owner) {
                        let mut d = self.armies[i].size * 0.12 * rand::gen_range(0.7f32, 1.3f32);
                        if self.armies[j].owner == self.provs[self.armies[j].prov].owner {
                            d *= 0.75; // defender bonus
                        }
                        dmg[j] += d;
                    }
                }
            }
        }
        for i in 0..self.armies.len() {
            self.armies[i].size -= dmg[i];
        }
        self.armies.retain(|a| a.size > 0.5);
        // capture undefended enemy provinces
        let occ = self.occupancy();
        for i in 0..self.armies.len() {
            let o = self.armies[i].owner;
            let pv = self.armies[i].prov;
            let po = self.provs[pv].owner;
            if self.provs[pv].sea || !at_war(o, po) {
                continue;
            }
            let contested = occ[&pv].iter().any(|&j| at_war(o, self.armies[j].owner));
            if !contested {
                self.provs[pv].owner = o;
                self.changed.push(pv);
                self.dirty = true;
            }
        }
        // reinforce at home
        for a in self.armies.iter_mut() {
            if self.provs[a.prov].owner == a.owner && a.size < 30.0 {
                a.size += 0.1;
            }
        }
        // merge idle friendly stacks in the same province
        let occ = self.occupancy();
        let mut remove = vec![false; self.armies.len()];
        for v in occ.values() {
            for x in 0..v.len() {
                let i = v[x];
                if remove[i] || !self.armies[i].path.is_empty() {
                    continue;
                }
                for y in (x + 1)..v.len() {
                    let j = v[y];
                    if remove[j] || !self.armies[j].path.is_empty() || self.armies[i].owner != self.armies[j].owner {
                        continue;
                    }
                    let s = self.armies[j].size;
                    self.armies[i].size += s;
                    if self.sel_army == Some(self.armies[j].id) {
                        self.sel_army = Some(self.armies[i].id);
                    }
                    remove[j] = true;
                }
            }
        }
        let mut idx = 0;
        self.armies.retain(|_| {
            let keep = !remove[idx];
            idx += 1;
            keep
        });
        if let Some(id) = self.sel_army {
            if !self.armies.iter().any(|a| a.id == id) {
                self.sel_army = None;
            }
        }
        let mine = land[0];
        let enemy: usize = (0..NC).filter(|&c| at_war(0, c)).map(|c| land[c]).sum();
        if mine == 0 {
            self.result = Some(false);
        } else if enemy == 0 {
            self.result = Some(true);
        }
    }
}

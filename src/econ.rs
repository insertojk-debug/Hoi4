use crate::game::*;
use crate::world::*;

pub struct Item {
    pub title: String,
    pub line2: String,
    pub line3: String,
    pub ok: bool,
}

// what the menu for a tile shows: a header line and 0-3 buttons
pub struct MenuInfo {
    pub header: String,
    pub items: Vec<Item>,
    pub train: bool, // true: the buttons train soldiers, false: they build factories
}

fn secs(ticks: u32, speed: f32) -> String {
    format!("{:.0}s", ticks as f32 * 0.2 / speed)
}

impl Game {
    // building and training both need you to still hold your capital
    pub fn has_capital(&self, o: usize) -> bool {
        let c = self.roy.players[o].capital;
        c != usize::MAX && self.provs[c].owner == o
    }

    pub fn soldiers_at(&self, p: usize, owner: usize) -> usize {
        self.armies.iter().filter(|a| a.prov == p && a.owner == owner).count()
    }

    pub fn start_build(&mut self, owner: usize, p: usize, kind: usize) -> Result<(), &'static str> {
        if !self.has_capital(owner) {
            return Err("Capital lost - you can't build");
        }
        if self.provs[p].owner != owner || self.provs[p].sea {
            return Err("Not your land");
        }
        if self.bld[p] != 0 {
            return Err("Something is already built here");
        }
        if self.roy.players[owner].stars < BUILD_COST[kind] {
            return Err("Not enough stars");
        }
        self.roy.players[owner].stars -= BUILD_COST[kind];
        self.bld[p] = kind as u8 + 1;
        self.bld_done[p] = self.roy.tick + BUILD_TIME[kind];
        match kind {
            0 => self.roy.players[owner].civ += 1,
            _ => self.roy.players[owner].mil += 1,
        }
        Ok(())
    }

    // train a soldier in a finished military factory
    pub fn recruit(&mut self, owner: usize, p: usize, kind: usize) -> Result<(), &'static str> {
        if !self.has_capital(owner) {
            return Err("Capital lost - you can't train");
        }
        if self.provs[p].owner != owner || self.bld[p] != 2 || self.bld_done[p] > self.roy.tick {
            return Err("Needs a finished military factory");
        }
        if self.soldiers_at(p, owner) >= MAX_STACK {
            return Err("Tile is full (3 soldiers)");
        }
        if self.roy.players[owner].stars < STAR_COST[kind] {
            return Err("Not enough stars");
        }
        self.roy.players[owner].stars -= STAR_COST[kind];
        self.spawn(owner, p, 10.0, kind);
        Ok(())
    }

    pub fn menu_content(&self, p: usize) -> MenuInfo {
        let speed = SPEEDS[self.speed_i];
        let stars = self.roy.players[0].stars;
        let cap = if self.roy.players[0].capital == p { "CAPITAL - " } else { "" };
        let mk = |header: String| MenuInfo { header, items: vec![], train: false };
        if !self.has_capital(0) {
            return mk("Capital lost - you can't build or train".to_string());
        }
        match self.bld[p] {
            0 => MenuInfo {
                header: format!("{}Build here ({:.0} stars)", cap, stars),
                items: (0..2)
                    .map(|k| Item {
                        title: BUILD_NAMES[k].to_string(),
                        line2: format!("{} stars", BUILD_COST[k] as i32),
                        line3: if k == 0 { format!("+{} stars / {}", INCOME as i32, secs(CYCLE, speed)) } else { "trains soldiers".to_string() },
                        ok: stars >= BUILD_COST[k],
                    })
                    .collect(),
                train: false,
            },
            b => {
                let name = BUILD_NAMES[b as usize - 1];
                if self.bld_done[p] > self.roy.tick {
                    return mk(format!("{}{}: ready in {}", cap, name, secs(self.bld_done[p] - self.roy.tick, speed)));
                }
                if b == 1 {
                    return mk(format!("{}{}: +{} stars every {}", cap, name, INCOME as i32, secs(CYCLE, speed)));
                }
                if self.soldiers_at(p, 0) >= MAX_STACK {
                    return mk(format!("{}{}: tile is full (3 soldiers max)", cap, name));
                }
                MenuInfo {
                    header: format!("{}{} - train ({:.0} stars)", cap, name, stars),
                    items: (0..3)
                        .map(|k| Item {
                            title: KIND_NAMES[k].to_string(),
                            line2: format!("{} stars", STAR_COST[k] as i32),
                            line3: ["balanced", "strong attack", "fast + tough"][k].to_string(),
                            ok: stars >= STAR_COST[k],
                        })
                        .collect(),
                    train: true,
                }
            }
        }
    }

    pub fn menu_pick(&mut self, p: usize, idx: usize) -> Result<String, &'static str> {
        if self.menu_content(p).train {
            self.recruit(0, p, idx)?;
            Ok(format!("{} trained", KIND_NAMES[idx]))
        } else {
            self.start_build(0, p, idx)?;
            Ok(format!("{} started", BUILD_NAMES[idx]))
        }
    }
}

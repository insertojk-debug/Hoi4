use crate::diplo::{allied, at_war, break_alliance, declare_war, make_peace, propose_alliance, propose_peace};
use crate::game::*;
use crate::royale::PLAYERS;
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
    pub action: u8, // what the buttons do: 0 build, 1 train soldiers, 2 train ships, 3 diplomacy
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
        if kind == 2 && !self.provs[p].coast {
            return Err("A dockyard needs a coastline tile");
        }
        if self.roy.players[owner].stars < BUILD_COST[kind] {
            return Err("Not enough stars");
        }
        self.roy.players[owner].stars -= BUILD_COST[kind];
        self.bld[p] = kind as u8 + 1;
        self.bld_done[p] = self.roy.tick + BUILD_TIME[kind];
        match kind {
            0 => self.roy.players[owner].civ += 1,
            1 => self.roy.players[owner].mil += 1,
            _ => {}
        }
        Ok(())
    }

    // train a soldier in a finished military factory, or a ship in a finished dockyard
    pub fn recruit(&mut self, owner: usize, p: usize, kind: usize) -> Result<(), &'static str> {
        if !self.has_capital(owner) {
            return Err("Capital lost - you can't train");
        }
        let ship = is_naval(kind);
        let need = if ship { 3 } else { 2 };
        if self.provs[p].owner != owner || self.bld[p] != need || self.bld_done[p] > self.roy.tick {
            return Err(if ship { "Needs a finished dockyard" } else { "Needs a finished military factory" });
        }
        // soldiers appear on the factory tile, ships in a free water tile next to the dockyard
        let at = if ship {
            match self.provs[p].nadj.iter().copied().find(|&s| self.soldiers_at(s, owner) < MAX_STACK) {
                Some(s) => s,
                None => return Err("No free water next to the dockyard"),
            }
        } else {
            if self.soldiers_at(p, owner) >= MAX_STACK {
                return Err("Tile is full (3 soldiers)");
            }
            p
        };
        if self.roy.players[owner].stars < STAR_COST[kind] {
            return Err("Not enough stars");
        }
        self.roy.players[owner].stars -= STAR_COST[kind];
        self.spawn(owner, at, HP[kind], kind);
        Ok(())
    }

    pub fn menu_content(&self, p: usize) -> MenuInfo {
        let speed = SPEEDS[self.speed_i];
        let stars = self.roy.players[0].stars;
        let cap = if self.roy.players[0].capital == p { "CAPITAL - " } else { "" };
        let mk = |header: String| MenuInfo { header, items: vec![], action: 0 };
        let other = self.provs[p].owner;
        if other < PLAYERS && other != 0 {
            // another player's land: diplomacy
            let (war, ally) = (at_war(self, 0, other), allied(self, 0, other));
            let status = if war { "AT WAR" } else if ally { "ALLIED" } else { "at peace" };
            let it = |t: &str, l2: &str, ok: bool| Item { title: t.to_string(), line2: l2.to_string(), line3: String::new(), ok };
            return MenuInfo {
                header: format!("Bot {} ({} tiles) - {}", other, self.roy.players[other].land, status),
                items: vec![
                    it("Declare War", if ally { "betray the ally" } else { "take their land" }, !war),
                    it(if ally { "Break Alliance" } else { "Propose Alliance" }, if ally { "back to peace" } else { "no fighting" }, !war),
                    it("Propose Peace", "end the war", war),
                ],
                action: 3,
            };
        }
        if !self.has_capital(0) {
            return mk("Capital lost - you can't build or train".to_string());
        }
        match self.bld[p] {
            0 => MenuInfo {
                header: format!("{}Build here ({:.0} stars)", cap, stars),
                // a dockyard can only be built on the coastline
                items: (0..if self.provs[p].coast { 3 } else { 2 })
                    .map(|k| Item {
                        title: BUILD_NAMES[k].to_string(),
                        line2: format!("{} stars", BUILD_COST[k] as i32),
                        line3: match k {
                            0 => format!("+{} stars / {}", INCOME as i32, secs(CYCLE, speed)),
                            1 => "trains soldiers".to_string(),
                            _ => "trains ships".to_string(),
                        },
                        ok: stars >= BUILD_COST[k],
                    })
                    .collect(),
                action: 0,
            },
            b => {
                let name = BUILD_NAMES[b as usize - 1];
                if self.bld_done[p] > self.roy.tick {
                    return mk(format!("{}{}: ready in {}", cap, name, secs(self.bld_done[p] - self.roy.tick, speed)));
                }
                if b == 1 {
                    return mk(format!("{}{}: +{} stars every {}", cap, name, INCOME as i32, secs(CYCLE, speed)));
                }
                if b == 3 {
                    if !self.provs[p].nadj.iter().any(|&s| self.soldiers_at(s, 0) < MAX_STACK) {
                        return mk(format!("{}{}: no free water next to it", cap, name));
                    }
                    return MenuInfo {
                        header: format!("{}{} - build ships ({:.0} stars)", cap, name, stars),
                        items: (3..6)
                            .map(|k| Item {
                                title: KIND_NAMES[k].to_string(),
                                line2: format!("{} stars", STAR_COST[k] as i32),
                                line3: ["balanced", "tough, slow", "strong attack"][k - 3].to_string(),
                                ok: stars >= STAR_COST[k],
                            })
                            .collect(),
                        action: 2,
                    };
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
                    action: 1,
                }
            }
        }
    }

    pub fn menu_pick(&mut self, p: usize, idx: usize) -> Result<String, &'static str> {
        match self.menu_content(p).action {
            0 => {
                self.start_build(0, p, idx)?;
                Ok(format!("{} started", BUILD_NAMES[idx]))
            }
            3 => {
                let d = self.provs[p].owner;
                match idx {
                    0 => {
                        if at_war(self, 0, d) {
                            return Err("Already at war");
                        }
                        declare_war(self, 0, d);
                        Ok(format!("You declared war on Bot {}", d))
                    }
                    1 => {
                        if allied(self, 0, d) {
                            break_alliance(self, 0, d);
                            return Ok(format!("Alliance with Bot {} ended", d));
                        }
                        if at_war(self, 0, d) {
                            return Err("Make peace first");
                        }
                        if propose_alliance(self, 0, d) {
                            Ok(format!("Bot {} accepted your alliance", d))
                        } else {
                            Ok(format!("Bot {} refused the alliance", d))
                        }
                    }
                    _ => {
                        if !at_war(self, 0, d) {
                            return Err("You are not at war");
                        }
                        if propose_peace(self, 0, d) {
                            Ok(format!("Bot {} accepted peace", d))
                        } else {
                            let _ = make_peace;
                            Ok(format!("Bot {} refused peace", d))
                        }
                    }
                }
            }
            a => {
                let kind = if a == 2 { idx + 3 } else { idx };
                self.recruit(0, p, kind)?;
                Ok(format!("{} trained", KIND_NAMES[kind]))
            }
        }
    }
}

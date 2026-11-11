use macroquad::prelude::*;

use crate::game::*;
use crate::royale::PLAYERS;

const WAR_START: u32 = 900; // bots declare no wars before this tick
const PLAYER_GRACE: u32 = 2500; // ...and none against you before this one

pub fn at_war(g: &Game, a: usize, b: usize) -> bool {
    a != b && a < PLAYERS && b < PLAYERS && g.roy.war_with[a].contains(&(b as u16))
}

pub fn allied(g: &Game, a: usize, b: usize) -> bool {
    a != b && a < PLAYERS && b < PLAYERS && g.roy.ally_with[a].contains(&(b as u16))
}

// soldiers only fight, and only capture land, when their players are at war
pub fn hostile(g: &Game, a: usize, b: usize) -> bool {
    at_war(g, a, b)
}

fn add(v: &mut Vec<u16>, x: usize) {
    if !v.contains(&(x as u16)) {
        v.push(x as u16);
    }
}

fn del(v: &mut Vec<u16>, x: usize) {
    v.retain(|&y| y as usize != x);
}

// how much the bots want war: high while many are alive, low as their numbers shrink
pub fn appetite(g: &Game) -> f32 {
    ((g.roy.alive as f32 - 40.0) / 460.0).clamp(0.05, 1.0)
}

pub fn declare_war(g: &mut Game, a: usize, b: usize) {
    del(&mut g.roy.ally_with[a], b); // attacking an ally breaks the alliance
    del(&mut g.roy.ally_with[b], a);
    add(&mut g.roy.war_with[a], b);
    add(&mut g.roy.war_with[b], a);
    if b == 0 {
        g.say(&format!("Bot {} declared war on you!", a));
    }
}

pub fn make_peace(g: &mut Game, a: usize, b: usize) {
    del(&mut g.roy.war_with[a], b);
    del(&mut g.roy.war_with[b], a);
}

pub fn break_alliance(g: &mut Game, a: usize, b: usize) {
    del(&mut g.roy.ally_with[a], b);
    del(&mut g.roy.ally_with[b], a);
}

// asking for an alliance: the other side decides
pub fn propose_alliance(g: &mut Game, from: usize, to: usize) -> bool {
    if at_war(g, from, to) || allied(g, from, to) {
        return false;
    }
    let mut p = 0.25 + 0.6 * (1.0 - appetite(g));
    if g.roy.war_with[from].iter().any(|e| g.roy.war_with[to].contains(e)) {
        p += 0.3; // a common enemy makes friends
    }
    let ok = rand::gen_range(0.0f32, 1.0) < p.min(0.95);
    if ok {
        add(&mut g.roy.ally_with[from], to);
        add(&mut g.roy.ally_with[to], from);
    }
    ok
}

// asking to end a war: more likely when the world is calm or when the other side is the smaller one
pub fn propose_peace(g: &mut Game, from: usize, to: usize) -> bool {
    if !at_war(g, from, to) {
        return false;
    }
    let mut p = 0.15 + 0.65 * (1.0 - appetite(g));
    if g.roy.players[to].land < g.roy.players[from].land {
        p += 0.2;
    }
    let ok = rand::gen_range(0.0f32, 1.0) < p.min(0.95);
    if ok {
        make_peace(g, from, to);
    }
    ok
}

// a player has been eliminated: drop all their wars and alliances
pub fn clear(g: &mut Game, d: usize) {
    for e in g.roy.war_with[d].clone() {
        del(&mut g.roy.war_with[e as usize], d);
    }
    for e in g.roy.ally_with[d].clone() {
        del(&mut g.roy.ally_with[e as usize], d);
    }
    g.roy.war_with[d].clear();
    g.roy.ally_with[d].clear();
}

// a random player owning land next to one of i's provinces
fn neighbour_owner(g: &Game, i: usize) -> Option<usize> {
    let m = g.roy.owned[i].len();
    if m == 0 {
        return None;
    }
    for _ in 0..8 {
        let p = g.roy.owned[i][rand::gen_range(0, m)] as usize;
        let adj = &g.provs[p].adj;
        if adj.is_empty() {
            continue;
        }
        let o = g.provs[adj[rand::gen_range(0, adj.len())]].owner;
        if o < PLAYERS && o != i {
            return Some(o);
        }
    }
    None
}

// the bots' diplomacy: eager for war while many are alive, then more and more peaceful
pub fn ai_step(g: &mut Game) {
    let tick = g.roy.tick;
    if tick < WAR_START {
        return;
    }
    let w = appetite(g);
    for i in 1..PLAYERS {
        if !g.roy.players[i].alive || (tick as usize + i * 3) % 20 != 0 {
            continue;
        }
        let aggr = g.roy.players[i].aggr;
        let wars = g.roy.war_with[i].len();
        // end a war
        if wars > 0 && rand::gen_range(0.0f32, 1.0) < (1.0 - w) * 0.2 {
            let e = g.roy.war_with[i][rand::gen_range(0, wars)] as usize;
            if propose_peace(g, i, e) && e == 0 {
                g.say(&format!("Bot {} made peace with you", i));
            }
        }
        // make a friend (bots do not ask the player)
        if rand::gen_range(0.0f32, 1.0) < (1.0 - w) * 0.1 {
            if let Some(n) = neighbour_owner(g, i) {
                if n != 0 && !at_war(g, i, n) {
                    propose_alliance(g, i, n);
                }
            }
        }
        // pick a fight with a neighbour
        let max_wars = if aggr > 0.7 { 3 } else { 2 };
        if g.roy.war_with[i].len() < max_wars && rand::gen_range(0.0f32, 1.0) < w * (0.3 + aggr) * 0.7 {
            if let Some(n) = neighbour_owner(g, i) {
                // give a human player a few minutes before bots turn on them
                let grace = n == 0 && tick < PLAYER_GRACE;
                if !at_war(g, i, n) && !allied(g, i, n) && !grace {
                    declare_war(g, i, n);
                }
            }
        }
    }
}

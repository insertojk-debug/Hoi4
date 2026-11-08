use macroquad::prelude::*;
use std::collections::HashMap;

use crate::game::*;
use crate::royale::{claim, eliminate, release};
use crate::world::*;

pub fn occupancy(g: &Game) -> HashMap<usize, Vec<usize>> {
    let mut m: HashMap<usize, Vec<usize>> = HashMap::new();
    for (i, a) in g.armies.iter().enumerate() {
        m.entry(a.prov).or_default().push(i);
    }
    m
}

// one tick for every soldier: marching, taking land, fighting each other
pub fn step(g: &mut Game) {
    // how many soldiers each player has in each tile (at most MAX_STACK fit)
    let mut cnt: HashMap<(usize, usize), usize> = HashMap::new();
    for a in &g.armies {
        *cnt.entry((a.owner, a.prov)).or_insert(0) += 1;
    }
    // marching: slow, faster for tanks, sea lanes are slower still
    let occ = occupancy(g);
    for i in 0..g.armies.len() {
        let (pv, o) = (g.armies[i].prov, g.armies[i].owner);
        let nx = match g.armies[i].path.front() {
            Some(&n) => n,
            None => continue,
        };
        let here = g.provs[pv].owner;
        let fighting = occ.get(&pv).map_or(false, |v| v.iter().any(|&j| g.armies[j].owner != o));
        // a soldier stuck in a fight may retreat to its own land or free land, but cannot push on
        let advancing = g.provs[nx].owner != o && g.provs[nx].owner != NEUTRAL;
        if advancing && ((here != o && here != NEUTRAL) || fighting) {
            continue; // still fighting for this province
        }
        let retreating = !advancing && here != o && here != NEUTRAL;
        let base = if g.provs[pv].sea_adj.contains(&nx) { 0.06 } else { 0.15 };
        g.armies[i].prog += if retreating { 1.0 } else { base * SPD[g.armies[i].kind] };
        if g.armies[i].prog >= 0.999 {
            if *cnt.get(&(o, nx)).unwrap_or(&0) >= MAX_STACK {
                g.armies[i].prog = 0.999; // the next tile is full: wait for room
                continue;
            }
            g.armies[i].prog = 0.0;
            g.armies[i].path.pop_front();
            g.armies[i].prov = nx;
            *cnt.entry((o, pv)).or_insert(1) -= 1;
            *cnt.entry((o, nx)).or_insert(0) += 1;
            if g.provs[nx].owner == NEUTRAL && !g.provs[nx].sea {
                claim(g, nx, o); // free land belongs to the first soldier to walk in
            }
        }
    }
    // soldiers of different players standing in the same tile fight
    let occ = occupancy(g);
    let mut dmg = vec![0f32; g.armies.len()];
    for (p, v) in occ.iter() {
        if v.len() < 2 {
            continue;
        }
        for &i in v {
            for &j in v {
                if g.armies[i].owner != g.armies[j].owner {
                    let mut d = g.armies[i].size * 0.15 * ATK[g.armies[i].kind] * rand::gen_range(0.7f32, 1.3f32);
                    d /= DEF[g.armies[j].kind];
                    if g.armies[j].owner == g.provs[*p].owner {
                        d *= 0.75; // defender bonus
                    }
                    dmg[j] += d;
                }
            }
        }
    }
    for i in 0..g.armies.len() {
        g.armies[i].size -= dmg[i];
    }
    g.armies.retain(|a| a.size >= 1.0);
    // a soldier standing on enemy land with no enemy soldiers around simply takes it
    let occ = occupancy(g);
    let mut dead: Vec<usize> = Vec::new();
    for i in 0..g.armies.len() {
        let (p, o) = (g.armies[i].prov, g.armies[i].owner);
        let d = g.provs[p].owner;
        if d == o || d == NEUTRAL || d == DEAD || g.provs[p].sea {
            continue;
        }
        if occ[&p].iter().any(|&j| g.armies[j].owner != o) {
            continue; // enemy soldiers are still here: keep fighting
        }
        release(g, p, d);
        claim(g, p, o);
        if g.roy.players[d].land == 0 {
            dead.push(d); // removed after the loop so the army list stays intact
        }
    }
    for d in dead {
        if g.roy.players[d].alive {
            eliminate(g, d);
        }
    }
    if let Some(id) = g.sel_army {
        if !g.armies.iter().any(|a| a.id == id) {
            g.sel_army = None;
        }
    }
}

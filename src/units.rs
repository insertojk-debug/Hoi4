use macroquad::prelude::*;
use std::collections::HashMap;

use crate::game::*;
use crate::royale::{claim, eliminate, release};
use crate::world::*;

fn occupancy(g: &Game) -> HashMap<usize, Vec<usize>> {
    let mut m: HashMap<usize, Vec<usize>> = HashMap::new();
    for (i, a) in g.armies.iter().enumerate() {
        m.entry(a.prov).or_default().push(i);
    }
    m
}

// one tick for every soldier: marching, taking free land, fighting each other, wearing down enemy provinces
pub fn step(g: &mut Game) {
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
        if (here != o && here != NEUTRAL) || fighting {
            continue; // still fighting for this province
        }
        let base = if g.provs[pv].sea_adj.contains(&nx) { 0.06 } else { 0.15 };
        g.armies[i].prog += base * SPD[g.armies[i].kind];
        if g.armies[i].prog >= 0.999 {
            g.armies[i].prog = 0.0;
            g.armies[i].path.pop_front();
            g.armies[i].prov = nx;
            if g.provs[nx].owner == NEUTRAL && !g.provs[nx].sea {
                claim(g, nx, o); // free land belongs to the first soldier to walk in
            }
        }
    }
    // soldiers of different players standing in the same province fight
    let occ = occupancy(g);
    let mut dmg = vec![0f32; g.armies.len()];
    for (p, v) in occ.iter() {
        if v.len() < 2 {
            continue;
        }
        for &i in v {
            for &j in v {
                if g.armies[i].owner != g.armies[j].owner {
                    let mut d = g.armies[i].size * 0.12 * ATK[g.armies[i].kind] * rand::gen_range(0.7f32, 1.3f32);
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
    // a soldier alone in an enemy province takes it if it is strong enough compared to the troops defending each province
    let occ = occupancy(g);
    let mut dead: Vec<usize> = Vec::new();
    for i in 0..g.armies.len() {
        let (p, o) = (g.armies[i].prov, g.armies[i].owner);
        let d = g.provs[p].owner;
        if d == o || d == NEUTRAL || d == DEAD || g.provs[p].sea {
            continue;
        }
        if occ[&p].iter().any(|&j| g.armies[j].owner != o) {
            continue; // other soldiers are in the way
        }
        let k = g.armies[i].kind;
        let land = g.roy.players[d].land.max(1) as f32;
        let dens = g.roy.players[d].troops / land; // how many troops defend each of their provinces
        let power = g.armies[i].size * ATK[k];
        if power >= dens * 0.8 {
            // strong enough: the province falls, its defenders are wiped out and the winner loots some troops
            g.armies[i].size -= dens * 0.3 / DEF[k];
            g.roy.players[d].troops = (g.roy.players[d].troops - dens * 2.0).max(0.0);
            let cap = 40.0 * g.roy.players[o].land as f32 + 100.0;
            g.roy.players[o].troops = (g.roy.players[o].troops + dens).min(cap);
            release(g, p, d);
            claim(g, p, o);
            if g.roy.players[d].land == 0 {
                dead.push(d); // removed after the loop so the army list stays intact
            }
        } else {
            // repelled: the attacker bleeds every tick it stays
            g.armies[i].size -= dens * 0.25 / DEF[k];
            g.roy.players[d].troops = (g.roy.players[d].troops - power * 0.3).max(0.0);
        }
    }
    for d in dead {
        if g.roy.players[d].alive {
            eliminate(g, d);
        }
    }
    g.armies.retain(|a| a.size >= 1.0);
    // merge idle soldiers of the same player and type standing together
    let mut seen: HashMap<(usize, usize, usize), usize> = HashMap::new();
    let mut remove = vec![false; g.armies.len()];
    for i in 0..g.armies.len() {
        if !g.armies[i].path.is_empty() {
            continue;
        }
        let key = (g.armies[i].owner, g.armies[i].prov, g.armies[i].kind);
        match seen.get(&key) {
            Some(&j) => {
                let s = g.armies[i].size;
                g.armies[j].size += s;
                if g.sel_army == Some(g.armies[i].id) {
                    g.sel_army = Some(g.armies[j].id);
                }
                remove[i] = true;
            }
            None => {
                seen.insert(key, i);
            }
        }
    }
    let mut idx = 0;
    g.armies.retain(|_| {
        let keep = !remove[idx];
        idx += 1;
        keep
    });
    if let Some(id) = g.sel_army {
        if !g.armies.iter().any(|a| a.id == id) {
            g.sel_army = None;
        }
    }
}

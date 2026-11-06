use std::collections::HashMap;

use crate::game::*;
use crate::royale::{claim, eliminate, release};
use crate::world::*;

// one tick of your soldiers: marching, taking free land, fighting enemy provinces, merging
pub fn step(g: &mut Game) {
    // marching: slower than the expansion, faster for tanks, sea lanes are slow
    for i in 0..g.armies.len() {
        let pv = g.armies[i].prov;
        let nx = match g.armies[i].path.front() {
            Some(&n) => n,
            None => continue,
        };
        let here = g.provs[pv].owner;
        if here != 0 && here != NEUTRAL {
            continue; // still fighting for this province
        }
        let base = if g.provs[pv].sea_adj.contains(&nx) { 0.06 } else { 0.15 };
        g.armies[i].prog += base * SPD[g.armies[i].kind];
        if g.armies[i].prog >= 0.999 {
            g.armies[i].prog = 0.0;
            g.armies[i].path.pop_front();
            g.armies[i].prov = nx;
            if g.provs[nx].owner == NEUTRAL && !g.provs[nx].sea {
                claim(g, nx, 0); // free land is yours the moment a soldier walks in
            }
        }
    }
    // battles: a soldier standing in an enemy province wears down its troops until it falls
    for i in 0..g.armies.len() {
        let p = g.armies[i].prov;
        let d = g.provs[p].owner;
        if d == 0 || d == NEUTRAL {
            continue;
        }
        let k = g.armies[i].kind;
        let land = g.roy.players[d].land.max(1) as f32;
        let dens = g.roy.players[d].troops / land;
        let power = g.armies[i].size * ATK[k];
        g.armies[i].size -= dens * 0.10 / DEF[k];
        g.roy.players[d].troops = (g.roy.players[d].troops - power * 0.7).max(0.0);
        let dens2 = g.roy.players[d].troops / land;
        if g.armies[i].size >= 1.0 && dens2 <= power * 0.4 {
            g.roy.players[d].troops = (g.roy.players[d].troops - dens2).max(0.0);
            release(g, p, d);
            claim(g, p, 0);
            if g.roy.players[d].land == 0 {
                eliminate(g, d);
            }
        }
    }
    g.armies.retain(|a| a.size >= 1.0);
    // merge idle soldiers of the same type standing together
    let mut seen: HashMap<(usize, usize), usize> = HashMap::new();
    let mut remove = vec![false; g.armies.len()];
    for i in 0..g.armies.len() {
        if !g.armies[i].path.is_empty() {
            continue;
        }
        let key = (g.armies[i].prov, g.armies[i].kind);
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
    // how hard your soldiers are to dislodge
    g.garrison.clear();
    for a in &g.armies {
        *g.garrison.entry(a.prov).or_insert(0.0) += a.size * DEF[a.kind];
    }
}

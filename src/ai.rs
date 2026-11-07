use macroquad::prelude::*;
use crate::game::*;
use crate::world::*;

const PEACE: u32 = 800; // ticks before bots start building armies
const RESERVE: f32 = 60.0; // troops a bot keeps at home

// bots: train soldiers at their borders and send them at neighbours
pub fn step(g: &mut Game) {
    let tick = g.roy.tick;
    if tick < PEACE {
        return;
    }
    let n = g.roy.players.len();
    let mut count = vec![0u32; n];
    for a in &g.armies {
        count[a.owner] += 1;
    }
    // training
    for i in 1..n {
        if !g.roy.players[i].alive || (tick as usize + i) % 8 != 0 {
            continue;
        }
        let (land, aggr) = (g.roy.players[i].land, g.roy.players[i].aggr);
        if count[i] >= 3 + land / 6 || rand::gen_range(0.0f32, 1.0) > aggr {
            continue;
        }
        let kind = g.roy.players[i].kind;
        if g.roy.players[i].troops < TCOST[kind] + RESERVE {
            continue;
        }
        // mass the army in one border province so the stack grows big enough to win
        let is_border = |g: &Game, p: usize| {
            g.provs[p].owner == i
                && g.provs[p].adj.iter().any(|&q| {
                    let o = g.provs[q].owner;
                    o != NEUTRAL && o != DEAD && o != i
                })
        };
        let mut stage = g.roy.players[i].stage;
        if stage == usize::MAX || !is_border(g, stage) {
            stage = usize::MAX;
            let m = g.roy.owned[i].len();
            for _ in 0..8 {
                let p = g.roy.owned[i][rand::gen_range(0, m)] as usize;
                if is_border(g, p) {
                    stage = p;
                    break;
                }
            }
            g.roy.players[i].stage = stage;
        }
        if stage != usize::MAX {
            g.recruit(i, stage, kind);
            count[i] += 1;
        }
    }
    // orders: idle soldiers look at their neighbours
    for ai in 0..g.armies.len() {
        let (o, pv, id) = (g.armies[ai].owner, g.armies[ai].prov, g.armies[ai].id);
        if o == 0 || !g.armies[ai].path.is_empty() || (id + tick) % 6 != 0 {
            continue;
        }
        let power = g.armies[ai].size * ATK[g.armies[ai].kind];
        let mut free: Option<usize> = None;
        let mut weakest: Option<(usize, f32)> = None;
        for &q in &g.provs[pv].adj {
            let d = g.provs[q].owner;
            if d == NEUTRAL {
                free = Some(q);
            } else if d != o && d != DEAD {
                let dens = g.roy.players[d].troops / g.roy.players[d].land.max(1) as f32;
                if weakest.map_or(true, |w| dens < w.1) {
                    weakest = Some((q, dens));
                }
            }
        }
        let target = if free.is_some() {
            free
        } else {
            match weakest {
                Some((q, dens)) if power >= dens * 0.9 || rand::gen_range(0, 100) < 1 => Some(q),
                _ => None,
            }
        };
        if let Some(q) = target {
            g.armies[ai].path.push_back(q);
            g.armies[ai].prog = 0.0;
        }
    }
}

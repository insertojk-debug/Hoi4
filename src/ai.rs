use macroquad::prelude::*;

use crate::game::*;
use crate::units::occupancy;
use crate::world::*;

const PEACE: u32 = 1500; // ticks before bots start building armies
const RESERVE: f32 = 60.0; // troops a bot keeps back

// bots: train soldiers along their borders and send them into neighbouring land
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
    // training: a new soldier at a random border province
    for i in 1..n {
        if !g.roy.players[i].alive || (tick as usize + i) % 8 != 0 {
            continue;
        }
        let (land, aggr, kind) = (g.roy.players[i].land, g.roy.players[i].aggr, g.roy.players[i].kind);
        if count[i] >= 3 + land / 4 || rand::gen_range(0.0f32, 1.0) > aggr || g.roy.players[i].troops < TCOST[kind] + RESERVE {
            continue;
        }
        let m = g.roy.owned[i].len();
        for _ in 0..8 {
            let p = g.roy.owned[i][rand::gen_range(0, m)] as usize;
            let border = g.provs[p].adj.iter().any(|&q| {
                let o = g.provs[q].owner;
                o != NEUTRAL && o != DEAD && o != i
            });
            if border {
                g.recruit(i, p, kind);
                count[i] += 1;
                break;
            }
        }
    }
    // orders: idle soldiers look at the provinces next to them
    let occ = occupancy(g);
    let strength = |g: &Game, p: usize, owner: usize| -> f32 {
        occ.get(&p).map_or(0.0, |v| v.iter().filter(|&&j| g.armies[j].owner != owner).map(|&j| g.armies[j].size * ATK[g.armies[j].kind]).sum())
    };
    for ai in 0..g.armies.len() {
        let (o, pv, id) = (g.armies[ai].owner, g.armies[ai].prov, g.armies[ai].id);
        if o == 0 || !g.armies[ai].path.is_empty() || (id + tick) % 6 != 0 {
            continue;
        }
        let power = g.armies[ai].size * ATK[g.armies[ai].kind];
        let mut free: Option<usize> = None;
        let mut open: Option<usize> = None; // enemy land nobody is defending
        let mut fight: Option<usize> = None; // enemy land we are strong enough to fight for
        for &q in &g.provs[pv].adj {
            let d = g.provs[q].owner;
            if d == NEUTRAL {
                free = Some(q);
            } else if d != o && d != DEAD {
                let s = strength(g, q, o);
                if s == 0.0 {
                    open = Some(q);
                } else if power >= s * 1.2 {
                    fight = Some(q);
                }
            }
        }
        if let Some(q) = free.or(open).or(fight) {
            g.armies[ai].path.push_back(q);
            g.armies[ai].prog = 0.0;
        }
    }
}

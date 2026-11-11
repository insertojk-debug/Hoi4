use macroquad::prelude::*;

use crate::diplo::{appetite, hostile};
use crate::game::*;
use crate::units::occupancy;
use crate::world::*;


// bots: build factories, train soldiers, and send them into neighbouring land
pub fn step(g: &mut Game) {
    let tick = g.roy.tick;
    let n = g.roy.players.len();
    let mut count = vec![0u32; n];
    for a in &g.armies {
        count[a.owner] += 1;
    }
    for i in 1..n {
        if !g.roy.players[i].alive || (tick as usize + i) % 10 != 0 || !g.has_capital(i) {
            continue;
        }
        let (land, civ, mil, kind, aggr) = {
            let p = &g.roy.players[i];
            (p.land, p.civ, p.mil, p.kind, p.aggr)
        };
        let m = g.roy.owned[i].len();
        // economy first: more civilian factories, plus a few military ones
        let want_civ = civ < 2 + land / 5 && civ <= mil * 3 + 1;
        let want_mil = mil < 1 + land / 12;
        let build = if want_civ { Some(0) } else if want_mil { Some(1) } else { None };
        if let Some(k) = build {
            if g.roy.players[i].stars >= BUILD_COST[k] {
                for _ in 0..6 {
                    let p = g.roy.owned[i][rand::gen_range(0, m)] as usize;
                    if g.bld[p] == 0 && g.start_build(i, p, k).is_ok() {
                        break;
                    }
                }
            }
        }
        // then soldiers, at a random finished military factory
        let at_war = !g.roy.war_with[i].is_empty();
        let wants_army = at_war || rand::gen_range(0.0f32, 1.0) < appetite(g) * 0.3;
        if wants_army && count[i] < 3 + land / 3 && rand::gen_range(0.0f32, 1.0) < aggr && g.roy.players[i].stars >= STAR_COST[kind] {
            for _ in 0..10 {
                let p = g.roy.owned[i][rand::gen_range(0, m)] as usize;
                if g.bld[p] == 2 && g.recruit(i, p, kind).is_ok() {
                    count[i] += 1;
                    break;
                }
            }
        }
    }
    // orders: idle soldiers look at the provinces next to them
    let occ = occupancy(g);
    let strength = |g: &Game, p: usize, owner: usize| -> f32 {
        occ.get(&p).map_or(0.0, |v| v.iter().filter(|&&j| hostile(g, owner, g.armies[j].owner)).map(|&j| g.armies[j].size * ATK[g.armies[j].kind]).sum())
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
            } else if d != o && hostile(g, o, d) {
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

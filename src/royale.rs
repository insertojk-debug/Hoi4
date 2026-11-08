use macroquad::prelude::*;

use crate::game::*;
use crate::world::*;

pub const PLAYERS: usize = 501; // you + 500 bots
const STORM_START: u32 = 4000; // tick when the safe zone starts to shrink
const STORM_END: u32 = 12000; // tick when it has shrunk to a single province
const EXPAND_P: i32 = 6; // % chance per action that nearby free land is claimed (slow natural spread)

pub struct Player {
    pub stars: f32,
    pub land: u32,
    pub alive: bool,
    pub aggr: f32,
    pub color: Color,
    pub kind: usize,  // the soldier type a bot likes to build
    pub capital: usize, // the first tile: holding it lets you build and train
    pub civ: u32,     // civilian factories (finished or being built)
    pub mil: u32,     // military factories
}

pub struct Royale {
    pub players: Vec<Player>,
    pub owned: Vec<Vec<u32>>, // provinces per player
    pub slot: Vec<u32>,       // position of each province inside its owner's list
    pub started: bool,
    pub over: Option<bool>,
    pub focus: Option<usize>,
    pub alive: usize,
    pub tick: u32,
    pub top: Vec<(usize, u32)>,
    pub rank: usize,
    pub place: usize,
    pub total_land: u32,
    pub center: Vec2, // middle of the safe zone (map tiles)
    pub r0: f32,
    pub zone_r: f32,
    pub land_list: Vec<u32>,
}

impl Royale {
    pub fn new() -> Royale {
        Royale {
            players: vec![],
            owned: vec![],
            slot: vec![],
            started: false,
            over: None,
            focus: None,
            alive: 0,
            tick: 0,
            top: vec![],
            rank: 0,
            place: 0,
            total_land: 1,
            center: vec2(0.0, 0.0),
            r0: 1.0,
            zone_r: 1.0,
            land_list: vec![],
        }
    }
}

fn hsv(h: f32, s: f32, v: f32) -> Color {
    let i = (h * 6.0).floor();
    let f = h * 6.0 - i;
    let (p, q, t) = (v * (1.0 - s), v * (1.0 - f * s), v * (1.0 - (1.0 - f) * s));
    let (r, g, b) = match (i as i32) % 6 {
        0 => (v, t, p),
        1 => (q, v, p),
        2 => (p, v, t),
        3 => (p, q, v),
        4 => (t, p, v),
        _ => (v, p, q),
    };
    Color::new(r, g, b, 1.0)
}

pub fn owner_color(g: &Game, owner: usize) -> Color {
    if owner == DEAD {
        Color::new(0.16, 0.05, 0.08, 1.0)
    } else if owner == NEUTRAL || owner >= g.roy.players.len() {
        Color::new(0.30, 0.31, 0.34, 1.0)
    } else {
        g.roy.players[owner].color
    }
}

pub fn claim(g: &mut Game, p: usize, i: usize) {
    g.provs[p].owner = i;
    g.roy.slot[p] = g.roy.owned[i].len() as u32;
    g.roy.owned[i].push(p as u32);
    g.roy.players[i].land += 1;
    match g.bld[p] {
        1 => g.roy.players[i].civ += 1,
        2 => g.roy.players[i].mil += 1,
        _ => {}
    }
    g.changed.push(p);
    g.dirty = true;
}

pub fn release(g: &mut Game, p: usize, d: usize) {
    let s = g.roy.slot[p] as usize;
    g.roy.owned[d].swap_remove(s);
    if s < g.roy.owned[d].len() {
        let moved = g.roy.owned[d][s] as usize;
        g.roy.slot[moved] = s as u32;
    }
    g.roy.players[d].land -= 1;
    match g.bld[p] {
        1 => g.roy.players[d].civ = g.roy.players[d].civ.saturating_sub(1),
        2 => g.roy.players[d].mil = g.roy.players[d].mil.saturating_sub(1),
        _ => {}
    }
}

// start a fresh Territory Royale on the existing world map: all land is free, 500 bots pick spawn points
pub fn init(g: &mut Game) {
    g.armies.clear();
    for b in g.bld.iter_mut() {
        *b = 0;
    }
    for b in g.bld_done.iter_mut() {
        *b = 0;
    }
    g.sel_army = None;
    g.menu = None;
    g.paused = false;
    g.speed_i = 1;
    g.acc = 0.0;
    let mut land: Vec<usize> = vec![];
    for (i, p) in g.provs.iter_mut().enumerate() {
        p.owner = NEUTRAL;
        if !p.sea {
            land.push(i);
        }
    }
    let mut r = Royale::new();
    r.total_land = land.len() as u32;
    r.land_list = land.iter().map(|&p| p as u32).collect();
    let c = land[rand::gen_range(0, land.len())];
    r.center = g.provs[c].pos;
    r.r0 = land.iter().map(|&p| g.provs[p].pos.distance(r.center)).fold(1.0, f32::max);
    r.zone_r = r.r0;
    r.slot = vec![0; g.provs.len()];
    r.owned = vec![vec![]; PLAYERS];
    for i in 0..PLAYERS {
        let color = if i == 0 {
            Color::new(1.0, 0.95, 0.2, 1.0)
        } else {
            let mut h = (i as f32 * 0.618034) % 1.0;
            if h > 0.10 && h < 0.20 {
                h += 0.12; // keep yellow for the player
            }
            hsv(h, 0.45 + 0.45 * rand::gen_range(0.0f32, 1.0), 0.55 + 0.35 * rand::gen_range(0.0f32, 1.0))
        };
        let roll = rand::gen_range(0, 100);
        let kind = if roll < 60 { 0 } else if roll < 85 { 1 } else { 2 };
        r.players.push(Player { stars: START_STARS, land: 0, alive: i != 0, aggr: rand::gen_range(0.2f32, 1.0), color, kind, capital: usize::MAX, civ: 0, mil: 0 });
    }
    r.alive = PLAYERS - 1;
    g.roy = r;
    // shuffle the land and give the first 500 provinces to the bots
    for k in (1..land.len()).rev() {
        let j = rand::gen_range(0, k + 1);
        land.swap(k, j);
    }
    for b in 0..(PLAYERS - 1) {
        claim(g, land[b], b + 1);
        g.roy.players[b + 1].capital = land[b];
    }
    stats(g);
    g.recolor_all();
}

fn stats(g: &mut Game) {
    let mut v: Vec<(usize, u32)> =
        g.roy.players.iter().enumerate().filter(|(_, p)| p.alive).map(|(i, p)| (i, p.land)).collect();
    v.sort_by(|a, b| b.1.cmp(&a.1));
    g.roy.rank = v.iter().position(|x| x.0 == 0).map(|r| r + 1).unwrap_or(0);
    v.truncate(5);
    g.roy.top = v;
}

// tapping: the first tap chooses your spawn; later taps pick soldiers, train them, or steer your expansion
pub fn tap(g: &mut Game, p: usize) -> Option<usize> {
    if g.roy.over.is_some() {
        return None;
    }
    if !g.roy.started {
        if g.provs[p].sea || g.provs[p].owner != NEUTRAL {
            g.say("Pick free land");
            return None;
        }
        claim(g, p, 0);
        g.roy.players[0].capital = p;
        g.roy.players[0].alive = true;
        g.roy.alive += 1;
        g.roy.started = true;
        stats(g);
        g.flush();
        return Some(p);
    }
    g.menu = None;
    if g.tap_map(p) {
        return None; // a selected soldier was sent there
    }
    if g.provs[p].owner == 0 && !g.provs[p].sea {
        g.menu = Some(p); // your own land: build or train here
        return None;
    }
    {
        // anything else: steer where your territory grows and show what is there
        g.roy.focus = if g.roy.focus == Some(p) { None } else { Some(p) };
        let d = g.provs[p].owner;
        if d < PLAYERS {
            let foe: f32 = g.armies.iter().filter(|a| a.prov == p && a.owner == d).map(|a| a.size * ATK[a.kind]).sum();
            let msg = if foe > 0.0 { format!("Bot {}: defended, strength {:.0}", d, foe) } else { format!("Bot {}: undefended - send a soldier!", d) };
            g.say(&msg);
        }
    }
    None
}

pub fn eliminate(g: &mut Game, d: usize) {
    g.roy.players[d].alive = false;
    g.roy.alive -= 1;
    g.armies.retain(|a| a.owner != d);
    if d == 0 {
        g.roy.place = g.roy.alive + 1;
        g.roy.over = Some(false);
    }
}

// natural spread: slowly, only onto free land next to your own (taking land from others needs soldiers)
fn act(g: &mut Game, i: usize) {
    let n = g.roy.owned[i].len();
    if n == 0 || rand::gen_range(0, 100) >= EXPAND_P {
        return;
    }
    let focus = if i == 0 { g.roy.focus } else { None };
    let tries = if focus.is_some() { 10 } else { 5 };
    let mut cands: Vec<(usize, bool)> = Vec::new();
    for _ in 0..tries {
        let from = g.roy.owned[i][rand::gen_range(0, n)] as usize;
        for &q in &g.provs[from].adj {
            if g.provs[q].owner == NEUTRAL {
                cands.push((q, false));
            }
        }
        if cands.is_empty() && rand::gen_range(0, 100) < 6 {
            for &q in &g.provs[from].sea_adj {
                if !g.provs[q].sea && g.provs[q].owner == NEUTRAL {
                    cands.push((q, true));
                }
            }
        }
    }
    if cands.is_empty() {
        return;
    }
    let (q, link) = match focus {
        Some(f) => {
            let fp = g.provs[f].pos;
            *cands
                .iter()
                .min_by(|a, b| {
                    let (da, db) = (g.provs[a.0].pos.distance(fp), g.provs[b.0].pos.distance(fp));
                    da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
                })
                .unwrap()
        }
        None => cands[rand::gen_range(0, cands.len())],
    };
    let _ = link;
    claim(g, q, i);
}

// the safe zone shrinks around its centre; land outside it is lost for good
fn storm(g: &mut Game) {
    let t = g.roy.tick;
    if t < STORM_START || t % 10 != 0 {
        return;
    }
    let frac = ((t - STORM_START) as f32 / (STORM_END - STORM_START) as f32).min(1.0);
    g.roy.zone_r = (g.roy.r0 * (1.0 - frac)).max(CELL * 0.5);
    let (c, zr) = (g.roy.center, g.roy.zone_r);
    let mut hit = false;
    let mut victims: Vec<usize> = Vec::new();
    for k in 0..g.roy.land_list.len() {
        let p = g.roy.land_list[k] as usize;
        let o = g.provs[p].owner;
        if o == DEAD || g.provs[p].pos.distance(c) <= zr {
            continue;
        }
        if o != NEUTRAL {
            release(g, p, o);
            if g.roy.players[o].land == 0 && g.roy.players[o].alive {
                victims.push(o);
            }
        }
        g.provs[p].owner = DEAD;
        g.changed.push(p);
        hit = true;
    }
    if hit {
        g.dirty = true;
        let provs = &g.provs;
        g.armies.retain(|a| provs[a.prov].owner != DEAD);
        for d in victims {
            eliminate(g, d);
        }
    }
}

pub fn step(g: &mut Game) {
    g.roy.tick += 1;
    let n = g.roy.players.len();
    // income: every civilian factory pays its owner stars each cycle
    if g.roy.tick % CYCLE == 0 {
        let t = g.roy.tick;
        for k in 0..g.roy.land_list.len() {
            let p = g.roy.land_list[k] as usize;
            if g.bld[p] == 1 && g.bld_done[p] <= t {
                let o = g.provs[p].owner;
                if o < PLAYERS && g.roy.players[o].alive {
                    g.roy.players[o].stars += INCOME;
                }
            }
        }
    }
    let start = (g.roy.tick as usize * 7) % n;
    for k in 0..n {
        let i = (start + k) % n;
        if !g.roy.players[i].alive {
            continue;
        }
        let acts = 1 + (g.roy.players[i].land / 20).min(2);
        for _ in 0..acts {
            act(g, i);
        }
    }
    crate::ai::step(g);
    crate::units::step(g);
    storm(g);
    if g.roy.tick % 5 == 0 {
        stats(g);
    }
    if g.roy.over.is_none() && g.roy.alive <= 1 && g.roy.players[0].alive {
        g.roy.over = Some(true);
    }
}

pub fn draw(g: &Game, fs: f32, top: f32, sw: f32, sh: f32) {
    let r = &g.roy;
    draw_rectangle(0.0, 0.0, sw, top, Color::new(0.1, 0.1, 0.14, 1.0));
    let txt = if !r.started {
        "Tap free land to choose where you spawn".to_string()
    } else {
        let me = &r.players[0];
        format!(
            "Alive {}/{}   Land {} ({:.1}%)   Stars {:.0}   Soldiers {}   Rank {}   Zone {:.0}%",
            r.alive,
            PLAYERS,
            me.land,
            me.land as f32 / r.total_land as f32 * 100.0,
            me.stars,
            g.armies.iter().filter(|a| a.owner == 0).count(),
            r.rank,
            r.zone_r / r.r0 * 100.0
        )
    };
    draw_text(&txt, fs * 0.6, top * 0.68, fs, WHITE);
    if r.started {
        let (w, rh) = (fs * 7.5, fs * 1.2);
        let x = sw - w - 8.0;
        draw_rectangle(x, top + 8.0, w, rh * r.top.len() as f32 + 10.0, Color::new(0.05, 0.06, 0.1, 0.75));
        for (k, &(i, land)) in r.top.iter().enumerate() {
            let y = top + 14.0 + k as f32 * rh;
            draw_rectangle(x + 8.0, y + 2.0, rh * 0.7, rh * 0.7, r.players[i].color);
            let who = if i == 0 { "YOU".to_string() } else { format!("Bot {}", i) };
            draw_text(&format!("{} {}  {}", k + 1, who, land), x + 8.0 + rh, y + rh * 0.7, fs * 0.8, WHITE);
        }
    }
    if let Some(win) = r.over {
        draw_rectangle(0.0, 0.0, sw, sh, Color::new(0.0, 0.0, 0.0, 0.7));
        let big = if win { "VICTORY".to_string() } else { "ELIMINATED".to_string() };
        let (dx, dy) = (sw / 2.0, sh / 2.0);
        let d = measure_text(&big, None, (fs * 3.0) as u16, 1.0);
        draw_text(&big, dx - d.width / 2.0, dy - fs, fs * 3.0, WHITE);
        let sub = if win { "Last one standing!".to_string() } else { format!("You placed #{} of {}", r.place, PLAYERS) };
        let d = measure_text(&sub, None, (fs * 1.2) as u16, 1.0);
        draw_text(&sub, dx - d.width / 2.0, dy + fs, fs * 1.2, YELLOW);
        let d = measure_text("Tap for menu", None, fs as u16, 1.0);
        draw_text("Tap for menu", dx - d.width / 2.0, dy + fs * 2.6, fs, GRAY);
    }
}

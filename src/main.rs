use macroquad::prelude::*;

mod coast;
mod game;
mod gen;
mod royale;
mod sim;
mod world;

use game::*;
use world::*;

const SPEEDS: [f32; 4] = [1.0, 2.0, 4.0, 8.0];

struct Lay {
    fs: f32,
    top: f32,
    bar: f32,
    base: f32,
    avail_h: f32,
}

fn layout() -> Lay {
    let sw = screen_width();
    let sh = screen_height();
    let fs = (sh / 24.0).clamp(14.0, 48.0);
    let top = fs * 1.8;
    let bar = fs * 3.2;
    let avail_h = (sh - top - bar).max(1.0);
    let base = (sw / MW as f32).min(avail_h / MH as f32);
    Lay { fs, top, bar, base, avail_h }
}

fn ctext(t: &str, cx: f32, cy: f32, fs: f32, col: Color) {
    let d = measure_text(t, None, fs as u16, 1.0);
    draw_text(t, cx - d.width / 2.0, cy + d.height / 2.0, fs, col);
}

fn date(day: u32) -> String {
    format!("{}-{:02}-{:02}", 1936 + day / 360, (day % 360) / 30 + 1, day % 30 + 1)
}

// bottom bar buttons. World War: PLAY, SPEED, ZOOM+, ZOOM-.  Royale: PLAY, SPEED, ATTACK, ZOOM+, ZOOM-
const BW: [f32; 4] = [1.4, 1.4, 0.8, 0.8];
const BWR: [f32; 5] = [1.2, 1.2, 1.9, 0.7, 0.7];

// the train menu: panel and three unit buttons around a province on screen
fn menu_layout(l: &Lay, sw: f32, sh: f32, anchor: Vec2) -> [Rect; 3] {
    let bw = (l.fs * 5.2).min((sw - 40.0) / 3.2);
    let bh = l.fs * 3.3;
    let gap = l.fs * 0.35;
    let total = bw * 3.0 + gap * 2.0;
    let x0 = (anchor.x - total / 2.0).clamp(gap + 4.0, (sw - total - gap - 4.0).max(gap + 4.0));
    let mut y0 = anchor.y - bh - l.fs * 1.6;
    if y0 < l.top + gap + 4.0 {
        y0 = anchor.y + l.fs * 1.6;
    }
    y0 = y0.clamp(l.top + gap + 4.0, sh - l.bar - bh - gap - 4.0);
    [0, 1, 2].map(|k| Rect::new(x0 + k as f32 * (bw + gap), y0, bw, bh))
}

fn in_menu(r: &[Rect; 3], gap: f32, m: Vec2) -> bool {
    let (x0, y0) = (r[0].x - gap, r[0].y - gap);
    let (x1, y1) = (r[2].x + r[2].w + gap, r[0].y + r[0].h + gap);
    m.x >= x0 && m.x <= x1 && m.y >= y0 && m.y <= y1
}

fn button_x(sw: f32, i: usize, mode: u8) -> (f32, f32) {
    let bw: &[f32] = if mode == 1 { &BWR } else { &BW };
    let total: f32 = bw.iter().sum();
    let x: f32 = bw[..i].iter().sum::<f32>() / total * sw;
    (x, bw[i] / total * sw)
}

fn conf() -> Conf {
    Conf {
        window_title: "HOI Clone".to_string(),
        window_width: 1280,
        window_height: 720,
        ..Default::default()
    }
}

#[macroquad::main(conf)]
async fn main() {
    let mut g = Game::new();
    let mut screen = 0u8; // 0 = title, 1 = playing, 2 = loading
    let mut start_mode = 0u8;
    let mut cam = vec2(MW as f32 / 2.0, MH as f32 / 2.0);
    let mut zoom = 1.0f32;
    let mut press: Option<(Vec2, Vec2)> = None;
    let mut moved = false;
    let mut pinch_prev: Option<f32> = None;
    loop {
        let l = layout();
        let sw = screen_width();
        let sh = screen_height();
        let dt = get_frame_time();
        let mid_y = l.top + l.avail_h / 2.0;
        let (mx, my) = mouse_position();

        // ---- title and loading screens ----
        if screen != 1 {
            clear_background(Color::new(0.05, 0.05, 0.08, 1.0));
            let sc = (sw / MW as f32).min(sh / MH as f32);
            draw_texture_ex(
                &g.tex,
                sw / 2.0 - MW as f32 * sc / 2.0,
                sh / 2.0 - MH as f32 * sc / 2.0,
                Color::new(0.35, 0.35, 0.4, 1.0),
                DrawTextureParams { dest_size: Some(vec2(MW as f32 * sc, MH as f32 * sc)), ..Default::default() },
            );
            if screen == 2 {
                ctext("Loading...", sw / 2.0, sh / 2.0, l.fs * 1.6, WHITE);
                next_frame().await;
                if start_mode == 1 {
                    if g.cols != royale::ROY_COLS {
                        g = Game::new_grid(royale::ROY_COLS, royale::ROY_ROWS);
                    }
                    royale::init(&mut g);
                    cam = vec2(MW as f32 / 2.0, MH as f32 / 2.0);
                    zoom = 1.0;
                } else {
                    if !g.fresh || g.cols != COLS {
                        g = Game::new();
                    }
                    g.fresh = false;
                    g.mode = 0;
                    cam = g.provs[g.armies[0].prov].pos;
                    zoom = 12.0;
                }
                screen = 1;
                continue;
            }
            ctext("WORLD CONQUEST", sw / 2.0, sh * 0.18, l.fs * 2.6, WHITE);
            ctext("Choose a game mode", sw / 2.0, sh * 0.29, l.fs, GRAY);
            let (bw, bh) = ((sw * 0.8).min(l.fs * 24.0), l.fs * 4.2);
            let titles = [
                ("TERRITORY ROYALE", "Spawn, expand and outlast 500 bots"),
                ("WORLD WAR", "Command Infantry, Artillery and Tanks"),
            ];
            for (k, (name, sub)) in titles.iter().enumerate() {
                let (x, y) = (sw / 2.0 - bw / 2.0, sh * (0.42 + 0.22 * k as f32));
                draw_rectangle(x, y, bw, bh, Color::new(0.2, 0.28, 0.45, 0.95));
                draw_rectangle_lines(x, y, bw, bh, 3.0, Color::new(0.75, 0.8, 0.95, 1.0));
                ctext(name, sw / 2.0, y + bh * 0.36, l.fs * 1.5, WHITE);
                ctext(sub, sw / 2.0, y + bh * 0.72, l.fs * 0.8, Color::new(0.8, 0.85, 0.6, 1.0));
                if is_mouse_button_pressed(MouseButton::Left) && mx >= x && mx <= x + bw && my >= y && my <= y + bh {
                    start_mode = if k == 0 { 1 } else { 0 };
                    screen = 2;
                }
            }
            next_frame().await;
            continue;
        }

        // ---- input ----
        let (_, wy) = mouse_wheel();
        if wy != 0.0 {
            zoom = (zoom * (1.0 + 0.1 * wy.signum())).clamp(1.0, 24.0);
        }
        let ts = touches();
        if ts.len() >= 2 {
            let d = ts[0].position.distance(ts[1].position);
            if let Some(pd) = pinch_prev {
                if pd > 1.0 {
                    zoom = (zoom * d / pd).clamp(1.0, 24.0);
                }
            }
            pinch_prev = Some(d);
            press = None;
        } else {
            pinch_prev = None;
            if is_mouse_button_pressed(MouseButton::Left) {
                if g.result.is_some() || (g.mode == 1 && g.roy.over.is_some()) {
                    screen = 0;
                } else if g.menu.is_some() && {
                    let p = g.menu.unwrap();
                    let sc = l.base * zoom;
                    let anchor = vec2(sw / 2.0 + (g.provs[p].pos.x - cam.x) * sc, mid_y + (g.provs[p].pos.y - cam.y) * sc);
                    let rects = menu_layout(&l, sw, sh, anchor);
                    in_menu(&rects, l.fs * 0.35, vec2(mx, my))
                } {
                    // tap inside the train menu
                    let p = g.menu.unwrap();
                    let sc = l.base * zoom;
                    let anchor = vec2(sw / 2.0 + (g.provs[p].pos.x - cam.x) * sc, mid_y + (g.provs[p].pos.y - cam.y) * sc);
                    let rects = menu_layout(&l, sw, sh, anchor);
                    for k in 0..3 {
                        if rects[k].contains(vec2(mx, my)) {
                            if g.recruit(0, p, k) {
                                g.say(&format!("{} trained", KIND_NAMES[k]));
                                g.menu = None;
                            } else {
                                g.say("Not enough industry or manpower");
                            }
                        }
                    }
                } else if my > sh - l.bar {
                    let mut bi = 0;
                    for i in 0..(if g.mode == 1 { 5 } else { 4 }) {
                        let (x, w) = button_x(sw, i, g.mode);
                        if mx >= x && mx < x + w {
                            bi = i;
                        }
                    }
                    match (g.mode, bi) {
                        (_, 0) => g.paused = !g.paused,
                        (_, 1) => g.speed_i = (g.speed_i + 1) % SPEEDS.len(),
                        (1, 2) => g.roy.attack = (g.roy.attack + 1) % ATTACK.len(),
                        (1, 3) | (0, 2) => zoom = (zoom * 1.4).clamp(1.0, 24.0),
                        _ => zoom = (zoom / 1.4).clamp(1.0, 24.0),
                    }
                } else if my > l.top {
                    press = Some((vec2(mx, my), cam));
                    moved = false;
                }
            }
            if let Some((start, cam0)) = press {
                let scale = l.base * zoom;
                if is_mouse_button_down(MouseButton::Left) {
                    let delta = vec2(mx, my) - start;
                    if delta.length() > l.fs * 0.4 {
                        moved = true;
                    }
                    if moved {
                        cam = cam0 - delta / scale;
                    }
                }
                if is_mouse_button_released(MouseButton::Left) {
                    if !moved {
                        let tx = ((mx - sw / 2.0) / scale + cam.x).floor();
                        let ty = ((my - mid_y) / scale + cam.y).floor();
                        if tx >= 0.0 && ty >= 0.0 && (tx as usize) < MW && (ty as usize) < MH {
                            let p = g.tile[ty as usize * MW + tx as usize] as usize;
                            if g.mode == 1 {
                                if let Some(sp) = royale::tap(&mut g, p) {
                                    cam = g.provs[sp].pos;
                                    zoom = 6.0;
                                }
                            } else {
                                g.tap_map(p);
                            }
                        }
                    }
                    press = None;
                }
            }
        }
        // keep camera inside the map
        let scale = l.base * zoom;
        let hw = sw / 2.0 / scale;
        let hh = l.avail_h / 2.0 / scale;
        cam.x = if hw >= MW as f32 / 2.0 { MW as f32 / 2.0 } else { cam.x.clamp(hw, MW as f32 - hw) };
        cam.y = if hh >= MH as f32 / 2.0 { MH as f32 / 2.0 } else { cam.y.clamp(hh, MH as f32 - hh) };

        // ---- simulation ----
        if g.mode == 1 {
            if g.roy.started && g.roy.over.is_none() && !g.paused {
                g.acc += dt * SPEEDS[g.speed_i];
                let mut n = 0;
                while g.acc >= 0.2 && n < 20 {
                    g.acc -= 0.2;
                    royale::step(&mut g);
                    n += 1;
                }
                if n == 20 {
                    g.acc = 0.0;
                }
            }
            // redraw the map a few times a second (many provinces change hands every tick)
            g.flush_t += dt;
            if g.dirty && g.flush_t > 0.1 {
                g.flush();
                g.flush_t = 0.0;
            }
        } else {
            if !g.paused && g.result.is_none() {
                g.acc += dt * SPEEDS[g.speed_i];
                while g.acc >= 0.5 {
                    g.acc -= 0.5;
                    g.step_day();
                }
            }
            if g.dirty {
                g.flush();
            }
        }
        if g.toast_t > 0.0 {
            g.toast_t -= dt;
        }

        // ---- draw ----
        clear_background(Color::new(0.05, 0.05, 0.08, 1.0));
        draw_texture_ex(
            &g.tex,
            sw / 2.0 - cam.x * scale,
            mid_y - cam.y * scale,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(MW as f32 * scale, MH as f32 * scale)),
                ..Default::default()
            },
        );
        let to_screen = |p: Vec2| vec2(sw / 2.0 + (p.x - cam.x) * scale, mid_y + (p.y - cam.y) * scale);

        // sea lanes
        for (a, pa) in g.provs.iter().enumerate() {
            for &b in &pa.sea_adj {
                if a < b {
                    let s = to_screen(pa.pos);
                    let e = to_screen(g.provs[b].pos);
                    draw_line(s.x, s.y, e.x, e.y, 2.0, Color::new(0.5, 0.8, 1.0, 0.55));
                }
            }
        }
        if g.mode == 1 {
            if let Some(f) = g.roy.focus {
                let c = to_screen(g.provs[f].pos);
                draw_circle_lines(c.x, c.y, l.fs * 1.2, 3.0, YELLOW);
                draw_circle(c.x, c.y, l.fs * 0.2, YELLOW);
            }
        }
        if let Some(p) = g.sel_prov {
            let c = to_screen(g.provs[p].pos);
            draw_circle_lines(c.x, c.y, l.fs * 1.4, 3.0, WHITE);
        }
        if let Some(id) = g.sel_army {
            if let Some(a) = g.armies.iter().find(|a| a.id == id) {
                let mut prev = to_screen(g.provs[a.prov].pos);
                for &q in &a.path {
                    let nx = to_screen(g.provs[q].pos);
                    draw_line(prev.x, prev.y, nx.x, nx.y, 3.0, YELLOW);
                    prev = nx;
                }
            }
        }
        let cell = CELL * scale;
        let bw = (cell * 0.95).min(l.fs * 2.2);
        let bh = bw * 0.6;
        let mut stack = vec![0u8; g.provs.len()];
        for a in g.armies.iter() {
            let k = stack[a.prov] as f32;
            stack[a.prov] = stack[a.prov].saturating_add(1);
            if k >= 3.0 {
                continue; // don't draw more than three boxes per province
            }
            let c = to_screen(g.provs[a.prov].pos);
            if bw < l.fs * 1.2 {
                // zoomed out: just a coloured marker
                let m = (bw * 0.6).max(5.0);
                draw_rectangle(c.x - m / 2.0 + k * 2.0, c.y - m / 2.0, m, m, country_color(a.owner));
                if g.sel_army == Some(a.id) {
                    draw_rectangle_lines(c.x - m / 2.0 - 2.0, c.y - m / 2.0 - 2.0, m + 4.0, m + 4.0, 3.0, YELLOW);
                }
                continue;
            }
            let x = c.x - bw / 2.0 + k * (bw + 4.0);
            let y = c.y - bh / 2.0;
            draw_rectangle(x, y, bw, bh, country_color(a.owner));
            let selected = g.sel_army == Some(a.id);
            draw_rectangle_lines(
                x,
                y,
                bw,
                bh,
                if selected { 5.0 } else { 2.0 },
                if selected { YELLOW } else { WHITE },
            );
            ctext(&format!("{}{}", KIND_LETTER[a.kind], a.size as i32), x + bw / 2.0, y + bh / 2.0, bh * 0.8, WHITE);
        }

        // train menu (like Polytopia): three unit choices next to the province
        if let Some(p) = g.menu {
            let anchor = to_screen(g.provs[p].pos);
            let rects = menu_layout(&l, sw, sh, anchor);
            let gap = l.fs * 0.35;
            let (px, py) = (rects[0].x - gap, rects[0].y - gap);
            let (pw, ph) = (rects[2].x + rects[2].w + gap - px, rects[0].h + gap * 2.0);
            draw_rectangle(px, py, pw, ph, Color::new(0.05, 0.06, 0.1, 0.92));
            draw_rectangle_lines(px, py, pw, ph, 2.0, Color::new(0.6, 0.65, 0.8, 1.0));
            for k in 0..3 {
                let r = rects[k];
                let ok = g.can_afford(0, k);
                let bg = if ok { Color::new(0.22, 0.30, 0.45, 1.0) } else { Color::new(0.20, 0.20, 0.22, 1.0) };
                let fg = if ok { WHITE } else { Color::new(0.55, 0.55, 0.58, 1.0) };
                draw_rectangle(r.x, r.y, r.w, r.h, bg);
                draw_rectangle_lines(r.x, r.y, r.w, r.h, 2.0, Color::new(0.75, 0.8, 0.95, 0.8));
                ctext(KIND_NAMES[k], r.x + r.w / 2.0, r.y + r.h * 0.3, l.fs * 0.95, fg);
                ctext(&format!("{} IC  {} MP", COST[k].0 as i32, COST[k].1 as i32), r.x + r.w / 2.0, r.y + r.h * 0.62, l.fs * 0.7, fg);
                let note = ["balanced", "strong attack", "fast + tough"][k];
                ctext(note, r.x + r.w / 2.0, r.y + r.h * 0.84, l.fs * 0.6, Color::new(0.8, 0.85, 0.6, if ok { 1.0 } else { 0.5 }));
            }
        }

        // top HUD
        if g.mode == 0 {
            draw_rectangle(0.0, 0.0, sw, l.top, Color::new(0.1, 0.1, 0.14, 1.0));
            let info = match g.sel_prov {
                Some(p) if !g.provs[p].sea => format!("Province: {}", NAMES[g.provs[p].owner]),
                Some(_) => "Province: Sea".to_string(),
                None => "Tap a province".to_string(),
            };
            draw_text(
                &format!("{}   MP {:.0}   IC {:.0}   |   {}", date(g.day), g.cs[0].mp, g.cs[0].ic, info),
                l.fs * 0.6,
                l.top * 0.68,
                l.fs,
                WHITE,
            );

        }

        // bottom bar
        draw_rectangle(0.0, sh - l.bar, sw, l.bar, Color::new(0.1, 0.1, 0.14, 1.0));
        let mut labels = vec![
            if g.paused { "> PLAY".to_string() } else { "|| PAUSE".to_string() },
            format!("SPEED x{}", SPEEDS[g.speed_i] as i32),
        ];
        if g.mode == 1 {
            let a = (ATTACK[g.roy.attack] * 100.0) as i32;
            labels.push(if a == 0 { "ATTACK OFF".to_string() } else { format!("ATTACK {}%", a) });
        }
        labels.push("+".to_string());
        labels.push("-".to_string());
        for (i, t) in labels.iter().enumerate() {
            let (x, w) = button_x(sw, i, g.mode);
            draw_rectangle(x + 3.0, sh - l.bar + 4.0, w - 6.0, l.bar - 8.0, Color::new(0.2, 0.22, 0.3, 1.0));
            ctext(t, x + w / 2.0, sh - l.bar / 2.0, l.fs * 0.85, WHITE);
        }

        if g.toast_t > 0.0 {
            ctext(&g.toast, sw / 2.0, sh - l.bar - l.fs * 1.2, l.fs * 1.1, YELLOW);
        }
        if g.mode == 1 {
            royale::draw(&g, l.fs, l.top, sw, sh);
        }
        if let Some(win) = g.result {
            draw_rectangle(0.0, 0.0, sw, sh, Color::new(0.0, 0.0, 0.0, 0.7));
            ctext(if win { "VICTORY" } else { "DEFEAT" }, sw / 2.0, sh / 2.0 - l.fs, l.fs * 3.0, WHITE);
            ctext("Tap to restart", sw / 2.0, sh / 2.0 + l.fs * 2.0, l.fs, GRAY);
        }

        next_frame().await;
    }
}

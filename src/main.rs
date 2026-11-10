use macroquad::prelude::*;

mod ai;
mod econ;
mod game;
mod gen;
mod geo;
mod royale;
mod units;
mod world;

use game::*;
use world::*;

// bottom bar: PLAY/PAUSE, SPEED, ZOOM+, ZOOM-
const BW: [f32; 4] = [1.4, 1.4, 0.8, 0.8];

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

fn button_x(sw: f32, i: usize) -> (f32, f32) {
    let total: f32 = BW.iter().sum();
    let x: f32 = BW[..i].iter().sum::<f32>() / total * sw;
    (x, BW[i] / total * sw)
}

// the build / train menu: a panel with a header and up to three buttons, placed next to a tile
fn menu_layout(l: &Lay, sw: f32, sh: f32, anchor: Vec2, n: usize, header_w: f32) -> (Rect, Vec<Rect>) {
    let bw = (l.fs * 6.6).min((sw - 40.0) / 3.3);
    let bh = l.fs * 3.6;
    let gap = l.fs * 0.35;
    let hh = l.fs * 1.5;
    let items_w = if n > 0 { n as f32 * bw + (n as f32 - 1.0) * gap } else { 0.0 };
    let pw = (items_w.max(header_w) + gap * 2.0).min(sw - 8.0);
    let ph = hh + if n > 0 { bh + gap } else { 0.0 } + gap * 0.5;
    let x0 = (anchor.x - pw / 2.0).clamp(4.0, (sw - pw - 4.0).max(4.0));
    let mut y0 = anchor.y - ph - l.fs * 1.2;
    if y0 < l.top + 4.0 {
        y0 = anchor.y + l.fs * 1.2;
    }
    y0 = y0.clamp(l.top + 4.0, (sh - l.bar - ph - 4.0).max(l.top + 4.0));
    let panel = Rect::new(x0, y0, pw, ph);
    let start = x0 + (pw - items_w) / 2.0;
    let items = (0..n).map(|k| Rect::new(start + k as f32 * (bw + gap), y0 + hh, bw, bh)).collect();
    (panel, items)
}

// where each soldier's box is drawn: up to three side by side under the tile centre
fn soldier_rects(g: &Game, to_screen: &dyn Fn(Vec2) -> Vec2, cell: f32, fs: f32, sw: f32, sh: f32, top: f32) -> Vec<(usize, Rect)> {
    let marker = cell < fs * 0.7;
    let bw = (cell * 0.6).clamp(fs * 0.9, fs * 2.2);
    let bh = bw * 0.75;
    let mut n_at = vec![0u8; g.provs.len()];
    for a in &g.armies {
        n_at[a.prov] = n_at[a.prov].saturating_add(1);
    }
    let mut k_at = vec![0u8; g.provs.len()];
    let mut out = Vec::new();
    for (i, a) in g.armies.iter().enumerate() {
        let k = k_at[a.prov] as usize;
        k_at[a.prov] = k_at[a.prov].saturating_add(1);
        if k >= 6 {
            continue;
        }
        let c = to_screen(g.provs[a.prov].pos);
        if c.x < -bw * 4.0 || c.x > sw + bw * 4.0 || c.y < top - bh || c.y > sh + bh {
            continue; // off screen
        }
        if marker {
            let m = (cell * 0.8).max(6.0);
            out.push((i, Rect::new(c.x - m / 2.0 + k as f32 * 2.0, c.y - m / 2.0, m, m)));
        } else {
            let n = n_at[a.prov].min(6) as f32;
            let total = n * bw + (n - 1.0) * 3.0;
            out.push((i, Rect::new(c.x - total / 2.0 + k as f32 * (bw + 3.0), c.y + cell * 0.08, bw, bh)));
        }
    }
    out
}

fn conf() -> Conf {
    Conf {
        window_title: "World Conquest".to_string(),
        window_width: 1280,
        window_height: 720,
        ..Default::default()
    }
}

#[macroquad::main(conf)]
async fn main() {
    // show a message first: building the world takes a moment
    clear_background(Color::new(0.05, 0.05, 0.08, 1.0));
    ctext("Generating the world...", screen_width() / 2.0, screen_height() / 2.0, (screen_height() / 14.0).clamp(18.0, 60.0), WHITE);
    next_frame().await;
    let mut g = Game::new();
    royale::init(&mut g);
    let mut screen = 0u8; // 0 = title, 1 = playing
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

        // ---- title screen ----
        if screen == 0 {
            clear_background(Color::new(0.05, 0.05, 0.08, 1.0));
            let sc = (sw / MW as f32).min(sh / MH as f32);
            draw_texture_ex(
                &g.tex,
                sw / 2.0 - MW as f32 * sc / 2.0,
                sh / 2.0 - MH as f32 * sc / 2.0,
                Color::new(0.35, 0.35, 0.4, 1.0),
                DrawTextureParams { dest_size: Some(vec2(MW as f32 * sc, MH as f32 * sc)), ..Default::default() },
            );
            ctext("WORLD CONQUEST", sw / 2.0, sh * 0.2, l.fs * 2.6, WHITE);
            ctext("Pick your capital, build factories, train soldiers,", sw / 2.0, sh * 0.31, l.fs * 0.9, GRAY);
            ctext("and outlast 500 rival nations.", sw / 2.0, sh * 0.31 + l.fs * 1.2, l.fs * 0.9, GRAY);
            let (bw, bh) = ((sw * 0.7).min(l.fs * 20.0), l.fs * 3.6);
            let (x, y) = (sw / 2.0 - bw / 2.0, sh * 0.55);
            draw_rectangle(x, y, bw, bh, Color::new(0.2, 0.28, 0.45, 0.95));
            draw_rectangle_lines(x, y, bw, bh, 3.0, Color::new(0.75, 0.8, 0.95, 1.0));
            ctext("PLAY", sw / 2.0, y + bh / 2.0, l.fs * 1.7, WHITE);
            if is_mouse_button_pressed(MouseButton::Left) && mx >= x && mx <= x + bw && my >= y && my <= y + bh {
                royale::init(&mut g);
                cam = vec2(MW as f32 / 2.0, MH as f32 / 2.0);
                zoom = 1.0;
                press = None;
                screen = 1;
            }
            next_frame().await;
            continue;
        }

        let scale = l.base * zoom;
        let cell = CELL * scale;
        let to_screen = move |p: Vec2| vec2(sw / 2.0 + (p.x - cam.x) * scale, mid_y + (p.y - cam.y) * scale);

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
                let mouse = vec2(mx, my);
                let menu_geo = g.menu.map(|p| {
                    let info = g.menu_content(p);
                    let hw = measure_text(&info.header, None, (l.fs * 0.9) as u16, 1.0).width + l.fs;
                    let (panel, items) = menu_layout(&l, sw, sh, to_screen(g.provs[p].pos), info.items.len(), hw);
                    (p, panel, items)
                });
                if g.roy.over.is_some() {
                    screen = 0;
                } else if menu_geo.as_ref().map_or(false, |m| m.1.contains(mouse)) {
                    // a tap inside the menu
                    let (p, _, items) = menu_geo.unwrap();
                    for (k, r) in items.iter().enumerate() {
                        if r.contains(mouse) {
                            match g.menu_pick(p, k) {
                                Ok(msg) => {
                                    g.say(&msg);
                                    g.menu = None;
                                }
                                Err(e) => g.say(e),
                            }
                        }
                    }
                } else if my > sh - l.bar {
                    g.menu = None;
                    let mut bi = 0;
                    for i in 0..4 {
                        let (x, w) = button_x(sw, i);
                        if mx >= x && mx < x + w {
                            bi = i;
                        }
                    }
                    match bi {
                        0 => g.paused = !g.paused,
                        1 => g.speed_i = (g.speed_i + 1) % SPEEDS.len(),
                        2 => zoom = (zoom * 1.4).clamp(1.0, 24.0),
                        _ => zoom = (zoom / 1.4).clamp(1.0, 24.0),
                    }
                } else if my > l.top {
                    press = Some((mouse, cam));
                    moved = false;
                }
            }
            if let Some((start, cam0)) = press {
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
                    if !moved && g.roy.started && g.roy.over.is_none() {
                        // a tap on one of your soldiers selects that soldier
                        let rects = soldier_rects(&g, &to_screen, cell, l.fs, sw, sh, l.top);
                        let mouse = vec2(mx, my);
                        let hit = rects.iter().rev().find(|(i, r)| g.armies[*i].owner == 0 && r.contains(mouse)).map(|(i, _)| g.armies[*i].id);
                        if let Some(id) = hit {
                            g.sel_army = if g.sel_army == Some(id) { None } else { Some(id) };
                            g.menu = None;
                            press = None;
                        }
                    }
                    if press.is_some() && !moved {
                        let tx = ((mx - sw / 2.0) / scale + cam.x).floor();
                        let ty = ((my - mid_y) / scale + cam.y).floor();
                        if tx >= 0.0 && ty >= 0.0 && (tx as usize) < MW && (ty as usize) < MH {
                            let p = g.tile[ty as usize * MW + tx as usize] as usize;
                            // zoomed far out the soldier boxes are tiny: a tap on the tile picks its first soldier
                            let first = g.armies.iter().find(|a| a.owner == 0 && a.prov == p).map(|a| a.id);
                            if g.roy.started && cell < l.fs * 0.7 && g.sel_army.is_none() && first.is_some() {
                                g.sel_army = first;
                            } else if let Some(sp) = royale::tap(&mut g, p) {
                                cam = g.provs[sp].pos; // jump to your capital
                                zoom = 12.0;
                            }
                        }
                    }
                    press = None;
                }
            }
        }
        // keep the camera inside the map
        let scale = l.base * zoom;
        let hw = sw / 2.0 / scale;
        let hh = l.avail_h / 2.0 / scale;
        cam.x = if hw >= MW as f32 / 2.0 { MW as f32 / 2.0 } else { cam.x.clamp(hw, MW as f32 - hw) };
        cam.y = if hh >= MH as f32 / 2.0 { MH as f32 / 2.0 } else { cam.y.clamp(hh, MH as f32 - hh) };

        // ---- simulation ----
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
        if g.toast_t > 0.0 {
            g.toast_t -= dt;
        }

        // ---- draw ----
        let cell = CELL * scale;
        let to_screen = move |p: Vec2| vec2(sw / 2.0 + (p.x - cam.x) * scale, mid_y + (p.y - cam.y) * scale);
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
        // the safe zone: a white ring marks the last safe spot, the red circle is the shrinking edge
        {
            let c = to_screen(g.roy.center);
            draw_circle_lines(c.x, c.y, (l.fs * 0.8).max(CELL * 2.0 * scale), 3.0, WHITE);
            draw_circle(c.x, c.y, l.fs * 0.15, WHITE);
            if g.roy.zone_r < g.roy.r0 * 0.995 {
                draw_circle_lines(c.x, c.y, g.roy.zone_r * scale, 3.0, Color::new(1.0, 0.3, 0.3, 0.9));
            }
        }
        // where you are steering your expansion
        if let Some(f) = g.roy.focus {
            let c = to_screen(g.provs[f].pos);
            draw_circle_lines(c.x, c.y, l.fs * 1.2, 3.0, YELLOW);
            draw_circle(c.x, c.y, l.fs * 0.2, YELLOW);
        }
        // your capital
        if g.roy.started {
            let cp = g.roy.players[0].capital;
            let c = to_screen(g.provs[cp].pos);
            let col = if g.has_capital(0) { Color::new(1.0, 0.85, 0.2, 1.0) } else { RED };
            draw_circle_lines(c.x, c.y, (l.fs * 0.7).max(cell * 0.6), 3.0, col);
        }
        // factories
        if cell >= l.fs * 0.7 {
            let ic = (cell * 0.42).max(l.fs * 0.55);
            for &pu in &g.roy.land_list {
                let p = pu as usize;
                if g.bld[p] == 0 {
                    continue;
                }
                let c = to_screen(g.provs[p].pos);
                if c.x < -ic || c.x > sw + ic || c.y < l.top - ic || c.y > sh {
                    continue;
                }
                let done = g.bld_done[p] <= g.roy.tick;
                let (col, label) = match (g.bld[p], done) {
                    (1, true) => (Color::new(0.95, 0.8, 0.2, 1.0), "$"),
                    (_, true) => (Color::new(0.85, 0.3, 0.3, 1.0), "M"),
                    _ => (Color::new(0.5, 0.5, 0.55, 1.0), ".."),
                };
                let r = Rect::new(c.x - ic / 2.0, c.y - cell * 0.22 - ic / 2.0, ic, ic);
                draw_rectangle(r.x, r.y, r.w, r.h, col);
                draw_rectangle_lines(r.x, r.y, r.w, r.h, 2.0, BLACK);
                ctext(label, c.x, c.y - cell * 0.22, ic * 0.8, BLACK);
            }
        }
        // path of the selected soldier
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
        // soldiers: up to three boxes side by side on a tile
        let rects = soldier_rects(&g, &to_screen, cell, l.fs, sw, sh, l.top);
        for (i, r) in &rects {
            let a = &g.armies[*i];
            let col = royale::owner_color(&g, a.owner);
            let selected = g.sel_army == Some(a.id);
            if cell < l.fs * 0.7 {
                draw_rectangle(r.x, r.y, r.w, r.h, col);
                draw_rectangle_lines(r.x, r.y, r.w, r.h, if selected { 3.0 } else { 1.0 }, if selected { RED } else { BLACK });
                continue;
            }
            draw_rectangle(r.x, r.y, r.w, r.h, Color::new(col.r * 0.55, col.g * 0.55, col.b * 0.55, 1.0));
            let mine = a.owner == 0;
            draw_rectangle_lines(r.x, r.y, r.w, r.h, if selected { 5.0 } else if mine { 3.0 } else { 1.5 }, if selected { YELLOW } else { WHITE });
            ctext(&format!("{}{}", KIND_LETTER[a.kind], a.size.ceil() as i32), r.x + r.w / 2.0, r.y + r.h / 2.0, r.h * 0.8, WHITE);
        }

        // the build / train menu
        if let Some(p) = g.menu {
            let info = g.menu_content(p);
            let hw = measure_text(&info.header, None, (l.fs * 0.9) as u16, 1.0).width + l.fs;
            let (panel, items) = menu_layout(&l, sw, sh, to_screen(g.provs[p].pos), info.items.len(), hw);
            draw_rectangle(panel.x, panel.y, panel.w, panel.h, Color::new(0.05, 0.06, 0.1, 0.94));
            draw_rectangle_lines(panel.x, panel.y, panel.w, panel.h, 2.0, Color::new(0.6, 0.65, 0.8, 1.0));
            ctext(&info.header, panel.x + panel.w / 2.0, panel.y + l.fs * 0.75, l.fs * 0.9, WHITE);
            for (k, r) in items.iter().enumerate() {
                let it = &info.items[k];
                let bg = if it.ok { Color::new(0.22, 0.30, 0.45, 1.0) } else { Color::new(0.20, 0.20, 0.22, 1.0) };
                let fg = if it.ok { WHITE } else { Color::new(0.55, 0.55, 0.58, 1.0) };
                draw_rectangle(r.x, r.y, r.w, r.h, bg);
                draw_rectangle_lines(r.x, r.y, r.w, r.h, 2.0, Color::new(0.75, 0.8, 0.95, 0.8));
                ctext(&it.title, r.x + r.w / 2.0, r.y + r.h * 0.28, l.fs * 0.8, fg);
                ctext(&it.line2, r.x + r.w / 2.0, r.y + r.h * 0.58, l.fs * 0.75, fg);
                ctext(&it.line3, r.x + r.w / 2.0, r.y + r.h * 0.82, l.fs * 0.6, Color::new(0.8, 0.85, 0.6, if it.ok { 1.0 } else { 0.5 }));
            }
        }

        // bottom bar
        draw_rectangle(0.0, sh - l.bar, sw, l.bar, Color::new(0.1, 0.1, 0.14, 1.0));
        let labels = [
            if g.paused { "> PLAY".to_string() } else { "|| PAUSE".to_string() },
            format!("SPEED x{}", SPEEDS[g.speed_i] as i32),
            "+".to_string(),
            "-".to_string(),
        ];
        for (i, t) in labels.iter().enumerate() {
            let (x, w) = button_x(sw, i);
            draw_rectangle(x + 3.0, sh - l.bar + 4.0, w - 6.0, l.bar - 8.0, Color::new(0.2, 0.22, 0.3, 1.0));
            ctext(t, x + w / 2.0, sh - l.bar / 2.0, l.fs * 0.85, WHITE);
        }
        if g.toast_t > 0.0 {
            ctext(&g.toast, sw / 2.0, sh - l.bar - l.fs * 1.2, l.fs * 1.1, YELLOW);
        }
        royale::draw(&g, l.fs, l.top, sw, sh);

        next_frame().await;
    }
}

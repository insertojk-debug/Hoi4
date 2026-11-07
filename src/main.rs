use macroquad::prelude::*;

mod ai;
mod coast;
mod game;
mod gen;
mod royale;
mod units;
mod world;

use game::*;
use world::*;

const SPEEDS: [f32; 4] = [1.0, 2.0, 4.0, 8.0];
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

// the train menu: three unit buttons next to a province on screen
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
            ctext("Pick where you start, grow your land, train an army,", sw / 2.0, sh * 0.31, l.fs * 0.9, GRAY);
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
                let sc = l.base * zoom;
                let menu_rects = g.menu.map(|p| {
                    let a = vec2(sw / 2.0 + (g.provs[p].pos.x - cam.x) * sc, mid_y + (g.provs[p].pos.y - cam.y) * sc);
                    menu_layout(&l, sw, sh, a)
                });
                if g.roy.over.is_some() {
                    screen = 0;
                } else if menu_rects.map_or(false, |r| in_menu(&r, l.fs * 0.35, vec2(mx, my))) {
                    // a tap inside the train menu
                    let (p, rects) = (g.menu.unwrap(), menu_rects.unwrap());
                    for k in 0..3 {
                        if rects[k].contains(vec2(mx, my)) {
                            if g.recruit(0, p, k) {
                                g.say(&format!("{} trained", KIND_NAMES[k]));
                                g.menu = None;
                            } else {
                                g.say("Not enough troops");
                            }
                        }
                    }
                } else if my > sh - l.bar {
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
                            if let Some(sp) = royale::tap(&mut g, p) {
                                cam = g.provs[sp].pos; // jump to your spawn
                                zoom = 9.0;
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
        // the safe zone
        {
            let c = to_screen(g.roy.center);
            // the last safe spot: the final winner is whoever owns it
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
        // soldiers (yours and the bots')
        let cell = CELL * scale;
        let bw = (cell * 0.95).min(l.fs * 2.2);
        let bh = bw * 0.6;
        let mut stack = vec![0u8; g.provs.len()];
        for a in g.armies.iter() {
            let k = stack[a.prov] as f32;
            stack[a.prov] = stack[a.prov].saturating_add(1);
            if k >= 3.0 {
                continue; // at most three boxes per province
            }
            let c = to_screen(g.provs[a.prov].pos);
            if c.x < -bw || c.x > sw + bw || c.y < l.top - bh || c.y > sh {
                continue; // off screen
            }
            let col = royale::owner_color(&g, a.owner);
            let selected = g.sel_army == Some(a.id);
            if bw < l.fs * 0.9 {
                // zoomed out: just a marker
                let m = (bw * 0.9).max(5.0);
                draw_rectangle(c.x - m / 2.0 + k * 2.0, c.y - m / 2.0, m, m, col);
                draw_rectangle_lines(c.x - m / 2.0, c.y - m / 2.0, m, m, if selected { 3.0 } else { 1.0 }, if selected { RED } else { BLACK });
                continue;
            }
            let x = c.x - bw / 2.0 + k * (bw + 4.0);
            let y = c.y - bh / 2.0;
            draw_rectangle(x, y, bw, bh, Color::new(col.r * 0.55, col.g * 0.55, col.b * 0.55, 1.0));
            let mine = a.owner == 0;
            draw_rectangle_lines(x, y, bw, bh, if selected { 5.0 } else if mine { 3.0 } else { 1.5 }, if selected { YELLOW } else { WHITE });
            ctext(&format!("{}{}", KIND_LETTER[a.kind], a.size as i32), x + bw / 2.0, y + bh / 2.0, bh * 0.8, WHITE);
        }

        // train menu (like Polytopia): three unit choices next to the province
        if let Some(p) = g.menu {
            let rects = menu_layout(&l, sw, sh, to_screen(g.provs[p].pos));
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
                ctext(&format!("{} troops", TCOST[k] as i32), r.x + r.w / 2.0, r.y + r.h * 0.62, l.fs * 0.7, fg);
                let note = ["balanced", "strong attack", "fast + tough"][k];
                ctext(note, r.x + r.w / 2.0, r.y + r.h * 0.84, l.fs * 0.6, Color::new(0.8, 0.85, 0.6, if ok { 1.0 } else { 0.5 }));
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

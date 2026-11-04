use macroquad::prelude::*;

mod coast;
mod game;
mod gen;
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

// bottom bar buttons: PLAY, SPEED, RECRUIT, ZOOM+, ZOOM-
const BW: [f32; 5] = [1.2, 1.2, 2.2, 0.6, 0.6];

fn button_x(sw: f32, i: usize) -> (f32, f32) {
    let total: f32 = BW.iter().sum();
    let x: f32 = BW[..i].iter().sum::<f32>() / total * sw;
    (x, BW[i] / total * sw)
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
    let mut cam = g.provs[g.armies[0].prov].pos;
    let mut zoom = 12.0f32;
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
                if g.result.is_some() {
                    g = Game::new();
                    cam = g.provs[g.armies[0].prov].pos;
                    zoom = 12.0;
                } else if my > sh - l.bar {
                    let mut bi = 0;
                    for i in 0..5 {
                        let (x, w) = button_x(sw, i);
                        if mx >= x && mx < x + w {
                            bi = i;
                        }
                    }
                    match bi {
                        0 => g.paused = !g.paused,
                        1 => g.speed_i = (g.speed_i + 1) % SPEEDS.len(),
                        2 => {
                            let ok = g.sel_prov.filter(|&p| !g.provs[p].sea && g.provs[p].owner == 0);
                            match ok {
                                Some(p) => {
                                    if !g.recruit(0, p) {
                                        g.say("Need 10 IC + 20 manpower");
                                    }
                                }
                                None => g.say("Select one of your provinces"),
                            }
                        }
                        3 => zoom = (zoom * 1.4).clamp(1.0, 24.0),
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
                            g.tap_map(p);
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
            ctext(&format!("{}", a.size as i32), x + bw / 2.0, y + bh / 2.0, bh * 0.8, WHITE);
        }

        // top HUD
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

        // bottom bar
        draw_rectangle(0.0, sh - l.bar, sw, l.bar, Color::new(0.1, 0.1, 0.14, 1.0));
        let labels = [
            if g.paused { "> PLAY".to_string() } else { "|| PAUSE".to_string() },
            format!("SPEED x{}", SPEEDS[g.speed_i] as i32),
            "RECRUIT 10IC 20MP".to_string(),
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
        if let Some(win) = g.result {
            draw_rectangle(0.0, 0.0, sw, sh, Color::new(0.0, 0.0, 0.0, 0.7));
            ctext(if win { "VICTORY" } else { "DEFEAT" }, sw / 2.0, sh / 2.0 - l.fs, l.fs * 3.0, WHITE);
            ctext("Tap to restart", sw / 2.0, sh / 2.0 + l.fs * 2.0, l.fs, GRAY);
        }

        next_frame().await;
    }
}

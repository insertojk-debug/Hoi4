use macroquad::prelude::*;

use crate::world::*;

// real coastlines (Natural Earth, simplified), packed by a script into world.bin
const DATA: &[u8] = include_bytes!("world.bin");
const MS: usize = 3; // one mask cell covers 3x3 map tiles

pub struct Geo {
    w: usize,
    h: usize,
    land: Vec<u8>,
    reg: Vec<u16>, // which landmass (polygon) a cell belongs to
}

struct Rd<'a> {
    d: &'a [u8],
    p: usize,
}

impl<'a> Rd<'a> {
    fn u32(&mut self) -> u32 {
        let v = u32::from_le_bytes([self.d[self.p], self.d[self.p + 1], self.d[self.p + 2], self.d[self.p + 3]]);
        self.p += 4;
        v
    }
    fn i16(&mut self) -> i16 {
        let v = i16::from_le_bytes([self.d[self.p], self.d[self.p + 1]]);
        self.p += 2;
        v
    }
    fn ring(&mut self) -> Vec<Vec2> {
        let n = self.u32();
        (0..n)
            .map(|_| {
                let lon = self.i16() as f32 / 100.0;
                let lat = self.i16() as f32 / 100.0;
                proj(lon, lat) / MS as f32
            })
            .collect()
    }
}

// calls f(row, x_from, x_to) for every run of mask cells inside the ring
fn spans(ring: &[Vec2], w: usize, h: usize, mut f: impl FnMut(usize, usize, usize)) {
    let n = ring.len();
    let (mut ymin, mut ymax) = (f32::MAX, f32::MIN);
    for p in ring {
        ymin = ymin.min(p.y);
        ymax = ymax.max(p.y);
    }
    let y0 = ymin.floor().max(0.0) as usize;
    let y1 = (ymax.ceil().min(h as f32 - 1.0)).max(0.0) as usize;
    let mut xs: Vec<f32> = Vec::new();
    for y in y0..=y1 {
        let yc = y as f32 + 0.5;
        xs.clear();
        for i in 0..n {
            let (a, b) = (ring[i], ring[(i + 1) % n]);
            if (a.y <= yc && b.y > yc) || (b.y <= yc && a.y > yc) {
                xs.push(a.x + (yc - a.y) * (b.x - a.x) / (b.y - a.y));
            }
        }
        xs.sort_by(|a, b| a.partial_cmp(b).unwrap());
        for k in 0..xs.len() / 2 {
            let xa = (xs[2 * k] - 0.5).ceil().max(0.0);
            let xb = (xs[2 * k + 1] - 0.5).floor().min(w as f32 - 1.0);
            if xb >= xa {
                f(y, xa as usize, xb as usize);
            }
        }
    }
}

fn area(r: &[Vec2]) -> f32 {
    let mut s = 0.0;
    for i in 0..r.len() {
        let (a, b) = (r[i], r[(i + 1) % r.len()]);
        s += a.x * b.y - b.x * a.y;
    }
    s.abs() / 2.0
}

pub fn build() -> Geo {
    let (w, h) = (MW / MS + 1, MH / MS + 1);
    let mut land = vec![0u8; w * h];
    let mut reg = vec![0u16; w * h];
    let mut rd = Rd { d: DATA, p: 0 };
    let np = rd.u32() as usize;
    let mut polys: Vec<Vec<Vec<Vec2>>> = Vec::new();
    for _ in 0..np {
        let nr = rd.u32();
        polys.push((0..nr).map(|_| rd.ring()).collect());
    }
    // land = inside an odd number of rings (so holes work)
    for rings in &polys {
        for ring in rings {
            spans(ring, w, h, |y, a, b| {
                for x in a..=b {
                    land[y * w + x] ^= 1;
                }
            });
        }
    }
    // landmass ids: biggest first, so an island keeps its own id
    let mut order: Vec<usize> = (0..polys.len()).collect();
    order.sort_by(|&a, &b| area(&polys[b][0]).partial_cmp(&area(&polys[a][0])).unwrap());
    for &k in &order {
        let id = (k + 1) as u16;
        spans(&polys[k][0], w, h, |y, a, b| {
            for x in a..=b {
                reg[y * w + x] = id;
            }
        });
    }
    // big lakes are water
    let nl = rd.u32();
    for _ in 0..nl {
        let ring = rd.ring();
        spans(&ring, w, h, |y, a, b| {
            for x in a..=b {
                land[y * w + x] = 0;
            }
        });
    }
    Geo { w, h, land, reg }
}

impl Geo {
    // is this map position land, and which landmass is it
    pub fn at(&self, p: Vec2) -> (bool, u16) {
        let x = ((p.x / MS as f32) as i32).clamp(0, self.w as i32 - 1) as usize;
        let y = ((p.y / MS as f32) as i32).clamp(0, self.h as i32 - 1) as usize;
        (self.land[y * self.w + x] != 0, self.reg[y * self.w + x])
    }
}

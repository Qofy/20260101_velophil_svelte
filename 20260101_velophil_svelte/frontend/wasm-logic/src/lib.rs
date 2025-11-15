use wasm_bindgen::prelude::*;
use js_sys::{Float32Array, Int32Array, Uint32Array};

#[wasm_bindgen]
pub fn positions_table(columns: &Int32Array, rows: &Int32Array) -> Float32Array {
    let n = columns.length().min(rows.length()) as usize;
    let mut out = vec![0.0f32; n * 3];
    for i in 0..n {
        let col = columns.get_index(i as u32) as f32;
        let row = rows.get_index(i as u32) as f32;
        let x = (col * 140.0) - 1330.0;
        let y = -(row * 180.0) + 990.0;
        let z = 0.0;
        out[i*3] = x;
        out[i*3+1] = y;
        out[i*3+2] = z;
    }
    Float32Array::from(out.as_slice())
}

#[wasm_bindgen]
pub fn positions_sphere(n: u32) -> Float32Array {
    let n = n as usize;
    let mut out = vec![0.0f32; n * 3];
    let pi = std::f32::consts::PI;
    for i in 0..n {
        let i_f = i as f32;
        let total = n as f32;
        let phi = (-1.0 + (2.0 * i_f) / total).acos();
        let theta = (total * pi).sqrt() * phi;
        let radius = 800.0f32;
        let x = radius * phi.sin() * theta.cos();
        let y = radius * phi.sin() * theta.sin();
        let z = radius * phi.cos();
        out[i*3] = x; out[i*3+1] = y; out[i*3+2] = z;
    }
    Float32Array::from(out.as_slice())
}

#[wasm_bindgen]
pub fn positions_helix(n: u32) -> Float32Array {
    let n = n as usize;
    let mut out = vec![0.0f32; n * 3];
    let pi = std::f32::consts::PI;
    for i in 0..n {
        let theta = (i as f32) * 0.175 + pi;
        let y = -((i as f32) * 8.0) + 450.0;
        let radius = 900.0f32;
        let x = radius * theta.cos();
        let z = radius * theta.sin();
        out[i*3] = x; out[i*3+1] = y; out[i*3+2] = z;
    }
    Float32Array::from(out.as_slice())
}

#[wasm_bindgen]
pub fn positions_grid(n: u32) -> Float32Array {
    let n = n as usize;
    let mut out = vec![0.0f32; n * 3];
    for i in 0..n {
        let x = (((i % 5) as f32) * 400.0) - 800.0;
        let y = (-(((i / 5) % 5) as f32) * 400.0) + 800.0;
        let z = (((i / 25) as f32) * 1000.0) - 2000.0;
        out[i*3] = x; out[i*3+1] = y; out[i*3+2] = z;
    }
    Float32Array::from(out.as_slice())
}

#[wasm_bindgen]
pub fn interpolate(current: &Float32Array, target: &Float32Array, lerp: f32) -> Float32Array {
    let n = current.length().min(target.length()) as usize;
    let mut out = vec![0.0f32; n];
    for i in 0..n {
        let c = current.get_index(i as u32);
        let t = target.get_index(i as u32);
        out[i] = c + (t - c) * lerp;
    }
    Float32Array::from(out.as_slice())
}

// Rectangles marquee selection: rects_ltrb is [l0,t0,r0,b0, l1,t1,r1,b1, ...]
#[wasm_bindgen]
pub fn marquee_select(min_x: f32, min_y: f32, max_x: f32, max_y: f32, rects_ltrb: &Float32Array) -> Uint32Array {
    let len = rects_ltrb.length() as usize;
    if len < 4 { return Uint32Array::new_with_length(0); }
    let n = len / 4;
    let mut hits: Vec<u32> = Vec::with_capacity(n);
    for i in 0..n {
        let l = rects_ltrb.get_index((i*4) as u32);
        let t = rects_ltrb.get_index((i*4 + 1) as u32);
        let r = rects_ltrb.get_index((i*4 + 2) as u32);
        let b = rects_ltrb.get_index((i*4 + 3) as u32);
        // overlap test
        if r >= min_x && l <= max_x && b >= min_y && t <= max_y {
            hits.push(i as u32);
        }
    }
    let out = Uint32Array::new_with_length(hits.len() as u32);
    for (i, v) in hits.iter().enumerate() { out.set_index(i as u32, *v); }
    out
}

// Nearest rectangle to (px, py) within max_dist (pixels); returns -1 if none.
#[wasm_bindgen]
pub fn hit_test_nearest(px: f32, py: f32, rects_ltrb: &Float32Array, max_dist: f32) -> i32 {
    let len = rects_ltrb.length() as usize;
    if len < 4 { return -1; }
    let n = len / 4;
    let mut best_i: i32 = -1;
    let mut best_d2: f32 = max_dist * max_dist;
    for i in 0..n {
        let l = rects_ltrb.get_index((i*4) as u32);
        let t = rects_ltrb.get_index((i*4 + 1) as u32);
        let r = rects_ltrb.get_index((i*4 + 2) as u32);
        let b = rects_ltrb.get_index((i*4 + 3) as u32);
        // distance from point to AABB
        let dx = if px < l { l - px } else if px > r { px - r } else { 0.0 };
        let dy = if py < t { t - py } else if py > b { py - b } else { 0.0 };
        let d2 = dx*dx + dy*dy;
        if d2 <= best_d2 {
            best_d2 = d2;
            best_i = i as i32;
        }
    }
    best_i
}

// Count overlaps of rects against pin rectangles. Returns counts per pin.
#[wasm_bindgen]
pub fn pin_counts(rects_ltrb: &Float32Array, pins_ltrb: &Float32Array) -> Uint32Array {
    let rlen = rects_ltrb.length() as usize;
    let plen = pins_ltrb.length() as usize;
    if rlen < 4 || plen < 4 { return Uint32Array::new_with_length(0); }
    let rn = rlen / 4;
    let pn = plen / 4;
    let mut counts: Vec<u32> = vec![0; pn];
    for p in 0..pn {
        let pl = pins_ltrb.get_index((p*4) as u32);
        let pt = pins_ltrb.get_index((p*4 + 1) as u32);
        let pr = pins_ltrb.get_index((p*4 + 2) as u32);
        let pb = pins_ltrb.get_index((p*4 + 3) as u32);
        let mut c = 0u32;
        for i in 0..rn {
            let l = rects_ltrb.get_index((i*4) as u32);
            let t = rects_ltrb.get_index((i*4 + 1) as u32);
            let r = rects_ltrb.get_index((i*4 + 2) as u32);
            let b = rects_ltrb.get_index((i*4 + 3) as u32);
            if r >= pl && l <= pr && b >= pt && t <= pb { c += 1; }
        }
        counts[p] = c;
    }
    let out = Uint32Array::new_with_length(pn as u32);
    for (i, v) in counts.iter().enumerate() { out.set_index(i as u32, *v); }
    out
}

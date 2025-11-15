// Web Worker for hit-testing and pin counts (pure JS loops)
(self as any).onmessage = (e: MessageEvent) => {
  const data: any = e.data || {};
  try {
    if (data.type === 'init') {
      (self as any).postMessage({ type: 'ready' });
      return;
    }
    if (data.type === 'hitNearest') {
      const { id, px, py, rectsBuf, maxDist } = data;
      const rects = new Float32Array(rectsBuf);
      const t0 = (performance.now?.() ?? Date.now());
      let best = -1;
      let bestD2 = (maxDist || 32) * (maxDist || 32);
      const n = (rects.length / 4) | 0;
      for (let i = 0; i < n; i++) {
        const l = rects[i*4], t = rects[i*4+1], r = rects[i*4+2], b = rects[i*4+3];
        const dx = px < l ? (l - px) : (px > r ? (px - r) : 0);
        const dy = py < t ? (t - py) : (py > b ? (py - b) : 0);
        const d2 = dx*dx + dy*dy;
        if (d2 <= bestD2) { bestD2 = d2; best = i; }
      }
      const dt = (performance.now?.() ?? Date.now()) - t0;
      (self as any).postMessage({ id, ok: true, idx: best, dt });
      return;
    }
    if (data.type === 'pinCounts') {
      const { id, rectsBuf, pinsBuf } = data;
      const rects = new Float32Array(rectsBuf);
      const pins = new Float32Array(pinsBuf);
      const t0 = (performance.now?.() ?? Date.now());
      const rn = (rects.length / 4) | 0;
      const pn = (pins.length / 4) | 0;
      const counts = new Array(pn).fill(0);
      for (let p = 0; p < pn; p++) {
        const pl = pins[p*4], pt = pins[p*4+1], pr = pins[p*4+2], pb = pins[p*4+3];
        let c = 0;
        for (let i = 0; i < rn; i++) {
          const l = rects[i*4], t = rects[i*4+1], r = rects[i*4+2], b = rects[i*4+3];
          if (r >= pl && l <= pr && b >= pt && t <= pb) c++;
        }
        counts[p] = c;
      }
      const dt = (performance.now?.() ?? Date.now()) - t0;
      (self as any).postMessage({ id, ok: true, counts, dt });
      return;
    }
  } catch (err: any) {
    const id = (e.data || {}).id;
    (self as any).postMessage({ id, ok: false, error: String(err?.message || err) });
  }
};

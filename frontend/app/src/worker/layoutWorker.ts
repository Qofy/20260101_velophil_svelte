// Dedicated worker that runs WASM layout + interpolation off the main thread
// eslint-disable-next-line @typescript-eslint/ban-ts-comment
// @ts-ignore
import initWasm, { positions_table, positions_sphere, positions_helix, positions_grid, interpolate } from '../wasm/wasm_logic.js';

let ready = false;
let n = 0;
let cols: Int32Array | null = null;
let rows: Int32Array | null = null;
let layout: 'table' | 'sphere' | 'helix' | 'grid' | 'custom' = 'table';
let customPositions: Float32Array | null = null;
let buf0: Float32Array = new Float32Array(0);
let buf1: Float32Array = new Float32Array(0);
let cp: Float32Array = new Float32Array(0); // compute-from buffer
let np: Float32Array = new Float32Array(0); // compute-into buffer
let target: Float32Array = new Float32Array(0);
let layoutTarget: Float32Array = new Float32Array(0);
let sharedPositions: Float32Array | null = null; // legacy single-SAB
// SAB ring (triple buffer)
let ringBuffer: SharedArrayBuffer | null = null;
let ring: Float32Array[] = [];
let ringCount = 0;
let floatsPer = 0;
let writeIdx = 0;
let ctrl: Int32Array | null = null; // [writeIdx, seq]

let focusedIndex: number | null = null;
let flipOnPlace = false;
const LERP = 0.08;

async function ensureWasm() {
  if (!ready) {
    await initWasm();
    ready = true;
    // eslint-disable-next-line no-restricted-globals
    (self as any).postMessage({ type: 'ready' });
  }
}

function computeLayoutTarget(): void {
  if (!ready || n <= 0) return;
  if (layout === 'table' && cols && rows) {
    layoutTarget = positions_table(cols, rows);
  } else if (layout === 'sphere') {
    layoutTarget = positions_sphere(n);
  } else if (layout === 'helix') {
    layoutTarget = positions_helix(n);
  } else if (layout === 'grid') {
    layoutTarget = positions_grid(n);
  } else if (layout === 'custom' && customPositions && customPositions.length === n * 3) {
    layoutTarget = customPositions.slice() as Float32Array;
  } else {
    layoutTarget = new Float32Array(n * 3);
  }
  if (target.length !== layoutTarget.length) target = new Float32Array(layoutTarget.length);
  target.set(layoutTarget);
}

function applyFocusOverride(): void {
  if (focusedIndex !== null && !flipOnPlace) {
    const i = focusedIndex | 0;
    target[i * 3 + 0] = 0;
    target[i * 3 + 1] = 0;
    target[i * 3 + 2] = 500;
  }
}

// eslint-disable-next-line no-restricted-globals
(self as any).onmessage = async (e: MessageEvent) => {
  const data = e.data || {};
  switch (data.type) {
    case 'init':
      await ensureWasm();
      break;
    case 'setN':
      n = Number(data.n || 0);
      buf0 = new Float32Array(n * 3);
      buf1 = new Float32Array(n * 3);
      cp = buf0;
      np = buf1;
      target = new Float32Array(n * 3);
      layoutTarget = new Float32Array(n * 3);
      break;
    case 'setTableData': {
      const c = data.cols as number[] | Int32Array;
      const r = data.rows as number[] | Int32Array;
      cols = c instanceof Int32Array ? c : new Int32Array(c);
      rows = r instanceof Int32Array ? r : new Int32Array(r);
      break;
    }
    case 'setLayout':
      layout = data.layout;
      computeLayoutTarget();
      applyFocusOverride();
      break;
    case 'setSharedPositions': {
      const buf = data.buffer as SharedArrayBuffer;
      if (buf) sharedPositions = new Float32Array(buf);
      break;
    }
    case 'setSharedRing': {
      ringBuffer = data.buffer as SharedArrayBuffer;
      ringCount = Number(data.ring || 0);
      floatsPer = Number(data.floatsPer || 0);
      ring = [];
      if (ringBuffer && ringCount > 0 && floatsPer > 0) {
        for (let k = 0; k < ringCount; k++) {
          ring.push(new Float32Array(ringBuffer, k * floatsPer * 4, floatsPer));
        }
      }
      writeIdx = 0;
      break;
    }
    case 'setSharedCtrl': {
      const buf = data.buffer as SharedArrayBuffer;
      if (buf) ctrl = new Int32Array(buf);
      break;
    }
    case 'setCustomPositions': {
      const arr = data.positions as Float32Array | number[];
      customPositions = arr instanceof Float32Array ? arr : new Float32Array(arr);
      if (layout === 'custom') {
        computeLayoutTarget();
        applyFocusOverride();
      }
      break;
    }
    case 'focus': {
      const idx = Number(data.index);
      focusedIndex = Number.isFinite(idx) ? idx : null;
      flipOnPlace = !!data.flipOnPlace;
      // recompute override
      computeLayoutTarget();
      applyFocusOverride();
      break;
    }
    case 'unfocus': {
      focusedIndex = null;
      computeLayoutTarget();
      break;
    }
    case 'tick': {
      await ensureWasm();
      if (cp.length === 0 || target.length === 0) {
        // eslint-disable-next-line no-restricted-globals
        (self as any).postMessage({ type: 'positions', positions: new Float32Array(0) });
        break;
      }
      // Triple-buffer ring path
      if (ring.length === ringCount && ringCount > 0 && floatsPer === cp.length && ctrl) {
        const L = cp.length;
        const out = ring[writeIdx];
        for (let i = 0; i < L; i++) {
          const c = cp[i];
          const t = target[i];
          out[i] = c + (t - c) * LERP;
        }
        // publish index
        Atomics.store(ctrl, 0, writeIdx);
        Atomics.add(ctrl, 1, 1);
        writeIdx = (writeIdx + 1) % ringCount;
        // next round interpolate from what we just wrote
        cp = out;
        // notify
        // eslint-disable-next-line no-restricted-globals
        (self as any).postMessage({ type: 'positionsReady' });
        break;
      }
      // Legacy single-SAB path (not used in ring mode)
      if (sharedPositions && sharedPositions.length === cp.length) {
        const L = cp.length;
        for (let i = 0; i < L; i++) {
          const c = cp[i];
          const t = target[i];
          np[i] = c + (t - c) * LERP;
        }
        // copy into shared buffer
        sharedPositions.set(np);
        // swap cp/np for next round
        const tmp2 = cp; cp = np; np = tmp2;
        // notify positions ready
        // eslint-disable-next-line no-restricted-globals
        (self as any).postMessage({ type: 'positionsReady' });
        break;
      }
      // Transfer path
      {
        const L = cp.length;
        for (let i = 0; i < L; i++) {
          const c = cp[i];
          const t = target[i];
          np[i] = c + (t - c) * LERP;
        }
        // transfer np and swap
        // eslint-disable-next-line no-restricted-globals
        (self as any).postMessage({ type: 'positions', positions: np }, [np.buffer]);
        const tmp = cp; cp = np; np = tmp;
        break;
      }
    }
  }
};

// Kick off WASM init
ensureWasm();
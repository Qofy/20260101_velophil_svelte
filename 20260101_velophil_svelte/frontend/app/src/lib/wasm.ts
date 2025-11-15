// WASM module wrapper
import init, * as wasmExports from '../wasm/wasm_logic.js';

export type Wasm = typeof wasmExports;

let wasmInstance: Wasm | null = null;

export async function initWasm(): Promise<Wasm> {
  if (wasmInstance) {
    return wasmInstance;
  }

  // Initialize WASM module
  await init();

  wasmInstance = wasmExports;
  return wasmInstance;
}

// Re-export WASM functions for convenience
export const {
  positions_table,
  positions_sphere,
  positions_helix,
  positions_grid,
  interpolate,
  marquee_select,
  hit_test_nearest,
  pin_counts,
} = wasmExports;

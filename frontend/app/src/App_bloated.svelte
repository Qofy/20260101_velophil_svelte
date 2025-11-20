<script lang="ts">
  import { onMount } from 'svelte';
import { initWasm, type Wasm } from './lib/wasm';
  import { createEngine, type Engine, type Layout } from './lib/engine';
  import InfoPanel from './components/InfoPanel.svelte';
  import Status from './components/Status.svelte';
  import OrderOverlay from './components/OrderOverlay.svelte';
  import SaveLoadControls from './components/SaveLoadControls.svelte';
  import SaveLoadOverlay from './components/SaveLoadOverlay.svelte';
  import EditorPanel from './components/EditorPanel.svelte';
import ZonePanel from './components/ZonePanel.svelte';
  import ZoneLegend from './components/ZoneLegend.svelte';
  import ServerPanel from './components/ServerPanel.svelte';
  import ClientPanel from './components/ClientPanel.svelte';
  import ReservPanel from './components/ReservPanel.svelte';
  import OrdersPanel from './components/OrdersPanel.svelte';
  import PaymentsPanel from './components/PaymentsPanel.svelte';
  import LegendOverlay from './components/LegendOverlay.svelte';
  import PerfOverlay from './components/PerfOverlay.svelte';
  import SettingsPanel from './components/SettingsPanel.svelte';
  import KitchenPanel from './components/KitchenPanel.svelte';
  import PinsPanel from './components/PinsPanel.svelte';
  import LoginOverlay from './components/LoginOverlay.svelte';
  import WordlDictsPanel from './components/WordlDictsPanel.svelte';
  import WebGLBoard from './components/WebGLBoard.svelte';
  import JoinSessionOverlay from './components/JoinSessionOverlay.svelte';
  import RegisterOverlay from './components/RegisterOverlay.svelte';
  import SyncStatus from './components/SyncStatus.svelte';
  import ConnectionTestOverlay from './components/ConnectionTestOverlay.svelte';
  import AdminClaimsPanel from './components/AdminClaimsPanel.svelte';
import { pingServer, saveOrder, saveReservation, postKitchenPrint, kitchenTestPrint, listRuntimeCounts } from './lib/api';
  import { flushPending, getPending } from './lib/sync';
  import { API_BASE_DEFAULT, API_TOKEN_ALLOW_LS_OVERRIDE, CONFLICT_POLICY_DEFAULT } from './lib/config';
import { backendOnline, backendMessageStore, pendingOpsCount, kitchenUseRelay, kitchenUseTest, kitchenTestLatencyMs, kitchenTestFailRate, runtimeEventsCursor, runtimeOrders, runtimeReservations, runtimePayments, kitchenJobs, runtimeCounts, runtimeCountsWindowMinutes, controlsConfig, type ControlsConfig, conflictPolicy, currentUser, isFirstUserSystem } from './lib/stores';
  import { GRID_CELL_X, GRID_CELL_Y } from './lib/grid';
import { completeTable } from './lib/data';
  import { perfStart, perfEnd, perfMark, perfSnapshot } from './lib/perf';

let engine: Engine | null = null;
  let wasmMod: Wasm | null = null;
  // Worker for offloaded hit-testing and pin counts
  let hitWorker: Worker | null = null;
  let hitWorkerReady = false;
  let workerReqId = 1;
  const workerPending = new Map<number, (res: any)=>void>();
  function startHitWorker() {
    try {
      // eslint-disable-next-line @typescript-eslint/ban-ts-comment
      // @ts-ignore
      hitWorker = new Worker(new URL('./worker/hitWorker.ts', import.meta.url), { type: 'module' });
      hitWorker.onmessage = (e: MessageEvent) => {
        const d: any = e.data || {};
        if (d.type === 'ready') { hitWorkerReady = true; return; }
        const id = d.id;
        const fn = id ? workerPending.get(id) : null;
        if (fn) { workerPending.delete(id); fn(d); }
      };
      hitWorker.postMessage({ type: 'init' });
      hitWorkerReady = true;
    } catch {}
  }
  function workerCall<T=any>(type: string, payload: any, transfer?: Transferable[]): Promise<T> {
    return new Promise((resolve, reject) => {
      if (!hitWorker) { reject(new Error('worker not available')); return; }
      const id = workerReqId++;
      workerPending.set(id, (d: any) => {
        if (d && d.ok) resolve(d as T); else reject(new Error(String(d?.error || 'worker error')));
      });
      const msg = { id, type, ...payload };
      try { hitWorker.postMessage(msg, transfer || []); } catch (e) { workerPending.delete(id); reject(e as any); }
    });
  }
  let layout: Layout = 'table';
  let zoom = 0.6;

  // UI state reflected from engine
  let rotation = { x: 0, y: 0, z: 0 };
  let translation = { x: 0, y: 0, z: 0 };
  let selected: string | null = null;
let elementCount = 118;

  // Selection state
  let selectionMode: boolean = false;
  let selectedIndices: number[] = [];
  let disabledMap: Record<number, boolean> = {};
  // Named selections UI state
  let selectionName: string = '';
  let selectionNames: string[] = [];
  let showSelectionOverlay: boolean = false;
  async function refreshSelectionNames() { try { const p = await import('./lib/persistence'); const r = await p.listSelections(); selectionNames = Array.isArray(r?.names) ? r.names : []; } catch {} }
  async function doSaveSelection(name?: string) {
    const nm = String((name ?? selectionName) || '').trim();
    if (!nm) { alert('Enter a selection name'); return; }
    try { const p = await import('./lib/persistence'); await p.saveSelection(nm, selectedIndices); alert('Selection saved'); await refreshSelectionNames(); } catch {}
  }
  async function doLoadSelection(n: string) {
    try { const p = await import('./lib/persistence'); const r = await p.loadSelection(n); const arr = Array.isArray(r?.data) ? (r!.data as number[]) : []; if (arr && arr.length) { selectedIndices = Array.from(new Set(arr.filter((i)=> Number.isFinite(i) && i>=0))).sort((a,b)=>a-b); engine?.setSelectedIndices(selectedIndices); selectionMode = true; } } catch {}
  }
  async function doDeleteSelection(n: string) { try { const p = await import('./lib/persistence'); await p.deleteSelection(n); await refreshSelectionNames(); } catch {} }
  function onSelectionOverlaySelect(e: CustomEvent) { try { const n = (e as any).detail && (e as any).detail.name; if (n) { doLoadSelection(String(n)); showSelectionOverlay = false; } } catch {} }
  function onSelectionOverlayDelete(e: CustomEvent) { try { const n = (e as any).detail && (e as any).detail.name; if (n) { doDeleteSelection(String(n)); } } catch {} }

  // Marquee selection state
let marqueeActive: boolean = false;
  let marqueeStart = { x: 0, y: 0 };
  let marqueeEnd = { x: 0, y: 0 };

  // Hover highlight state
let hoverCheckPending = false;
  let latestPointer = { x: 0, y: 0 };

  // Pins state
  let showPins = false;
  type Pin = { id: string; name: string; x0n: number; y0n: number; x1n: number; y1n: number };
  let pins: Pin[] = [];
  let pinCounts: number[] = [];
  let pinAddMode = false;
  let pinDraftActive = false; let pinDraftStart = { x: 0, y: 0 }; let pinDraftEnd = { x: 0, y: 0 };
  function loadPins() {
    try { const s = localStorage.getItem('pinsRects') || '[]'; const arr = JSON.parse(s); if (Array.isArray(arr)) pins = arr.filter(Boolean); } catch {}
  }
  function savePins() { try { localStorage.setItem('pinsRects', JSON.stringify(pins)); } catch {} }
  function renamePin(id: string, name: string) { const p = pins.find(p=>p.id===id); if (p) { p.name = String(name||'').trim() || p.name; savePins(); } }
  function deletePin(id: string) { pins = pins.filter(p=>p.id!==id); savePins(); schedulePinCounts(); }
  function toggleAddPin() { pinAddMode = !pinAddMode; }
  function schedulePinCounts() { if (!showPins) return; if (pinCountsRaf) return; pinCountsRaf = requestAnimationFrame(updatePinCounts); }
  let pinCountsRaf = 0;

  // Server persistence for Pins
  let pinsConfigName: string = 'default';
  let pinsServerNames: string[] = [];
  async function refreshPinsNames() {
    try { const api = await import('./lib/api'); const r = await api.listPins(); pinsServerNames = Array.isArray(r?.names) ? r.names : []; } catch {}
  }
  async function savePinsToServer() {
    const dto = pins.map(p => ({ id: p.id, name: p.name, x0n: p.x0n, y0n: p.y0n, x1n: p.x1n, y1n: p.y1n }));
    try { const api = await import('./lib/api'); await api.savePins(pinsConfigName || 'default', dto); await refreshPinsNames(); } catch {}
  }
  async function loadPinsFromServer(name: string) {
    if (!name) return;
    try { const api = await import('./lib/api'); const r = await api.loadPins(name); const arr = (r?.data || []) as any[]; if (Array.isArray(arr)) {
      pins = arr.map((o:any) => ({ id: String(o.id || `pin-${Math.random().toString(36).slice(2)}`), name: String(o.name || ''), x0n: Number(o.x0n||0), y0n: Number(o.y0n||0), x1n: Number(o.x1n||0), y1n: Number(o.y1n||0) }));
      savePins(); schedulePinCounts();
    } } catch {}
  }
async function updatePinCounts() {
    pinCountsRaf = 0;
    if (!showPins || !pins.length) { pinCounts = []; return; }
    const rectsAny = engine?.getElementRects?.() || [] as any[];
    const rects = new Float32Array(rectsAny.length * 4);
    for (let i=0;i<rectsAny.length;i++) { const r:any = rectsAny[i]; rects[i*4]=r.left; rects[i*4+1]=r.top; rects[i*4+2]=r.right; rects[i*4+3]=r.bottom; }
    const pinsArr = new Float32Array(pins.length * 4);
    const w = window.innerWidth, h = window.innerHeight;
    for (let i=0;i<pins.length;i++) { const p = pins[i]; const l = Math.min(p.x0n, p.x1n)*w, t=Math.min(p.y0n,p.y1n)*h, r=Math.max(p.x0n,p.x1n)*w, b=Math.max(p.y0n,p.y1n)*h; pinsArr[i*4]=l; pinsArr[i*4+1]=t; pinsArr[i*4+2]=r; pinsArr[i*4+3]=b; }
const t0 = perfStart('pins.counts');
    let counts: number[] = [];
    if (hitWorker) {
      try {
        const res: any = await workerCall('pinCounts', { rectsBuf: rects.buffer, pinsBuf: pinsArr.buffer }, [rects.buffer, pinsArr.buffer]);
        counts = Array.from((res?.counts || []) as number[]);
      } catch {
        counts = [];
      }
    } else if (wasmMod) { const out = wasmMod.pin_counts(rects, pinsArr); counts = Array.from(out as unknown as ArrayLike<number>); }
    else {
      counts = pins.map(()=>0);
      for (let pi=0;pi<pins.length;pi++) {
        const l=pinsArr[pi*4], t=pinsArr[pi*4+1], r=pinsArr[pi*4+2], b=pinsArr[pi*4+3];
        let c=0; for (let i=0;i<rectsAny.length;i++){ const rr:any = rectsAny[i]; if (rr.right>=l && rr.left<=r && rr.bottom>=t && rr.top<=b) c++; }
        counts[pi]=c;
      }
    }
    perfEnd('pins.counts', t0);
    pinCounts = counts;
  }

async function doHoverCheck(x: number, y: number) {
    const t0r = perfStart('rects.get_hover');
    const rects = engine?.getElementRects?.() || [];
    perfEnd('rects.get_hover', t0r);
    if (!rects || rects.length === 0) { engine?.setHighlightedIndices([]); return; }
    let idx = -1;
if (hitWorker) {
      const arr = new Float32Array(rects.length * 4);
      for (let k = 0; k < rects.length; k++) {
        const r: any = (rects as any[])[k];
        arr[k*4] = r.left; arr[k*4+1] = r.top; arr[k*4+2] = r.right; arr[k*4+3] = r.bottom;
      }
      const t0h = perfStart('hit.worker');
      try {
        const res: any = await workerCall('hitNearest', { px: x, py: y, rectsBuf: arr.buffer, maxDist: 32.0 }, [arr.buffer]);
        idx = Number(res?.idx ?? -1);
      } catch {
        idx = -1;
      }
      perfEnd('hit.worker', t0h);
    } else if (wasmMod) {
      const arr = new Float32Array(rects.length * 4);
      for (let k = 0; k < rects.length; k++) {
        const r: any = (rects as any[])[k];
        arr[k*4] = r.left; arr[k*4+1] = r.top; arr[k*4+2] = r.right; arr[k*4+3] = r.bottom;
      }
      const t0h = perfStart('hit.nearest');
      idx = wasmMod.hit_test_nearest(x, y, arr, 32.0);
      perfEnd('hit.nearest', t0h);
    } else {
      // Fallback: scan JS
      let bestD2 = 32*32;
      for (let k = 0; k < (rects as any[]).length; k++) {
        const r: any = (rects as any[])[k];
        const l = r.left, t = r.top, rr = r.right, b = r.bottom;
        const dx = x < l ? (l - x) : (x > rr ? (x - rr) : 0);
        const dy = y < t ? (t - y) : (y > b ? (y - b) : 0);
        const d2 = dx*dx + dy*dy;
        if (d2 <= bestD2) { bestD2 = d2; idx = k; }
      }
    }
    if (idx >= 0 && idx < rects.length) engine?.setHighlightedIndices([ (rects as any[])[idx].i ]);
    else engine?.setHighlightedIndices([]);
  }

  // Feature toggles
  let zoomBoardOnFlip = true;
  let zoomCardOnClick = true;
  let flipOnPlace = false;
  let orderOnDoubleClick = false;
  let showCoordinates = false;
  let orderOnFront = false;
  let addOrderOnFlip = false;
  let showGrid = false;
  let snapToGrid = false;
  // Drag overlay state
  let dragActive = false;
  let overlayAnimRaf = 0;
  function startOverlayAnim() {
    if (overlayAnimRaf) return;
    const tick = () => {
      if (dragActive) {
        drawGrid();
        overlayAnimRaf = requestAnimationFrame(tick);
      } else {
        cancelAnimationFrame(overlayAnimRaf);
        overlayAnimRaf = 0;
      }
    };
    overlayAnimRaf = requestAnimationFrame(tick);
  }
  function stopOverlayAnim() {
    if (overlayAnimRaf) { cancelAnimationFrame(overlayAnimRaf); overlayAnimRaf = 0; }
  }
  let dragIndex: number = -1;
  let dragWorldX = 0;
  let dragWorldY = 0;
  let gridStyle: 'dots' | 'lines' = 'dots';
  function toggleGridStyle() { gridStyle = gridStyle === 'dots' ? 'lines' : 'dots'; saveGridPrefs(); setTimeout(drawGrid, 0); }
  function ensureWordlUserId(): string {
    try {
      const s = localStorage.getItem('wordlUserId');
      if (s && s.trim()) return s.trim();
    } catch {}
    const id = 'u-' + Math.random().toString(36).slice(2,10);
    try { localStorage.setItem('wordlUserId', id); } catch {}
    return id;
  }
  function joinInviteIndex(i: number) { try { (engine as any && (engine as any).startWordlForIndex) ? (engine as any).startWordlForIndex(i) : void 0; } catch {} }

  // ---- Claims to per-card masks helpers ----
  function parseCardSpecToSet(spec: string): Set<number> {
    // spec examples: "1,2,5-9,12"; returns 0-based indices in range [0, elementCount)
    const set = new Set<number>();
    const parts = String(spec||'').split(/[,\s]+/).map(s=>s.trim()).filter(Boolean);
    for (const p of parts) {
      const m = p.match(/^([0-9]+)\-([0-9]+)$/);
      if (m) {
        let a = Number(m[1]), b = Number(m[2]);
        if (Number.isFinite(a) && Number.isFinite(b)) {
          if (a > b) { const t=a; a=b; b=t; }
          for (let k=a; k<=b; k++) { const idx = k-1; if (idx>=0 && idx<elementCount) set.add(idx); }
        }
      } else {
        const v = Number(p);
        if (Number.isFinite(v)) { const idx = v-1; if (idx>=0 && idx<elementCount) set.add(idx); }
      }
    }
    return set;
  }
  function extractCardsSpec(scopes: string[], prefix: string): string | null {
    // Find first scope line like `${prefix}:cards:...`
    for (const s of scopes) {
      const idx = s.indexOf(prefix + ':cards:');
      if (idx === 0) return s.slice((prefix + ':cards:').length);
    }
    return null;
  }
  function buildMasksFromClaims() {
    try {
      const roles = ($currentUser?.roles || []) as string[];
      const scopes = ($currentUser?.scopes || []) as string[];
      const isAdmin = roles.includes('admin');
      const allTrue = Array.from({length: elementCount}, ()=>true);
      const allFalse = Array.from({length: elementCount}, ()=>false);

      let orderMask = allFalse.slice();
      let reservMask = allFalse.slice();
      let wordlMask = allFalse.slice();

      // Global grants
      const globOrder = isAdmin || scopes.includes('orders:write');
      const globReserv = isAdmin || scopes.includes('reservations:write');
      const globGame = isAdmin || scopes.includes('game:play');

      if (globOrder) orderMask = allTrue.slice(); else {
        const spec = extractCardsSpec(scopes, 'orders');
        if (spec) { const set = parseCardSpecToSet(spec); orderMask = Array.from({length:elementCount}, (_,i)=>set.has(i)); }
      }
      if (globReserv) reservMask = allTrue.slice(); else {
        const spec = extractCardsSpec(scopes, 'reservations');
        if (spec) { const set = parseCardSpecToSet(spec); reservMask = Array.from({length:elementCount}, (_,i)=>set.has(i)); }
      }
      if (globGame) wordlMask = allTrue.slice(); else {
        const spec = extractCardsSpec(scopes, 'game');
        if (spec) { const set = parseCardSpecToSet(spec); wordlMask = Array.from({length:elementCount}, (_,i)=>set.has(i)); }
      }

      try { (engine as any)?.setCardPermissionMasks?.({ order: orderMask, reserv: reservMask, wordl: wordlMask }); } catch {}
    } catch {}
  }

  async function inviteWordl() {
    try {
      let defaultIdx = typeof ordersFilterIndex === 'number' && ordersFilterIndex !== null ? Number(ordersFilterIndex) : 0;
      if (!(defaultIdx >= 0 && defaultIdx < elementCount)) defaultIdx = 0;
      const s = prompt('Invite for card index (1-118)?', String(defaultIdx + 1));
      if (!s) return;
      const num = Number(s);
      if (!Number.isFinite(num)) return;
      const i = Math.max(0, Math.min(elementCount - 1, Math.floor(num - 1)));
      try { engine?.setShowWordlButton?.(true); } catch {}
      (engine as any)?.startWordlForIndex?.(i);
      // Create session and copy link immediately
      const uid = ensureWordlUserId();
      const { wordleCreateSession, wordleJoinSession } = await import('./lib/api');
      const sess = await wordleCreateSession({ cardIndex: i, boardId: 'default' });
      if (sess && sess.id) {
        await wordleJoinSession(sess.id, uid, wordlPlayerName || undefined);
        const u = new URL(window.location.href);
        u.searchParams.set('wordleSession', sess.id);
        u.searchParams.set('cardIndex', String(i));
        const out = u.toString();
        lastInviteUrl = out;
        try { await navigator.clipboard.writeText(out); alert('Invite link copied to clipboard'); } catch { alert(out); }
      }
    } catch (e) { console.warn('invite error', e); }
  }
  // Fade radius in card units
  let fadeRadiusCards: number = 1.0;
  function onFadeRadiusInput(e: Event) {
    const v = Number((e.target as HTMLInputElement).value);
    fadeRadiusCards = Math.max(0.3, Math.min(3, v));
    saveGridPrefs();
    setTimeout(drawGrid, 0);
  }
  function saveGridPrefs() {
    try {
      localStorage.setItem('gridStyle', gridStyle);
      localStorage.setItem('fadeRadiusCards', String(fadeRadiusCards));
    } catch {}
  }
  function loadGridPrefs() {
    try {
      const gs = localStorage.getItem('gridStyle');
      if (gs === 'dots' || gs === 'lines') gridStyle = gs;
      const fr = Number(localStorage.getItem('fadeRadiusCards'));
      if (Number.isFinite(fr) && fr >= 0.3 && fr <= 3) fadeRadiusCards = fr;
    } catch {}
  }
  // Rotation lock during drag
  let rotationLockedDuringDrag = false;
  let savedRotation = { x: 0, y: 0, z: 0 };

  // Drag HUD near card
  let hudVisible = false;
  let hudLeft = 0;
  let hudTop = 0;
  let hudText = '';

  // Keyboard help
  let helpVisible = false;
  let reservOnFront = false;
  let reservOnFlip = false;
  let colorByZone = false;
  let zoneFilter = '';

  // Perf overlay
  let perfEnabled = false;
  let perf = { fps: 0, frameMs: 0, workerMs: 0, workerMode: 'NONE' as 'MAIN' | 'SAB' | 'SAB3' | 'XFER' | 'NONE' };
  let perfExtra: { name: string; avg: number; count: number }[] = [];
  let fallbackMode = false; // force main-thread engine (no layout worker)
  function togglePerf() {
    perfEnabled = !perfEnabled;
    engine?.setPerfEnabled?.(perfEnabled);
    try { localStorage.setItem('perfEnabled', perfEnabled ? '1' : '0'); } catch {}
  }
  function toggleFallbackMode() {
    fallbackMode = !fallbackMode;
    try { engine?.setWorkerEnabled?.(!fallbackMode); } catch {}
    try { localStorage.setItem('fallbackMode', fallbackMode ? '1' : '0'); } catch {}
  }
  function setConflictPolicyHandler(v: 'prompt' | 'server' | 'local') {
    conflictPolicy.set(v as any);
    try { localStorage.setItem('conflictPolicy', String(v)); } catch {}
  }

  function setLayout(l: Layout) {
    layout = l;
    engine?.setLayout(l);
    saveCameraStateDebounced();
    if (l === 'custom') {
      // initialize history with current positions
      customHistory = [];
      customFuture = [];
      setTimeout(() => pushCustomSnapshot(), 0);
    }
  }

  function onToggle(e: CustomEvent) {
    const { name, value } = (e as any).detail;
    switch (name) {
      case 'zoomBoardOnFlip': zoomBoardOnFlip = value; engine?.setZoomBoardOnFlip(value); break;
      case 'zoomCardOnClick': zoomCardOnClick = value; engine?.setZoomCardOnClick(value); break;
      case 'flipOnPlace': flipOnPlace = value; engine?.setFlipOnPlace(value); break;
      case 'orderOnDoubleClick': orderOnDoubleClick = value; engine?.setOrderOnDoubleClick(value); break;
      case 'showCoordinates': showCoordinates = value; engine?.setShowCoordinates(value); break;
      case 'orderOnFront': orderOnFront = value; engine?.setOrderOnFront(value); break;
      case 'addOrderOnFlip': addOrderOnFlip = value; engine?.setAddOrderOnFlip(value); break;
      case 'showGrid': showGrid = value; engine?.setShowGrid(value); try { localStorage.setItem('showGrid', value ? '1' : '0'); } catch {} break;
      case 'snapToGrid': snapToGrid = value; engine?.setSnapToGrid(value); try { localStorage.setItem('snapToGrid', value ? '1' : '0'); } catch {} break;
      case 'reservOnFront': reservOnFront = value; engine?.setReservOnFront(value); break;
      case 'reservOnFlip': reservOnFlip = value; engine?.setReservOnFlip(value); break;
      case 'colorByZone': colorByZone = value; (engine as any)?.setColorByZone?.(value); break;
      case 'zoneFilter': zoneFilter = String(value || ''); (engine as any)?.setZoneFilter?.(zoneFilter); break;
      case 'showWordlButton': showWordlButton = !!value; try{localStorage.setItem('showWordlButton', showWordlButton ? '1':'0');}catch{}; engine?.setShowWordlButton?.(showWordlButton); break;
    }
  }

  let orderOverlayVisible = false;
  let currentOrderIndex: number | null = null;
  let showEditor = false;
  let showZone = false;
  let showServer = false;
  let showClient = false;
  let showReserv = false;
  let showSettings = false;
  let showKitchen = false;
  let showOrders = false;
  let showWordlDicts = false;
  let showPayments = false;
  let showLegend = false;
  let showConnTest = false;
  let showUserAdmin = false;
  // Claims-based UI gates
  $: roles = ($currentUser?.roles || []);
  $: scopes = ($currentUser?.scopes || []);
  $: isAdmin = roles.includes('admin');
  $: canOrders = isAdmin || scopes.includes('orders:write');
  $: canReservations = isAdmin || scopes.includes('reservations:write');
  $: canKitchen = isAdmin || scopes.includes('kitchen:manage');
  $: canSettings = isAdmin || scopes.includes('settings:write');
  $: canPins = isAdmin || scopes.includes('pins:manage');
  $: canRuntimeRead = isAdmin || scopes.includes('runtime:read');
  $: canGame = isAdmin || scopes.includes('game:play');
  let showSyncStatus = false;
  let showWordlButton = false;
  // Login gate
  let showLogin = false;
  let engineStarted = false;
  // UI visibility toggles
  let showInfoPanel = true;
  let showTabs = true;
  let showGpuDemo = false;
  function setFallbackMode(v: boolean) { fallbackMode = !!v; try { engine?.setWorkerEnabled?.(!fallbackMode); } catch {}; try { localStorage.setItem('fallbackMode', fallbackMode ? '1' : '0'); } catch {} }
  function setShowGpuDemo(v: boolean) { showGpuDemo = !!v; try { localStorage.setItem('showGpuDemo', showGpuDemo ? '1' : '0'); } catch {} }
  // Wordl scoreboard overlay state
  let wordlScore: any = null;
  let showJoin = false;
  let showRegister = false;
  let customUseMode = false; // custom layout: USE toggles drag vs use mode
  let wordlZoomFactor = 1.0;
  let lastInviteUrl: string = '';
  let showOrderTabs = true; // RESERV, CLIENT, ZONES, ORDERS, PAYMENTS, KITCHEN
  let showArrangementTabs = true; // TABLE, SPHERE, HELIX, GRID, CUSTOM, EDITOR, GRID toggle, RESET
  let showSliderRadius = true;
  let showSliderOrbit = true;
  let showSliderPan = true;
  let showSliderZoom = true;
  let wordlMultiplayer = false;
  let wordlPlayerName = '';
  let ordersFilterIndex: number | null = null;
  let ordersFilterSymbol: string | null = null;
  let apiBaseLocal: string = '';
  const allowApiOverrideFlag: boolean = ((import.meta as any).env?.VITE_API_ALLOW_LS_OVERRIDE === '1' || String((import.meta as any).env?.VITE_API_ALLOW_LS_OVERRIDE || '').toLowerCase() === 'true');

  // Backend status
  let backendStatus: 'online' | 'offline' | 'syncing' = 'offline';
  let backendMessage = '';

  // Handlers passed to children
  function applyCustomPositions(data: Float32Array) {
    engine?.setCustomPositions(data);
  }
  function getPositionsSafe(): Float32Array {
    return engine?.getPositions() ?? new Float32Array();
  }
  function openOrderOverlay(i?: number | null) { orderOverlayVisible = true; currentOrderIndex = (typeof i === 'number') ? i : null; }
  function closeOrderOverlay() { orderOverlayVisible = false; currentOrderIndex = null; }

  // Group actions
  let groupOrderMode = false;
  async function groupOrderAction() {
    if (!selectedIndices.length) return;
    groupOrderMode = true;
    openOrderOverlay(null);
  }
  async function groupReservAction() {
    if (!selectedIndices.length) return;
    const name = prompt('Reservation name for selected cards?');
    if (!name) return;
    for (const idx of selectedIndices) {
      try { await saveReservation({ index: idx, symbol: String(completeTable[idx*5]||''), name: String(name), ts: Date.now() }); } catch {}
    }
    alert('Group reservation saved');
  }
  async function groupZoneAction() {
    if (!selectedIndices.length) return;
    const zone = prompt('Zone name to assign to selected cards?');
    if (!zone) return;
    try {
      const zones = engine?.getZones?.() || [] as string[];
      for (const idx of selectedIndices) { zones[idx] = String(zone); }
      engine?.setZones?.(zones);
    } catch {}
  }
  async function groupClientAction() {
    if (!selectedIndices.length) return;
    const client = prompt('Client name to link to selected cards?');
    if (!client) return;
    try {
      const clients = (engine as any)?.getClients?.() || Array.from({length: elementCount}, ()=> '');
      for (const idx of selectedIndices) { clients[idx] = String(client); }
      (engine as any)?.setClients?.(clients);
    } catch {}
  }

  function drawGrid() {
    const canvas = document.getElementById('gridOverlay') as HTMLCanvasElement | null;
    if (!canvas) return;
    const dpr = window.devicePixelRatio || 1;
    const w = canvas.clientWidth;
    const h = canvas.clientHeight;
    canvas.width = Math.max(1, Math.floor(w * dpr));
    canvas.height = Math.max(1, Math.floor(h * dpr));
    const ctx = canvas.getContext('2d');
    if (!ctx) return;
    ctx.setTransform(1, 0, 0, 1, 0, 0); // reset
    ctx.scale(dpr, dpr);
    ctx.clearRect(0, 0, w, h);

    const pulse = dragActive ? (0.5 + 0.5 * Math.sin(performance.now() * 0.006)) : 0;

    // Match dot spacing to snap grid under current zoom and translation.
    const stepX = GRID_CELL_X * zoom;
    const stepY = GRID_CELL_Y * zoom;
    if (!(stepX > 0) || !(stepY > 0)) return;

    // Base grid alignment from world origin (ignoring rotation)
    const baseX = (w / 2) + (translation.x * zoom);
    const baseY = (h / 2) + (translation.y * zoom);

    // Find first dot position >= 0
    const mod = (a: number, n: number) => ((a % n) + n) % n;
    const startX = mod(-baseX, stepX);
    const startY = mod(-baseY, stepY);

    // If dragging, draw only within a radius around the dragged card with a glow fade
    if (dragActive) {
      // anchor at dragged card world position projected to screen
      const ax = (w / 2) + (translation.x + dragWorldX) * zoom;
      const ay = (h / 2) + (translation.y + dragWorldY) * zoom;
      const R = Math.max(120, 160) * (fadeRadiusCards || 1) * zoom; // configurable card radius
      const R2 = R * R;
      const minX = Math.max(0, Math.floor(ax - R));
      const maxX = Math.min(w, Math.ceil(ax + R));
      const minY = Math.max(0, Math.floor(ay - R));
      const maxY = Math.min(h, Math.ceil(ay + R));

      const baseColor = '0,255,255';
      if (gridStyle === 'dots') {
        // Update drag HUD near the anchor
        hudVisible = true;
        hudLeft = Math.round(ax + 14);
        hudTop = Math.round(ay - 22);
        const snapX = Math.round(dragWorldX / GRID_CELL_X) * GRID_CELL_X;
        const snapY = Math.round(dragWorldY / GRID_CELL_Y) * GRID_CELL_Y;
        const cellX = Math.round(dragWorldX / GRID_CELL_X);
        const cellY = Math.round(dragWorldY / GRID_CELL_Y);
        // hudText = `x:${dragWorldX.toFixed(0)} y:${dragWorldY.toFixed(0)} | snap:${snapX},${snapY} | cell:${cellX},${cellY}`;
// 7) Update the HUD text in App.svelte
// You mentioned:
        hudText = `x:${dragWorldX.toFixed(0)} y:${dragWorldY.toFixed(0)} | snap:${snapX},${snapY} | cell:${cellX},${cellY}`;

        // Preconfigure glow for dots
        ctx.shadowBlur = 8;
        for (let x = startX; x < w; x += stepX) {
          if (x < minX || x > maxX) continue;
          for (let y = startY; y < h; y += stepY) {
            if (y < minY || y > maxY) continue;
            const dx = x - ax;
            const dy = y - ay;
            const d2 = dx*dx + dy*dy;
            if (d2 > R2) continue;
            const d = Math.sqrt(d2);
            const t = 1 - (d / R); // 1 at center, 0 at radius
            const intensity = Math.max(0, Math.min(1, t));
            const alpha = 0.15 + 0.65 * (intensity * intensity); // ease
            const size = 2 + 2 * intensity;
            ctx.fillStyle = `rgba(${baseColor},${alpha})`;
            ctx.shadowColor = `rgba(${baseColor},${alpha})`;
            ctx.fillRect(Math.round(x - size/2), Math.round(y - size/2), size, size);
          }
        }
        // Emphasize the nearest dot to the card center
        const gx = baseX + Math.round((ax - baseX) / stepX) * stepX;
        const gy = baseY + Math.round((ay - baseY) / stepY) * stepY;
        const dx0 = gx - ax; const dy0 = gy - ay; const d02 = dx0*dx0 + dy0*dy0;
        if (d02 <= R2) {
          const d0 = Math.sqrt(d02);
          const t0 = 1 - (d0 / R);
          const size0 = 5 + 3 * t0;
          const alpha0 = 0.85;
          ctx.fillStyle = `rgba(${baseColor},${alpha0})`;
          ctx.shadowColor = `rgba(${baseColor},${alpha0})`;
          ctx.fillRect(Math.round(gx - size0/2), Math.round(gy - size0/2), size0, size0);
          // Crosshair at nearest dot
          ctx.lineWidth = 1.5;
          ctx.strokeStyle = `rgba(${baseColor},${alpha0})`;
          ctx.shadowBlur = 12;
          ctx.shadowColor = `rgba(${baseColor},${alpha0})`;
          const L = 8 + 6 * pulse;
          ctx.beginPath();
          ctx.moveTo(Math.round(gx - L), Math.round(gy) + 0.5);
          ctx.lineTo(Math.round(gx + L), Math.round(gy) + 0.5);
          ctx.moveTo(Math.round(gx) + 0.5, Math.round(gy - L));
          ctx.lineTo(Math.round(gx) + 0.5, Math.round(gy + L));
          ctx.stroke();
        }
        ctx.shadowBlur = 0; // reset
      } else {
        // Update drag HUD near the anchor
        hudVisible = true;
        hudLeft = Math.round(ax + 14);
        hudTop = Math.round(ay - 22);
        const snapX = Math.round(dragWorldX / GRID_CELL_X) * GRID_CELL_X;
        const snapY = Math.round(dragWorldY / GRID_CELL_Y) * GRID_CELL_Y;
        const cellX = Math.round(dragWorldX / GRID_CELL_X);
        const cellY = Math.round(dragWorldY / GRID_CELL_Y);
        // hudText = `x:${dragWorldX.toFixed(0)} y:${dragWorldY.toFixed(0)} | snap:${snapX},${snapY} | cell:${cellX},${cellY}`;
// 7) Update the HUD text in App.svelte
// You mentioned:
        hudText = `x:${dragWorldX.toFixed(0)} y:${dragWorldY.toFixed(0)} | snap:${snapX},${snapY} | cell:${cellX},${cellY}`;

        // Lines mode: draw grid lines clipped to a circle and mask with radial fade
        ctx.save();
        // Clip to circle area around the card
        ctx.beginPath();
        ctx.arc(ax, ay, R, 0, Math.PI * 2);
        ctx.clip();

        ctx.lineWidth = 1;
        ctx.strokeStyle = `rgba(${baseColor},0.6)`;
        ctx.shadowBlur = 8;
        ctx.shadowColor = `rgba(${baseColor},0.6)`;

        // Vertical lines
        for (let x = startX; x < w; x += stepX) {
          if (x < minX || x > maxX) continue;
          ctx.beginPath();
          ctx.moveTo(Math.round(x) + 0.5, minY);
          ctx.lineTo(Math.round(x) + 0.5, maxY);
          ctx.stroke();
        }
        // Horizontal lines
        for (let y = startY; y < h; y += stepY) {
          if (y < minY || y > maxY) continue;
          ctx.beginPath();
          ctx.moveTo(minX, Math.round(y) + 0.5);
          ctx.lineTo(maxX, Math.round(y) + 0.5);
          ctx.stroke();
        }
        // Emphasize nearest lines to the card center
        const nearestX = baseX + Math.round((ax - baseX) / stepX) * stepX;
        const nearestY = baseY + Math.round((ay - baseY) / stepY) * stepY;
        ctx.lineWidth = 2;
        ctx.strokeStyle = `rgba(${baseColor},0.9)`;
        ctx.shadowBlur = 12;
        ctx.shadowColor = `rgba(${baseColor},0.9)`;
        // vertical
        if (nearestX >= minX && nearestX <= maxX) {
          ctx.beginPath();
          ctx.moveTo(Math.round(nearestX) + 0.5, minY);
          ctx.lineTo(Math.round(nearestX) + 0.5, maxY);
          ctx.stroke();
        }
        // horizontal
        if (nearestY >= minY && nearestY <= maxY) {
          ctx.beginPath();
          ctx.moveTo(minX, Math.round(nearestY) + 0.5);
          ctx.lineTo(maxX, Math.round(nearestY) + 0.5);
          ctx.stroke();
        }
        // Crosshair ring at intersection
        if (nearestX >= minX && nearestX <= maxX && nearestY >= minY && nearestY <= maxY) {
          ctx.beginPath();
          const rr = 5 + 4 * pulse;
          ctx.arc(Math.round(nearestX) + 0.5, Math.round(nearestY) + 0.5, rr, 0, Math.PI * 2);
          ctx.stroke();
        }

        // Apply radial fade mask
        const g = ctx.createRadialGradient(ax, ay, 0, ax, ay, R);
        g.addColorStop(0, 'rgba(0,0,0,1)');
        g.addColorStop(1, 'rgba(0,0,0,0)');
        ctx.globalCompositeOperation = 'destination-in';
        ctx.fillStyle = g;
        ctx.fillRect(minX, minY, maxX - minX, maxY - minY);

        // Restore context (resets clip and composite op)
        ctx.restore();
        ctx.shadowBlur = 0;
      }
    } else {
      // Hide HUD when not dragging
      hudVisible = false;
      // Static full-grid dots
      ctx.fillStyle = 'rgba(127,255,255,0.35)';
      const dotSize = 2;
      for (let x = startX; x < w; x += stepX) {
        for (let y = startY; y < h; y += stepY) {
          ctx.fillRect(Math.round(x), Math.round(y), dotSize, dotSize);
        }
      }
    }
  }

  // Redraw grid whenever its dependencies change
$: if ((showGrid || dragActive), zoom, translation.x, translation.y, layout, dragWorldX, dragWorldY, fadeRadiusCards, gridStyle) { setTimeout(drawGrid, 0); }
  $: if (showPins, pins.length, zoom, translation.x, translation.y, rotation.x, rotation.y, rotation.z, layout) { if (showPins) schedulePinCounts(); }

  // Selection bounding overlay (dotted)
  let selectionGroupRect: { left: number; top: number; width: number; height: number } | null = null;
  $: if (selectionMode && selectedIndices.length > 0) {
    try {
      const rects = engine?.getElementRects?.() || [] as any[];
      let l = Infinity, t = Infinity, r = -Infinity, b = -Infinity;
      for (const i of selectedIndices) {
        const rr = rects.find((x:any)=>x.i===i);
        if (!rr) continue;
        l = Math.min(l, rr.left);
        t = Math.min(t, rr.top);
        r = Math.max(r, rr.right);
        b = Math.max(b, rr.bottom);
      }
      if (isFinite(l) && isFinite(t) && isFinite(r) && isFinite(b)) selectionGroupRect = { left: l, top: t, width: (r-l), height: (b-t) };
      else selectionGroupRect = null;
    } catch { selectionGroupRect = null; }
  } else { selectionGroupRect = null; }
  function handleResize() { if (showGrid || dragActive) drawGrid(); }
  // Custom layout Undo/Redo stacks
  let customHistory: Float32Array[] = [];
  let customFuture: Float32Array[] = [];
  function pushCustomSnapshot() {
    const pos = engine?.getPositions();
    if (!pos) return;
    customHistory.push(pos.slice() as Float32Array);
    if (customHistory.length > 50) customHistory.shift();
  }
  function undoCustom() {
    if (layout !== 'custom') return;
    if (customHistory.length < 2) return;
    const last = customHistory.pop() as Float32Array; // move current to future
    customFuture.push(last);
    const prev = customHistory[customHistory.length - 1];
    engine?.setCustomPositions(prev);
  }
  function redoCustom() {
    if (layout !== 'custom') return;
    const next = customFuture.pop();
    if (!next) return;
    customHistory.push(next.slice() as Float32Array);
    engine?.setCustomPositions(next);
  }

  function resetAllPositions() {
    // reset view to defaults
    engine?.setTranslation(0,0,0);
    engine?.setRotation(0,0,0);
    engine?.setZoom(0.6);
    // reset positions for current layout and clear focus/flip via engine
    engine?.resetPositions(layout);
    if (layout === 'custom') {
      customHistory = [];
      customFuture = [];
      setTimeout(() => pushCustomSnapshot(), 0);
    }
  }

  async function syncNow() {
    try {
      const res = await flushPending(API_BASE);
      pendingOpsCount.set(res.remaining);
    } catch {}
  }
  async function sendToKitchen(order: any) {
    try {
      const relay = localStorage.getItem('kitchenUseRelay') !== '0';
      const useTest = localStorage.getItem('kitchenUseTest') === '1';
      if (relay) {
        if (useTest) {
          const latency = Number(localStorage.getItem('kitchenTestLatencyMs') || '300');
          const failRate = Number(localStorage.getItem('kitchenTestFailRate') || '0');
          await kitchenTestPrint(order, { latencyMs: latency, failRate });
        } else {
          await postKitchenPrint(order, undefined, `cook-${order?.index||'x'}-${Date.now()}`);
        }
      } else {
        // direct device POST (dev mode)
        const s = localStorage.getItem('kitchenActiveUrls');
        const urls = s ? JSON.parse(s) as string[] : [];
        for (const u of urls) { try { await fetch(u, { method:'POST', headers:{'Content-Type':'application/json'}, body: JSON.stringify(order) }); } catch {} }
      }
    } catch {}
  }

function isSceneEventTarget(t: EventTarget | null): boolean {
    try {
      const el = document.getElementById('scene');
      return !!(el && t && (t as HTMLElement).nodeType === 1 && el.contains(t as Node));
    } catch { return false; }
  }
function onWheel(e: WheelEvent) {
    if (!isSceneEventTarget(e.target)) return; // allow native scrolling/slider changes outside scene
    // Suspend camera controls while dragging Wordl items
    if (engine?.isDraggingLetter?.() || engine?.isDraggingGhost?.()) { e.preventDefault(); return; }
    try { (window as any).sceneInput && ((window as any).sceneInput.wheel += 1); } catch {}
    const t0 = perfStart('evt.wheel');
    const c = getControls();
    // On most devices, wheel => zoom; ctrl/meta can be used for fine control
    const delta = (e.deltaY || 0) > 0 ? -0.1 : 0.1;
    const factor = e.ctrlKey || e.metaKey ? 0.5 * c.sensitivity.zoom : 1.0 * c.sensitivity.zoom;
    const nz = Math.max(0.1, Math.min(3, zoom + delta * factor));
    engine?.setZoom(nz);
    e.preventDefault();
    perfEnd('evt.wheel', t0);
  }
  // Basic mouse drag camera controls on background
  let dragMode: 'none' | 'rotate' | 'pan' | 'orbit' | 'roll' = 'none';
  let lastX = 0, lastY = 0;
function onPointerDown(e: PointerEvent) {
    const target = e.target as HTMLElement;
    // Ignore UI interactions outside the scene (settings, overlays, sliders)
    if (!isSceneEventTarget(target)) return;
    // Suspend camera controls while dragging Wordl items
    if (engine?.isDraggingLetter?.() || engine?.isDraggingGhost?.()) return;
    try { (window as any).sceneInput && ((window as any).sceneInput.pointerDown += 1); } catch {}
    const elTarget = target;
    // Add pin draft start if in add mode and background
    if (pinAddMode && (!elTarget || !elTarget.closest('.element'))) {
      pinDraftActive = true; pinDraftStart = { x: e.clientX, y: e.clientY }; pinDraftEnd = { x: e.clientX, y: e.clientY };
      e.preventDefault(); return;
    }
    const isCard = !!(target && target.closest('.element'));

    if (selectionMode && e.button === 0 && (!isCard) && (e.shiftKey || e.altKey)) {
      // Start marquee selection (Shift/Alt + drag on background)
      marqueeActive = true;
      marqueeStart = { x: e.clientX, y: e.clientY };
      marqueeEnd = { x: e.clientX, y: e.clientY };
      dragMode = 'none';
      e.preventDefault();
      return;
    }

    const btn = e.button; // 0 L, 1 M, 2 R
    const alt = e.altKey; const ctrl = e.ctrlKey || e.metaKey; const shift = e.shiftKey;
    const c = getControls();

    // If pointer is over a card, allow Shift+LMB to pan when using Orbit preset; otherwise ignore drags on cards
    if (isCard && !(c.preset === 'orbit' && btn === 0 && shift)) {
      return;
    }

    lastX = e.clientX; lastY = e.clientY;

    // Map input to action based on controls preset
    let mode: 'none'|'rotate'|'pan'|'zoom'|'roll' = 'none';

    if (c.preset === 'orbit') {
      if (btn === 1 || (c.emulateMMB && btn === 0 && shift) || (isCard && btn === 0 && shift)) mode = 'pan';
      else if (btn === 0) mode = 'rotate';
      else if (btn === 2 && c.allowRoll) mode = 'roll';
    } else if (c.preset === 'maya') {
      if (alt && btn === 0) mode = 'rotate';
      else if (alt && (btn === 1 || (c.emulateMMB && btn === 0 && shift))) mode = 'pan';
      else if (alt && btn === 2) mode = 'zoom';
    } else if (c.preset === 'cad') {
      if (btn === 1) mode = 'rotate';
      else if (shift && (btn === 1 || (c.emulateMMB && btn === 0))) mode = 'pan';
      else if (ctrl && (btn === 1 || (c.emulateMMB && btn === 0))) mode = 'zoom';
    }

    dragMode = mode === 'none' ? 'rotate' : (mode as any); // fallback to rotate
    try { document.body.classList.add('noselect-drag'); } catch {}
  }
function onPointerMove(e: PointerEvent) {
    const target = e.target as HTMLElement;
    if (!isSceneEventTarget(target)) return;
    // Suspend camera controls while dragging Wordl items
    if (engine?.isDraggingLetter?.() || engine?.isDraggingGhost?.()) return;
    if (pinDraftActive) { pinDraftEnd = { x: e.clientX, y: e.clientY }; e.preventDefault(); return; }
    if (selectionMode && !marqueeActive && dragMode === 'none') {
      // Throttle hover checks to animation frames
      latestPointer.x = e.clientX; latestPointer.y = e.clientY;
      if (!hoverCheckPending) {
        hoverCheckPending = true;
        requestAnimationFrame(() => {
          hoverCheckPending = false;
          if (selectionMode && !marqueeActive && dragMode === 'none') doHoverCheck(latestPointer.x, latestPointer.y);
        });
      }
    }
    if (marqueeActive) {
      const t0 = perfStart('evt.pointermove');
      marqueeEnd = { x: e.clientX, y: e.clientY };
      perfEnd('evt.pointermove', t0);
      e.preventDefault();
      return;
    }
    if (dragMode === 'none') return;
    const dx = e.clientX - lastX;
    const dy = e.clientY - lastY;
    lastX = e.clientX; lastY = e.clientY;
    const c = getControls();
    const sOrbit = Math.max(0.1, c.sensitivity.orbit);
    const sPan = Math.max(0.1, c.sensitivity.pan);
    const sZoom = Math.max(0.1, c.sensitivity.zoom);

    if (dragMode === 'pan') {
      const stepX = (dx / Math.max(zoom, 0.1)) * sPan;
      const stepY = (dy / Math.max(zoom, 0.1)) * sPan;
      engine?.setTranslation(translation.x + stepX, translation.y + stepY, translation.z);
    } else if (dragMode === 'rotate') {
      const k = 0.005 * sOrbit;
      const turntable = getControls().rotationModel === 'turntable';
      const dRx = turntable ? dy * k : dy * k;
      const dRy = turntable ? dx * k : dx * k;
      engine?.setRotation(rotation.x + dRx, rotation.y + dRy, rotation.z);
    } else if (dragMode === 'orbit') {
      const k = 0.005 * sOrbit;
      engine?.setRotation(rotation.x - dy * k, rotation.y - dx * k, rotation.z);
    } else if (dragMode === 'zoom') {
      const k = 0.01 * sZoom;
      const nz = Math.max(0.1, Math.min(3, zoom + (-dy) * k));
      engine?.setZoom(nz);
    } else if (dragMode === 'roll') {
      if (!c.allowRoll) return;
      const k = 0.005 * sOrbit;
      engine?.setRotation(rotation.x, rotation.y, rotation.z + dx * k);
    }
  }
function onPointerUp(e?: PointerEvent) {
    if (e && !isSceneEventTarget(e.target as any)) return;
    if (pinDraftActive) {
      const w = window.innerWidth, h = window.innerHeight;
      const x0 = Math.min(pinDraftStart.x, pinDraftEnd.x), x1 = Math.max(pinDraftStart.x, pinDraftEnd.x);
      const y0 = Math.min(pinDraftStart.y, pinDraftEnd.y), y1 = Math.max(pinDraftStart.y, pinDraftEnd.y);
      pinDraftActive = false;
      if ((x1-x0) * (y1-y0) >= 25) {
        const id = `pin-${Date.now()}`;
        const name = prompt('Pin name?', `Pin ${pins.length+1}`) || `Pin ${pins.length+1}`;
        pins = pins.concat([{ id, name, x0n: x0 / w, y0n: y0 / h, x1n: x1 / w, y1n: y1 / h }]);
        savePins(); schedulePinCounts();
      }
      e?.preventDefault();
      return;
    }
    if (marqueeActive) {
      marqueeActive = false;
      const t0r = perfStart('rects.get');
      const rects = engine?.getElementRects?.() || [];
      const rectsDt = perfEnd('rects.get', t0r);
      const minX = Math.min(marqueeStart.x, marqueeEnd.x);
      const maxX = Math.max(marqueeStart.x, marqueeEnd.x);
      const minY = Math.min(marqueeStart.y, marqueeEnd.y);
      const maxY = Math.max(marqueeStart.y, marqueeEnd.y);
const t0s = perfStart('marquee.scan');
      let hits: number[] = [];
      if (wasmMod && rects && rects.length) {
        const arr = new Float32Array(rects.length * 4);
        for (let k = 0; k < rects.length; k++) {
          const r: any = (rects as any[])[k];
          arr[k*4] = r.left; arr[k*4+1] = r.top; arr[k*4+2] = r.right; arr[k*4+3] = r.bottom;
        }
        const out = wasmMod.marquee_select(minX, minY, maxX, maxY, arr);
        hits = Array.from(out as unknown as ArrayLike<number>);
      } else {
        for (const r of rects as any[]) {
          const l = r.left, t = r.top, rr = r.right, b = r.bottom;
          if (rr >= minX && l <= maxX && b >= minY && t <= maxY) hits.push(r.i);
        }
      }
      const scanDt = perfEnd('marquee.scan', t0s);
      // combine selection based on modifier keys
      const ctrl = !!(e && (e.ctrlKey || e.metaKey));
      const alt = !!(e && e.altKey);
      let result: number[] = [];
      if (ctrl) {
        const set = new Set(selectedIndices);
        for (const h of hits) set.add(h);
        result = Array.from(set.values());
      } else if (alt) {
        const remove = new Set(hits);
        result = selectedIndices.filter(i => !remove.has(i));
      } else {
        result = hits;
      }
      selectedIndices = result.sort((a,b)=>a-b);
      engine?.setSelectedIndices(selectedIndices);
      // Log a compact snapshot occasionally
      const snap = perfSnapshot();
      if (snap && snap.length) {
        // Keep console output sparse
        if ((snap.find(x=>x.name==='marquee.scan')?.count || 0) % 10 === 0) {
          console.debug('[perf]', snap.reduce((m, c) => { m[c.name] = `${(c.totalMs/c.count).toFixed(2)}ms avg (${c.count})`; return m; }, {} as any));
        }
      }
    }
    dragMode = 'none';
    try { document.body.classList.remove('noselect-drag'); } catch {}
  }

  function getControls(): ControlsConfig {
    try {
      const s = localStorage.getItem('controlsConfig');
      if (s) return JSON.parse(s);
    } catch {}
    let cfg: ControlsConfig;
    controlsConfig.update((c)=> (cfg=c, c));
    return cfg!;
  }

  // Persist/restore camera state (layout, zoom, rotation, translation)
  function loadCameraState() {
    try {
      const s = localStorage.getItem('cameraState');
      if (!s) return;
      const st = JSON.parse(s);
      if (st && typeof st === 'object') {
        if (st.layout === 'table' || st.layout === 'sphere' || st.layout === 'helix' || st.layout === 'grid' || st.layout === 'custom') layout = st.layout;
        if (Number.isFinite(st.zoom)) zoom = Math.max(0.1, Math.min(3, Number(st.zoom)));
        if (st.rotation && Number.isFinite(st.rotation.x) && Number.isFinite(st.rotation.y) && Number.isFinite(st.rotation.z)) rotation = { x: Number(st.rotation.x), y: Number(st.rotation.y), z: Number(st.rotation.z) };
        if (st.translation && Number.isFinite(st.translation.x) && Number.isFinite(st.translation.y) && Number.isFinite(st.translation.z)) translation = { x: Number(st.translation.x), y: Number(st.translation.y), z: Number(st.translation.z) };
      }
    } catch {}
  }
  let saveCamTimer = 0;
  function saveCameraStateDebounced() {
    if (saveCamTimer) return; saveCamTimer = window.setTimeout(()=>{ saveCamTimer=0; try { localStorage.setItem('cameraState', JSON.stringify({ layout, zoom, rotation, translation })); } catch {} }, 300);
  }

  async function startEngine() {
    if (engineStarted) return;
    engineStarted = true;
    const wasm = await initWasm();
    wasmMod = wasm;
    if (typeof Worker !== 'undefined') startHitWorker();
    const scene = document.getElementById('scene') as HTMLDivElement;

    // Load persisted camera before engine start
    loadCameraState();

    engine = createEngine(wasm, scene, {
      onUpdate: (state) => {
        zoom = state.zoom;
        rotation = state.rotation;
        translation = state.translation;
        selected = state.selectedSymbol ?? null;
        // Pull scoreboard if a Wordl session is active
        try {
          const idx = (engine as any)?.getActiveWordlIndex?.();
          if (typeof idx === 'number' && idx >= 0) {
            wordlScore = (engine as any)?.getWordlScoreboard?.(idx) || null;
          } else { wordlScore = null; }
        } catch {}
        saveCameraStateDebounced();
      },
      onOrder: (i?: number) => { openOrderOverlay(typeof i === 'number' ? i : null); },
      onPerf: (m) => { perf = m; try { const snap = perfSnapshot(); const pick = new Set(['rects.get','marquee.scan','evt.pointermove','evt.wheel','rects.get_hover','hit.nearest','hit.worker','pins.counts']); perfExtra = snap.filter(s=>pick.has(s.name)).map(s=>({ name:s.name, avg: s.count ? s.totalMs/s.count : 0, count: s.count })).sort((a,b)=>b.avg-a.avg); } catch {} },
      onDrag: (s) => {
        dragActive = s.active;
        dragIndex = s.index;
        dragWorldX = s.x;
        dragWorldY = s.y;
        if (dragActive) {
          startOverlayAnim();
          try { document.body.classList.add('noselect-drag'); } catch {}
          // lock rotation for perfect grid alignment
          if (!rotationLockedDuringDrag && (Math.abs(rotation.x) > 1e-6 || Math.abs(rotation.y) > 1e-6 || Math.abs(rotation.z) > 1e-6)) {
            savedRotation = { ...rotation };
            engine?.setRotation(0, 0, 0);
            rotationLockedDuringDrag = true;
          }
          setTimeout(drawGrid, 0);
        } else {
          // drag ended in custom: snapshot positions for undo
          if (layout === 'custom') { pushCustomSnapshot(); customFuture = []; }
          stopOverlayAnim();
          try { document.body.classList.remove('noselect-drag'); } catch {}
          // restore rotation
          if (rotationLockedDuringDrag) {
            engine?.setRotation(savedRotation.x, savedRotation.y, savedRotation.z);
            rotationLockedDuringDrag = false;
          }
        }
      },
      onViewCard: (idx: number) => {
        showOrders = true;
        ordersFilterIndex = idx;
        const sym = String(completeTable[idx*5] ?? '');
        ordersFilterSymbol = sym;
      },
      onSelectToggle: (i: number) => {
        const set = new Set(selectedIndices);
        if (set.has(i)) set.delete(i); else set.add(i);
        selectedIndices = Array.from(set.values()).sort((a,b)=>a-b);
      },
    });
    engine.setPerfEnabled(perfEnabled);
    loadPins();
    refreshPinsNames();
    engine.setZoom(zoom);
    try { (engine as any).setWorkerEnabled?.(!fallbackMode); } catch {}
    engine.setLayout(layout);
    engine.setShowGrid(showGrid);
    engine.setSnapToGrid(snapToGrid);
    engine.setZoom(zoom);
    engine.setRotation(rotation.x, rotation.y, rotation.z);
    engine.setTranslation(translation.x, translation.y, translation.z);
    engine?.setShowWordlButton?.(showWordlButton);
    engine?.setWordlMultiplayerEnabled?.(wordlMultiplayer);
    engine?.setWordlDisplayName?.(wordlPlayerName || '');
    engine.start();
  }

  onMount(async () => {
    // expose scene input counters for tests
    try { (window as any).sceneInput = { wheel: 0, pointerDown: 0 }; } catch {}
    window.addEventListener('resize', handleResize);
    window.addEventListener('wheel', onWheel, { passive: false } as any);
    window.addEventListener('pointerdown', onPointerDown);
    window.addEventListener('pointermove', onPointerMove);
    window.addEventListener('pointerup', onPointerUp);
    // hide native context menu within scene while interacting
    const sceneEl = document.getElementById('scene');
    const preventCtx = (e: Event) => { if (sceneEl && (e.target as HTMLElement) && sceneEl.contains(e.target as Node)) e.preventDefault(); };
    window.addEventListener('contextmenu', preventCtx as any, { capture: true } as any);

    // Load persisted grid preferences
    loadGridPrefs();
    // Gate on login/token before engine start
    try {
      const token = localStorage.getItem('API_ACCESS_TOKEN') || '';
      showLogin = !token;
    } catch {}

    // Load UI preferences
    try {
      const pe = localStorage.getItem('perfEnabled');
      if (pe === '1' || pe === '0') perfEnabled = pe === '1';
      const fb = localStorage.getItem('fallbackMode');
      if (fb === '1' || fb === '0') fallbackMode = fb === '1';
      const gpu = localStorage.getItem('showGpuDemo');
      if (gpu === '1' || gpu === '0') showGpuDemo = gpu === '1';
      const cp = localStorage.getItem('conflictPolicy');
      if (cp === 'prompt' || cp === 'server' || cp === 'local') conflictPolicy.set(cp as any);
      else conflictPolicy.set(CONFLICT_POLICY_DEFAULT as any);
      const hv = localStorage.getItem('helpVisible');
      if (hv === '1' || hv === '0') helpVisible = hv === '1';
      const sg = localStorage.getItem('showGrid');
      if (sg === '1' || sg === '0') showGrid = sg === '1';
      const stg = localStorage.getItem('snapToGrid');
      if (stg === '1' || stg === '0') snapToGrid = stg === '1';
      const swb = localStorage.getItem('showWordlButton');
      if (swb === '1' || swb === '0') showWordlButton = swb === '1';
      const wzf = Number(localStorage.getItem('wordlZoomFactor')||'1');
      if (Number.isFinite(wzf) && wzf>=0.5 && wzf<=1.6) { wordlZoomFactor = wzf; try { engine?.setWordlZoomFactor?.(wordlZoomFactor); } catch {} }
      const wmp = localStorage.getItem('wordlMultiplayer');
      if (wmp === '1' || wmp === '0') wordlMultiplayer = wmp === '1';
      wordlPlayerName = (localStorage.getItem('wordlPlayerName') || '').slice(0,32);
      const sip = localStorage.getItem('showInfoPanel'); if (sip === '1' || sip === '0') showInfoPanel = sip === '1';
      const stb = localStorage.getItem('showTabs'); if (stb === '1' || stb === '0') showTabs = stb === '1';
      const sot = localStorage.getItem('showOrderTabs'); if (sot === '1' || sot === '0') showOrderTabs = sot === '1';
      const sat = localStorage.getItem('showArrangementTabs'); if (sat === '1' || sat === '0') showArrangementTabs = sat === '1';
      const ssr = localStorage.getItem('showSliderRadius'); if (ssr === '1' || ssr === '0') showSliderRadius = ssr === '1';
      const sso = localStorage.getItem('showSliderOrbit'); if (sso === '1' || sso === '0') showSliderOrbit = sso === '1';
      const ssp = localStorage.getItem('showSliderPan'); if (ssp === '1' || ssp === '0') showSliderPan = ssp === '1';
      const ssz = localStorage.getItem('showSliderZoom'); if (ssz === '1' || ssz === '0') showSliderZoom = ssz === '1';
    } catch {}

    // Keyboard shortcuts: G grid style, R radius preset, P perf overlay
    const keyHandler = (e: KeyboardEvent) => {
      if (e.target && (e.target as HTMLElement).tagName && /INPUT|TEXTAREA|SELECT/.test((e.target as HTMLElement).tagName)) return;
      // Suppress movement keys while typing in Wordl UI
      try { if ((engine as any)?.isWordlTyping?.()) return; } catch {}
      // Allow ESC to close Settings panel
      if ((e.key === 'Escape' || e.key === 'Esc') && showSettings) { e.preventDefault(); showSettings = false; return; }
      const k = e.key.toLowerCase();
      const ctrl = e.ctrlKey || e.metaKey;
      const c = getControls();
      if (k === 'g') { toggleGridStyle(); }
      else if ((e.key === 'Escape' || e.key === 'esc') && !showSettings) {
        // Close active Wordl if any
        try {
          const idx = (engine as any)?.getActiveWordlIndex?.();
          if (typeof idx === 'number' && idx >= 0) { (engine as any)?.closeWordlForIndex?.(idx); e.preventDefault(); return; }
        } catch {}
      }
      else if (k === 'r' && !e.shiftKey) {
        const presets = [0.5, 1.0, 1.5, 2.0];
        const idx = presets.findIndex((p) => Math.abs(p - fadeRadiusCards) < 1e-6);
        const next = presets[(idx + 1) % presets.length];
        fadeRadiusCards = next;
        saveGridPrefs();
        setTimeout(drawGrid, 0);
      } else if (k === 'p' && !e.shiftKey) { togglePerf(); }
      else if (k === '?' || (k === '/' && e.shiftKey)) { helpVisible = !helpVisible; try { localStorage.setItem('helpVisible', helpVisible ? '1' : '0'); } catch {} }
      else if (ctrl && k === 'z') { e.preventDefault(); undoCustom(); }
      else if (ctrl && (k === 'y' || (k === 'z' && e.shiftKey))) { e.preventDefault(); redoCustom(); }
      // Keyboard navigation
      else if (['arrowup','arrowdown','arrowleft','arrowright'].includes(k)) {
        e.preventDefault();
        const base = 20 / Math.max(zoom, 0.1) * Math.max(0.1, c.sensitivity.pan);
        const step = e.shiftKey ? base * 2 : base;
        if (k === 'arrowup') engine?.setTranslation(translation.x, translation.y + step, translation.z);
        if (k === 'arrowdown') engine?.setTranslation(translation.x, translation.y - step, translation.z);
        if (k === 'arrowleft') engine?.setTranslation(translation.x - step, translation.y, translation.z);
        if (k === 'arrowright') engine?.setTranslation(translation.x + step, translation.y, translation.z);
      } else if (k === '+' || k === '=' ) { e.preventDefault(); engine?.setZoom(Math.min(3, (zoom + 0.08 * Math.max(0.1, c.sensitivity.zoom)))); }
      else if (k === '-' || k === '_') { e.preventDefault(); engine?.setZoom(Math.max(0.1, (zoom - 0.08 * Math.max(0.1, c.sensitivity.zoom)))); }
      else if (k === 'q') { if (c.allowRoll) engine?.setRotation(rotation.x, rotation.y, rotation.z - 0.05 * Math.max(0.1, c.sensitivity.orbit)); }
      else if (k === 'e') { if (c.allowRoll) engine?.setRotation(rotation.x, rotation.y, rotation.z + 0.05 * Math.max(0.1, c.sensitivity.orbit)); }
      else if (k === 'f') { // simple focus: recenter
        engine?.setTranslation(0,0,translation.z);
      } else if (e.key === 'F4') { engine?.setTranslation(0,0,0); engine?.setRotation(0,0,0); engine?.setZoom(0.6); }
    };
    window.addEventListener('keydown', keyHandler);

    // Start engine only if not showing Login
    if (!showLogin) {
      await startEngine();
      // Check for invite in URL and prompt to join
      try {
        const url = new URL(window.location.href);
        const invId = url.searchParams.get('wordleSession');
        const invCard = Number(url.searchParams.get('cardIndex') || '-1');
        if (invId && Number.isFinite(invCard) && invCard >= 0 && invCard < elementCount) {
          if (confirm(`Join Wordl session on card #${invCard+1}?`)) {
            joinInviteIndex(invCard);
          }
        }
      } catch {}
    }

    // backend ping loop

    // backend ping loop
    async function poll() {
      try {
        const r = await pingServer();
        backendStatus = r.online ? 'online' : 'offline';
        backendMessage = r.online ? `HTTP ${r.status} ${Math.round(r.latencyMs || 0)}ms` : (r.error || 'unreachable');
        backendOnline.set(r.online);
        backendMessageStore.set(backendMessage);
        pendingOpsCount.set(getPending().length);
        if (r.online) {
          // attempt to flush pending offline changes (use current base precedence)
          let baseForFlush = API_BASE_DEFAULT;
          try { const ls = localStorage.getItem('API_BASE'); if (ls && ls.trim()) baseForFlush = ls.trim(); } catch {}
          try { const res = await flushPending(baseForFlush); pendingOpsCount.set(res.remaining); if (res.conflicts > 0) showSyncStatus = true; } catch {}
        }
      } catch (e: any) {
        backendStatus = 'offline';
        backendMessage = String(e?.message || e);
      }
      setTimeout(poll, 5000);
    }
    poll();

    // restore kitchen toggles to stores
    try {
      kitchenUseRelay.set(localStorage.getItem('kitchenUseRelay') !== '0');
      kitchenUseTest.set(localStorage.getItem('kitchenUseTest') === '1');
      kitchenTestLatencyMs.set(Number(localStorage.getItem('kitchenTestLatencyMs') || '300'));
      kitchenTestFailRate.set(Number(localStorage.getItem('kitchenTestFailRate') || '0'));
    } catch {}

    // ensure API base is visible in Settings
    let settingsApiBase = '';
    try { settingsApiBase = localStorage.getItem('API_BASE') || ''; } catch {}
    apiBaseLocal = settingsApiBase;

    // runtime events polling every 30s
    async function pollEvents() {
      try {
        const cursor = (localStorage.getItem('eventsCursor') || '') as string;
        const { listRuntimeEvents } = await import('./lib/api');
        const resp = await listRuntimeEvents({ sinceCursor: cursor, limit: 200, types: 'order,reservation,payment,kitchen_status', compact: 0 });
        if (Array.isArray(resp.items) && resp.items.length) {
          // append per type with simple dedupe by id
          const addDedup = (store, items) => {
            const current = new Set((items.map((it:any)=>it.id)).filter(Boolean));
            store.update((arr:any[]) => {
              const seen = new Set(arr.map((x:any)=>x.id));
              const adds = items.filter((x:any)=>!seen.has(x.id));
              const merged = arr.concat(adds);
              return merged.slice(-500);
            });
          };
          addDedup(runtimeOrders, resp.items.filter((e:any)=>e.type==='order'));
          addDedup(runtimeReservations, resp.items.filter((e:any)=>e.type==='reservation'));
          addDedup(runtimePayments, resp.items.filter((e:any)=>e.type==='payment'));
          addDedup(kitchenJobs, resp.items.filter((e:any)=>e.type==='kitchen_status'));
        }
        if (resp.nextCursor) { runtimeEventsCursor.set(resp.nextCursor); localStorage.setItem('eventsCursor', resp.nextCursor); }
      } catch {}
      setTimeout(pollEvents, 30000);
    }
    pollEvents();

    // runtime counts badges polling every 20s (window configurable)
    async function pollCounts() {
      try {
        const minutes = $runtimeCountsWindowMinutes;
        const since = Date.now() - Math.max(1, Number(minutes)||60) * 60 * 1000;
        const c = await listRuntimeCounts(since);
        runtimeCounts.set(c);
      } catch {}
      setTimeout(pollCounts, 20000);
    }
    // restore counts window from localStorage
    try { const m = Number(localStorage.getItem('countsWindowMinutes')||'60'); if (Number.isFinite(m) && m>0) runtimeCountsWindowMinutes.set(m); } catch {}
    pollCounts();

    return () => {
      window.removeEventListener('resize', handleResize);
      window.removeEventListener('keydown', keyHandler);
      window.removeEventListener('wheel', onWheel as any);
      window.removeEventListener('pointerdown', onPointerDown);
      window.removeEventListener('pointermove', onPointerMove);
      window.removeEventListener('pointerup', onPointerUp);
      window.removeEventListener('contextmenu', preventCtx as any, true as any);
      engine?.stop();
      try { hitWorker?.terminate(); } catch {}
    };
  });
</script>

<style>
  * { box-sizing: border-box; }
  #container { position: absolute; width: 100%; height: 100%; perspective: 1000px; }
  .scene { position: absolute; width: 100%; height: 100%; transform-style: preserve-3d; }
  .menu { position: absolute; bottom: 20px; width: 100%; text-align: center; z-index: 1000; }
  /* Ensure non-visible overlays never intercept clicks */
  /* Removed global override to avoid hiding login/register overlays */
  button {
    color: rgba(127,255,255,0.75);
    background: transparent;
    outline: 1px solid rgba(127,255,255,0.75);
    border: 0px;
    padding: 5px 10px;
    cursor: pointer;
    margin: 0 5px;
    text-transform: uppercase;
    font-size: 11px;
  }
  button.active { color: #000; background: rgba(0,255,255,0.75); }
  .sep { display:inline-block; margin: 0 8px; color: rgba(127,255,255,0.5); }
  .marquee {
    position: absolute;
    border: 1px solid rgba(127,255,255,0.8);
    background: rgba(0,255,255,0.15);
    pointer-events: none;
    z-index: 950;
  }
</style>

<div id="container">
  <div class="scene" id="scene"></div>
  <!-- Top-right share icon -->
  <div style="position:absolute;top:10px;right:10px;z-index:1400; display:flex; gap:6px;">
    <button title="Copy last invite link" on:click={() => {
      try {
        let link = lastInviteUrl;
        if (!link) {
          const url = new URL(window.location.href);
          const s = url.searchParams.get('wordleSession');
          const c = url.searchParams.get('cardIndex');
          if (s && c) {
            link = url.toString();
          }
        }
        if (link) {
          navigator.clipboard.writeText(link).then(()=>alert('Invite link copied')).catch(()=>alert(link));
        } else {
          alert('No invite link available yet. Use INVITE first.');
        }
      } catch (e) { alert('No invite link'); }
    }}>SHARE</button>
    <button title="Re-open last session" on:click={() => {
      try {
        const url = new URL(window.location.href);
        const s = url.searchParams.get('wordleSession');
        const c = Number(url.searchParams.get('cardIndex') || '-1');
        if (s && Number.isFinite(c) && c >= 0 && c < elementCount) {
          joinInviteIndex(c);
        } else {
          alert('No session in URL to reopen');
        }
      } catch { alert('No session to reopen'); }
    }}>REOPEN</button>
  </div>
  {#if showGpuDemo}
    <WebGLBoard visible={true} getPositions={getPositionsSafe} {zoom} {translation} rotation={rotation} perspectivePx={1000} />
  {/if}
</div>
{#if showGrid || dragActive}
  <canvas id="gridOverlay" style="position:absolute;top:0;left:0;width:100%;height:100%;pointer-events:none;z-index:900"></canvas>
  {#if dragActive && hudVisible}
    <div class="drag-hud" style="position:absolute;left:{hudLeft}px;top:{hudTop}px;z-index:905;padding:2px 6px;border:1px solid rgba(127,255,255,0.55);border-radius:4px;background:rgba(0,0,0,0.75);color:rgba(127,255,255,0.95);font-size:10px;pointer-events:none;">
      {hudText}
    </div>
  {/if}
{/if}
{#if marqueeActive}
  {#key `${marqueeStart.x},${marqueeStart.y},${marqueeEnd.x},${marqueeEnd.y}`}
    <div class="marquee" style="left:{Math.min(marqueeStart.x, marqueeEnd.x)}px; top:{Math.min(marqueeStart.y, marqueeEnd.y)}px; width:{Math.abs(marqueeEnd.x - marqueeStart.x)}px; height:{Math.abs(marqueeEnd.y - marqueeStart.y)}px;"></div>
  {/key}
{/if}
{#if selectionGroupRect}
  <div style="position:absolute; left:{selectionGroupRect.left}px; top:{selectionGroupRect.top}px; width:{selectionGroupRect.width}px; height:{selectionGroupRect.height}px; border: 1px dashed rgba(127,255,255,0.8); border-radius: 8px; pointer-events:none; z-index:920;"></div>
{/if}
{#if pinDraftActive}
  <div class="marquee" style="left:{Math.min(pinDraftStart.x, pinDraftEnd.x)}px; top:{Math.min(pinDraftStart.y, pinDraftEnd.y)}px; width:{Math.abs(pinDraftEnd.x - pinDraftStart.x)}px; height:{Math.abs(pinDraftEnd.y - pinDraftStart.y)}px; border-style: dashed;"></div>
{/if}
{#if showPins}
  {#each pins as p, i}
    {#key p.id}
    <div style="position:absolute;z-index:940;left:{Math.min(p.x0n,p.x1n)*window.innerWidth}px;top:{Math.min(p.y0n,p.y1n)*window.innerHeight}px;width:{Math.abs(p.x1n-p.x0n)*window.innerWidth}px;height:{Math.abs(p.y1n-p.y0n)*window.innerHeight}px;border:1px solid rgba(127,255,255,0.6);background:rgba(0,255,255,0.1);pointer-events:none;border-radius:4px;">
      <div style="position:absolute;left:4px;top:4px;background:rgba(0,0,0,0.6);border:1px solid rgba(127,255,255,0.5);padding:2px 6px;border-radius:4px;color:rgba(127,255,255,0.95);font-size:11px;">{p.name} ({pinCounts[i] || 0})</div>
    </div>
    {/key}
  {/each}
{/if}
{#if wordlScore}
  <div style="position:absolute;top:8px;left:50%;transform:translateX(-50%);z-index:1450;background:rgba(0,0,0,0.85);border:1px solid rgba(127,255,255,0.5);border-radius:6px;padding:6px 10px;color:rgba(127,255,255,0.95);font-size:11px;">
    <div style="font-weight:bold;text-align:center;margin-bottom:4px;">Wordl Scoreboard</div>
    <div>{#each Object.entries(wordlScore?.scores||{}) as [uid, sc]}<span style="margin-right:10px;">{(wordlScore?.participants||{})[uid] || uid}: <b>{sc}</b></span>{/each}</div>
    {#if wordlScore?.winner}
      <div style="margin-top:4px;color:rgba(0,255,127,0.95)">Winner: {(wordlScore?.participants||{})[wordlScore.winner] || wordlScore.winner}</div>
    {:else if wordlScore?.message}
      <div style="margin-top:4px;color:rgba(255,255,0,0.95)">{wordlScore.message}</div>
    {/if}
  </div>
{/if}
{#if showInfoPanel}
<InfoPanel
  on:toggle={onToggle}
  {layout}
  {elementCount}
  {zoom}
  {rotation}
  {translation}
  {selected}
  {zoomBoardOnFlip}
  {zoomCardOnClick}
  {flipOnPlace}
  {orderOnDoubleClick}
  {showCoordinates}
  {orderOnFront}
  {addOrderOnFlip}
  {showGrid}
  {snapToGrid}
  {reservOnFront}
  {reservOnFlip}
  {colorByZone}
  {zoneFilter}
  {showWordlButton}
/>
{/if}
<Status {backendStatus} message={backendMessage} />
{#if helpVisible}
  <div style="position:absolute;top:10px;left:10px;z-index:1300;background:rgba(0,0,0,0.85);border:1px solid rgba(127,255,255,0.5);border-radius:6px;padding:8px 10px;color:rgba(127,255,255,0.95);font-size:11px;line-height:1.5;max-width: 520px;">
    <div style="font-weight:bold;margin-bottom:4px;">Help & Controls</div>
    <div style="margin-bottom:4px;">• G: Toggle grid style (dots/lines), R: Cycle grid radius presets, P: Toggle performance overlay, ?: Toggle this help</div>
    <div style="margin-bottom:4px;">• Camera (Orbit preset): LMB rotate, MMB pan, Wheel zoom. Shift may emulate MMB depending on preset. Arrow keys pan; +/- zoom.</div>
    <div style="margin-bottom:4px;">• Custom layout drag: drag a card to move X/Y; Alt + drag adjusts Z; USE mode disables editing.</div>
    <div style="margin-bottom:4px;">• Wordl: click inside the Wordl ghost to focus typing. Letters type into current row; Backspace deletes prev; Delete clears current; Enter submits when 5 letters; invalid words show feedback.</div>
    <div style="margin-bottom:4px;">• Wordl ghosts: hold Meta/Ctrl + drag anywhere on the ghost to move freely; simple header drag clamps to viewport; Wordl Zoom slider scales ghosts.</div>
    <div>• SYNC: synchronize local pending changes with server; PROF: show FPS/frame/worker metrics; FALLBACK (in Settings): disable layout worker; GPU DEMO (in Settings): show WebGL demo overlay.</div>
  </div>
{/if}
{#if colorByZone}
  <ZoneLegend getZones={() => engine?.getZones() ?? []} />
{/if}
{#if perfEnabled}
  <PerfOverlay fps={perf.fps} frameMs={perf.frameMs} workerMs={perf.workerMs} workerMode={perf.workerMode} seqRate={perf.seqRate} extra={perfExtra} />
  {#if showPins}
    {@html (()=>{ schedulePinCounts(); return '' })()}
  {/if}
{/if}

<SaveLoadControls
  getPositions={getPositionsSafe}
  setCustomPositions={applyCustomPositions}
/>

{#if showSelectionOverlay}
  <SaveLoadOverlay
    visible={showSelectionOverlay}
    names={selectionNames}
    title="Selections"
    on:close={() => (showSelectionOverlay = false)}
    on:select={onSelectionOverlaySelect}
    on:delete={onSelectionOverlayDelete}
  />
{/if}

<div class="menu">
  {#if selectionMode}
    <div style="position:absolute;top:-44px;left:50%;transform:translateX(-50%);z-index:1100;background:rgba(0,0,0,0.85);border:1px solid rgba(127,255,255,0.5);padding:6px 8px;border-radius:6px;color:rgba(127,255,255,0.95);font-size:11px;display:flex;gap:8px;align-items:center;">
      <div>{selectedIndices.length} selected</div>
      <button on:click={() => { selectedIndices = []; engine?.setSelectedIndices([]); }}>Clear</button>
      <!-- Group actions -->
      <span style="margin-left:6px; opacity:0.8">GROUP:</span>
      <button on:click={() => groupReservAction()}>RESERV</button>
      <button on:click={() => groupZoneAction()}>ZONE</button>
      <button on:click={() => groupOrderAction()}>ORDER</button>
      <button on:click={() => groupClientAction()}>CLIENT</button>
      <!-- Named selections: name + SAVE / AS / LOAD -->
      <span class="sep">|</span>
      <input placeholder="Selection Name" bind:value={selectionName} maxlength="50" style="background:rgba(0,0,0,0.7); border:1px solid rgba(127,255,255,0.5); color:rgba(127,255,255,0.95); padding:3px 6px; border-radius:3px; font-size:11px;" />
      <button title="Save current selection" on:click={() => doSaveSelection()}>SAVE</button>
      <button title="Save As..." on:click={() => { const nm = prompt('Save selection as? (name)', selectionName || 'selection'); if (nm) { selectionName = nm; doSaveSelection(nm); } }}>AS</button>
      <button title="Load selection" on:click={async () => { await refreshSelectionNames(); showSelectionOverlay = true; }}>LOAD</button>
    </div>
  {/if}
  {#if showTabs}
    {#if showArrangementTabs}
      <button class:active={layout==='table'} on:click={() => setLayout('table')}>TABLE</button>
      <button class:active={layout==='sphere'} on:click={() => setLayout('sphere')}>SPHERE</button>
      <button class:active={layout==='helix'} on:click={() => setLayout('helix')}>HELIX</button>
      <button class:active={layout==='grid'} on:click={() => setLayout('grid')}>GRID</button>
      <button class:active={layout==='custom'} on:click={() => setLayout('custom')}>CUSTOM</button>
      {#if layout==='custom'}
        <button class:active={customUseMode} on:click={() => { customUseMode = !customUseMode; try { engine?.setCustomUseMode?.(customUseMode); } catch {} }} title="Toggle USE mode for custom layout">USE</button>
      {/if}
      <button on:click={resetAllPositions} title="Reset positions for current layout">RESET</button>
      <button on:click={() => { showEditor = !showEditor; showZone = false; }}>EDITOR</button>
    {/if}

    {#if showOrderTabs}
      <button style={canSettings ? '' : 'display:none'} on:click={() => { showZone = !showZone; showEditor = false; showZone=false; showServer=false; showClient=false; showReserv=false; }} data-testid="btn-zones">ZONES</button>
      <button on:click={() => { showClient = !showClient; showEditor = false; showZone=false; showServer=false; showClient=false; showReserv=false; }} data-testid="btn-client">CLIENT</button>
      <button class:active={selectionMode} on:click={() => { selectionMode = !selectionMode; engine?.setSelectionMode(selectionMode); if (!selectionMode) { selectedIndices = []; engine?.setSelectedIndices([]); engine?.setHighlightedIndices([]); } }} title="Toggle selection mode">SELECT</button>
      <button style={canReservations ? '' : 'display:none'} on:click={() => { showReserv = !showReserv; showEditor = false; showZone=false; showServer=false; showClient=false; }} data-testid="btn-reserv">
        RESERV {#if $runtimeCounts.reservations > 0}<span class="badge" title={`Count since last ${$runtimeCountsWindowMinutes} min`}>{$runtimeCounts.reservations}</span>{/if}
      </button>
      <button style={canOrders ? '' : 'display:none'} on:click={() => { showOrders = !showOrders; showEditor=false; showZone=false; showServer=false; showClient=false; showReserv=false; showPayments=false; }} data-testid="btn-orders">
        ORDERS {#if $runtimeCounts.orders > 0}<span class="badge" title={`Count since last ${$runtimeCountsWindowMinutes} min`}>{$runtimeCounts.orders}</span>{/if}
      </button>
      <button style={canOrders ? '' : 'display:none'} on:click={() => { showPayments = !showPayments; showEditor=false; showZone=false; showServer=false; showClient=false; showReserv=false; showOrders=false; }} data-testid="btn-payments">
        PAYMENTS {#if $runtimeCounts.payments > 0}<span class="badge pay" title={`Payments since last ${$runtimeCountsWindowMinutes} min`}>{$runtimeCounts.payments}</span>{/if}
      </button>
      <span class="sep">|</span>
      <button style={canKitchen ? '' : 'display:none'} on:click={() => { showKitchen = !showKitchen; showEditor=false; showZone=false; showServer=false; showClient=false; showReserv=false; showOrders=false; showPayments=false; }} data-testid="btn-kitchen">
        KITCHEN {#if Array.isArray($kitchenJobs) && $kitchenJobs.filter((j)=> (Date.now() - Number((j?.ts ?? j?.time ?? 0))) < 120000).length > 0}
          <span class="badge warn" title="Kitchen jobs in last 2 minutes">{$kitchenJobs.filter((j)=> (Date.now() - Number((j?.ts ?? j?.time ?? 0))) < 120000).length}</span>
        {/if}
      </button>
    {/if}

    <!-- Server and other utility tabs stay visible regardless of Orders toggle -->
    <button style={isAdmin ? '' : 'display:none'} on:click={() => { showServer = !showServer; showEditor = false; showZone=false; showClient=false; showReserv=false; }} data-testid="btn-server">SERVER</button>
    <button on:click={() => { showLegend = !showLegend; showEditor=false; showZone=false; showServer=false; showClient=false; showReserv=false; showOrders=false; showPayments=false; }} title="Badge legend (info)">ⓘ</button>
    <button data-testid="btn-settings" on:click={() => { showSettings = !showSettings; showEditor=false; showZone=false; showServer=false; showClient=false; showReserv=false; showOrders=false; showPayments=false; }} title={`Counts badges reflect events since last ${$runtimeCountsWindowMinutes} minutes. Adjust window in Settings.`}>
      SETTINGS
    </button>
    {#if $currentUser && ($currentUser.roles || []).includes('admin')}
      <button on:click={() => { showUserAdmin = !showUserAdmin; }} title="Manage users and permissions" data-testid="btn-user-admin">USER ADMIN</button>
    {/if}
    <button style={canPins ? '' : 'display:none'} on:click={() => { showPins = !showPins; if (showPins) schedulePinCounts(); }} class:active={showPins}>PINS</button>
    <!-- FALLBACK moved into Settings -->
    <button style={canRuntimeRead ? '' : 'display:none'} on:click={() => { showSyncStatus = !showSyncStatus; }}>SYNC STATUS</button>
    <button class:active={perfEnabled} on:click={togglePerf} title="Show performance overlay: FPS (higher is smoother), frame time (ms), worker time (ms), mode (MAIN/SAB/XFER)">PROF</button>
    <button on:click={syncNow} title="Synchronize local pending changes with server now">SYNC</button>

    {#if showArrangementTabs}
      <button on:click={toggleGridStyle}>GRID: {gridStyle.toUpperCase()}</button>
      <!-- GPU DEMO moved into Settings -->
    {/if}
  {:else}
    {#if showOrderTabs}
      <!-- When tabs hidden but Orders enabled, show Orders + Settings only -->
      <button on:click={() => { showZone = !showZone; showEditor = false; showServer=false; showClient=false; showReserv=false; }} data-testid="btn-zones">ZONES</button>
      <button on:click={() => { showClient = !showClient; showEditor = false; showZone=false; showServer=false; showClient=false; showReserv=false; }} data-testid="btn-client">CLIENT</button>
      <button class:active={selectionMode} on:click={() => { selectionMode = !selectionMode; engine?.setSelectionMode(selectionMode); if (!selectionMode) { selectedIndices = []; engine?.setSelectedIndices([]); engine?.setHighlightedIndices([]); } }} title="Toggle selection mode">SELECT</button>
      <button on:click={() => { showReserv = !showReserv; showEditor = false; showZone=false; showServer=false; showClient=false; }} data-testid="btn-reserv">
        RESERV {#if $runtimeCounts.reservations > 0}<span class="badge" title={`Count since last ${$runtimeCountsWindowMinutes} min`}>{$runtimeCounts.reservations}</span>{/if}
      </button>
      <button on:click={() => { showOrders = !showOrders; showEditor=false; showZone=false; showServer=false; showClient=false; showReserv=false; showPayments=false; }} data-testid="btn-orders">
        ORDERS {#if $runtimeCounts.orders > 0}<span class="badge" title={`Count since last ${$runtimeCountsWindowMinutes} min`}>{$runtimeCounts.orders}</span>{/if}
      </button>
      <button on:click={() => { showPayments = !showPayments; showEditor=false; showZone=false; showServer=false; showClient=false; showReserv=false; showOrders=false; }} data-testid="btn-payments">
        PAYMENTS {#if $runtimeCounts.payments > 0}<span class="badge pay" title={`Payments since last ${$runtimeCountsWindowMinutes} min`}>{$runtimeCounts.payments}</span>{/if}
      </button>
      <span class="sep">|</span>
      <button style={canSettings ? '' : 'display:none'} on:click={() => { showSettings = !showSettings; }} title="Open settings" data-testid="btn-settings">SETTINGS</button>
{:else}
      <button data-testid="btn-settings" on:click={() => { showSettings = !showSettings; }} title="Open settings">SETTINGS</button>
  {/if}
{/if}
</div>

<OrderOverlay bind:visible={orderOverlayVisible} on:close={() => { groupOrderMode = false; closeOrderOverlay(); }} orderIndex={currentOrderIndex} names={engine?.getNames() ?? []} zones={engine?.getZones() ?? []} on:submit={async (e) => { const detail = e.detail; try {
  if (groupOrderMode && Array.isArray(selectedIndices) && selectedIndices.length>0) {
    for (const idx of selectedIndices) { const payload = { ...(detail.data||{}), index: idx, symbol: String(completeTable[idx*5]||'') }; await saveOrder(payload); if (detail.kind === 'cook') { await sendToKitchen(payload); } }
  } else {
    await saveOrder(detail.data);
    if (detail.kind === 'cook') { await sendToKitchen(detail.data); }
  }
} catch {};
 groupOrderMode = false; closeOrderOverlay(); }} />
{#if showEditor}
  <EditorPanel on:close={() => showEditor=false} getNames={() => engine?.getNames() ?? []} setNames={(arr) => engine?.setNames(arr)} />
{/if}
{#if showZone}
  <ZonePanel on:close={() => showZone=false} getZones={() => engine?.getZones() ?? []} setZones={(arr) => engine?.setZones(arr)} setZoneColors={(m) => engine?.setZoneColors?.(m)} />
{/if}
{#if showServer}
  <ServerPanel on:close={() => showServer=false} />
{/if}
{#if showClient}
  <ClientPanel on:close={() => showClient=false} />
{/if}
{#if showReserv}
  <ReservPanel on:close={() => showReserv=false} />
{/if}
{#if showOrders}
  <OrdersPanel on:close={() => showOrders=false} filterIndex={ordersFilterIndex} filterSymbol={ordersFilterSymbol} getZones={() => engine?.getZones() ?? []} on:reservSummary={(e) => engine?.setReservationSummaries?.(e.detail.map)} />
{/if}
{#if showPayments}
  <PaymentsPanel on:close={() => showPayments=false} />
{/if}
{#if showLegend}
  <LegendOverlay on:close={() => showLegend=false} />
{/if}
{#if showUserAdmin}
  <AdminClaimsPanel on:close={() => { showUserAdmin = false; }}
    on:updated={async (e) => {
      try {
        const email = String((e.detail||{}).email||'');
        // If updated user is current user, refresh claims and re-apply permissions
        if (email && $currentUser && String($currentUser.account||'') === email) {
          const { authGetUserClaims } = await import('./lib/api');
          const c = await authGetUserClaims(email);
          currentUser.set({ account: email, roles: c.roles, scopes: c.scopes });
          const roles = Array.isArray(c.roles) ? c.roles : [];
          const scopes = Array.isArray(c.scopes) ? c.scopes : [];
          const isAdmin = roles.includes('admin');
          const allowOrder = isAdmin || scopes.includes('orders:write');
          const allowReserv = isAdmin || scopes.includes('reservations:write');
          const allowWordl = isAdmin || scopes.includes('game:play');
          try { engine?.setPermissions?.({ allowOrder, allowReserv, allowWordl }); } catch {}
        }
      } catch {}
    }} />
{/if}
{#if showSettings}
  <SettingsPanel
    {showGrid}
    {snapToGrid}
    {gridStyle}
    {fadeRadiusCards}
    viewZoom={zoom}
    setViewZoom={(v) => { const z = Math.max(0.1, Math.min(3, Number(v)||1)); engine?.setZoom(z); }}
    {perfEnabled}
    {helpVisible}
    fallbackMode={fallbackMode}
    setFallbackMode={(v)=> setFallbackMode(v)}
    showGpuDemo={showGpuDemo}
    setShowGpuDemo={(v)=> setShowGpuDemo(v)}
    showInfoPanel={showInfoPanel}
    setShowInfoPanel={(v) => { showInfoPanel = !!v; try{localStorage.setItem('showInfoPanel', v ? '1':'0');}catch{}; }}
    showTabs={showTabs}
    setShowTabs={(v) => { showTabs = !!v; try{localStorage.setItem('showTabs', v ? '1':'0');}catch{}; }}
    showOrderTabs={showOrderTabs}
    setShowOrderTabs={(v) => { showOrderTabs = !!v; try{localStorage.setItem('showOrderTabs', v ? '1':'0');}catch{}; }}
    showArrangementTabs={showArrangementTabs}
    setShowArrangementTabs={(v) => { showArrangementTabs = !!v; try{localStorage.setItem('showArrangementTabs', v ? '1':'0');}catch{}; }}
    showSliderRadius={showSliderRadius}
    setShowSliderRadius={(v) => { showSliderRadius = !!v; try{localStorage.setItem('showSliderRadius', v ? '1':'0');}catch{}; }}
    showSliderOrbit={showSliderOrbit}
    setShowSliderOrbit={(v) => { showSliderOrbit = !!v; try{localStorage.setItem('showSliderOrbit', v ? '1':'0');}catch{}; }}
    showSliderPan={showSliderPan}
    setShowSliderPan={(v) => { showSliderPan = !!v; try{localStorage.setItem('showSliderPan', v ? '1':'0');}catch{}; }}
    showSliderZoom={showSliderZoom}
    setShowSliderZoom={(v) => { showSliderZoom = !!v; try{localStorage.setItem('showSliderZoom', v ? '1':'0');}catch{}; }}
    showWordlButton={showWordlButton}
    setShowWordlButton={(v) => { showWordlButton = v; try{localStorage.setItem('showWordlButton', v ? '1':'0');}catch(e){}; engine?.setShowWordlButton?.(v); }}
    wordlMultiplayer={wordlMultiplayer}
    setWordlMultiplayer={(v) => { wordlMultiplayer = v; try{localStorage.setItem('wordlMultiplayer', v ? '1':'0');}catch(e){}; engine?.setWordlMultiplayerEnabled?.(v); }}
    wordlPlayerName={wordlPlayerName}
    setWordlPlayerName={(v) => { wordlPlayerName = (v||'').slice(0,32); try{localStorage.setItem('wordlPlayerName', wordlPlayerName);}catch(e){}; engine?.setWordlDisplayName?.(wordlPlayerName); }}
    wordlZoomFactor={wordlZoomFactor}
    setWordlZoomFactor={(v) => { wordlZoomFactor = Math.max(0.5, Math.min(1.6, Number(v)||1)); try{localStorage.setItem('wordlZoomFactor', String(wordlZoomFactor));}catch{}; engine?.setWordlZoomFactor?.(wordlZoomFactor); }}
    wordlLanguage={(localStorage.getItem('wordlLanguage') || 'en')}
    setWordlLanguage={(code) => { const c = String(code||'en'); try{localStorage.setItem('wordlLanguage', c);}catch{}; engine?.setWordlLanguage?.(c); }}
    onOpenWordlDicts={() => { showWordlDicts = true; }}
    viewPanX={translation.x}
    setViewPanX={(v) => { const x = Number(v)||0; engine?.setTranslation(x, translation.y, translation.z); }}
    viewPanY={translation.y}
    setViewPanY={(v) => { const y = Number(v)||0; engine?.setTranslation(translation.x, y, translation.z); }}
    viewOrbitX={rotation.x}
    setViewOrbitX={(v) => { const rx = Number(v)||0; engine?.setRotation(rx, rotation.y, rotation.z); }}
    viewOrbitY={rotation.y}
    setViewOrbitY={(v) => { const ry = Number(v)||0; engine?.setRotation(rotation.x, ry, rotation.z); }}
    apiBase={apiBaseLocal}
    setApiBase={(v) => { apiBaseLocal = v; try { if (v) localStorage.setItem('API_BASE', v); else localStorage.removeItem('API_BASE'); } catch {}; }}
    onTestConnection={() => { showConnTest = true; }}
    allowApiOverride={allowApiOverrideFlag}
    allowTokenOverride={API_TOKEN_ALLOW_LS_OVERRIDE}
    conflictPolicy={$conflictPolicy}
    setConflictPolicy={setConflictPolicyHandler}
    countsWindowMinutes={$runtimeCountsWindowMinutes}
    setCountsWindowMinutes={(v) => { const m = Math.max(1, Number(v)||60); runtimeCountsWindowMinutes.set(m); try{localStorage.setItem('countsWindowMinutes', String(m));}catch{}; }}
    setShowGrid={(v) => { showGrid = v; engine?.setShowGrid(v); try{localStorage.setItem('showGrid', v ? '1':'0');}catch{}; setTimeout(drawGrid,0);} }
    setSnapToGrid={(v) => { snapToGrid = v; engine?.setSnapToGrid(v); try{localStorage.setItem('snapToGrid', v ? '1':'0');}catch{}; }}
    setGridStyle={(s) => { gridStyle = s; try{localStorage.setItem('gridStyle', s);}catch{}; setTimeout(drawGrid,0);} }
    setFadeRadiusCards={(v) => { fadeRadiusCards = Math.max(0.3, Math.min(3, v)); try{localStorage.setItem('fadeRadiusCards', String(fadeRadiusCards));}catch{}; setTimeout(drawGrid,0);} }
    setPerfEnabled={(v) => { perfEnabled = v; engine?.setPerfEnabled?.(v); try{localStorage.setItem('perfEnabled', v ? '1':'0');}catch{}; }}
    setHelpVisible={(v) => { helpVisible = v; try{localStorage.setItem('helpVisible', v ? '1':'0');}catch{}; }}
    onClose={() => { showSettings = false; }}
    onOpenKitchen={() => { showKitchen = true; }}
  />
{/if}
{#if showWordlDicts}
  <WordlDictsPanel visible={true} on:close={() => { showWordlDicts = false; }} on:applied={() => { try { const c = localStorage.getItem('wordlLanguage') || 'en'; engine?.setWordlLanguage?.(c); } catch {} }} />
{/if}
{#if showKitchen}
  <KitchenPanel on:close={() => showKitchen = false} />
{/if}
{#if showPins}
  <PinsPanel pins={pins} counts={pinCounts} onRename={renamePin} onDelete={deletePin} onAddToggle={()=>{ pinAddMode=!pinAddMode; }} addMode={pinAddMode} onClose={() => { showPins = false; pinAddMode = false; }}
    configName={pinsConfigName} setConfigName={(v)=>{ pinsConfigName = v; }} serverNames={pinsServerNames} onSaveServer={savePinsToServer} onRefreshNames={refreshPinsNames} onLoadServer={loadPinsFromServer}
  />
{/if}
{#if showConnTest}
  <ConnectionTestOverlay apiBase={apiBaseLocal} on:close={() => showConnTest = false} />
{/if}
{#if showSyncStatus}
  <SyncStatus apiBase={apiBaseLocal || API_BASE_DEFAULT} />
{/if}
{#if showJoin}
  <JoinSessionOverlay visible={showJoin} elementCount={elementCount}
    on:cancel={() => { showJoin = false; }}
    on:join={(e) => { const { cardIndex } = e.detail || {}; showJoin = false; try { joinInviteIndex(Number(cardIndex)); } catch {} }} />
{/if}
{#if showRegister}
  <RegisterOverlay visible={showRegister}
    on:cancel={() => { showRegister = false; showLogin = true; alert('Failed to register'); }}
    on:fail={(e) => { showRegister = false; showLogin = true; alert('Failed to register'); }}
    on:success={async (e) => { try {
      const { email, password, playerName } = e.detail || {};
      showRegister = false;
      if (email && password) {
        try {
          const { authLogin, authGetUserClaims } = await import('./lib/api');
          const r = await authLogin(String(email), String(password));
          if (r?.token) localStorage.setItem('API_BEARER_TOKEN', r.token);
          if (playerName) { wordlPlayerName = String(playerName||'').slice(0,32); localStorage.setItem('wordlPlayerName', wordlPlayerName); engine?.setWordlDisplayName?.(wordlPlayerName); }
          // Load claims for newly registered+logged-in user (first-user may be admin)
          try {
            const c = await authGetUserClaims(String(email));
            currentUser.set({ account: String(email), roles: c.roles, scopes: c.scopes });
            const roles = Array.isArray(c.roles) ? c.roles : [];
            const scopes = Array.isArray(c.scopes) ? c.scopes : [];
            const isAdmin = roles.includes('admin');
            const allowOrder = isAdmin || scopes.includes('orders:write');
            const allowReserv = isAdmin || scopes.includes('reservations:write');
            const allowWordl = isAdmin || scopes.includes('game:play');
            try { engine?.setPermissions?.({ allowOrder, allowReserv, allowWordl }); } catch {}
          } catch {}
          showLogin = false; await startEngine();
          return;
        } catch {}
      }
      showLogin = true;
      alert('Registered. Please login.');
    } catch { showLogin = true; } }} />
{/if}
{#if showLogin}
  <LoginOverlay visible={showLogin} allowApiOverride={allowApiOverrideFlag}
    on:showRegister={() => { showLogin = false; showRegister = true; }}
    on:login={async (e) => {
      const { apiBase, token, playerName, account } = e.detail || {};
      try { if (apiBase && allowApiOverrideFlag) localStorage.setItem('API_BASE', apiBase); } catch {}
      try { if (token) localStorage.setItem('API_ACCESS_TOKEN', token); } catch {}
      try { if (playerName) { wordlPlayerName = String(playerName||'').slice(0,32); localStorage.setItem('wordlPlayerName', wordlPlayerName); engine?.setWordlDisplayName?.(wordlPlayerName); } } catch {}
      try {
        if (account) {
          const { authGetUserClaims } = await import('./lib/api');
          const c = await authGetUserClaims(String(account));
          currentUser.set({ account: String(account), roles: c.roles, scopes: c.scopes });
          const roles = Array.isArray(c.roles) ? c.roles : [];
          const scopes = Array.isArray(c.scopes) ? c.scopes : [];
          const isAdmin = roles.includes('admin');
          const allowOrder = isAdmin || scopes.includes('orders:write');
          const allowReserv = isAdmin || scopes.includes('reservations:write');
          const allowWordl = isAdmin || scopes.includes('game:play');
          try { engine?.setPermissions?.({ allowOrder, allowReserv, allowWordl }); } catch {}
        }
      } catch { currentUser.set({ account: String(account||'') }); }
      showLogin = false;
      await startEngine();
      // After login, consider joining via invite if present
      try {
        const url = new URL(window.location.href);
        const invId = url.searchParams.get('wordleSession');
        const invCard = Number(url.searchParams.get('cardIndex') || '-1');
        if (invId && Number.isFinite(invCard) && invCard >= 0 && invCard < elementCount) {
          if (confirm(`Join Wordl session on card #${invCard+1}?`)) {
            joinInviteIndex(invCard);
          }
        }
      } catch {}
    }} />
{/if}

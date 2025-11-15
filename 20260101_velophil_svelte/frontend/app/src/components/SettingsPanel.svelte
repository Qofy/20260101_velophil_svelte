<script lang="ts">
  import SaveLoadOverlay from './SaveLoadOverlay.svelte';
  export let showGrid: boolean = false;
  export let setShowGrid: (v: boolean) => void;
  export let snapToGrid: boolean = false;
  export let setSnapToGrid: (v: boolean) => void;
  export let gridStyle: 'dots' | 'lines' = 'dots';
  export let setGridStyle: (s: 'dots' | 'lines') => void;
  export let fadeRadiusCards: number = 1.0;
  export let setFadeRadiusCards: (v: number) => void;
  // View zoom (camera)
  export let viewZoom: number = 1.0;
  export let setViewZoom: (v: number) => void;
  // Wordl zoom factor (externalized control)
  export let wordlZoomFactor: number = 1.0;
  export let setWordlZoomFactor: (v: number) => void;
  export let wordlLanguage: string = 'en';
  export let setWordlLanguage: (code: string) => void;
  export let onOpenWordlDicts: (() => void) | undefined = undefined;
  export let perfEnabled: boolean = false;
  // Runtime toggles moved from menu
  export let fallbackMode: boolean = false;
  export let setFallbackMode: (v: boolean) => void;
  export let showGpuDemo: boolean = false;
  export let setShowGpuDemo: (v: boolean) => void;
  export let setPerfEnabled: (v: boolean) => void;
  export let helpVisible: boolean = false;
  export let setHelpVisible: (v: boolean) => void;
  export let onClose: (() => void) | undefined = undefined;
  export let onOpenKitchen: (() => void) | undefined = undefined;
  // API base URL configuration
  export let apiBase: string = '';
  export let setApiBase: (v: string) => void;
  export let onTestConnection: (() => void) | undefined = undefined;
  // Controls presets and options
  import { controlsConfig, type ControlsPreset } from '../lib/stores';
  let presetLocal: ControlsPreset = 'orbit';
  let zoomToCursorLocal = false;
  let rotationModelLocal: 'turntable' | 'trackball' = 'turntable';
  let orbitCenterLocal: 'world' | 'selection' | 'cursor' = 'world';
  let sensOrbit = 1.0, sensPan = 1.0, sensZoom = 1.0;
  let allowRollLocal = true;
  let emulateMMBLocal = true;

  // UI visibility toggles (controlled by parent)
  export let showInfoPanel: boolean = true;
  export let setShowInfoPanel: (v: boolean) => void;
  export let showTabs: boolean = true;
  export let setShowTabs: (v: boolean) => void;
  export let showOrderTabs: boolean = true;
  export let setShowOrderTabs: (v: boolean) => void;
  export let showArrangementTabs: boolean = true;
  export let setShowArrangementTabs: (v: boolean) => void;
  export let showSliderRadius: boolean = true;
  export let setShowSliderRadius: (v: boolean) => void;
  export let showSliderOrbit: boolean = true;
  export let setShowSliderOrbit: (v: boolean) => void;
  export let showSliderPan: boolean = true;
  export let setShowSliderPan: (v: boolean) => void;
  export let showSliderZoom: boolean = true;
  export let setShowSliderZoom: (v: boolean) => void;
  // Direct camera state
  export let viewPanX: number = 0;
  export let setViewPanX: (v: number) => void;
  export let viewPanY: number = 0;
  export let setViewPanY: (v: number) => void;
  export let viewOrbitX: number = 0;
  export let setViewOrbitX: (v: number) => void;
  export let viewOrbitY: number = 0;
  export let setViewOrbitY: (v: number) => void;
  function loadControls() {
    controlsConfig.subscribe((c) => {
      presetLocal = c.preset; zoomToCursorLocal = c.zoomToCursor; rotationModelLocal = c.rotationModel; orbitCenterLocal = c.orbitCenter;
      sensOrbit = c.sensitivity.orbit; sensPan = c.sensitivity.pan; sensZoom = c.sensitivity.zoom; allowRollLocal = c.allowRoll; emulateMMBLocal = c.emulateMMB;
    })();
  }
  function saveControls() {
    controlsConfig.set({
      preset: presetLocal,
      zoomToCursor: zoomToCursorLocal,
      rotationModel: rotationModelLocal,
      orbitCenter: orbitCenterLocal,
      sensitivity: { orbit: Math.max(0.1, sensOrbit), pan: Math.max(0.1, sensPan), zoom: Math.max(0.1, sensZoom) },
      allowRoll: allowRollLocal,
      emulateMMB: emulateMMBLocal,
    });
    try { localStorage.setItem('controlsConfig', JSON.stringify({ preset: presetLocal, zoomToCursor: zoomToCursorLocal, rotationModel: rotationModelLocal, orbitCenter: orbitCenterLocal, sensitivity: { orbit: sensOrbit, pan: sensPan, zoom: sensZoom }, allowRoll: allowRollLocal, emulateMMB: emulateMMBLocal })); } catch {}
  }
  // runtime counts window (minutes)
  export let countsWindowMinutes: number = 60;
  export let setCountsWindowMinutes: (v: number) => void;
  export let allowApiOverride: boolean = false;
  export let allowTokenOverride: boolean = false;
  // Settings presets
  let presetName = '';
  let presetNames: string[] = [];
  let showPresetOverlay = false;
  function loadPresetNames() { try { const s = localStorage.getItem('settingsPresets'); presetNames = s ? JSON.parse(s) : []; if (!Array.isArray(presetNames)) presetNames = []; } catch { presetNames = []; } }
  function savePresetNames() { try { localStorage.setItem('settingsPresets', JSON.stringify(Array.from(new Set(presetNames)))) } catch {} }
  function saveCurrentPreset(name: string) {
    const snap = {
      showGrid, snapToGrid, gridStyle, fadeRadiusCards, viewZoom,
      perfEnabled, helpVisible, showInfoPanel, showTabs, showOrderTabs, showArrangementTabs,
      viewPanX, viewPanY, viewOrbitX, viewOrbitY,
      showWordlButton, wordlMultiplayer, wordlPlayerName, wordlZoomFactor, wordlLanguage,
      apiBase, countsWindowMinutes, conflictPolicy,
      fallbackMode, showGpuDemo,
    };
    try { localStorage.setItem(`settingsPreset:${name}`, JSON.stringify(snap)); } catch {}
    if (!presetNames.includes(name)) { presetNames.push(name); savePresetNames(); }
  }
  function loadPreset(name: string) {
    try { const s = localStorage.getItem(`settingsPreset:${name}`); if (!s) return; const p = JSON.parse(s);
      setShowGrid(!!p.showGrid); setSnapToGrid(!!p.snapToGrid); setGridStyle(p.gridStyle==='lines'?'lines':'dots'); setFadeRadiusCards(Number(p.fadeRadiusCards)||1.0);
      setViewZoom(Number(p.viewZoom)||1.0);
      setPerfEnabled(!!p.perfEnabled); setHelpVisible(!!p.helpVisible);
      setShowInfoPanel(!!p.showInfoPanel); setShowTabs(!!p.showTabs); setShowOrderTabs(!!p.showOrderTabs); setShowArrangementTabs(!!p.showArrangementTabs);
      setViewPanX(Number(p.viewPanX)||0); setViewPanY(Number(p.viewPanY)||0); setViewOrbitX(Number(p.viewOrbitX)||0); setViewOrbitY(Number(p.viewOrbitY)||0);
      setShowWordlButton(!!p.showWordlButton); setWordlMultiplayer(!!p.wordlMultiplayer); setWordlPlayerName(String(p.wordlPlayerName||'')); setWordlZoomFactor(Number(p.wordlZoomFactor)||1.0);
      setWordlLanguage(String(p.wordlLanguage||'en'));
      setApiBase(String(p.apiBase||'')); setCountsWindowMinutes(Number(p.countsWindowMinutes)||60); setConflictPolicy(String(p.conflictPolicy||'prompt'));
      setFallbackMode(!!p.fallbackMode); setShowGpuDemo(!!p.showGpuDemo);
    } catch {}
  }
  function deletePreset(name: string) { try { localStorage.removeItem(`settingsPreset:${name}`); presetNames = presetNames.filter(n=>n!==name); savePresetNames(); } catch {} }
  // Wordl default dicts dropdown
  let presetDictNames: string[] = [];
  let defaultDictName: string = (localStorage.getItem('wordlDictsDefaultConfigName') || 'default');
  export let conflictPolicy: 'prompt' | 'server' | 'local' = 'prompt';
  export let setConflictPolicy: (v: 'prompt' | 'server' | 'local') => void;

  // Wordle toggle
  export let showWordlButton: boolean = false;
  export let setShowWordlButton: (v: boolean) => void;

  // Wordl multiplayer
  export let wordlMultiplayer: boolean = false;
  export let setWordlMultiplayer: (v: boolean) => void;
  export let wordlPlayerName: string = '';
  export let setWordlPlayerName: (v: string) => void;

  function resetDefaults() {
    setShowGrid(false);
    setSnapToGrid(false);
    setGridStyle('dots');
    setFadeRadiusCards(1.0);
    setPerfEnabled(false);
    setHelpVisible(false);
    setShowWordlButton(false);
  }
  function onShowGridChange(e: Event) { const t = e.target as HTMLInputElement; setShowGrid(!!t.checked); }
  function onSnapToGridChange(e: Event) { const t = e.target as HTMLInputElement; setSnapToGrid(!!t.checked); }
  function onPerfChange(e: Event) { const t = e.target as HTMLInputElement; setPerfEnabled(!!t.checked); }
  function onHelpChange(e: Event) { const t = e.target as HTMLInputElement; setHelpVisible(!!t.checked); }
  function onWordlChange(e: Event) { const t = e.target as HTMLInputElement; setShowWordlButton(!!t.checked); }
  function onWordlMultiChange(e: Event) { const t = e.target as HTMLInputElement; setWordlMultiplayer(!!t.checked); }
  function onWordlNameInput(e: Event) { const t = e.target as HTMLInputElement; setWordlPlayerName((t.value||'').slice(0,32)); }
  function onRadiusInput(e: Event) { const t = e.target as HTMLInputElement; const v = Number(t.value); setFadeRadiusCards(v); }
  function onWordlZoomInput(e: Event) { const t = e.target as HTMLInputElement; const v = Number(t.value); setWordlZoomFactor(Math.max(0.5, Math.min(1.6, isNaN(v) ? 1.0 : v))); }
  function onViewZoomInput(e: Event) { const t = e.target as HTMLInputElement; const v = Number(t.value); setViewZoom(Math.max(0.1, Math.min(3.0, isNaN(v) ? 1.0 : v))); }
  function onWordlLangChange(e: Event) { const t = e.target as HTMLSelectElement; setWordlLanguage(String(t.value||'en')); }
  function onCountsWindowInput(e: Event) { const t = e.target as HTMLInputElement; const v = Math.max(1, Math.min(1440, Number(t.value)||60)); setCountsWindowMinutes(v); }
  function onApiBaseInput(e: Event) { const t = e.target as HTMLInputElement; setApiBase((t.value || '').trim()); }
  // New toggles
  function onShowInfoPanelChange(e: Event) { const t = e.target as HTMLInputElement; setShowInfoPanel(!!t.checked); }
  function onFallbackChange(e: Event) { const t = e.target as HTMLInputElement; setFallbackMode(!!t.checked); }
  function onGpuDemoChange(e: Event) { const t = e.target as HTMLInputElement; setShowGpuDemo(!!t.checked); }
  function onShowTabsChange(e: Event) { const t = e.target as HTMLInputElement; setShowTabs(!!t.checked); }
  function onShowOrderTabsChange(e: Event) { const t = e.target as HTMLInputElement; setShowOrderTabs(!!t.checked); }
  function onShowArrangementTabsChange(e: Event) { const t = e.target as HTMLInputElement; setShowArrangementTabs(!!t.checked); }
  function onShowSliderRadiusChange(e: Event) { const t = e.target as HTMLInputElement; setShowSliderRadius(!!t.checked); }
  function onShowSliderOrbitChange(e: Event) { const t = e.target as HTMLInputElement; setShowSliderOrbit(!!t.checked); }
  function onShowSliderPanChange(e: Event) { const t = e.target as HTMLInputElement; setShowSliderPan(!!t.checked); }
  function onShowSliderZoomChange(e: Event) { const t = e.target as HTMLInputElement; setShowSliderZoom(!!t.checked); }
  // Token LS override (if allowed)
  let tokenLocal: string = '';
  let showToken: boolean = false;
  function loadToken() {
    try { const s = localStorage.getItem('API_ACCESS_TOKEN') || ''; tokenLocal = s; } catch {}
  }
  function onTokenInput(e: Event) {
    const t = e.target as HTMLInputElement; const v = (t.value || '').trim(); tokenLocal = v;
    try { if (allowTokenOverride) { if (v) localStorage.setItem('API_ACCESS_TOKEN', v); else localStorage.removeItem('API_ACCESS_TOKEN'); } } catch {}
  }
  function onPolicyChange(e: Event) { const t = e.target as HTMLSelectElement; const v = String(t.value||'prompt') as any; setConflictPolicy(v); }
  function onPanXInput(e: Event) { const t = e.target as HTMLInputElement; setViewPanX(Number(t.value)||0); }
  function onPanYInput(e: Event) { const t = e.target as HTMLInputElement; setViewPanY(Number(t.value)||0); }
  function onOrbitXInput(e: Event) { const t = e.target as HTMLInputElement; setViewOrbitX(Number(t.value)||0); }
  function onOrbitYInput(e: Event) { const t = e.target as HTMLInputElement; setViewOrbitY(Number(t.value)||0); }
</script>

<style>
.panel { position: absolute; top: 70px; right: 20px; width: 360px; max-height: 72vh; overflow-y: auto; background: rgba(0,0,0,0.9); border: 1px solid rgba(127,255,255,0.5); border-radius: 8px; padding: 12px; z-index: 1100; }
  .row { display: flex; align-items: center; justify-content: space-between; gap: 10px; margin: 6px 0; }
  .label { color: rgba(127,255,255,0.9); font-size: 12px; }
  .group { border-top: 1px solid rgba(127,255,255,0.2); margin-top: 8px; padding-top: 8px; }
  button, input[type="checkbox"], input[type="radio"], input[type="range"] {
    accent-color: rgba(0,255,255,0.75);
  }
  button { color: rgba(127,255,255,0.85); background: transparent; outline: 1px solid rgba(127,255,255,0.75); border: 0px; padding: 4px 8px; cursor: pointer; font-size: 11px; }
  /* Scrollbar: 45px wide, black-toned with thin dark yellow lines, always visible */
  .panel::-webkit-scrollbar { width: 45px; }
  .panel::-webkit-scrollbar-track { background: #000; border-radius: 6px; }
  .panel::-webkit-scrollbar-thumb {
    background: #000;
    border-left: 2px solid rgba(170,136,0,0.85);
    border-right: 2px solid rgba(170,136,0,0.85);
    box-shadow: inset 0 0 0 1px rgba(170,136,0,0.75);
    border-radius: 8px;
  }
  /* Firefox */
  .panel { scrollbar-color: rgba(170,136,0,0.85) #000; scrollbar-width: auto; }
</style>

<div class="panel" on:introstart={loadControls}>
  <div class="row"><div class="label">Show Grid</div><input type="checkbox" checked={showGrid} on:change={onShowGridChange} /></div>
  <div class="row"><div class="label">Snap to Grid</div><input type="checkbox" checked={snapToGrid} on:change={onSnapToGridChange} /></div>
  <div class="row"><div class="label">Grid Style</div>
    <div>
      <label style="color:rgba(127,255,255,0.85); font-size: 11px; margin-right: 8px;"><input type="radio" name="gridStyle" checked={gridStyle==='dots'} on:change={() => setGridStyle('dots')} /> dots</label>
      <label style="color:rgba(127,255,255,0.85); font-size: 11px;"><input type="radio" name="gridStyle" checked={gridStyle==='lines'} on:change={() => setGridStyle('lines')} /> lines</label>
    </div>
  </div>
  <div class="row"><div class="label">Grid Radius</div>
    <div style="color:rgba(127,255,255,0.85); font-size: 11px;">
      <input type="range" min="0.5" max="2.0" step="0.1" value={fadeRadiusCards} on:input={onRadiusInput} /> x{fadeRadiusCards.toFixed(1)}
    </div>
  </div>
  <div class="row"><div class="label">View Zoom</div>
    <div style="color:rgba(127,255,255,0.85); font-size: 11px;">
      <input type="range" min="0.1" max="3.0" step="0.05" value={viewZoom} on:input={onViewZoomInput} /> x{viewZoom.toFixed(2)}
    </div>
  </div>
  <div class="row"><div class="label">Wordl Zoom</div>
    <div style="color:rgba(127,255,255,0.85); font-size: 11px;">
      <input type="range" min="0.5" max="1.6" step="0.05" value={wordlZoomFactor} on:input={onWordlZoomInput} /> x{wordlZoomFactor.toFixed(2)}
    </div>
  </div>
  <div class="row"><div class="label">Wordl Language</div>
    <select value={wordlLanguage} on:change={onWordlLangChange}>
      <option value="en">English</option>
      <option value="es">Español</option>
    </select>
  </div>
  <div class="row"><div class="label">Wordl Dictionaries</div>
    <button on:click={() => onOpenWordlDicts?.()} title="Manage dictionary sources (URLs), save/load configs, and apply">Manage</button>
  </div>
  <div class="row"><div class="label">Default Dict Config</div>
    <select on:focus={() => { try { import('../lib/api').then(async (m)=>{ const r = await m.listWordlDictsConfigs(); const arr = Array.isArray(r?.names) ? r.names : []; presetDictNames = arr; }); } catch {} }} bind:value={defaultDictName} on:change={() => { try { localStorage.setItem('wordlDictsDefaultConfigName', defaultDictName); } catch {} }}>
      {#each presetDictNames as nm}
        <option value={nm}>{nm}</option>
      {/each}
    </select>
  </div>

  <div class="group">
    <div class="row"><div class="label">Direct Panning</div>
      <div style="flex:1; color:rgba(127,255,255,0.85); font-size: 11px; display:flex; gap:8px; align-items:center;">
        <label for="viewPanX">Pan X</label>
        <input id="viewPanX" type="range" min="-1200" max="1200" step="10" value={viewPanX} on:input={onPanXInput} />
        <label for="viewPanY">Pan Y</label>
        <input id="viewPanY" type="range" min="-1200" max="1200" step="10" value={viewPanY} on:input={onPanYInput} />
      </div>
    </div>
    <div class="row"><div class="label">Camera Orbit</div>
      <div style="flex:1; color:rgba(127,255,255,0.85); font-size: 11px; display:flex; gap:8px; align-items:center;">
        <label for="viewOrbitX">X</label>
        <input id="viewOrbitX" type="range" min="-1.57" max="1.57" step="0.01" value={viewOrbitX} on:input={onOrbitXInput} />
        <label for="viewOrbitY">Y</label>
        <input id="viewOrbitY" type="range" min="-3.14" max="3.14" step="0.01" value={viewOrbitY} on:input={onOrbitYInput} />
      </div>
    </div>
    <div class="row"><div class="label">Controls Preset</div>
      <select bind:value={presetLocal} on:change={saveControls}>
        <option value="orbit">Orbit (LMB orbit, MMB pan, Wheel zoom)</option>
        <option value="maya">Maya (Alt+LMB orbit, Alt+MMB pan, Alt+RMB dolly)</option>
        <option value="cad">CAD/Blender (MMB orbit, Shift+MMB pan, Ctrl+MMB zoom)</option>
      </select>
    </div>
    <div class="row">
      <div class="label">Rotation</div>
      <label style="color:rgba(127,255,255,0.85); font-size: 11px;"><input type="radio" name="rotModel" checked={rotationModelLocal==='turntable'} on:change={() => { rotationModelLocal='turntable'; saveControls(); }} /> turntable</label>
      <label style="color:rgba(127,255,255,0.85); font-size: 11px;"><input type="radio" name="rotModel" checked={rotationModelLocal==='trackball'} on:change={() => { rotationModelLocal='trackball'; saveControls(); }} /> trackball</label>
    </div>
    <div class="row">
      <div class="label">Orbit Center</div>
      <select bind:value={orbitCenterLocal} on:change={saveControls}>
        <option value="world">World</option>
        <option value="selection">Selection</option>
        <option value="cursor">Cursor</option>
      </select>
    </div>
    <div class="row">
      <div class="label">Zoom to Cursor</div>
      <input type="checkbox" bind:checked={zoomToCursorLocal} on:change={saveControls} />
    </div>
    <!-- Removed duplicate sensitivity sliders: Camera (Orbit), Panning, Zoom -->
    <div class="row">
      <div class="label">Other</div>
      <label style="color:rgba(127,255,255,0.85); font-size: 11px;"><input type="checkbox" bind:checked={allowRollLocal} on:change={saveControls} /> Allow roll</label>
      <label style="color:rgba(127,255,255,0.85); font-size: 11px;"><input type="checkbox" bind:checked={emulateMMBLocal} on:change={saveControls} /> Emulate MMB</label>
    </div>
  </div>

  <div class="group">
    <div class="row"><div class="label">Perf Overlay</div><input type="checkbox" checked={perfEnabled} on:change={onPerfChange} title="Show FPS/frame time/worker time overlay" /></div>
    <div class="row"><div class="label">Help Overlay</div><input type="checkbox" checked={helpVisible} on:change={onHelpChange} title="Show keys and controls summary" /></div>
    <div class="row"><div class="label">Fallback (no worker)</div><input type="checkbox" checked={fallbackMode} on:change={onFallbackChange} title="Force main-thread engine (no layout worker). Useful for compatibility; may reduce performance." /></div>
    <div class="row"><div class="label">GPU DEMO</div><input type="checkbox" checked={showGpuDemo} on:change={onGpuDemoChange} title="Show WebGL demo overlay for GPU rendering/compute." /></div>
    <div class="row"><div class="label">Interactive Periodic Table</div><input type="checkbox" checked={showInfoPanel} on:change={onShowInfoPanelChange} /></div>
    <div class="row"><div class="label">TABS</div><input type="checkbox" checked={showTabs} on:change={onShowTabsChange} /></div>
    <div class="row"><div class="label">Orders TAB</div><input type="checkbox" checked={showOrderTabs} on:change={onShowOrderTabsChange} /></div>
    <div class="row"><div class="label">Arrangements TAB</div><input type="checkbox" checked={showArrangementTabs} on:change={onShowArrangementTabsChange} /></div>
    <div class="row"><div class="label">Slider Radius</div><input type="checkbox" checked={showSliderRadius} on:change={onShowSliderRadiusChange} /></div>
    <div class="row"><div class="label">Show button Play Wordle</div><input type="checkbox" checked={showWordlButton} on:change={onWordlChange} /></div>
    <div class="row"><div class="label">Wordle Multiplayer</div><input type="checkbox" checked={wordlMultiplayer} on:change={onWordlMultiChange} /></div>
    <div class="row"><div class="label">Player name</div><input type="text" value={wordlPlayerName} on:input={onWordlNameInput} placeholder="Player" style="flex:1;" /></div>
  </div>

<div class="group" on:introstart={loadToken}>
    <div class="row" style="justify-content: space-between;">
      <div class="label">Kitchen</div>
      <button on:click={() => onOpenKitchen?.()}>Configure Kitchen</button>
    </div>
    <div class="row" style="margin-top:6px;">
      <div class="label">API Base URL</div>
      <input type="text" placeholder="http://127.0.0.1:8080" value={apiBase} on:input={onApiBaseInput} style="width: 100%;" disabled={!allowApiOverride} />
      <button on:click={() => { if (allowApiOverride) setApiBase(''); }} disabled={!allowApiOverride} title="Reset to default (clear local override)">Reset</button>
      <button on:click={() => onTestConnection?.()}>Test</button>
    </div>
    <div class="row" style="margin-top:6px;">
      <div class="label">Access Token</div>
      <input type={showToken ? 'text' : 'password'} placeholder="token" value={tokenLocal} on:input={onTokenInput} style="width: 100%;" disabled={!allowTokenOverride} />
      <button on:click={() => { showToken = !showToken; }} disabled={!allowTokenOverride} title="Show/Hide token">{showToken ? 'Hide' : 'Show'}</button>
      <button on:click={() => { if (allowTokenOverride) { tokenLocal=''; try{localStorage.removeItem('API_ACCESS_TOKEN')}catch{} } }} disabled={!allowTokenOverride} title="Clear token override">Clear</button>
    </div>
    <div class="row" style="margin-top:6px;">
      <div class="label">Conflict Policy</div>
      <select value={conflictPolicy} on:change={onPolicyChange}>
        <option value="prompt">Prompt on conflict</option>
        <option value="server">Keep server (discard local)</option>
        <option value="local">Keep local (overwrite server)</option>
      </select>
    </div>
    <div class="row" style="margin-top:6px;">
      <div class="label">Counts window (min)</div>
      <input type="number" min="1" max="1440" step="1" value={countsWindowMinutes} on:input={onCountsWindowInput} />
    </div>
  </div>

  <div class="group" on:introstart={loadPresetNames}>
    <div class="row"><div class="label">Settings Preset</div>
      <input type="text" placeholder="Preset name" bind:value={presetName} style="flex:1;" />
      <button on:click={() => { const nm = (presetName||'').trim() || prompt('Save settings as?', 'default') || ''; if (nm) { presetName = nm; saveCurrentPreset(nm); } }}>Save As</button>
      <button on:click={() => { loadPresetNames(); showPresetOverlay = true; }}>Load</button>
      <button on:click={() => { const nm = (presetName||'').trim(); if (!nm) return; if (confirm(`Delete settings preset "${nm}"?`)) { deletePreset(nm); } }}>DEL</button>
    </div>
  </div>

  <div class="row" style="justify-content: flex-end; gap: 6px; margin-top: 10px;">
    <button on:click={resetDefaults}>Reset</button>
    {#if onClose}
      <button on:click={() => onClose?.()}>Close</button>
    {/if}
  </div>
</div>

<SaveLoadOverlay visible={showPresetOverlay} names={presetNames} title="Settings Presets" on:close={() => showPresetOverlay = false} on:select={(e) => { const n = e && e.detail ? e.detail.name : (e && e.name); if (n) { loadPreset(String(n)); showPresetOverlay=false; } }} on:delete={(e) => { const n = e && e.detail ? e.detail.name : (e && e.name); if (n && confirm(`Delete preset "${n}"?`)) { deletePreset(String(n)); loadPresetNames(); } }} />

<script lang="ts">
  import { createEventDispatcher, onMount } from 'svelte';
  import SaveLoadOverlay from './SaveLoadOverlay.svelte';
  import { listKitchen, loadKitchen, saveKitchen, setActiveKitchen, getActiveKitchen, kitchenTestPrint, getKitchenTestPrinters } from '../lib/api';

  let configName = '';
  let urls: string[] = [];
  let allConfigs: string[] = [];
  let message = '';
  let newUrl = '';
  let showOverlay = false;
  let activeName: string | null = null;
  let useRelay = true;
  let useTest = false;
  let testLatency = 300;
  let testFailRate = 0;
  let testPrinters: { name: string; url: string }[] = [];
  function loadToggles() {
    try {
      useRelay = localStorage.getItem('kitchenUseRelay') !== '0';
      useTest = localStorage.getItem('kitchenUseTest') === '1';
      testLatency = Number(localStorage.getItem('kitchenTestLatencyMs') || '300');
      testFailRate = Number(localStorage.getItem('kitchenTestFailRate') || '0');
    } catch {}
  }
  function saveToggles() {
    try {
      localStorage.setItem('kitchenUseRelay', useRelay ? '1':'0');
      localStorage.setItem('kitchenUseTest', useTest ? '1':'0');
      localStorage.setItem('kitchenTestLatencyMs', String(testLatency));
      localStorage.setItem('kitchenTestFailRate', String(testFailRate));
    } catch {}
  }
  const dispatch = createEventDispatcher();
  function close() { dispatch('close'); }
  function onKey(e: KeyboardEvent) { if (e.key === 'Escape') close(); }

  async function refreshConfigs() { const res = await listKitchen(); allConfigs = res.names; }
  function addUrl() { const u = (newUrl||'').trim(); if (u) { urls = [...urls, u]; newUrl=''; saveActive(); } }
  function handleNewKeydown(e: KeyboardEvent) { if (e.key === 'Enter') addUrl(); }
  function removeUrl(i: number) { urls = urls.filter((_, idx) => idx !== i); saveActive(); }
  function saveActive() { try { localStorage.setItem('kitchenActiveUrls', JSON.stringify(urls)); } catch {} }
  async function doSave() {
    if (!configName.trim()) { message = 'Enter a name'; return; }
    await saveKitchen(configName.trim(), urls); message = 'Saved'; await refreshConfigs();
  }
  async function doLoad(n: string) {
    const res = await loadKitchen(n); if (res.data) { urls = [...res.data]; message = 'Loaded'; saveActive(); } else { message='Not found'; }
  }
  async function doTestPrint() {
    const sample = { index: 1, items:[{name:'Test Soup', qty:1}], ts: Date.now() };
    try {
      await kitchenTestPrint(sample, { latencyMs: testLatency, failRate: testFailRate });
      message = 'Test sent';
    } catch (e: any) {
      message = 'Test failed: ' + (e?.message || e);
    }
  }
  async function applyActive() {
    try {
      if (activeName) { await setActiveKitchen(activeName); message = 'Active set to ' + activeName; }
    } catch (e: any) {
      message = 'Set active failed: ' + (e?.message || e);
    }
  }
  onMount(async () => {
    refreshConfigs();
    try{ const s=localStorage.getItem('kitchenActiveUrls'); if (s) urls = JSON.parse(s);}catch{};
    loadToggles();
    try { const a = await getActiveKitchen(); activeName = a?.name ?? null; } catch {}
    try { testPrinters = await getKitchenTestPrinters(); } catch {}
    window.addEventListener('keydown', onKey); return () => window.removeEventListener('keydown', onKey);
  });
  function onOverlaySelect(e: CustomEvent) { const n = (e as any).detail?.name as string; if (n) { doLoad(n); showOverlay = false; } }
  async function onOverlayDelete(e: CustomEvent) { const n = (e as any).detail?.name as string; if (n) { const m = await import('../lib/api'); await m.deleteKitchenConfig(n); await refreshConfigs(); } }
</script>

<style>
  .panel { position: absolute; top: 100px; left: 50%; transform: translateX(-50%); width: 60%; max-width: 800px; bottom: 100px; overflow-y: auto; background: rgba(0,0,0,0.9); border: 1px solid rgba(127,255,255,0.5); border-radius: 8px; padding: 20px; z-index: 1200; }
  input[type="text"] { background: rgba(0,0,0,0.8); border: 1px solid rgba(127,255,255,0.5); color: rgba(127,255,255,0.9); padding: 5px 8px; border-radius: 3px; font-size: 11px; }
  .hdr { display:flex; gap:6px; align-items:center; margin-bottom: 8px; flex-wrap: wrap; }
  .list { margin-top: 8px; display: grid; grid-template-columns: 1fr auto; gap: 6px; }
  button { color: rgba(127,255,255,0.75); background: transparent; outline: 1px solid rgba(127,255,255,0.75); border: 0px; padding: 5px 8px; cursor: pointer; font-size: 11px; }
</style>

<div class="panel">
  <button title="Close" aria-label="Close" style="position:absolute;top:6px;right:10px;" on:click={close}>✖</button>
  <div class="hdr">
    <input placeholder="Kitchen Config Name" bind:value={configName} maxlength="50" />
    <button on:click={doSave}>SAVE AS</button>
    <button on:click={() => { showOverlay = true; refreshConfigs(); }}>LOAD</button>
    <input placeholder="Add printer URL..." bind:value={newUrl} on:keydown={handleNewKeydown} style="min-width:320px;" />
    <button on:click={addUrl}>Add</button>
    {#if message}<span style="color: rgba(127,255,255,0.8)">{message}</span>{/if}
  </div>
  <div class="list">
    {#each urls as u, i}
      <input type="text" bind:value={urls[i]} on:input={saveActive} />
      <button on:click={() => removeUrl(i)}>remove</button>
    {/each}
  </div>

  <div style="margin-top:12px; border-top:1px solid rgba(127,255,255,0.2); padding-top:8px;">
    <div style="color: rgba(0,255,255,0.85); font-weight:bold;">Relay Settings</div>
    <div style="display:flex; align-items:center; gap:8px; margin:6px 0;">
      <label style="color: rgba(127,255,255,0.85); font-size: 11px;"><input type="checkbox" bind:checked={useRelay} on:change={saveToggles} /> Use server relay</label>
      <label style="color: rgba(127,255,255,0.85); font-size: 11px;"><input type="checkbox" bind:checked={useTest} on:change={saveToggles} /> Enable test relay</label>
    </div>
    <div style="display:flex; align-items:center; gap:8px; margin:6px 0;">
      <label style="color: rgba(127,255,255,0.85); font-size: 11px;">Latency (ms) <input type="number" min="0" step="50" bind:value={testLatency} on:change={saveToggles} /></label>
      <label style="color: rgba(127,255,255,0.85); font-size: 11px;">Fail rate 0..1 <input type="number" min="0" max="1" step="0.05" bind:value={testFailRate} on:change={saveToggles} /></label>
      <button on:click={doTestPrint}>Test Print (Server)</button>
    </div>
  </div>

  <div style="margin-top:12px; border-top:1px solid rgba(127,255,255,0.2); padding-top:8px;">
    <div style="color: rgba(0,255,255,0.85); font-weight:bold;">Active Kitchen</div>
    <div style="display:flex; gap:8px; align-items:center;">
      <select bind:value={activeName} style="min-width:200px;">
        <option value="">(none)</option>
        {#each allConfigs as n}
          <option value={n}>{n}</option>
        {/each}
      </select>
      <button on:click={applyActive}>Set Active</button>
    </div>
  </div>
</div>
<SaveLoadOverlay
  visible={showOverlay}
  names={allConfigs}
  title="Kitchen Printer Configurations"
  on:close={() => showOverlay=false}
  on:select={onOverlaySelect}
  on:delete={onOverlayDelete}
/>

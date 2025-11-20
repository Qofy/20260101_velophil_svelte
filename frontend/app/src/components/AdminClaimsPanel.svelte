<script lang="ts">
  import { createEventDispatcher, onMount } from 'svelte';
  const dispatch = createEventDispatcher();

  type User = { email: string; name?: string };
  let users: User[] = [];
  let selectedEmail: string = '';
  let roles: string[] = [];
  let scopesText: string = '';
  let message = '';
  let loading = false;

  // Card-level scopes helpers
  let ordersCards = '';
  let reservationsCards = '';
  let gameCards = '';
  function parseExistingCardsScopes(scopes: string[]) {
    function extract(prefix: string) {
      for (const s of scopes) { if (s.startsWith(prefix+':cards:')) return s.slice((prefix+':cards:').length); }
      return '';
    }
    ordersCards = extract('orders');
    reservationsCards = extract('reservations');
    gameCards = extract('game');
  }
  function cardsChange() {
    try {
      const lines = String(scopesText||'').split(/\n+/);
      const filtered = lines.filter((ln)=>!(ln.startsWith('orders:cards:') || ln.startsWith('reservations:cards:') || ln.startsWith('game:cards:')));
      const out: string[] = filtered.filter(Boolean);
      if (ordersCards.trim()) out.push('orders:cards:'+ordersCards.trim());
      if (reservationsCards.trim()) out.push('reservations:cards:'+reservationsCards.trim());
      if (gameCards.trim()) out.push('game:cards:'+gameCards.trim());
      scopesText = out.join('\n');
    } catch {}
  }

  async function loadUsers() {
    try {
      const { authListUsers } = await import('../lib/api');
      const list = await authListUsers();
      users = Array.isArray(list) ? list.map((u:any)=>({ email: String(u?.email||u?.id||''), name: String(u?.name||'') })) : [];
    } catch (e:any) { message = `Failed to load users: ${e?.message||e}`; }
  }
  async function loadClaims(email: string) {
    try {
      const { authGetUserClaims } = await import('../lib/api');
      const c = await authGetUserClaims(email);
      roles = Array.isArray(c.roles) ? c.roles.slice() : [];
      const scopes = Array.isArray(c.scopes) ? c.scopes.slice() : [];
      scopesText = scopes.join('\n');
      parseExistingCardsScopes(scopes);
    } catch (e:any) { message = `Failed to load claims: ${e?.message||e}`; }
  }
  async function saveClaims() {
    if (!selectedEmail) return;
    loading = true; message = '';
    try {
      const { authUpdateUserClaims } = await import('../lib/api');
      const scopes = scopesText.split(/\n|,|\s+/).map(s=>s.trim()).filter(Boolean);
      await authUpdateUserClaims(selectedEmail, roles.slice(), scopes);
      message = 'Saved';
      // Notify parent so it can refresh current user's claims and gating without re-login
      dispatch('updated', { email: selectedEmail, roles: roles.slice(), scopes });
    } catch (e:any) { message = `Save failed: ${e?.message||e}`; }
    loading = false;
  }
  function close(){ dispatch('close'); }

  function toggleRole(r: string) {
    const i = roles.indexOf(r);
    if (i>=0) roles.splice(i,1); else roles.push(r);
    roles = roles.slice();
  }

  onMount(loadUsers);
</script>

<style>
  .overlay{ position:fixed; inset:0; background:rgba(0,0,0,0.9); z-index:3000; display:flex; align-items:center; justify-content:center; }
  .panel{ width:780px; max-width:96vw; background:rgba(0,0,0,0.96); border:1px solid rgba(127,255,255,0.5); border-radius:10px; padding:12px; color:rgba(127,255,255,0.92); }
  .title{ display:flex; justify-content:space-between; align-items:center; font-weight:bold; color:rgba(0,255,255,0.95); margin-bottom:8px; }
  .cols{ display:grid; grid-template-columns: 240px 1fr; gap:10px; }
  .list{ max-height:50vh; overflow:auto; border:1px solid rgba(127,255,255,0.2); border-radius:6px; }
  .row{ display:flex; align-items:center; justify-content:space-between; padding:6px 8px; border-bottom:1px solid rgba(127,255,255,0.1); cursor:pointer; }
  .row:hover{ background:rgba(0,255,255,0.06); }
  .active{ background:rgba(0,255,255,0.12); }
  .sec{ border:1px solid rgba(127,255,255,0.2); border-radius:6px; padding:8px; }
  .lbl{ font-size:11px; color:rgba(127,255,255,0.85); margin-bottom:4px; }
  textarea{ width:100%; min-height:160px; background:rgba(0,0,0,0.85); border:1px solid rgba(127,255,255,0.5); color:rgba(127,255,255,0.95); border-radius:4px; padding:6px; font-size:12px; }
  button{ color: rgba(127,255,255,0.85); background: transparent; outline: 1px solid rgba(127,255,255,0.75); border: 0px; padding: 6px 10px; cursor: pointer; font-size: 11px; text-transform: uppercase; border-radius: 4px; }
  .msg{ font-size:11px; color:rgba(255,200,0,0.95); margin-top:6px; }
</style>

<div class="overlay" role="dialog" aria-label="User Admin">
  <div class="panel">
    <div class="title">
      <div>User Admin / Claims</div>
      <div style="display:flex; gap:8px; align-items:center;">
        <button on:click={loadUsers} title="Reload user list">Reload</button>
        <button on:click={close}>✖</button>
      </div>
    </div>
    <div class="cols">
      <div class="list">
        {#each users as u}
          <div class="row {selectedEmail===u.email?'active':''}"
               on:click={() => { selectedEmail = u.email; loadClaims(u.email); }}
               role="button" tabindex="0" on:keydown={(e)=>{ if (e.key==='Enter' || e.key===' ') { selectedEmail = u.email; loadClaims(u.email); } }}
               data-testid="user-row" data-email={u.email}>
            <div>{u.email}</div>
            <div style="opacity:0.75; font-size:11px;">{u.name}</div>
          </div>
        {/each}
      </div>
      <div class="sec">
        {#if !selectedEmail}
          <div class="lbl">Select a user to edit claims</div>
        {:else}
          <div class="lbl">Roles</div>
          <div style="display:flex; gap:8px; margin-bottom:8px;">
            <label><input type="checkbox" checked={roles.includes('user')} on:change={()=>toggleRole('user')} /> user</label>
            <label><input type="checkbox" checked={roles.includes('admin')} on:change={()=>toggleRole('admin')} /> admin</label>
          </div>
          <div class="lbl">Card-specific access (comma/ranges; 1-based)</div>
          <div style="display:grid; grid-template-columns: 160px 1fr; gap:6px; margin-bottom:8px; align-items:center;">
            <div class="lbl">Orders: cards</div>
            <input type="text" bind:value={ordersCards} on:input={cardsChange} placeholder="e.g. 1,2,5-9" data-testid="claims-orders-cards" />
            <div class="lbl">Reservations: cards</div>
            <input type="text" bind:value={reservationsCards} on:input={cardsChange} placeholder="e.g. 3,4,10-12" data-testid="claims-reservations-cards" />
            <div class="lbl">Game: cards</div>
            <input type="text" bind:value={gameCards} on:input={cardsChange} placeholder="e.g. 7,8,20-25" data-testid="claims-game-cards" />
          </div>
          <div class="lbl">Scopes (one per line)</div>
          <textarea bind:value={scopesText} data-testid="claims-scopes" placeholder="e.g. runtime:read
orders:write
reservations:write
game:play
kitchen:manage
settings:write
orders:cards:1,2,5-9
reservations:cards:3,4,10-12
game:cards:7,8,20-25"></textarea>
          <div style="margin-top:8px; display:flex; gap:8px; justify-content:flex-end;">
            <button on:click={saveClaims} disabled={loading} data-testid="btn-save-claims">{loading ? 'Saving…' : 'Save'}</button>
          </div>
          {#if message}
            <div class="msg">{message}</div>
          {/if}
        {/if}
      </div>
  </div>
  </div>
</div>

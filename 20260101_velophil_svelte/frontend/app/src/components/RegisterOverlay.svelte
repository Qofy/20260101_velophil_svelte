<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  export let visible: boolean = false;
  const dispatch = createEventDispatcher();

  let playerName: string = '';
  let email: string = '';
  let note: string = '';
  let password: string = '';
  let message: string = '';
  let running = false;

  async function submit() {
    message = '';
    if (!email.trim() || !playerName.trim() || password.length < 6) {
      message = 'Please enter name, email and password (min 6 chars)';
      return;
    }
    running = true;
    try {
      localStorage.setItem('pendingRegistration', JSON.stringify({ playerName, email, note, ts: Date.now() }));
    } catch {}
    try {
      const { authRegister } = await import('../lib/api');
      const result = await authRegister(email, password);
      if (playerName) {
        localStorage.setItem('wordlPlayerName', playerName.slice(0, 32));
      }
      // Backend returns {user: {id, email, roles}}
      const userData = result.user || result;
      dispatch('success', userData);
    } catch (e: any) {
      message = e?.message || String(e);
      dispatch('fail', { error: e?.message || String(e) });
    } finally {
      running = false;
    }
  }

  function cancel() { dispatch('close'); }
</script>

<style>
  .overlay { position: fixed; inset: 0; background: rgba(0,0,0,0.92); z-index: 3000; display:flex; align-items:center; justify-content:center; }
  .panel { width: 460px; max-width: 92vw; background: rgba(0,0,0,0.95); border:1px solid rgba(127,255,255,0.5); border-radius: 10px; padding: 16px 14px; color: rgba(127,255,255,0.92); box-shadow: 0 0 22px rgba(0,255,255,0.25); }
  .title { font-weight: bold; font-size: 14px; color: rgba(0,255,255,0.95); margin-bottom: 10px; }
  .row { display:flex; flex-direction:column; gap:6px; margin: 8px 0; }
  label { font-size: 11px; color: rgba(127,255,255,0.85); }
  input[type="text"], input[type="email"], textarea { background: rgba(0,0,0,0.85); border: 1px solid rgba(127,255,255,0.5); color: rgba(127,255,255,0.95); padding: 6px 8px; border-radius: 4px; font-size: 12px; }
  textarea { min-height: 70px; }
  .actions { display:flex; gap:8px; justify-content:flex-end; margin-top:12px; }
  .msg { margin-top:6px; color: rgba(255,180,0,0.9); font-size: 11px; }
  button { color: rgba(127,255,255,0.85); background: transparent; outline: 1px solid rgba(127,255,255,0.75); border: 0px; padding: 6px 10px; cursor: pointer; font-size: 11px; text-transform: uppercase; border-radius: 4px; }
</style>

{#if visible}
  <div class="overlay">
    <div class="panel">
      <div class="title">Register / Request Access</div>
      <div class="row">
        <label for="regPlayer">Player Name</label>
        <input id="regPlayer" type="text" bind:value={playerName} placeholder="Player" maxlength="32" data-testid="register-player-name" />
      </div>
      <div class="row">
        <label for="regEmail">Email</label>
        <input id="regEmail" type="email" bind:value={email} placeholder="you@example.com" data-testid="register-email" />
      </div>
      <div class="row">
        <label for="regPassword">Password</label>
        <input id="regPassword" type="password" bind:value={password} placeholder="password" data-testid="register-password" />
      </div>
      <div class="row">
        <label for="regNote">Note (optional)</label>
        <textarea id="regNote" bind:value={note} placeholder="Anything we should know?" data-testid="register-note" />
      </div>
      {#if message}<div class="msg">{message}</div>{/if}
      <div class="actions" style="justify-content: space-between;">
        <div>
          <button on:click={() => dispatch('switchToLogin')} data-testid="btn-show-login">Back to Login</button>
        </div>
        <div style="display:flex; gap:8px;">
          <button on:click={cancel} disabled={running} data-testid="btn-register-cancel">Cancel</button>
          <button on:click={submit} disabled={running} data-testid="btn-register-submit">{running ? 'Submitting…' : 'Submit'}</button>
        </div>
      </div>
    </div>
  </div>
{/if}

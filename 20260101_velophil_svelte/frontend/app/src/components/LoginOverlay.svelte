<script lang="ts">
  import { createEventDispatcher } from 'svelte';

  export let visible: boolean = true;
  export let allowApiOverride: boolean = false;
  let loginRunning = false;
  let loginMsg: string = '';

  // Pre-fill from localStorage (if any)
  let apiBaseLocal: string = '';
  let tokenLocal: string = '';
  let playerName: string = '';
  let userOrEmail: string = '';
  let password: string = '';

  const dispatch = createEventDispatcher();

  function loadFromStorage() {
    try {
      apiBaseLocal = localStorage.getItem('API_BASE') || '';
      tokenLocal = localStorage.getItem('API_ACCESS_TOKEN') || '';
      if (!tokenLocal) {
        const envTok = String((import.meta as any).env?.VITE_API_ACCESS_TOKEN || '').trim();
        if (envTok) tokenLocal = envTok;
      }
      playerName = localStorage.getItem('wordlPlayerName') || '';
      userOrEmail = localStorage.getItem('lastLoginAccount') || '';
    } catch {}
  }

  async function saveAndLogin() {
    loginMsg = '';
    loginRunning = true;
    try {
      if (allowApiOverride) {
        if (apiBaseLocal) localStorage.setItem('API_BASE', apiBaseLocal);
        else localStorage.removeItem('API_BASE');
      }
      if (userOrEmail.trim() && password.trim()) {
        // Backend auth with email/password
        const { authLogin } = await import('../lib/api');
        const user = await authLogin(userOrEmail.trim(), password);
        // Save player name if provided
        if (playerName) localStorage.setItem('wordlPlayerName', playerName.slice(0,32));
        if (userOrEmail) localStorage.setItem('lastLoginAccount', userOrEmail);
        dispatch('success', user);
      } else {
        loginMsg = 'Please enter email and password';
      }
    } catch (e: any) {
      loginMsg = `Login failed: ${e?.message || e}`;
    } finally {
      loginRunning = false;
    }
  }

  function continueGuest() {
    // No token set; proceed anyway (engine may run in offline/demo mode)
    dispatch('success', null);
  }

  $: if (visible) loadFromStorage();
</script>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(0,0,0,0.92);
    z-index: 3000;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .panel {
    width: 420px;
    max-width: 92vw;
    background: rgba(0,0,0,0.95);
    border: 1px solid rgba(127,255,255,0.5);
    border-radius: 10px;
    padding: 16px 14px;
    color: rgba(127,255,255,0.92);
    box-shadow: 0 0 22px rgba(0,255,255,0.25);
  }
  .title { font-weight: bold; font-size: 14px; color: rgba(0,255,255,0.95); margin-bottom: 10px; }
  .row { display: flex; flex-direction: column; gap: 6px; margin: 8px 0; }
  label { font-size: 11px; color: rgba(127,255,255,0.85); }
  input[type="text"], input[type="password"] {
    background: rgba(0,0,0,0.85);
    border: 1px solid rgba(127,255,255,0.5);
    color: rgba(127,255,255,0.95);
    padding: 6px 8px;
    border-radius: 4px;
    font-size: 12px;
  }
  .actions { display: flex; gap: 8px; justify-content: flex-end; margin-top: 12px; }
  .msg { font-size: 11px; color: rgba(255,180,0,0.95); margin-top: 4px; }
  button {
    color: rgba(127,255,255,0.85);
    background: transparent;
    outline: 1px solid rgba(127,255,255,0.75);
    border: 0px;
    padding: 6px 10px;
    cursor: pointer;
    font-size: 11px;
    text-transform: uppercase;
    border-radius: 4px;
  }
  .hint { font-size: 11px; color: rgba(127,255,255,0.7); margin-top: 4px; }
</style>

{#if visible}
  <div class="overlay">
    <div class="panel">
      <div class="title">Login to Continue</div>
      <div class="row">
        <label for="loginPlayer">Player Name (for multiplayer presence)</label>
        <input id="loginPlayer" type="text" bind:value={playerName} placeholder="Player" maxlength="32" data-testid="login-player-name" />
      </div>
      <div class="row">
        <label for="loginUser">Email or Username</label>
        <input id="loginUser" type="text" bind:value={userOrEmail} placeholder="you@example.com or username" data-testid="login-username" />
      </div>
      <div class="row">
        <label for="loginPassword">Password</label>
        <input id="loginPassword" type="password" bind:value={password} placeholder="password" data-testid="login-password" />
      </div>
      {#if allowApiOverride}
        <div class="row">
          <label for="loginApiBase">API Base URL</label>
          <input id="loginApiBase" type="text" bind:value={apiBaseLocal} placeholder="http://127.0.0.1:8080" data-testid="login-api-base" />
        </div>
      {/if}
      <div class="row">
        <label for="loginToken">Access Token</label>
        <input id="loginToken" type="password" bind:value={tokenLocal} placeholder="token (optional if using password)" data-testid="login-access-token" />
        <div class="hint">Use password above for backend auth, or paste an access token directly. You can change it later in Settings.</div>
      </div>
      <div class="actions" style="justify-content: space-between;">
        <div>
          <button on:click={() => dispatch('switchToRegister')} data-testid="btn-show-register">Register</button>
        </div>
        <div style="display:flex; gap:8px;">
          <button on:click={continueGuest} title="Proceed without token (limited)" data-testid="btn-continue-guest">Continue as Guest</button>
          <button on:click={saveAndLogin} disabled={loginRunning} data-testid="btn-login">{loginRunning ? 'Logging in…' : 'Login'}</button>
        </div>
      </div>
      {#if loginMsg}
        <div class="msg">{loginMsg}</div>
      {/if}
    </div>
  </div>
{/if}

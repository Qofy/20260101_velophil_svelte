<script lang="ts">
  import { onMount } from 'svelte';
  import { initWasm, type Wasm } from './lib/wasm';
  import { createEngine, type Engine, type Layout } from './lib/engine';
  import { currentUser, backendOnline } from './lib/stores';
  import LoginOverlay from './components/LoginOverlay.svelte';
  import RegisterOverlay from './components/RegisterOverlay.svelte';
  import Status from './components/Status.svelte';

  let wasm: Wasm | null = null;
  let engine: Engine | null = null;
  let layout: Layout = 'table';
  let showLogin = false;
  let showRegister = false;
  let loading = true;
  let user: any = null;

  // Subscribe to current user
  currentUser.subscribe(value => {
    user = value;
  });

  onMount(async () => {
    try {
      // Initialize WASM
      wasm = await initWasm();

      // Create engine with dummy scene
      const scene = { updatePositions: (positions: Float32Array) => {
        console.log('Updated positions:', positions.length);
      }};
      engine = createEngine(wasm, scene, { itemCount: 100 });
      engine.setLayout('table');

      // Check backend health
      const response = await fetch('/health');
      backendOnline.set(response.ok);

      // Check if user is logged in
      await checkAuth();

      loading = false;
    } catch (error) {
      console.error('Initialization error:', error);
      loading = false;
    }
  });

  async function checkAuth() {
    try {
      const response = await fetch('/api/auth/me', {
        credentials: 'include'
      });
      if (response.ok) {
        const data = await response.json();
        currentUser.set(data);
      }
    } catch (error) {
      console.error('Auth check failed:', error);
    }
  }

  async function handleLogout() {
    try {
      await fetch('/api/auth/logout', {
        method: 'POST',
        credentials: 'include'
      });
      currentUser.set(null);
      user = null;
    } catch (error) {
      console.error('Logout failed:', error);
    }
  }

  function setLayout(l: Layout) {
    layout = l;
    engine?.setLayout(l);
  }

  function handleLoginSuccess(event: CustomEvent) {
    showLogin = false;
    currentUser.set(event.detail);
    user = event.detail;
  }

  function handleRegisterSuccess(event: CustomEvent) {
    showRegister = false;
    currentUser.set(event.detail);
    user = event.detail;
  }
</script>

<main>
  {#if loading}
    <div class="loading">
      <p>Initializing...</p>
    </div>
  {:else}
    <header>
      <h1>VeloAssure Template</h1>
      <div class="header-right">
        <Status />
        {#if user}
          <span class="user-info">
            {user.email}
            {#if user.roles?.includes('admin')}
              <span class="badge">Admin</span>
            {/if}
          </span>
          <button on:click={handleLogout}>Logout</button>
        {:else}
          <button on:click={() => showLogin = true}>Login</button>
          <button on:click={() => showRegister = true}>Register</button>
        {/if}
      </div>
    </header>

    <div class="content">
      {#if user}
        <section class="demo">
          <h2>WASM Layout Demo</h2>
          <div class="controls">
            <button class:active={layout==='table'} on:click={() => setLayout('table')}>Table</button>
            <button class:active={layout==='sphere'} on:click={() => setLayout('sphere')}>Sphere</button>
            <button class:active={layout==='helix'} on:click={() => setLayout('helix')}>Helix</button>
            <button class:active={layout==='grid'} on:click={() => setLayout('grid')}>Grid</button>
          </div>
          <div class="demo-info">
            <p>Current Layout: <strong>{layout}</strong></p>
            <p>WASM Module: <strong>✓ Loaded</strong></p>
            <p>Engine Status: <strong>{engine ? '✓ Running' : '✗ Not initialized'}</strong></p>
          </div>
        </section>

        <section class="api-info">
          <h2>Backend Integration</h2>
          <ul>
            <li>✓ Authentication with HttpOnly Cookies</li>
            <li>✓ PASETO v4.local tokens</li>
            <li>✓ Token refresh with rotation</li>
            <li>✓ Security headers (CSP, HSTS)</li>
            <li>✓ CORS configuration</li>
            <li>✓ Database with backup & replication</li>
          </ul>
        </section>
      {:else}
        <div class="welcome">
          <h2>Welcome to VeloAssure</h2>
          <p>A clean, secure starter template with:</p>
          <ul>
            <li>Rust + Actix-web backend</li>
            <li>Svelte + WASM frontend</li>
            <li>Cookie-based authentication</li>
            <li>PostgreSQL replication support</li>
            <li>Automated backups</li>
          </ul>
          <p>Please login or register to continue.</p>
        </div>
      {/if}
    </div>
  {/if}

  {#if showLogin}
    <LoginOverlay
      visible={true}
      on:close={() => showLogin = false}
      on:success={handleLoginSuccess}
      on:switchToRegister={() => { showLogin = false; showRegister = true; }}
    />
  {/if}

  {#if showRegister}
    <RegisterOverlay
      visible={true}
      on:close={() => showRegister = false}
      on:success={handleRegisterSuccess}
      on:switchToLogin={() => { showRegister = false; showLogin = true; }}
    />
  {/if}
</main>

<style>
  :global(body) {
    margin: 0;
    padding: 0;
    font-family: system-ui, -apple-system, sans-serif;
    background: #1a1a1a;
    color: #e0e0e0;
  }

  main {
    min-height: 100vh;
    display: flex;
    flex-direction: column;
  }

  .loading {
    display: flex;
    align-items: center;
    justify-content: center;
    min-height: 100vh;
    font-size: 1.2rem;
  }

  header {
    background: #2d2d2d;
    padding: 1rem 2rem;
    display: flex;
    justify-content: space-between;
    align-items: center;
    border-bottom: 2px solid #3d3d3d;
  }

  h1 {
    margin: 0;
    font-size: 1.5rem;
    color: #4a9eff;
  }

  .header-right {
    display: flex;
    align-items: center;
    gap: 1rem;
  }

  .user-info {
    font-size: 0.9rem;
    padding: 0.5rem 1rem;
    background: #3d3d3d;
    border-radius: 4px;
  }

  .badge {
    display: inline-block;
    padding: 0.2rem 0.5rem;
    margin-left: 0.5rem;
    background: #4a9eff;
    color: white;
    border-radius: 3px;
    font-size: 0.75rem;
    font-weight: bold;
  }

  button {
    padding: 0.5rem 1rem;
    background: #4a9eff;
    color: white;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.9rem;
    transition: background 0.2s;
  }

  button:hover {
    background: #3a8eef;
  }

  button.active {
    background: #2a7edf;
    box-shadow: inset 0 2px 4px rgba(0,0,0,0.3);
  }

  .content {
    flex: 1;
    padding: 2rem;
    max-width: 1200px;
    width: 100%;
    margin: 0 auto;
  }

  .welcome {
    text-align: center;
    padding: 3rem 2rem;
  }

  .welcome h2 {
    color: #4a9eff;
    margin-bottom: 1.5rem;
  }

  .welcome ul {
    text-align: left;
    max-width: 500px;
    margin: 2rem auto;
    list-style: none;
    padding: 0;
  }

  .welcome li {
    padding: 0.5rem 0;
    padding-left: 1.5rem;
    position: relative;
  }

  .welcome li::before {
    content: "✓";
    position: absolute;
    left: 0;
    color: #4a9eff;
  }

  .demo {
    background: #2d2d2d;
    padding: 2rem;
    border-radius: 8px;
    margin-bottom: 2rem;
  }

  .demo h2 {
    margin-top: 0;
    color: #4a9eff;
  }

  .controls {
    display: flex;
    gap: 0.5rem;
    margin: 1.5rem 0;
    flex-wrap: wrap;
  }

  .demo-info {
    margin-top: 1.5rem;
    padding: 1rem;
    background: #3d3d3d;
    border-radius: 4px;
  }

  .demo-info p {
    margin: 0.5rem 0;
  }

  .api-info {
    background: #2d2d2d;
    padding: 2rem;
    border-radius: 8px;
  }

  .api-info h2 {
    margin-top: 0;
    color: #4a9eff;
  }

  .api-info ul {
    list-style: none;
    padding: 0;
  }

  .api-info li {
    padding: 0.5rem 0;
    padding-left: 1.5rem;
    position: relative;
  }

  .api-info li::before {
    content: "✓";
    position: absolute;
    left: 0;
    color: #4a9eff;
    font-weight: bold;
  }
</style>

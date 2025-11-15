<script lang="ts">
  export interface NavAction {
    label: string;
    icon?: 'map' | 'search' | 'menu' | string;
  }

  export interface Brand {
    label: string;
    logo?: string;
  }

  export let brand: Brand;
  export let actions: NavAction[] = [];
</script>

<nav class="nav">
  <div class="brand">
    <div class="logo" aria-hidden="true"></div>
    <span>{brand?.label}</span>
  </div>
  <div class="actions">
    {#each actions as action}
      <button type="button" class="nav-btn" aria-label={action.label}>
        {#if action.icon === 'map'}
          <svg viewBox="0 0 24 24" role="img" aria-hidden="true"><path d="M3 5.5 9 3l6 2.5 6-2.5v15L15 20 9 17.5 3 20z"/></svg>
        {:else if action.icon === 'search'}
          <svg viewBox="0 0 24 24" role="img" aria-hidden="true"><path d="M10.5 3a7.5 7.5 0 1 1 0 15 7.5 7.5 0 0 1 0-15zm0 2a5.5 5.5 0 1 0 0 11 5.5 5.5 0 0 0 0-11zm7.44 9.56 4.5 4.5-1.88 1.88-4.5-4.5z"/></svg>
        {:else if action.icon === 'menu'}
          <svg viewBox="0 0 24 24" role="img" aria-hidden="true"><path d="M3 6h18v2H3zm0 5h18v2H3zm0 5h18v2H3z"/></svg>
        {/if}
        <span>{action.label}</span>
      </button>
    {/each}
  </div>
</nav>

<style>
  .nav {
    position: sticky;
    top: 0;
    z-index: 12;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.75rem 2rem;
    background: rgba(255, 255, 255, 0.92);
    box-shadow: 0 1px 8px rgba(19, 33, 60, 0.08);
    backdrop-filter: blur(8px);
  }

  .brand {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    font-weight: 700;
    font-size: 1.1rem;
    color: #0d3150;
  }

  .logo {
    width: 56px;
    height: 32px;
    border-radius: 999px 999px 16px 16px;
    background: linear-gradient(135deg, #ffb100, #ff6c00);
  }

  .actions {
    display: flex;
    align-items: center;
    gap: 1rem;
  }

  .nav-btn {
    display: flex;
    align-items: center;
    gap: 0.35rem;
    border: none;
    background: transparent;
    font: inherit;
    font-weight: 600;
    color: #0d3150;
    letter-spacing: 0.02em;
    cursor: pointer;
    padding: 0.35rem 0.5rem;
  }

  .nav-btn svg {
    width: 20px;
    height: 20px;
    fill: currentColor;
  }

  @media (max-width: 720px) {
    .actions span {
      display: none;
    }
  }
</style>

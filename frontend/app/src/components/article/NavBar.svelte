<script lang="ts">
  import { onMount } from "svelte";

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

  let showSearch = false;
  let searchQuery = '';
  let searchInput: HTMLInputElement | null = null;
  let showNav = false;

  const navlist=[
    "Räder",
    "Zubehör",
    "Service",
    "Ergonomie",
    "Aktivitäten",
    "Galerie",
    "Über uns",
    "Kontakt",
    "Newsletter"
  ]

  onMount(()=>{
    let prevScrollpos = window.pageYOffset;
window.onscroll = function() {
let currentScrollPos = window.pageYOffset;
  const navElement = document.getElementById("nav");
  if (navElement) {
    if (prevScrollpos > currentScrollPos) {
      navElement.style.top = "0";
    } else {
      navElement.style.top = "-100px";
    }
  }
  prevScrollpos = currentScrollPos;
}
  })
</script>

<nav class="nav" id="nav">
  <!-- <div class="brand"> -->
    <div class="logo">
   <img src={brand?.label}  alt="logo"/>
    </div>
  <!-- </div> -->
  <div class="actions">
    {#each actions as action}
      <button
        type="button"
        class="nav-btn"
        aria-label={action.label}
        aria-expanded={action.icon === 'search' ? showSearch : undefined}
        aria-controls={action.icon === 'search' ? 'search-input' : undefined}
        on:click={async () => {
          if (action.icon === 'search') {
            showSearch = true;
            // await tick();
            searchInput?.focus();
          } else if (action.icon === 'menu') {
            showNav = !showNav;
          }
        }}
      >
        {#if action.icon === 'map'}
          <svg viewBox="0 0 24 24" role="img" aria-hidden="true"><path d="M3 5.5 9 3l6 2.5 6-2.5v15L15 20 9 17.5 3 20z"/></svg>
        {:else if action.icon === 'search'}
          {#if showSearch}
            <input
              id="search-input"
              bind:this={searchInput}
              class="search-input"
              type="search"
              placeholder="Suche..."
              bind:value={searchQuery}
              on:blur={() => { showSearch = false }}
              on:keydown={(e) => { if (e.key === 'Escape') { showSearch = false } }}
            />
          {:else}
            <svg viewBox="0 0 24 24" role="img" aria-hidden="true"><path d="M10.5 3a7.5 7.5 0 1 1 0 15 7.5 7.5 0 0 1 0-15zm0 2a5.5 5.5 0 1 0 0 11 5.5 5.5 0 0 0 0-11zm7.44 9.56 4.5 4.5-1.88 1.88-4.5-4.5z"/></svg>
          {/if}
        {:else if action.icon === 'menu'}
          <svg viewBox="0 0 24 24" role="img" aria-hidden="true"><path d="M3 6h18v2H3zm0 5h18v2H3zm0 5h18v2H3z"/></svg>
        {/if}

        <span>{action.label}</span>
      </button>
    {/each}

    
  </div>

</nav>
  {#if showNav}
    <div class="navLink">
      {#each navlist as nav, index}
        <a href="#" >{nav}</a>
      {/each}
    </div>
  {/if}

<style>
  .nav {
    /* position: sticky; */
    top: 0;
    z-index: 12;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.75rem 2rem;
    height: auto;
    /* background: rgba(255, 255, 255, 0.92); */
    /* box-shadow: 0 1px 8px rgba(19, 33, 60, 0.08); */
    backdrop-filter: blur(8px);
   background-image: linear-gradient(180deg, #e0d5bb, #e3d9c0);
   transition: top 0.3s;
   /* position: static; */

  }

  .brand {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    font-weight: 700;
    font-size: 1.1rem;
     color: #ce2f24;
  }

   .logo {
    width: 30%;
    height: 30%;
    display: flex;
    align-items: center;
  }

  .actions {
    display: flex;
    align-items: center;
    gap: 1rem;
  }

  .nav-btn {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.35rem;
    border: none;
    background: transparent;
    font: inherit;
    font-weight: 300;
    font-size: 13px;
     color: #ce2f24;
    letter-spacing: 0.02em;
    cursor: pointer;
    padding: 0.35rem 0.5rem;
  }

  .nav-btn svg {
    width: 40px;
    height: 40px;
    fill: currentColor;
  }

  /* .search-wrap removed: search input is now inline inside the button */

  .search-input {
    width: 260px;
    padding: 0.4rem 0.6rem;
    border-radius: 6px;
    border: 1px solid rgba(0,0,0,0.12);
    outline: none;
    font-size: 0.95rem;
  }

  .search-input:focus {
    box-shadow: 0 2px 10px rgba(0,0,0,0.08);
  }

   .navLink a{
    text-decoration: none;
     display: flex; 
     color: #fff;

  }

  .navLink{
    background-color: #f59e0b;
    display: flex;
    align-items: center;
    height: 40px;
    justify-content: center;
    gap: 5rem;
    /* color: #fff; */
    font-size: .8rem;
    font-weight: 300;
    /* position: sticky;
    top: 0;
    z-index: 10; */
  } 

  @media (max-width: 720px) {
    .actions span {
      display: none;
    }
    .navLink{
      display: block;
      flex-direction: column;
      /* z-index: 100; */
      height: 100%;
      background-color: #ce2f24;
    }

    .navLink a{
      /* background-color: #be220a; */
      border: 1px solid red;
      padding: .5rem;

    }

    /* .actions{
      
    } */
  }
</style>

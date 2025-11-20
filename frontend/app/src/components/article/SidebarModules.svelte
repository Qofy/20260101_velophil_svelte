<script lang="ts">
  export interface MapModule {
    type: 'map';
    title: string;
    image: string;
    pois: string[];
  }

  export interface LinksModule {
    type: 'links';
    title: string;
    items: string[];
  }

  export type SidebarModule = MapModule | LinksModule;

  export let modules: SidebarModule[] = [];
</script>

<aside class="sidebar">
  {#each modules as module}
    <div class={`module ${module.type}`}>
      <h4>{module.title}</h4>
      {#if module.type === 'map'}
        <img src={module.image} alt={module.title} loading="lazy" decoding="async" />
        <ul>
          {#each module.pois as poi}
            <li>{poi}</li>
          {/each}
        </ul>
      {:else}
        <ul>
          {#each module.items as item}
            <li>{item}</li>
          {/each}
        </ul>
      {/if}
    </div>
  {/each}
</aside>

<style>
  .sidebar {
    position: sticky;
    top: 120px;
    display: flex;
    flex-direction: column;
    gap: 1.5rem;
  }

  .module {
    border-radius: 20px;
    padding: 1.5rem;
    background: #f6f8fb;
    box-shadow: 0 16px 30px rgba(15, 42, 68, 0.12);
  }

  h4 {
    margin-top: 0;
    margin-bottom: 1rem;
    text-transform: uppercase;
    letter-spacing: 0.2em;
    font-size: 0.85rem;
    color: #0f4b75;
  }

  img {
    width: 100%;
    border-radius: 16px;
    margin-bottom: 1rem;
  }

  ul {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 0.65rem;
    color: #1c2f3f;
    font-weight: 500;
  }
</style>

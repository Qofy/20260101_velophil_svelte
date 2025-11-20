<script lang="ts">
  export interface Crumb {
    label: string;
    href?: string;
  }

  export let items: Crumb[] = [];
</script>

<nav class="breadcrumbs" aria-label="Breadcrumb">
  <ol>
    {#each items as crumb, index}
      <li>
        {#if crumb.href && index < items.length - 1}
          <a href={crumb.href}>{crumb.label}</a>
        {:else}
          <span aria-current="page">{crumb.label}</span>
        {/if}
      </li>
    {/each}
  </ol>
</nav>

<style>
  .breadcrumbs {
    font-size: 0.95rem;
    color: #111212;
    margin: 1rem 0 2rem;
    /* position: sticky;
    top: 100px;
    z-index: 1; */
  }

  ol {
    list-style: none;
    display: flex;
    flex-wrap: wrap;
    gap: 0.35rem;
    padding: 0;
    margin: 0;
  }

  li {
    display: flex;
    align-items: center;
    gap: 0.35rem;
  }

  li:not(:last-child)::after {
    content: '›';
    color: #9eb1c4;
  }

  a {
    color: inherit;
    text-decoration: none;
  }

  a:hover {
    text-decoration: underline;
  }

  span[aria-current='page'] {
    font-weight: 600;
  }
</style>

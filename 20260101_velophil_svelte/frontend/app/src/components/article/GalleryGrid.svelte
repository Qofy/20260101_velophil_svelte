<script lang="ts">
  export interface GalleryItem {
    src: string;
    alt?: string;
  }

  export let title = '';
  export let items: GalleryItem[] = [];
</script>

<section class="gallery">
  {#if title}<h3>{title}</h3>{/if}
  <div class="grid">
    {#each items as item}
      <figure>
        <img src={item.src} alt={item.alt} loading="lazy" decoding="async" />
      </figure>
    {/each}
  </div>
</section>

<style>
  .gallery {
    margin: 4rem 0;
  }

  h3 {
    margin-bottom: 1.5rem;
    font-size: 2rem;
    color: #092441;
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
    gap: 1.2rem;
  }

  figure {
    margin: 0;
    border-radius: 16px;
    overflow: hidden;
    box-shadow: 0 12px 30px rgba(15, 42, 68, 0.18);
  }

  img {
    width: 100%;
    display: block;
    height: auto;
  }
</style>

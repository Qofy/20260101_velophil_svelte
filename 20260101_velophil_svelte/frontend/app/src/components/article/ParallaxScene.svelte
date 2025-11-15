<script lang="ts">
  export interface Layer {
    src: string;
    alt?: string;
  }

  export let layers: Layer[] = [];
  export let caption = '';
</script>

<section class="parallax" style={`--layers:${Math.max(layers.length, 1)}`}
  aria-label={caption || 'Parallax Szene'}>
  {#each layers as layer, index}
    <div class="panel" style={`z-index:${index + 1}`}>
      <img src={layer.src} alt={layer.alt} loading="lazy" decoding="async" />
    </div>
  {/each}
  {#if caption}
    <p class="caption">{caption}</p>
  {/if}
</section>

<style>
  .parallax {
    --layers: 1;
    position: relative;
    width: 100vw;
    margin: 6rem 0;
    left: 50%;
    margin-left: -50vw;
    height: calc(var(--layers) * 100vh);
    pointer-events: none;
  }

  .panel {
    position: sticky;
    top: 0;
    height: 100vh;
    overflow: hidden;
  }

  img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    transform: scale(1.05);
  }

  .caption {
    position: sticky;
    top: calc(100vh - 5rem);
    margin: 0;
    padding-left: clamp(1rem, 8vw, 6rem);
    color: #fff;
    font-size: clamp(1.5rem, 3vw, 2.6rem);
    text-shadow: 0 12px 40px rgba(0, 0, 0, 0.6);
    font-weight: 600;
    z-index: 60;
  }
</style>

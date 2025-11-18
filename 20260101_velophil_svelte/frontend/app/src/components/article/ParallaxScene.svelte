<script lang="ts">
  export interface Layer {
    src: string;
    alt?: string;
  }

  export type ParallaxWidth = 'full' | 'half';
  export type ParallaxAlign = 'left' | 'center' | 'right';

  export let layers: Layer[] = [];
  export let caption = '';
  export let width: ParallaxWidth = 'full';
  export let align: ParallaxAlign = 'center';
  export let spacing = 0;
</script>

<section
  class={`parallax ${width} align-${align}`}
  style={`--spacing:${spacing}px; --layers:${Math.max(layers.length, 1)};`}
  aria-label={caption || 'Parallax Szene'}
>
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
    position: relative;
    width: 100%;
    height: calc(var(--layers, 1) * 100vh + var(--spacing, 0px));
    margin: 6rem 0;
    pointer-events: none;
  }

  .parallax.full {
    width: 99vw;
    left: 50%;
  right: 50%;
  margin-left: -50vw;
  margin-right: -50vw;
  }

  .parallax.half {
    width: clamp(320px, 48%, 540px);
    
  }

  .parallax.align-left {
    --offset: -42vw;
  }

  .parallax.align-center {
    --offset: -16vw;
  }

  .parallax.align-right {
    --offset: 8vw;
  }

  .panel {
    position: sticky;
    top: 0;
    height: 100vh;
    overflow: hidden;
    background-color: bisque;
  }

  .panel img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    /* transform: scale(1.05); */
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

  .parallax::after {
    content: '';
    position: absolute;
    inset: auto 0 0 0;
    height: var(--spacing, 0px);
  }

  @media print {
    .parallax {
      position: static;
      width: 100%;
      left: 0;
      margin: 0;
      padding: 0;
      height: auto;
      --offset: 0px;
    }

    .panel {
      position: static;
      height: auto;
    }

    .panel img {
      transform: none;
    }

    .caption {
      position: static;
      margin-top: 1rem;
    }
  }
</style>

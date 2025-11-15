<script lang="ts">
  import { onMount } from 'svelte';

  export interface Layer {
    src: string;
    alt?: string;
    speed?: number;
  }

  export let id = '';
  export let height = 720;
  export let layers: Layer[] = [];
  export let caption = '';

  let container: HTMLElement;
  let layerRefs: HTMLElement[] = [];
  let raf = 0;
  let active = false;

  function schedule() {
    if (raf || !active) return;
    raf = requestAnimationFrame(updateTransforms);
  }

  function updateTransforms() {
    raf = 0;
    if (!container) return;
    const rect = container.getBoundingClientRect();
    const viewport = window.innerHeight || 1;
    const range = rect.height + viewport;
    const progress = Math.min(1, Math.max(0, (viewport - rect.top) / range));
    const centerBias = progress - 0.5;

    layerRefs.forEach((layerEl, index) => {
      if (!layerEl) return;
      const speed = layers[index]?.speed ?? 8;
      const translate = centerBias * speed * 12;
      layerEl.style.transform = `translate3d(0, ${translate.toFixed(2)}px, 0)`;
    });
  }

  onMount(() => {
    const observer = new IntersectionObserver(
      entries => {
        active = entries.some(entry => entry.isIntersecting);
        if (active) schedule();
      },
      { threshold: [0, 0.1, 1] }
    );
    if (container) observer.observe(container);

    const scrollHandler = () => schedule();
    window.addEventListener('scroll', scrollHandler, { passive: true });
    window.addEventListener('resize', scrollHandler, { passive: true });
    schedule();

    return () => {
      observer.disconnect();
      window.removeEventListener('scroll', scrollHandler);
      window.removeEventListener('resize', scrollHandler);
    };
  });
</script>

<section class="parallax" style={`height:${height}px`} id={id} bind:this={container}>
  {#each layers as layer, index}
    <div class="layer" style={`z-index:${index + 1}`} bind:this={layerRefs[index]}>
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
    overflow: hidden;
    margin: 4rem 0;
    border-radius: 20px;
  }

  .layer {
    position: absolute;
    inset: 0;
  }

  .layer img {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .caption {
    position: absolute;
    left: clamp(1rem, 6vw, 4rem);
    bottom: clamp(1rem, 4vw, 3rem);
    color: #fff;
    font-size: 1.5rem;
    font-weight: 600;
    text-shadow: 0 8px 24px rgba(0, 0, 0, 0.4);
    margin: 0;
    z-index: 40;
  }
</style>

<script lang="ts">
  export interface HeroImage {
    src: string;
    alt: string;
    width?: number;
    height?: number;
  }

  export interface HeroContent {
    kicker?: string;
    headline: string;
    subline?: string;
    image: HeroImage;
  }

  export let hero: HeroContent;
  export let eyebrow = '';
  export let author = '';
  export let date = '';
  export let badge: { label: string; logo?: string; alt?: string } | null = null;
</script>

<section class="hero">
  <div class="meta">
    <p class="eyebrow">{eyebrow}</p>
    <h1>{hero?.headline}</h1>
    {#if hero?.subline}<p class="subline">{hero.subline}</p>{/if}
    <div class="credits">
      <span>{author}</span>
      <span>·</span>
      <span>{date}</span>
    </div>
  </div>

  <figure>
    <img
      src={hero?.image?.src}
      alt={hero?.image?.alt}
      loading="lazy"
      decoding="async"
      width={hero?.image?.width}
      height={hero?.image?.height}
    />
  </figure>

  {#if badge}
    <aside class="badge">
      <strong>{badge.label}</strong>
      {#if badge.logo}
        <img src={badge.logo} alt={badge.alt} loading="lazy" decoding="async" />
      {/if}
    </aside>
  {/if}
</section>

<style>
  .hero {
    position: relative;
    padding-bottom: 2rem;
  }

  .meta {
    max-width: 820px;
    margin-bottom: 1.5rem;
  }

  .eyebrow {
    text-transform: uppercase;
    letter-spacing: 0.38em;
    color: #4c6d88;
    font-size: 0.85rem;
    margin-bottom: 0.5rem;
  }

  h1 {
    font-size: clamp(2.4rem, 5vw, 3.8rem);
    line-height: 1.1;
    color: #0f2a44;
    margin: 0 0 0.75rem;
    font-weight: 500;
  }

  .subline {
    font-size: 1.15rem;
    color: #24496b;
    margin: 0 0 1rem;
  }

  .credits {
    display: flex;
    gap: 0.5rem;
    color: #4c6d88;
    font-size: 0.95rem;
  }

  figure {
    margin: 0;
    border-radius: 16px;
    overflow: hidden;
    box-shadow: 0 24px 60px rgba(15, 42, 68, 0.2);
  }

  figure img {
    width: 100%;
    height: auto;
    display: block;
  }

  .badge {
    position: absolute;
    left: 0;
    top: 60%;
    transform: translateY(-50%);
    background: #fff;
    border-radius: 16px;
    padding: 0.85rem 1rem;
    box-shadow: 0 8px 20px rgba(12, 48, 76, 0.2);
    width: 220px;
  }

  .badge strong {
    display: block;
    font-size: 0.75rem;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: #0f4b75;
    margin-bottom: 0.5rem;
  }

  .badge img {
    width: 100%;
    height: auto;
  }

  @media (max-width: 900px) {
    .badge {
      position: relative;
      top: auto;
      transform: none;
      margin-top: 1rem;
      width: auto;
    }
  }
</style>

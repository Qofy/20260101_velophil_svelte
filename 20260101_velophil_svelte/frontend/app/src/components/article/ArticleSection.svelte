<script lang="ts">
  export interface SectionImage {
    src: string;
    alt?: string;
  }

  export interface SectionContent {
    type?: 'text' | 'quote';
    title?: string;
    subtitle?: string;
    body?: string[];
    image?: SectionImage;
    quote?: string;
    cite?: string;
  }

  export let section: SectionContent;
  export let flip = false;
</script>

<section class={`article-section ${flip ? 'flip' : ''}`}>
  {#if section.image}
    <figure>
      <img src={section.image.src} alt={section.image.alt} loading="lazy" decoding="async" />
    </figure>
  {/if}
  <div class="copy">
    {#if section.type === 'quote'}
      <blockquote>
        <p>“{section.quote}”</p>
        {#if section.cite}<cite>{section.cite}</cite>{/if}
      </blockquote>
    {:else}
      {#if section.subtitle}<p class="subtitle">{section.subtitle}</p>{/if}
      {#if section.title}<h3>{section.title}</h3>{/if}
      {#if section.body}
        {#each section.body as paragraph}
          <p>{paragraph}</p>
        {/each}
      {/if}
    {/if}
  </div>
</section>

<style>
  .article-section {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
    gap: clamp(1.5rem, 4vw, 4rem);
    align-items: center;
    margin: 4rem 0;
  }

  .article-section.flip {
    direction: rtl;
  }

  .article-section.flip > * {
    direction: ltr;
  }

  figure {
    margin: 0;
    border-radius: 18px;
    overflow: hidden;
    box-shadow: 0 18px 50px rgba(15, 42, 68, 0.2);
  }

  figure img {
    display: block;
    width: 100%;
    height: auto;
  }

  .copy {
    font-size: 1.05rem;
    color: #1c2f3f;
    line-height: 1.7;
  }

  .copy h3 {
    font-size: 2rem;
    margin: 0 0 0.75rem;
    color: #092441;
  }

  .copy .subtitle {
    text-transform: uppercase;
    letter-spacing: 0.2em;
    font-size: 0.85rem;
    color: #5c7c93;
    margin: 0 0 0.75rem;
  }

  blockquote {
    margin: 0;
    font-size: clamp(1.4rem, 3vw, 2rem);
    font-style: italic;
    color: #0f2a44;
  }

  cite {
    display: block;
    margin-top: 0.75rem;
    text-transform: uppercase;
    letter-spacing: 0.35em;
    font-size: 0.75rem;
    color: #4c6d88;
  }

  @media (max-width: 900px) {
    .article-section {
      grid-template-columns: 1fr;
    }

    .article-section.flip {
      direction: ltr;
    }
  }
</style>

<script lang="ts">
  import { onMount } from 'svelte';
  import NavBar from './components/article/NavBar.svelte';
  import Breadcrumbs from './components/article/Breadcrumbs.svelte';
  import HeroSection from './components/article/HeroSection.svelte';
  import VideoPlaceholder from './components/article/VideoPlaceholder.svelte';
  import ParallaxScene from './components/article/ParallaxScene.svelte';
  import ArticleSection from './components/article/ArticleSection.svelte';
  import GalleryGrid from './components/article/GalleryGrid.svelte';
  import SidebarModules from './components/article/SidebarModules.svelte';
  import FooterCta from './components/article/FooterCta.svelte';
  import TextBlock from './components/article/TextBlock.svelte';

  let article: any = null;
  let loading = true;
  let error = '';
  let scenesById: Record<string, any> = {};

  onMount(async () => {
    try {
      const response = await fetch('/data/article.json');
      if (!response.ok) throw new Error('Konnte Artikel nicht laden');
      article = await response.json();
      scenesById = {};
      article?.parallaxScenes?.forEach((scene: any) => {
        scenesById[scene.id] = scene;
      });
    } catch (err) {
      error = err instanceof Error ? err.message : 'Unbekannter Fehler';
    } finally {
      loading = false;
    }
  });
</script>

<svelte:head>
  <title>{article?.meta?.title ?? 'Bikepacking in MV'}</title>
</svelte:head>

<main class="page">
  {#if loading}
    <div class="state">Lade Erlebnis …</div>
  {:else if error}
    <div class="state error">{error}</div>
  {:else if article}
    <NavBar brand={article.nav.brand} actions={article.nav.actions} />
    <div class="layout">
      <div class="article">
        <Breadcrumbs items={article.breadcrumbs} />
        <HeroSection
          hero={article.hero}
          eyebrow={article.meta.eyebrow}
          author={article.meta.author}
          date={article.meta.date}
          badge={article.meta.badge}
        />

        {#each article.blocks as block, index}
          {#if block.type === 'text-only'}
            <TextBlock body={block.body} />
          {:else if block.type === 'video'}
            <VideoPlaceholder poster={block.poster} title={block.title} body={block.body} actions={block.actions} />
          {:else if block.type === 'parallax'}
            {#if scenesById[block.sceneId]}
              <ParallaxScene
                layers={scenesById[block.sceneId].layers}
                caption={block.caption ?? scenesById[block.sceneId].caption}
              />
            {/if}
          {:else if block.type === 'gallery'}
            <GalleryGrid title={block.title} items={block.items} />
          {:else if block.type === 'quote'}
            <ArticleSection section={{ type: 'quote', quote: block.quote, cite: block.cite }} />
          {:else}
            <ArticleSection section={{ ...block, type: 'text' }} flip={block.flip ?? false} />
          {/if}
        {/each}

        <FooterCta
          title={article.footer.cta.title}
          body={article.footer.cta.body}
          action={article.footer.cta.action}
          legal={article.footer.legal}
        />
      </div>
      <SidebarModules modules={article.sidebar.modules} />
    </div>
  {/if}
</main>

<style>
  :global(body) {
    margin: 0;
    font-family: 'Inter', 'Segoe UI', system-ui, -apple-system, BlinkMacSystemFont, sans-serif;
    background: #f3f6fb;
    color: #1c2f3f;
  }

  .page {
    min-height: 100vh;
    background: linear-gradient(180deg, #f5f9ff 0%, #fdfdfd 300px);
  }

  .layout {
    display: grid;
    grid-template-columns: minmax(0, 2.5fr) minmax(220px, 1fr);
    gap: clamp(1.5rem, 4vw, 4rem);
    width: min(1200px, 90vw);
    margin: 0 auto;
    padding-bottom: 4rem;
  }

  .article {
    padding: 1rem 0 0;
  }

  .state {
    padding: 4rem 1rem;
    text-align: center;
    font-size: 1.2rem;
    color: #4c6d88;
  }

  .state.error {
    color: #b83232;
  }

  @media (max-width: 1020px) {
    .layout {
      grid-template-columns: 1fr;
    }

    .article {
      padding-top: 0;
    }
  }
</style>

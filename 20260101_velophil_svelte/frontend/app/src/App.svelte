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

  interface ArticleData {
    meta: any;
    nav: any;
    hero: any;
    breadcrumbs: any[];
    video: any;
    parallaxScenes: any[];
    sections: any[];
    sidebar: any;
    footer: any;
  }

  let article: ArticleData | null = null;
  let loading = true;
  let error = '';
  let timeline: Array<{ kind: 'section' | 'parallax'; data: any }> = [];

  onMount(async () => {
    try {
      const response = await fetch('/data/article.json');
      if (!response.ok) throw new Error('Konnte Artikel nicht laden');
      article = await response.json();
      buildTimeline();
    } catch (err) {
      error = err instanceof Error ? err.message : 'Unbekannter Fehler';
    } finally {
      loading = false;
    }
  });

  function buildTimeline() {
    if (!article) return;
    const sections = article.sections ?? [];
    const scenes = article.parallaxScenes ?? [];
    let alternator = 0;

    const preparedSections = sections.map(section => {
      if (section.type === 'text') {
        const flip = alternator % 2 === 1;
        alternator += 1;
        return { ...section, flip };
      }
      return section;
    });

    const anchors = [1, 3];
    const tempTimeline: Array<{ kind: 'section' | 'parallax'; data: any }> = preparedSections.map(section => ({
      kind: 'section',
      data: section
    }));

    let sceneIndex = 0;
    anchors.forEach(anchor => {
      if (scenes[sceneIndex]) {
        const insertionIndex = Math.min(anchor + sceneIndex + 1, tempTimeline.length);
        tempTimeline.splice(insertionIndex, 0, { kind: 'parallax', data: scenes[sceneIndex] });
        sceneIndex += 1;
      }
    });

    for (; sceneIndex < scenes.length; sceneIndex++) {
      tempTimeline.push({ kind: 'parallax', data: scenes[sceneIndex] });
    }

    timeline = tempTimeline;
  }
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
        <VideoPlaceholder
          poster={article.video.poster}
          title={article.video.title}
          body={article.video.body}
          actions={article.video.actions}
        />
        {#each timeline as block}
          {#if block.kind === 'parallax'}
            <ParallaxScene {...block.data} />
          {:else}
            {#if block.data.type === 'gallery'}
              <GalleryGrid title={block.data.title} items={block.data.items} />
            {:else}
              <ArticleSection section={block.data} flip={block.data.flip} />
            {/if}
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

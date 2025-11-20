<script lang="ts">
  import VideoPlaceholder from "./VideoPlaceholder.svelte";
  import { onDestroy, onMount } from "svelte";
  import bg1 from "../../../public/assets/article/bg1.jpg"

  // interface HeroImage {
  //   src: string | string[];
  //   alt: string;
  //   width?: number;
  //   height?: number;
  // }

  interface HeroContent {
    kicker?: string;
    headline: string;
    subline?: string;
    // image: HeroImage;
  }

  export let hero: HeroContent;
  export let eyebrow = '';
  export let author = '';
  export let date = '';
  
  // export let images: string[] = [];
  // export let interval: number = 4000;
  // export let autoplay: boolean = true;

  // let current = 0;
  // let timer: number | null = null;

  
  // $: heroImages = Array.isArray(hero?.image?.src) ? hero.image.src as string[] : (hero?.image?.src ? [hero.image.src as string] : []);
  // $: imageList = (images && images.length) ? images : heroImages;
  // $: imageUrl = imageList && imageList.length ? imageList[current] : (typeof hero?.image?.src === 'string' ? hero.image.src : undefined);

  // function start() {
  //   if (!autoplay) return;
  //   if (!imageList || imageList.length <= 1) return;
  //   stop();
  //   timer = window.setInterval(() => {
  //     current = (current + 1) % imageList.length;
  //   }, interval) as unknown as number;
  // }

  // function stop() {
  //   if (timer != null) {
  //     clearInterval(timer as any);
  //     timer = null;
  //   }
  // }

  // onMount(() => start());
  // onDestroy(() => stop());
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

  <div style="background-image: url({bg1});" class="fig">
    <!-- <img src="" alt="">  -->
  </div>

  <!-- <div>
    <p>
      Es ist Freitagabend, 
      die Temperaturen sind mild, 
      und das Wochenende steht vor der Tür. 
      Perfekte Aussichten also für ein Mikroabenteuer vor der Haustür! 
      Und was wäre da reizvoller als eine Bikepacking-Tour mit Gravelbikes und ein paar Freunden? Gerade 
      Ausflügler aus der Hauptstadt können beispielsweise die Route zwischen Neustrelitz und Demmin gut mit der Bahn erreichen. Sie führt durch Wälder, Wiesen und Felder, mitten durch den Müritz-Nationalpark und die Mecklenburgische Schweiz und behält dabei Flüsse, Seen und Feuchtgebiete jederzeit in Reichweite.
    </p>
    <p>
      Neben der landschaftlichen Abwechslung und zahlreichen Wow-Momenten fordert die Tour quer 
      durch Mecklenburg-Vorpommern übrigens auch sportlich anspruchsvolle Radfahrer: Denn auf gut 150 Kilometern 
      kommen erstaunliche 1.000 Höhenmeter zusammen! Und wer als überzeugter Bikepacker clever und mit entsprechender Packliste packt, 
      bringt sein leichtes Gepäck für ein erlebnisreiches Wochenende bequem am Fahrrad unter – ganz nach dem Motto: »reduce it to the max«.
    </p>
    <p>Übrigens: Besonders reizvoll ist die Bikepacking-Tour im Frühjahr. Denn dann kleidet die Mecklenburgische Schweiz 
      ihre Kurven und Hügel in ein blühendes Gewand aus Rapsgelb und Knallgrün, links und rechts des Weges kauen Kühe 
      gemächlich auf saftigen Halmen, und knorrige Kastanienbäume haben allerorts ihre Kerzen angesteckt.</p>
  </div> -->

   <!-- {#if videoBlock}
    <VideoPlaceholder poster={videoBlock.poster} title={videoBlock.title} body={videoBlock.body} actions={videoBlock.actions} />
  {/if} -->
  <!-- <figure>
    <img
      src={hero?.image?.src}
      alt={hero?.image?.alt}
      loading="lazy"
      decoding="async"
      width={hero?.image?.width}
      height={hero?.image?.height}
    />
  </figure> -->

  <!-- {#if badge}
    <aside class="badge">
      <strong>{badge.label}</strong>
      {#if badge.logo}
        <img src={badge.logo} alt={badge.alt} loading="lazy" decoding="async" />
      {/if}
    </aside>
  {/if} -->
</section>

<style>
  .hero {
    /* position: relative; */
    padding-bottom: 2rem;
    /* display: flex;
    flex-direction: column;
    align-items: center; */
    display: flex;
    flex-direction: column;
    width: 100%;
    gap: 1rem;
  }

  .meta {
    max-width: 820px;
    margin-bottom: 1.5rem;
  }

  .eyebrow {
    text-transform: uppercase;
    letter-spacing: 0.38em;
    color: #1f1d1d;
    font-size: 0.85rem;
    margin-bottom: 0.5rem;
  }

  h1 {
    font-size: clamp(2.4rem, 4vw, 3.8rem);
    line-height: 1.1;
    color: #ce2f24;
    margin: 0 0 0.75rem;
    font-weight: 700;
  }

  .subline {
    font-size: 1.15rem;
        color: #1f1d1d;
    margin: 0 0 1rem;
    font-weight: 500;
  }

  .credits {
    display: flex;
    gap: 0.5rem;
        color: #1f1d1d;
    font-size: 0.95rem;
    font-weight: 500;
  }

   .fig{
    width: 100%;
    height: 80dvh;
    /* background-color: #ce2f24; */
    background-position: center;
    background-size: cover;
    animation-name: bgautoply;
    /* animation-timing-function: linear; */
    /* animation-timing-function: linear; */
    animation-fill-mode: forwards;
    animation-duration: 30s;
    animation-delay: 2s;
    animation-iteration-count: infinite;
  } 

  @keyframes bgautoply{
    0%{
      background-image: url('../../../public/assets/article/bg1.jpg');

    }

    20%{
      background-image: url("../../../public/assets/article/bg2.jpg");


    }

    40%{
           background-image: url('../../../public/assets/article/bg1.jpg');

    }
    
     60%{
      background-image: url("../../../public/assets/article/bg2.jpg")
        
      }

      80%{
           background-image: url('../../../public/assets/article/bg1.jpg');
      

      }

      100%{
      background-image: url("../../../public/assets/article/bg2.jpg")

      }
  }
  /* figure {
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
  } */
</style>

<script lang="ts">
  export let body: string[] = [];
  export let header: string | undefined = undefined;

</script>

<section class="text-block">
  {#if header}
    <h2 class="tb-header">{header}</h2>
  {/if}
  {#each body as paragraph}
    <p>{paragraph}</p>
  {/each}
</section>

<style>
  .text-block {
    max-width: 760px;
    margin: 3rem auto;
    font-size: 1.08rem;
    line-height: 1.8;
        color: #1c2f3f;

  }
  .text-block h2{
    font-size: 2.5rem;
    color: #ce2f24;
    font-weight: 500;
  }

  p {
    margin: 0 0 1.5rem;
  }

  .tb-header {
    margin: 0 0 1rem;
    font-size: 3rem;
    color: #0f2a44;
    font-weight: 300;
  }
</style>

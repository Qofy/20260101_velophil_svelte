<script lang="ts">
  export let body: string[] = [];
</script>

<section class="text-block">
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
    color: #12304b;
  }

  p {
    margin: 0 0 1.5rem;
  }
</style>

<script lang="ts">
  export let title = '';
  export let body = '';
  export let action = '';
  export let legal: string[] = [];
</script>

<footer class="site-footer">
  <div class="cta">
    <div>
      <h3>{title}</h3>
      <p>{body}</p>
    </div>
    <button>{action}</button>
  </div>
  <div class="legal">
    {#each legal as entry}
      <a href="/" rel="nofollow">{entry}</a>
    {/each}
  </div>
</footer>

<style>
  .site-footer {
    margin-top: 4rem;
    padding: 3rem 0 2rem;
    border-top: 1px solid rgba(9, 36, 65, 0.08);
  }

  .cta {
    display: flex;
    justify-content: space-between;
    gap: 2rem;
    align-items: center;
    padding: 2rem;
    border-radius: 20px;
    background: linear-gradient(135deg, #0f4b75, #1b7bbf);
    color: #fff;
  }

  .cta h3 {
    margin: 0 0 0.5rem;
    font-size: 2rem;
  }

  .cta p {
    margin: 0;
    max-width: 460px;
  }

  button {
    border: none;
    border-radius: 999px;
    padding: 0.9rem 2rem;
    font-weight: 600;
    font-size: 1rem;
    background: #ffba08;
    color: #092441;
    cursor: pointer;
  }

  .legal {
    margin-top: 1.5rem;
    display: flex;
    gap: 1.5rem;
    flex-wrap: wrap;
    color: #4c6d88;
  }

  .legal a {
    color: inherit;
    text-decoration: none;
  }
</style>

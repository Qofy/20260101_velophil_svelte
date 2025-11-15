<script lang="ts">
  export interface VideoAction {
    label: string;
    variant?: 'primary' | 'secondary';
  }

  export let poster = '';
  export let title = '';
  export let body = '';
  export let actions: VideoAction[] = [];
</script>

<section class="video-placeholder">
  <div class="media">
    <img src={poster} alt={title} loading="lazy" decoding="async" />
  </div>
  <div class="overlay">
    <h3>{title}</h3>
    <p>{body}</p>
    <div class="buttons">
      {#each actions as action}
        <button class:primary={action.variant === 'primary'}>{action.label}</button>
      {/each}
    </div>
  </div>
</section>

<style>
  .video-placeholder {
    position: relative;
    border-radius: 20px;
    overflow: hidden;
    margin: 3rem 0;
  }

  .media img {
    width: 100%;
    display: block;
  }

  .overlay {
    position: absolute;
    inset: 0;
    background: linear-gradient(120deg, rgba(16, 20, 46, 0.8), rgba(23, 0, 18, 0.7));
    color: #fff;
    padding: clamp(1.5rem, 4vw, 3rem);
    display: flex;
    flex-direction: column;
    justify-content: center;
    gap: 0.75rem;
  }

  h3 {
    margin: 0;
    font-size: clamp(1.3rem, 3vw, 2rem);
  }

  p {
    margin: 0;
    max-width: 32ch;
    font-size: 1rem;
    line-height: 1.5;
  }

  .buttons {
    display: flex;
    gap: 0.75rem;
    flex-wrap: wrap;
  }

  button {
    border-radius: 999px;
    border: 1px solid rgba(255, 255, 255, 0.5);
    background: transparent;
    color: #fff;
    padding: 0.5rem 1.6rem;
    font-weight: 600;
    cursor: pointer;
  }

  button.primary {
    background: #ffba08;
    color: #222;
    border-color: #ffba08;
  }
</style>

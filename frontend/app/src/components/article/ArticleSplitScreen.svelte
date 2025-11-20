<script lang="ts">
    // Default image (public path) — use string path to avoid TS import diagnostics
    export let src: string = '/assets/parallax/split.jpg';
    export let header: string = '';
    export let head: string = '';
    export let body: string[] = [];
</script>

<figure class="figure">
    <div class="overFlow">

        {#if header}
            <h1>{header}</h1>
        {/if}
        {#if head}
            <h3>{head}</h3>
        {/if}
        {#each body as paragraph}
            <p>{paragraph}</p>
        {/each}
    </div>

    <div class="split-right">
        <img src={src} alt="split" />
    </div>
</figure>


<style>
.figure{
    width: 99vw;
  left: 50%;
  right: 50%;
  margin-left: -50vw;
  margin-right: -50vw;
  /* padding: 0; */
  position: relative;
    height: 70vw;
    /* width: 100%; */
    background-repeat: no-repeat;
    background-position: 100%;
    overflow: scroll;
    scroll-behavior: smooth;
    scrollbar-width: none;
    background-size: cover;
    /* background-color: #10d8d8; */
    display: flex;

}

.figure div{
width: 50%;
height: 100%;
}

.split-right img{
height: 100%;
width: 100%;
}

.figure div:first-child{
    /* background-color: red; */
    overflow: scroll;
    scrollbar-width: none;
    display: flex;
    flex-direction: column;
     align-items: center;
    padding: 5rem;
    overflow-x: hidden;
    /* gap: 0; */
}

.figure div:first-child h1{
    /* color: red; */
    font-size: 5rem;
    line-height: 80px;
    width: 400px;
    font-weight: 300;
    text-align: center;
}

.figure div:first-child h3{
    font-size: 2rem;
    width: 500px;
    text-align: center;
}

.figure div:first-child p{
    font-size: 1.3rem;
    width: 500px;
    text-align: center;
}
</style>
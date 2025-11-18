<script>
import figure from "../../public/assets/parallax/figure.jpg"
</script>

<figure class="figure" style="background-image: url('{figure}');">
   <div class="overFlow">
    <div style="background-color: red;">Red</div>
    <div style="background-color: yellow;">Yellow</div>
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
}

.overFlow{
    width: 50%;
    height: 110vw;
    position: absolute;
    top: 1278px;
    display: flex;
    flex-direction: column;
    background-color: aqua;
}

.overFlow div{
height: 50%;
width: 100%;
}
</style>
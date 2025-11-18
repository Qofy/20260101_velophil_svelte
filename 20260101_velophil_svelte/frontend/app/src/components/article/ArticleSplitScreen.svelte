<script>
import split from "../../../public/assets/parallax/split.jpg"
</script>

<figure class="figure">
   <div class="overFlow">
    
   </div>

   <div class="split-right">
    <img src={split} alt="split">
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
    background-color: #10d8d8;
    display: flex;

}

.figure div{
width: 50%;
height: 100%;
}



/*  */

/* .overFlow div{
height: 50%;
width: 100%;
}
.overFlow div:last-child{
    padding: 2rem 5rem;
    font-size: 1.3rem;
} */
</style>
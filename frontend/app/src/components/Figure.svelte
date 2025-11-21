<script>
import figure from "../../public/assets/parallax/figure.jpg";
import bg from "../../public/assets/article/fggb.jpg"
</script>

<figure class="figure" style="background-image: url('{figure}');">
   <div class="overFlow">
    <div style="background-image: url('{bg}');" class="rep"></div>
    <div style="background-color: #f3f6fb;" class="text-cont" >
        <h1 style="color: #ce2f24;">Fahrrad-Reparaturkurse</h1>
        <p>
           In den Wintermonaten von November 
           bis Februar bieten wir einen besonderen 
           Service an: Sie kommen an einem vorher 
           gebuchten Samstag mit Ihrem Fahrrad zu uns und ein 
           Mechaniker/eine Mechanikerin wird sich Ihren Fragen widmen und 
           Ihnen zeigen,… befördert.
        </p>
        <p>
            Nach ein paar festen Tritten in die Pedale erwartet 
            Sarah und ihre beiden Kumpels Vito und Thomas vor den 
            Toren der historischen Residenzstadt im Müritz-Nationalpark 
            ein Paradies aus Grün und Blau: Vorbei am Zierker See und 
            dem Großen Prälanksee geht es die ersten dreißig Kilometer 
            gemächlich voran. Wer sich fürs Bikepacking ohne Zelt entschieden 
            hat, dem bietet sich im familiengeführten Bett + 
            Bike-Hotel »Kranichrast« in Schwarzenhof eine 
            gute Gelegenheit für die erste Übernachtung. 
            Nichts als Naturgeräusche und sanft rauschende Baumwipfel begleiten 
            die glücklichen Städter hier in den Schlaf. 
        </p>
    </div>

    
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

.rep{
background-repeat: no-repeat;
background-position: center;
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
height: 100%;
width: 100%;
}
.overFlow div:last-child{
    padding: 2rem 5rem;
    font-size: 1.3rem;
}

@media (max-width: 720px) {
  .figure {
    width: 100vw;
    height: auto;
    min-height: 109vh;
  }

  .overFlow {
    /* position: static10 */
    transform: none;
    width: 100%;
    margin-top: 1rem;
    box-shadow: none;
    border-radius: 0;
    background: #fff;
    display: block;
  }
  .rep {
    /* height: clamp(200px, 36vw, 360px); */
    height: 100% !important;
  }

  .overFlow div{
    /* height: 100vh; */
    width: 100%;
  }

  .overFlow > div:last-child {
    padding: 1rem;
    font-size: 1rem;
  }

}


@media (min-width: 721px) and (max-width: 900px) {
  .figure {
    height: clamp(420px, 55vw, 640px);
  }

  .overFlow {
    /* right: 4%; */
   transform: none;
    width: 100%;
    margin-top: 1rem;
    box-shadow: none;
    border-radius: 0;
    background: #fff;
    display: block;
  }
}

@media (min-width: 901px){
  .overFlow{
    display: block;
  }
 }

 .overFlow div{
  height: 50%;
 }

 .text-cont{
  height: 140% !important;
 }

</style>
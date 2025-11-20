<script>
import figure from "../../public/assets/parallax/figure.jpg";
import bg from "../../public/assets/article/fggb.jpg"
</script>

<figure class="figure" style="background-image: url('{figure}');">
   <div class="overFlow">
    <div style="background-image: url('{bg}');" class="rep"></div>
    <div style="background-color: #f3f6fb;">
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
height: 50%;
width: 100%;
}
.overFlow div:last-child{
    padding: 2rem 5rem;
    font-size: 1.3rem;
}
</style>
<script>
import figure from "../../public/assets/parallax/figure.jpg"
</script>

<figure class="figure" style="background-image: url('{figure}');">
   <div class="overFlow">
    <div style="background-color: red;">Red</div>
    <div style="background-color: #f3f6fb;">
        <p>
            »Vorsicht an Gleis 3! Der Regionalexpress 
            aus Berlin fährt ein.« Langsam öffnen sich am Neustrelitzer Hauptbahnhof die Türen des Zugs. 
            Gerade einmal eine Stunde und fünfzehn Minuten hat er 
            aus dem Zentrum der Hauptstadt bis hierher gebraucht.
            Und das Beste: Die Räder samt Gepäck hat er gleich mit befördert.
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
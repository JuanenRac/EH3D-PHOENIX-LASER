<p align="center">
  <img src="images/EH3D_PHOENIX_LASER_BANNER.svg" alt="EH3D-PHOENIX-LASER banner" width="100%">
</p>

# <img src="images/brand/icon-64.png" alt="" width="40" height="40" align="absmiddle"> EH3D-PHOENIX-LASER
<p align="center"><a href="README.md">🇺🇸 English</a> | <a href="README_spa.md">🇪🇸 Español</a> | <a href="README_fra.md">🇫🇷 Français</a> | 🇮🇹 <b>Italiano</b> | <a href="README_deu.md">🇩🇪 Deutsch</a> | <a href="README_zho.md">🇨🇳 中文</a> | <a href="README_jpn.md">🇯🇵 日本語</a> | <a href="README_por.md">🇵🇹 Português</a></p>

### Incisione laser da un'immagine e taglio da un disegno: retinatura, G-code raster, contorni SVG e DXF e griglia di calibrazione

<p align="center">
  <img src="https://img.shields.io/badge/License-GPL%203.0--or--later-blue.svg" alt="GPL 3.0 or later">
  <img src="https://img.shields.io/badge/Stack-Rust-DEA584.svg" alt="Rust">
  <img src="https://img.shields.io/badge/Languages-8-00E5FF.svg" alt="Languages">
  <img src="https://img.shields.io/badge/Version-0.1.6-0f6cbd.svg" alt="Version">
  <img src="https://img.shields.io/badge/Maturity-functional-4CAF50.svg" alt="Maturity">
</p>

---

**Verifica di onestà - cosa funziona oggi:** una libreria Rust e un programma a riga di comando (`phoenix-laser`) con 1055 test che girano su un computer: trasforma un'immagine in punti con la retinatura (dithering) e scrive il G-code di un laser che brucia riga per riga con potenza dinamica e extracorsa, legge disegni SVG, DXF e LBRN2 e scrive il G-code che taglia i loro contorni (prima quelli interni, con la larghezza del fascio, ponti e più passate) e incide le loro forme con righe, con le impostazioni di ogni livello in un file, brucia testo, codici QR e codici a barre, controlla un programma rispetto ai limiti della macchina e scrive una griglia di prova potenza contro avanzamento. Ha una finestra (compilata con `--features gui`), il telaio di pannelli della suite con dodici pannelli e quattro spazi di lavoro, che apre un'immagine o un disegno, mostra il percorso del laser, permette di cambiare le impostazioni ed esporta il G-code; la finestra non è stata vista su uno schermo da una persona. Il lettore dei progetti LBRN2 è stato misurato su dodici progetti reali di un produttore di laser (i dodici si leggono e l'unica immagine che contengono viene decodificata e incisa a punti; il verso del loro asse y non è mai stato confermato con un pezzo reale). Può anche inviare un programma al plugin laser del server EH3D e seguire, sospendere o fermare il lavoro (`phoenix-laser server`), provato solo contro un sostituto del server. **Nulla di qui ha girato su un laser reale**: nulla di ciò che ne deriva, né altro, è stato tagliato o bruciato su una macchina.

---

> **Stato di sicurezza:** questo programma non pilota da sé alcuna macchina e non è un sistema di sicurezza: scrive G-code, lo controlla e può inviarlo al plugin laser del server EH3D, che per ora è un simulatore. Ciò che produce deve essere controllato da una persona prima di eseguire qualsiasi cosa su una macchina; nulla di questo ha girato su hardware reale.


## 📋 In breve

| | |
|---|---|
| Versione | `0.1.6` |
| Maturità | Funzionale |
| Test | 1055 |
| Stack tecnologico | Rust |
| Su hardware reale | Non provato: nulla di questo ha girato su una macchina |

**Vai a:** [Panoramica](#-panoramica) · [Architettura](#-architettura) · [Avvio rapido](#-avvio-rapido) · [Test](#-test) · [Struttura](#-struttura-del-repository) · [Correlati](#-progetti-correlati)

## 🚦 Stato del progetto

Ogni riga è una parte di questo repository e il semaforo dice a che punto è oggi. È scritta in base a ciò che il repository può mostrare (test, compilazioni, esecuzioni), non a ciò che è previsto.

🟢 Fatto e verificato · 🟡 Funziona, con limiti noti · 🟠 Fatto a metà o non ancora verificato · 🔴 Non fatto o non funzionante

| Area | Stato | Cosa significa qui |
|---|---|---|
| Nucleo | 🟡 | trasforma un'immagine in punti con la retinatura e scrive il G-code di un laser che brucia riga per riga con potenza dinamica ed extracorsa, legge disegni SVG e DXF e li taglia o li incide per livello (contorni ordinati, larghezza del fascio, ponticelli, passate, tratteggio), e una griglia di prova potenza contro avanzamento; provato su un computer con input normali e danneggiati, non con una raccolta di file reali. |
| Riga di comando | 🟡 | `phoenix-laser` (`engrave`, `calibrate`, `info`, `cut`, `vector`, `layers`) esegue le stesse funzioni della libreria e stampa il risultato; ogni errore torna come messaggio e codice di uscita diverso da zero. |
| Test | 🟡 | 1055 test passano (`cargo test`), con casi normali, casi limite e input danneggiati. |
| Sicurezza | 🟡 | Rust sicuro e semplice (`forbid(unsafe_code)`): nulla va in panico con i file ricevuti e un valore errato è un errore con il suo motivo; nessuna revisione indipendente. |
| Finestra | 🟠 | Scritta (egui, `--features gui`) come telaio di pannelli della suite: aprire o trascinare un'immagine o un disegno SVG, DXF o LBRN2, vedere il percorso del laser (il taglio in un colore, lo spostamento attenuato), modificare la tabella dei livelli, le modalità testo, codice QR, codice a barre, segni, cornice e fuoco, una griglia di calibrazione, zoom e spostamento, stimare il tempo, controllare, rapporto, inviare al server, esportare; la sua logica è testata, ma non è stata vista su uno schermo da una persona. |
| Hardware reale | 🔴 | Non provato: nulla di questo è stato eseguito su una macchina. |
| Documentazione | 🟡 | README in otto lingue, il changelog e le due immagini disegnate da `docs/`. |

## 🎯 Panoramica

* **L'immagine** (`eh3d-image`): PNG (grigio, colore, trasparente, 8 o 16 bit) e PGM (testo e binario) diventano livelli di grigio, con le parti trasparenti come bianco e la dimensione limitata perché un file ostile non possa chiedere gigabyte; `resize` fa la media dei pixel così che un punto sia grande quanto il fascio.
* **I punti** (`src/dither.rs`): una soglia o la diffusione dell'errore di Floyd-Steinberg o Atkinson; i test verificano che l'oscurità di un grigio si conservi (un grigio medio brucia circa metà dei punti) e che il bianco non bruci nulla e il nero tutto.
* **Il laser** (`src/raster.rs`, `src/calibrate.rs`): una riga dopo l'altra, da sinistra a destra e poi da destra a sinistra, `M4` con `S0` durante lo spostamento e la potenza scelta dove brucia, un overscan fisso o calcolato dall'accelerazione (v al quadrato su 2a) e una griglia di quadrati con una potenza per colonna e un avanzamento per riga per trovare ciò che si adatta a un materiale.
* **L'intestazione e i numeri di ogni file G-code** (`src/gcode_out.rs`): il file si apre con righe di commento che dicono il generatore e la sua versione, le unità (mm, G21), l'immagine da cui è nato con il suo SHA-256 e gli utensili (il laser e la sua potenza); una griglia di calibrazione dice di non avere origine. Un numero si scrive in un solo modo (al massimo tre decimali, niente `-0`) e un valore non finito viene rifiutato invece di essere scritto come `NaN` o `inf`.
* **`--dry-run`:** `engrave` calcola il risultato e lo conta (punti, righe, tratti, lunghezza bruciata, tempo) senza scrivere il file di uscita, che allora può essere omesso.
* **I disegni** (`src/svg.rs`, `src/dxf.rs`, `src/flatten.rs`, `src/vector.rs`): un sottoinsieme sicuro di SVG (tracciati con tutti i comandi, forme, trasformazioni, gruppi, unità e un `use` di un elemento dello stesso file) e le entità di un DXF (linee, polilinee con archi, archi, cerchi, ellissi, spline, blocchi con i loro inserimenti, unità) diventano forme in millimetri; script, immagini, un `use` di un altro file, entità dichiarate e tutto ciò che è sconosciuto vengono saltati o rifiutati, la dimensione, la profondità e il numero di elementi (conta ciò che porta un `use`) e di punti sono limitati dalle impostazioni, un `use` che torna su se stesso è un errore, e le curve diventano tratti rettilinei entro una tolleranza.
* **Taglio e incisione** (`src/order.rs`, `src/offset.rs`, `src/cut.rs`): le forme sono raggruppate per livello (DXF) o colore (SVG); ogni gruppo viene tagliato (linee sciolte unite, prima i contorni dentro un altro, il minor spostamento, la larghezza del fascio tolta dal lato giusto, ponticelli lasciati, più passate con il fuoco che avanza) o inciso (forme chiuse riempite di linee con un angolo, con la regola pari-dispari) con le sue impostazioni. `phoenix-laser layers DRAWING --accel A` dà anche il tempo di ogni livello con la testa che frena e accelera a ogni angolo a `A` mm/s2 (e quanto di esso il laser brucia), calcolato dall'anteprima del programma.
* **Il file di impostazioni** (`src/vconfig.rs`, `docs/vector-settings.md`): ogni valore da regolare (i limiti dei lettori, la macchina, le impostazioni di ogni livello) è in un file di testo semplice controllato riga per riga; nulla di tutto ciò è nel codice.
* **La prova di fuoco** (`src/calibrate.rs`, `[focus]` nel file delle impostazioni): `phoenix-laser focus` brucia una scala di linee corte, ciascuna con la testa a un'altra altezza, e la linea più nitida mostra dove il fascio è a fuoco; il passo di altezza non ha un valore predefinito (cosa sia giù dipende dalla macchina), la potenza rispetta il limite di potenza e la testa torna alla prima altezza. Con `--powers 10,20,30` (fino a 10 potenze) incide invece una griglia di messa a fuoco: le stesse linee una volta per ogni potenza, affiancate con una colonna per potenza, per leggere quale potenza incide la linea più sottile all'altezza del fuoco; la testa va una sola volta a ogni altezza e incide tutte le colonne di quella riga, e ogni potenza rispetta il limite di potenza.
* **La griglia di prova di un materiale** (`src/calibrate.rs`, `[calibrate]` nel file delle impostazioni): `phoenix-laser calibrate --material NOME --materials FILE` distribuisce le potenze e le velocità della griglia intorno a ciò che dice il materiale della libreria, entro i limiti e senza ripetere valori, e `--labels` brucia la potenza di ogni colonna e la velocità di ogni riga accanto ai quadrati con un carattere a tratti, così la griglia si può leggere poi sul materiale; un materiale oltre un limite viene rifiutato, mai abbassato. Con un materiale le cifre escono per impostazione predefinita (`--no-labels` le toglie) e la potenza e la velocità minime della scala sono le chiavi `min_power_percent` e `min_feed_mm_min`.
* **Testo** (`src/text.rs`, `[text]` nel file delle impostazioni): `phoenix-laser text OUT.nc --text TESTO` incide un nome, un numero di serie o un marchio con un carattere a tratti del programma (maiuscole, cifre, spazio e `. , - + / : ! ? ( ) = _ '`; una minuscola è incisa come maiuscola, un a capo inizia una riga) e senza file di carattere. L'altezza di una maiuscola, gli spazi tra lettere e righe, la potenza, la velocità e i passaggi sono impostazioni; un carattere che il font non ha, un testo di più di `max_chars` caratteri, un'altezza oltre `max_height_mm`, una potenza o una velocità oltre il limite, o un lavoro che uscirebbe dal piano sono rifiutati, mai tagliati, e il testo non è mai scritto in un commento del programma. Le righe di un testo su più righe possono essere allineate a sinistra, al centro o a destra (`--align`), il testo può essere inciso come immagine speculare per il retro di vetro o acrilico (`--mirror`) e un'incisione che durerebbe più di `max_minutes` viene rifiutata.
* **Codice QR** (`src/qr.rs`, `[qr]` nel file delle impostazioni): `phoenix-laser qr OUT.nc --text DATI` incide dei dati come un codice QR proprio del programma (versioni da 1 a 4, da 21 a 33 moduli per lato, in modalità byte, con i quattro livelli di correzione degli errori di Reed-Solomon e la maschera con la penalità più bassa), riempiendo ogni modulo scuro con delle linee; il simbolo è stato riletto con un decodificatore indipendente in tutte le versioni e i livelli, e i test lo rileggono con un lettore proprio. Il lato del modulo, la distanza tra le linee, il livello, la potenza, la velocità, i passaggi e i limiti sono impostazioni; dati oltre `max_bytes`, una versione oltre `max_version`, un simbolo più grande di `max_size_mm`, un riempimento di più di `max_lines` movimenti, una potenza o una velocità oltre il limite o un lavoro fuori dal piano vengono rifiutati, mai tagliati né ridotti, e i dati non vengono mai scritti in un commento del programma. `--print` mostra il simbolo come testo. Disattivato per impostazione predefinita: `--invert` incide invece i moduli chiari (con `--border-modules`, un bordo inciso attorno), `--mirror` incide l'immagine speculare per il retro di vetro o acrilico, `--overscan-mm` porta la testa fino a ogni linea e oltre a laser spento (rifiutato se uscisse dal piano) e un'incisione che durerebbe più di `max_minutes` viene rifiutata.
* **Codice a barre** (`src/barcode.rs`, `[barcode]` nel file delle impostazioni): `phoenix-laser barcode OUT.nc --text DATI` incide dei dati come un codice a barre Code 128 proprio del programma (i caratteri dallo spazio a `~` nel set B, e un testo di un numero pari di cifre nel set C, largo la metà, con il suo simbolo di controllo), riempiendo ogni barra con linee lungo la sua altezza; i test lo rileggono con un lettore proprio, ma non è stato letto con uno scanner. La larghezza della barra più stretta, l'altezza, la distanza tra le linee, la potenza, la velocità, i passaggi e i limiti sono impostazioni; un byte che non è un carattere stampabile, più di `max_chars` caratteri, un codice più largo di `max_width_mm`, un riempimento di più di `max_lines` movimenti, un'incisione che durerebbe più di `max_minutes`, una potenza o una velocità oltre il limite o un lavoro fuori dal piano vengono rifiutati, mai tagliati né ridotti, e i dati non vengono mai scritti in un commento del programma. `--mirror` incide l'immagine speculare per il retro di vetro o acrilico.
* **Tratteggio incrociato** (`src/cut.rs`, `hatch_cross` in un livello): un livello `engrave` con `hatch_cross = on` (o `--cross` in `vector` e `trace`) è riempito una seconda volta con linee ad angolo retto con le prime, con la stessa spaziatura, a ogni passaggio, e il limite di linee vale per ogni gruppo; l'energia di un millimetro quadrato che dà l'elenco dei livelli è raddoppiata, e l'anteprima e la verifica del programma lo vedono come qualsiasi altro programma.
* **La cornice del lavoro** (`src/frame.rs`, `[frame]` nel file delle impostazioni): `phoenix-laser frame` scrive un programma che percorre il rettangolo del lavoro con il laser spento, da una dimensione o dal riquadro di ciò che un programma G-code brucia, per vedere dove cadrà; una debole potenza pilota richiede ogni volta `--confirm-laser-on`, è limitata al 5 % (`max_pilot_percent` di `[frame]` cambia il tetto) e rispetta il limite di potenza, e una cornice fuori dal piano viene rifiutata.
* **Segni di posizionamento** (`src/frame.rs`, `[marks]` nel file delle impostazioni): `phoenix-laser marks OUT.nc --size W,H` incide con poca potenza una L a ogni angolo del rettangolo del lavoro (sui suoi bordi, o a `margin_mm` all'esterno) e, con `--centre-cross`, una croce al centro, per poter rimettere il materiale nello stesso punto e allineare un secondo lavoro al primo. A differenza della cornice, i segni vengono incisi: la potenza e la velocità rispettano i limiti della macchina, e un braccio più lungo di metà di un lato del rettangolo o dei segni che uscirebbero dal piano vengono rifiutati, mai tagliati. La lunghezza dei bracci, il margine, la potenza, la velocità e i passaggi sono impostazioni. `phoenix-laser align PICTURE.pgm --size W,H --px-per-mm S` trova i segni in un'immagine del piano ripresa dall'alto e dice di quanto si discostano da dove dovrebbero essere (lo spostamento da sommare all'origine del lavoro e la rotazione), rifiutando un'immagine in cui manca un segno, è troppo lontano o è troppo ruotato; è stato provato solo su immagini disegnate da `--simulate`, mai su una vera telecamera.
* **L'anteprima** (`src/preview.rs`): `phoenix-laser preview` legge un programma G-code (proprio o altrui), stampa i movimenti, la lunghezza bruciata e percorsa, il riquadro, le potenze e un tempo che conta la frenata negli angoli, avverte di ciò che sembra pericoloso (laser acceso in uno spostamento rapido, nessun avanzamento, potenza oltre il massimo, lavoro fuori dal piano, nessuna fine) e disegna il percorso come SVG; i test rileggono ciò che scrive l'incisione e verificano la stessa lunghezza. Con `--laser-watts W` (la potenza del laser a piena potenza) dice anche l'energia che il programma incide, dalla potenza di ogni movimento come parte di `--smax` per il tempo in cui incide, e la sua potenza media.
* **La libreria dei materiali** (`src/materials.rs`, `examples/materials.cfg`): ciò che si addice a ogni materiale e modo di lavorare si scrive una volta in un file di testo (`[material NOME]`: potenza, velocità, passate, passo di fuoco, larghezza del fascio, tratteggio, dimensione del punto, dithering e curva dei toni) e si sceglie con `--material NOME --materials FILE`; le opzioni della riga di comando prevalgono e il limite di potenza vale su entrambi. `phoenix-laser materials FILE` elenca la libreria.
* **Il limite di potenza** (`src/safety.rs`, `max_power_percent` in `[machine]`, `--max-power`): un programma la cui potenza supera il limite viene rifiutato con un errore che nomina il livello o l'opzione, mai abbassato senza dirlo; il limite è 100 % finché un file o un'opzione non dice meno.
* **Profili di macchina** (`[profile NAME]` nel file delle impostazioni, `--profile NAME`): chi ha due macchine, o sostituisce il modulo laser con un altro, tiene un profilo per ciascuna (la corsa del piano, il limite di potenza, il limite di velocità, il valore di piena potenza e i watt); il profilo scelto è la macchina, e `--max-speed` affianca `--max-power`: una velocità oltre il limite viene rifiutata, mai abbassata.
* **Contorni da un'immagine** (`src/trace.rs`, `[trace]` nel file delle impostazioni): `phoenix-laser trace` segue il contorno di un logo, di una sagoma o di una maschera invece di bruciarlo punto per punto; i livelli di grigio si separano con una soglia, il bordo si trova con marching squares a precisione sub-pixel, si semplifica finché nessun punto dista più di `simplify_mm` dalla linea e le macchie si scartano; il risultato è un livello di un disegno, tagliato (o riempito di linee) con le stesse impostazioni e gli stessi controlli, e la dimensione dell'immagine, i pezzi di bordo, i contorni e i punti sono tutti limitati.
* **Toni, materiali e limiti nella finestra** (`src/window.rs`): le caselle della curva dei toni (gamma, contrasto, luminosità, inversione e livelli automatici) lavorano l'immagine con la stessa funzione della riga di comando; una libreria di materiali si carica da un file (o da quello indicato da `PHOENIX_LASER_MATERIALS`) e scegliere un materiale riempie le caselle di potenza, velocità, punto, dithering, toni e ogni livello di un disegno; i limiti di potenza e di velocità sono anch'essi caselle e un valore oltre il limite viene rifiutato, mai abbassato. I valori iniziali vengono dal file delle impostazioni (`[tone]`, `[machine]`).
* **Materiali modificati nella finestra** (`src/window.rs`, `src/materials.rs`): la sezione dei materiali può modificare, copiare, creare dalle caselle (potenza, velocità, punto, sovracorsa, retinatura e curva dei toni) ed eliminare un materiale, e creare una nuova libreria; una casella per ogni chiave del file accoglie i valori, e il salvataggio riscrive la sezione di quel materiale al suo posto (o la aggiunge in fondo) senza toccare i commenti, le righe vuote, l'ordine, i fine riga né gli altri materiali. Un valore fuori dal suo intervallo, un nome vuoto, troppo lungo o già usato, un file che non sia UTF-8 o non sia una libreria valida vengono rifiutati, con la chiave nominata e il file intatto, senza mai tagliare; eliminare richiede una seconda pressione, e una nuova libreria non viene mai scritta su un file esistente.
* **Il test di messa a fuoco nella finestra** (`src/window.rs`): una modalità con le caselle della sezione `[focus]` (l'altezza della prima linea, il passo, il numero e la lunghezza delle linee, la distanza, la velocità e la potenza); il test si calcola con la stessa funzione di `phoenix-laser focus`, potenza e velocità rispettano i limiti, non c'è un passo predefinito perché dov'è il basso dipende dalla macchina, e un avviso che il test muove Z resta nelle note. Una casella per le potenze della griglia (fino a 10, separate da virgole) fa incidere alla finestra la griglia di messa a fuoco di `focus --powers`: le linee una volta per ogni potenza, una colonna per ciascuna; la casella della potenza singola allora non si usa, e una potenza che non sia maggiore di 0 fino a 100, che superi il limite o in numero eccessivo viene rifiutata.
* **Testo nella finestra** (`src/window.rs`): una modalità con una casella per le parole e le caselle della sezione `[text]` (l'altezza di una maiuscola, la lettera più alta, gli spazi tra lettere e righe, la potenza, la velocità, i passaggi e il massimo di caratteri); il testo è calcolato con la stessa funzione di `phoenix-laser text`, dall'origine della macchina, così una lettera che il carattere non ha, un testo oltre un limite, una potenza o una velocità oltre il limite della macchina o un lavoro fuori dal piano (quando il file delle impostazioni lo indica) viene rifiutato e la riga di stato dice perché; viene scritto come `text.nc` con l'intestazione della riga di comando. L'allineamento delle righe, l'interruttore speculare e la durata massima sono anche nella finestra.
* **Codice QR nella finestra** (`src/window.rs`): una modalità con una casella per i dati, la scelta del livello di correzione degli errori e le caselle della sezione `[qr]` (il lato del modulo, la distanza tra le linee, la potenza, la velocità, i passaggi, il massimo di byte, la versione e il lato massimi, e il massimo di movimenti di incisione); il simbolo è calcolato con la stessa funzione di `phoenix-laser qr`, dall'origine della macchina, così dati che non entrano, un simbolo oltre un limite, una potenza o una velocità oltre il limite della macchina o un lavoro fuori dal piano (quando il file delle impostazioni lo indica) vengono rifiutati e la riga di stato dice perché; viene scritto come `qr.nc` con l'intestazione della riga di comando. Gli interruttori di inversione e specchio e le caselle di bordo, rincorsa e durata massima sono anche nella finestra.
* **Segni di posizionamento nella finestra** (`src/window.rs`, `[marks]` nel file delle impostazioni): una modalità con caselle per la dimensione del rettangolo, il braccio di ogni L, il margine, la potenza, la velocità e le passate e un interruttore per la croce al centro (un file delle impostazioni riempie le caselle di `[marks]`); i segni si calcolano con la stessa funzione di `phoenix-laser marks` dall'origine della macchina, e un braccio più lungo di mezzo lato, una potenza o una velocità sopra il limite o segni fuori dal piano vengono rifiutati, con la casella o il motivo nella riga di stato. Si scrive come `marks.nc` con l'intestazione solita.
* **Codice a barre Code 128 nella finestra** (`src/window.rs`, `[barcode]` nel file delle impostazioni): una modalità con una casella per i dati, un interruttore per l'immagine speculare e le caselle della sezione `[barcode]` (la larghezza della barra più stretta, l'altezza, la distanza tra le linee, la potenza, la velocità, le passate e i limiti; un file delle impostazioni le riempie); il codice si calcola con la stessa funzione di `phoenix-laser barcode` dall'origine della macchina, e dati che non siano testo stampabile o troppo lunghi, un codice oltre un limite, una potenza o una velocità oltre il limite della macchina o un lavoro fuori dal piano vengono rifiutati, con la casella o il motivo nella riga di stato. Si scrive come `barcode.nc` con l'intestazione solita.
* **La cornice nella finestra** (`src/window.rs`, `[frame]` nel file delle impostazioni): una modalità che inquadra l'immagine o il disegno aperto con il laser spento; margine, giri e velocità sono caselle. Una potenza pilota maggiore di 0 accende il laser, quindi la finestra chiede e non si calcola né si scrive nulla finché la persona non risponde sì, ogni volta (cambiare una casella, cambiare modalità o scrivere il file ritira la risposta), sotto il tetto di `max_pilot_percent`, il limite di potenza e il piano.
* **L'anteprima nella finestra** (`src/window.rs`): ogni G-code calcolato dalla finestra è contato dalla stessa anteprima di `phoenix-laser preview` (movimenti, lunghezza bruciata e percorsa, un tempo che conta gli angoli con l'accelerazione della casella e ciò che sembra pericoloso, come un lavoro fuori dal piano del file delle impostazioni), e un programma di qualsiasi origine (`.nc`, `.ngc`, `.gcode`, `.gc`, `.tap`) può essere aperto o trascinato sulla finestra per vederne il percorso e le note. Si legge come testo, non si esegue mai e non si riscrive. La potenza del laser si può scrivere in una casella (il file delle impostazioni e un profilo di macchina la riempiono da `laser_watts`; 0 significa che non è nota): l'anteprima dice allora anche l'energia che il programma brucia e la sua potenza media, come fa `phoenix-laser preview --laser-watts`, e un valore che non sia un numero da 0 a 1000 viene rifiutato.
* **Il contorno nella finestra** (`src/window.rs`, `[trace]` nel file delle impostazioni): una modalità che segue il contorno dell'immagine aperta con la stessa funzione di `phoenix-laser trace` e lo taglia, o riempie le sue forme con linee, come un livello chiamato `trace`; le caselle sono la soglia, la semplificazione e il pezzo minimo, con un interruttore per un inchiostro chiaro; la potenza e la velocità sono quelle delle caselle del laser e rispettano i limiti, e i limiti dell'immagine (`max_pixels`, `max_contours`, `max_segments`) sono quelli del file delle impostazioni, così un'immagine troppo dettagliata è un errore e non un programma lento.
* **Il profilo macchina nella finestra** (`src/window.rs`, `[profile NAME]` nel file delle impostazioni): un elenco nel pannello laterale con le macchine del file; quella scelta è la macchina, come la fa `--profile`, così le caselle del valore di piena potenza e dei limiti di potenza e velocità prendono i suoi valori e il piano del profilo è quello che controllano la cornice e l'anteprima. Ciò che un profilo omette viene da `[machine]`, un profilo che non è nel file non cambia nulla, e i limiti rifiutano ancora una potenza o una velocità superiore.
* **La griglia da un materiale nella finestra** (`src/window.rs`, `[calibrate]` nel file delle impostazioni): la modalità di calibrazione elenca i materiali della libreria; con uno scelto, la griglia si distribuisce intorno alla sua potenza e alla sua velocità con la stessa scala di `phoenix-laser calibrate --material` (caselle per i passi, lo scarto e la potenza e la velocità minime, tenute tra queste e i limiti), e un interruttore incide le cifre accanto ai quadrati con l'altezza della sua casella. Senza scelta è la griglia delle due liste, come prima.
* **La verifica del programma nella finestra** (`src/window.rs`, `[verify]` nel file delle impostazioni): un programma G-code aperto per essere visto è verificato anche da `phoenix-laser verify` con i limiti della finestra e il piano del file delle impostazioni, e il pannello laterale dice se è superata (con l'inizio del suo SHA-256) o cosa non va; una casella tiene l'extracorsa consentita oltre il piano a laser spento (0 per impostazione predefinita, fino a 100 mm), e un programma che incide fuori dal piano non supera la verifica qualunque cosa dica la casella.
* **Il controllo di un programma** (`src/verify.rs`): `phoenix-laser verify PROGRAMMA.nc` legge un programma G-code (di questo programma o di un altro) come testo, non lo esegue mai, e dà lo SHA-256 del file esatto, ciò che fa e un verdetto: passa solo sotto il limite di potenza e di velocità della macchina, dentro il piano (il lavoro fuori fallisce; `--overscan-mm` o `overscan_mm` in `[verify]` lascia che la testa superi il piano di quella distanza con il laser spento, come l'overscan di un altro programma, e bruciare fuori dal piano fallisce sempre), con il laser spento in ogni movimento rapido e un avanzamento in ogni movimento che brucia. `--json` dà il rapporto che un altro programma può conservare accanto a un'autorizzazione legata a quell'hash, con il profilo macchina, lo SHA-256 del file delle impostazioni da cui vengono i limiti e i nomi dei controlli fatti, e il codice di uscita è 1 quando fallisce. `report` scrive quel controllo come qualcosa da leggere prima di un lavoro, nella lingua di `--lang` (inglese, spagnolo, tedesco, francese, italiano, giapponese, cinese o portoghese del Portogallo): lo SHA-256, i movimenti e il tempo, la potenza e la velocità massime rispetto ai limiti, l'energia, se il materiale era verificato, le zone vietate e gli interblocchi, e un esito.
* **Una verifica legata al programma che è stato autorizzato** (`src/verify.rs`): `phoenix-laser verify PROGRAMMA.nc --expect-sha256 HASH` fa fallire il programma quando il suo SHA-256 non è quello di 64 cifre esadecimali indicato, e dice quale ha, così un programma cambiato dopo essere stato guardato non viene eseguito per errore; il fallimento compare anche nel rapporto JSON.
* **Zone vietate, interblocchi, il rapporto e l'elenco delle impostazioni** (`src/guard.rs`, `src/report.rs`, `src/contract.rs`): i rettangoli del piano che un programma deve evitare (`[zone NOME]`) e lo stato dell'arresto di emergenza, della cabina e dell'aria assistita come lo dice il file delle impostazioni (`[interlocks]`) fanno fallire il controllo e impediscono l'invio di un programma, di un lavoro vettoriale e di uno sparo di prova; ciò che va al server è limitato al 40 % della potenza salvo che `[send]` indichi un altro limite; `phoenix-laser report` scrive il controllo da leggere prima di un lavoro, in otto lingue; `phoenix-laser settings` elenca ogni impostazione perché un altro programma la legga. Nessun sensore né arresto di emergenza reale è stato provato.
* **Invio al server EH3D** (`src/server.rs`, `[server]` nel file delle impostazioni): `phoenix-laser server send PROGRAMMA.nc` invia un programma G-code come lavoro del plugin `laser` di EH3D-SERVER, con il client condiviso della suite (`eh3d-serverclient` di EH3D-COMMON: le chiavi `server_address`, `server_token`, `server_machine`, `server_timeout_s`, `server_retries` e `server_mdns`, tentativi ripetuti con pause crescenti, una richiesta che crea qualcosa non viene mai inviata due volte e i rifiuti del server sono detti con il loro codice nella lingua del sistema). Prima di partire, il programma viene verificato rispetto ai limiti della macchina delle impostazioni (il limite di potenza e di velocità, il piano) e uno che non passa non viene inviato; `--follow` resta finché il lavoro termina; `status`, `job`, `pause` e `resume` chiedono della macchina e del lavoro e lo fermano o lo lasciano proseguire; `stop` è l'ARRESTO DI EMERGENZA della macchina laser: il lavoro termina subito e la macchina viene bloccata, così non accetta alcun programma e non gliene viene inviato nessuno finché un amministratore non esegue `release`. Il token non viene mai stampato (può stare nella variabile `PHOENIX_LASER_SERVER_TOKEN`) e viaggia in chiaro su HTTP semplice. I comandi locali non vengono toccati. Provato contro un sostituto del plugin laser, in memoria e su un socket di questo computer; il plugin del server è a sua volta un simulatore, e nulla di tutto ciò è stato eseguito con un laser vero. `phoenix-laser server vector DRAWING.svg|dxf` invia un disegno come lavoro vettoriale (i tracciati e un'operazione per livello, prima verificati contro i limiti delle impostazioni), e `phoenix-laser server test-fire --power-percent P --confirm-fire` fa sparare il laser per un istante per trovare il fuoco (il server lo rifiuta mentre un lavoro è in corso, mentre un arresto di emergenza blocca la macchina, con il vano aperto o l'assistenza d'aria spenta, e oltre il proprio limite).
* **Invio al server dalla finestra** (`src/window.rs`, `src/linkpane.rs`, `[server]` nel file delle impostazioni): la sezione del server della finestra mostra il pannello della suite (l'indirizzo, il token, la macchina, i limiti del collegamento, lo stato del collegamento e un accesso con nome e password), e il suo pulsante invia il programma calcolato come lavoro della macchina laser, mostra a che punto è e permette di fermarlo, lasciarlo proseguire o arrestarlo con un pulsante di ARRESTO DI EMERGENZA (il lavoro termina subito e la macchina resta bloccata finché un amministratore non la sblocca); una cornice che accende il laser e non è stata confermata e un programma che non supera il controllo dei limiti non vengono inviati, e un programma aperto solo per vederlo viene inviato così com'è soltanto dopo che supera il controllo e la persona seleziona la casella che lo dice. Chiudere la finestra lascia il lavoro dov'è sul server.
* **Progetti LBRN2** (`src/lbrn2.rs`, `docs/lbrn2.md`, `examples/lbrn2_survey.rs`): `cut`, `vector`, `layers` e `convert` (e la finestra) leggono anche i progetti `.lbrn2`. I rettangoli, i tracciati (tratti retti e curve di Bézier), i gruppi con le loro trasformazioni e il contorno di un testo diventano un disegno in millimetri; ogni livello del progetto diventa una riga della tabella con potenza, velocità (il progetto ha mm/s, la tabella mm/min), passate, distanza di riempimento e ordine come suggerimenti su cui valgono ancora i limiti della macchina, e un livello che non viene inviato al laser è saltato. Le immagini dentro un progetto (un PNG o un PGM in base64) vengono decodificate entro dei limiti e incise a punti con `phoenix-laser engrave PROGETTO.lbrn2 OUT.nc` (la potenza e la velocità del livello dell'immagine si usano quando la riga di comando non ne dà, e un valore oltre il limite è rifiutato, mai abbassato); si leggono anche un'ellisse e un poligono (scritti solo dai nomi dei loro attributi, un prototipo), i ponti di un livello e il contatore del testo variabile, e un'immagine oltre un limite o in un altro formato, e le forme di un tipo non visto negli esempi reali, restano fuori e sono contate nelle note con un codice stabile. Il formato è letto dal comportamento di dodici progetti reali, senza specifica, e l'asse y è preso come quello di un SVG salvo che `lbrn2_flip_y = off` dica altro: il verso non è mai stato confermato con un pezzo reale, e ogni lettura lo dice nelle sue note. `phoenix-laser text OUT.nc --text 'N{counter:4} {name}' --csv FILE.csv` (o `--counter INIZIO,PASSO,QUANTI`, o `--counter-project PROGETTO.lbrn2`) scrive un programma per ogni riga di un file CSV, tutti controllati prima che il primo sia scritto, e `--preview` mostra il testo di ogni riga senza scrivere nulla. `cargo run --release --example lbrn2_survey -- CARTELLA` legge tutti i file `.lbrn2` di una cartella e dice quali si leggono, quali no e perché, e quali elementi e attributi contengono i file che il lettore non usa.
* **File di prova, il vecchio formato di progetto e i preset come testo** (`src/testfiles.rs`, `src/presets.rs`, `src/dxf.rs`, `src/lbrn2.rs`): il lettore DXF disegna il contorno di un `HATCH` e di un `SOLID`; i progetti `.lbrn` (il vecchio formato, con i tracciati scritti come elementi `V` e `P`) sono letti come i `.lbrn2` e i loro testi sono disegnati con il carattere a tratti; `--project-layers` dà ai livelli di un progetto la loro potenza, velocità e passate sulla riga di comando; `calibrate` accetta intervalli (`--powers 20:80:20`); `phoenix-laser matrix` scrive la matrice potenza contro velocità come progetto con un livello per ogni coppia; `phoenix-laser ytest` scrive il progetto che mostra dove punta l'asse Y; un libro di preset si conserva come testo e lo elenca `phoenix-laser presets`. Non è stato bruciato nulla: la matrice e la prova dell'asse Y non sono state aperte in un altro programma.
* **La finestra di pannelli** (`src/panels.rs`, `src/gui.rs`): la finestra è il telaio di pannelli della suite (`eh3d-dock`): dodici pannelli (vista, livelli, impostazioni del lavoro, proprietà, libreria, materiali, inquadratura, macchina, simulazione e tempo, lavoro, rapporto e console) che si possono togliere, rimettere, spostare, raggruppare in schede, far fluttuare, ripiegare e ingrandire; quattro spazi di lavoro per compiti, non per livelli di esperienza (Prepara, Esegui, Verifica e Compatto); una barra dei menu con File, Modifica, Visualizza, Strumenti, Finestra e Aiuto; e, fuori da ogni pannello perché nessuno spazio di lavoro possa nasconderli, l'arresto del lavoro e lo stato degli interblocchi. La disposizione è conservata nel file delle impostazioni come righe che iniziano con `dock.`, che il lettore delle impostazioni lascia stare. I pannelli, gli spazi di lavoro e i menu sono dati con test; la finestra non è stata vista su uno schermo da una persona.
* **Punti di partenza della base comune dei materiali** (`src/commonmat.rs`, funzionalità `common-machining`): un materiale, un tipo di laser (diodo o CO2), taglio o incisione, la potenza del laser in watt e lo spessore danno la potenza, la velocità e le passate che ha la base comune, esattamente come le ha, con la nota sempre in vista che nessuno le ha provate su una macchina, e i motivi quando il materiale non deve incontrare un laser (allora non c'è risposta). Fuori dall'intervallo che la base conosce non c'è risposta neppure, mai un'ipotesi. La funzionalità è attiva per impostazione predefinita (la build è fissata a una versione di EH3D-COMMON che ha il modulo `machining`); `--no-default-features` la lascia fuori.
* **Sotto-livelli** (`src/sublayers.rs`, `[sublayer LIVELLO/ETICHETTA]` nel file delle impostazioni): un livello può essere lavorato più di una volta, per esempio un riempimento e poi un contorno, ogni passata con il proprio modo, potenza, velocità, passate, passo di fuoco e tratteggio. Un sotto-livello parte dalle impostazioni del suo livello ed è trattenuto dai limiti della macchina come un livello (un errore nomina il livello e l'etichetta, e nulla viene abbassato in silenzio); il lavoro va prima incidendo e per ultimo tagliando, ogni tipo secondo il suo `order`, e `layers` elenca le passate con il loro tempo. Il server riceve solo i livelli, e lo dice.
* **Dove va il lavoro, la sua cornice e il suo tempo** (`src/job.rs`, `[job]` nel file delle impostazioni): il lavoro è posizionato da uno dei nove punti del suo riquadro (`anchor`), o lasciato dove è stato disegnato con `align = page`; la cornice del lavoro è il suo riquadro con il margine di `[frame]`, tracciata a laser spento e rifiutata se uscisse dal piano; il tempo è un intervallo, da ogni movimento al suo avanzamento senza frenata fino al tempo dell'anteprima con la frenata a ogni angolo a `accel_mm_s2`, e le passate sono in entrambi.
* **Preimpostazioni e regolazione in tempo reale** (`src/presets.rs`, `src/live.rs`, `[live]` nel file delle impostazioni): una preimpostazione di un materiale e di uno spessore è assegnata a un livello (una copia) o collegata a esso (il livello segue la preimpostazione; una modifica dice quali livelli ha raggiunto ed è rifiutata per intero se uno non può accettarla). I valori del database comune sono usati così come sono, segnati come non provati su una macchina, e i secondi valori di altre fonti non sono usati se non richiesti. La regolazione in tempo reale di potenza e velocità è un calcolo trattenuto da quattro limiti (il proprio intervallo, che per impostazione predefinita non alza mai un valore, la macchina, il server e il pilota) e dice quale limite ha trattenuto un valore; nulla viene inviato a una macchina.
* **Il rapporto nella finestra** (`src/window.rs`): il rapporto di `phoenix-laser report` si può fare dal programma calcolato dalla finestra, nella lingua della finestra, e viene scartato quando cambia qualcosa.
* **Dithering ordinato** (`src/dither.rs`): `--dither bayer` (anche nella finestra) decide ogni punto in base al suo posto e al suo grigio con una matrice di Bayer 8 x 8, quindi il motivo è regolare e lo stesso grigio brucia sempre gli stessi punti; i test verificano che ogni livello della matrice compaia una volta e che la scurezza di un grigio si conservi.
* **La curva dei toni** (`src/tone.rs`): prima di fare i punti i grigi si possono stirare su tutta la gamma (`--auto-level`), cambiare di contrasto e luminosità, curvare con un gamma o invertire, con un'unica tabella di 256 voci; con i valori predefiniti l'immagine non cambia affatto.
* **Un G-code verificato:** il programma di un disegno viene verificato riga per riga prima di essere consegnato: nessun laser acceso durante un movimento rapido o senza velocità, nessuna `S` oltre la piena potenza, nessun movimento fuori dal piano, `M5` alla fine; il file non viene mai scritto sopra il disegno né sopra le impostazioni da cui deriva.
* **Progetti della suite** (`src/eh3dfile.rs`, `docs/eh3d-project.md`): `phoenix-laser project` tiene il lavoro nel ramo `laser/` di un progetto `.eh3d`, un solo file con un ramo per ogni programma: `save` vi mette il disegno, le impostazioni, la libreria dei materiali e i programmi; `import` conserva byte per byte un disegno di un altro programma (LBRN2, LBRN, SVG o DXF) con un rapporto di ciò che è stato letto, di ciò che non lo è stato e di ciò che si perde in esportazione; `take` copia un contorno o una mesh da un altro ramo, come `cad/`, con l'hash e il formato della sorgente, e `list` e `open` dicono quando è superata; `open` ed `extract` scrivono file senza sovrascriverne alcuno. Viene scritto solo `laser/`: gli altri rami, noti o no, conservano i loro byte. È stato provato solo con progetti inventati.
* **Il progetto nella finestra** (`src/suitewin.rs`, `src/suitecore.rs`, `docs/eh3d-project.md`): File > Progetto della suite (.eh3d) apre un progetto, lo salva e lo salva con un altro nome mantenendo tutti i rami, importa un file conservando l'originale, prende un file di un altro ramo, estrae un ramo ed elenca i rami. Ciò che non è aggiornato o è firmato resta in vista; prima di scrivere su un progetto non aperto lì, su un ramo cambiato da quando è stato letto o su una firma, la finestra chiede. Aprire un progetto non allenta mai i limiti, gli interblocchi o il server delle proprie impostazioni, e un file di impostazioni con il token del server non viene messo in un progetto. La logica è provata senza schermo; nessuna persona ha provato le voci su uno schermo.

## 🧩 Ruolo in EH3D

EH3D-PHOENIX-LASER è un programma della EH3D Fabrication Suite. Funziona come libreria e riga di comando e, compilato con `--features gui`, come finestra con l'aspetto comune agli altri programmi della suite. Legge e scrive file semplici, quindi può essere usato da solo o tra gli altri programmi.

## 🏛️ Architettura

<p align="center">
  <img src="images/architecture.svg" alt="Architettura di EH3D-PHOENIX-LASER: chi lo usa, che cosa contiene e da che cosa dipende" width="100%">
</p>


Come si incastrano le parti di questo repository: chi lo usa, cosa contiene e da cosa dipende. L'immagine è disegnata da `docs/architecture.json` e verificata con il codice.

## 🖼️ Schermate

Queste immagini sono scattate dal programma stesso con `--shot` (vedi Avvio rapido).

<p align="center">
  <img src="images/screens/window-vector.png" alt="Taglio di un disegno: il percorso del laser sulla piastra degli esempi, con la tabella dei livelli sotto" width="100%">
</p>

<p align="center">
  <img src="images/screens/window-engrave.png" alt="Incisione di un'immagine: i punti del gradiente degli esempi colorati secondo la loro potenza" width="100%">
</p>

## ✨ Funzionalità

* **Libreria e riga di comando:** ogni comando di `phoenix-laser` è una funzione della libreria, così un altro programma della suite può chiamarla senza avviare un processo.
* **Riga di comando rigorosa:** un'opzione scritta male (`--powr`) o una parola in più è un errore che la nomina, non qualcosa da ignorare; altrimenti il programma farebbe la sua uscita con i valori predefiniti.
* **Test con input danneggiati:** i test forniscono anche file vuoti, spazzatura, numeri enormi e file tagliati, e nessuno può andare in panico.
* **Nessuna ipotesi:** un valore privo di senso è un errore che dice quale e perché, mai un risultato che sembra solo giusto.
* **Nessun codice unsafe:** `#![forbid(unsafe_code)]` nella libreria e nel programma.

## 🔒 Sicurezza e protezione

| Minaccia | Difesa |
|---|---|
| Un file danneggiato, tagliato o vuoto | nessun panico: i test tagliano un input valido in ogni posizione e forniscono spazzatura |
| Un numero privo di senso (negativo, non finito, assurdamente grande) | rifiutato con un errore che nomina il valore; nulla viene arrotondato in qualcosa di plausibile |
| Uso non sicuro della memoria | Rust sicuro e semplice con `forbid(unsafe_code)` |
| Un disegno ostile (script, entità dichiarate, annidamento profondo, milioni di punti, numeri enormi) | script, immagini e collegamenti non vengono mai eseguiti né scaricati; un'entità dichiarata è un errore; la dimensione, la profondità, gli elementi, le forme, i punti e i tratti di una curva sono limitati; una forma con un punto fuori intervallo viene lasciata fuori |
| Un programma che potrebbe accendere il laser senza velocità o oltre la piena potenza | ogni programma viene verificato riga per riga prima di essere consegnato e quello che non passa non viene scritto |
| Una potenza superiore a quella che macchina o materiale devono ricevere | il limite di potenza (`max_power_percent`, `--max-power`) rifiuta un programma che lo supera con un errore che nomina il livello o l'opzione; non viene mai abbassata senza dirlo |
| Una libreria di materiali rotta o enorme | il file ha un limite di dimensione e di numero di materiali; un valore fuori intervallo, una chiave sconosciuta o un nome ripetuto è un errore che nomina la riga; l'output non viene mai scritto sopra di essa |
| Un programma G-code di altra provenienza che si legge prima di eseguirlo | l'anteprima legge solo testo, con un limite di dimensione, non esegue né scarica nulla, ignora i numeri assurdi e avverte di laser acceso in uno spostamento rapido, avanzamento assente, potenza oltre il massimo, lavoro fuori dal piano e fine assente |
| Il laser che si accende solo per posizionare il lavoro | la cornice si percorre con il laser spento (nessuna potenza, mandrino mai acceso); una potenza pilota richiede `--confirm-laser-on` a ogni esecuzione, anche se la chiede il file delle impostazioni, resta al tetto o sotto (5 % se `max_pilot_percent` non ne indica un altro), sotto il limite di potenza e si spegne prima della fine del programma |
| Una testa mandata nel verso sbagliato in Z da una prova di fuoco | il passo di altezza non ha un valore predefinito e la prova è rifiutata finché la persona non lo dà; il programma dice che muove Z, la corsa è limitata (50 mm in tutto), la testa torna alla prima altezza e la potenza rispetta il limite di potenza |
| Un testo troppo lungo, alto o forte, o che brucerebbe fuori dal piano | si accettano solo i caratteri del carattere tipografico (una lettera che non ha viene rifiutata, mai saltata), il numero di caratteri, l'altezza, la potenza e la velocità restano entro limiti che le impostazioni possono restringere, il lavoro fuori dal piano è rifiutato invece di essere tagliato, e il testo non viene mai scritto in un commento del programma |
| Dati che non entrano nel codice QR, un simbolo troppo grande, denso o forte, o che brucerebbe fuori dal piano | la versione, il numero di byte, la dimensione, il numero di movimenti di incisione, la potenza e la velocità restano entro limiti che le impostazioni possono restringere, e ciò che li supera è rifiutato invece di essere tagliato o ridotto; i dati non vengono mai scritti in un commento del programma |
| Il file di uscita scelto sopra il disegno o le impostazioni da cui deriva | rifiutato, nella finestra e nei comandi |
| Un programma inviato a un server, un token che potrebbe essere visto, o una macchina fermata e poi riavviata | il programma viene verificato rispetto ai limiti della macchina prima di partire e uno che non passa non viene inviato; il token non viene mai stampato (né in un messaggio né in un `Debug`), viene dal file delle impostazioni o dall'ambiente e viaggia in chiaro su HTTP semplice, quindi è per una rete fidata o un tunnel (il programma lo dice quando l'indirizzo è pubblico); l'arresto è quello di emergenza, che blocca la macchina finché un amministratore non la sblocca, e a una macchina bloccata non si invia alcun programma |

**Cosa non è difeso:** un risultato prodotto senza errori non è un risultato verificato. Nulla qui conosce i limiti di una macchina e il programma non ha avuto revisione indipendente. Segnalate una vulnerabilità in privato (vedi [SECURITY.md](SECURITY.md)).

## 🔌 Interfacce

La libreria è `eh3d_phoenix_laser` (`use eh3d_phoenix_laser::...`); la riga di comando è `phoenix-laser`. I comandi sono questi; ciascuno stampa il risultato sull'output standard e segnala un errore sull'output di errore con un codice di uscita diverso da zero. Non c'è accesso alla rete.

```text
phoenix-laser engrave IMAGE OUT.nc --width-mm 50 [--dot-mm 0.1] [--feed 3000] [--power 50] [--smax 1000]
              [--dither fs|atkinson|threshold|bayer] [--threshold 128] [--overscan MM|auto] [--accel 1000] [--origin X,Y] [--dry-run]
              [--gamma 1] [--contrast 0] [--brightness 0] [--invert] [--auto-level] [--max-power 100] [--max-speed MM/MIN] [--config FILE] [--profile NAME]
              [--material NAME --materials FILE]
phoenix-laser calibrate OUT.nc --powers 20,40,60,80|20:80:20 --feeds 1000,2000,3000|1000:3000:1000 [--smax 1000] [--max-power 100] [--max-speed MM/MIN] [--config FILE] [--profile NAME]
phoenix-laser calibrate OUT.nc --material NAME --materials FILE [--steps 5] [--spread 40] [--labels|--no-labels] [--label-mm 3] [--smax 1000] [--max-power 100] [--max-speed MM/MIN] [--config FILE] [--profile NAME]
phoenix-laser matrix OUT.lbrn2 --powers 20:80:20 --feeds 600:2400:600 [--square-mm 8] [--gap-mm 3] [--outline]
phoenix-laser ytest OUT.lbrn2
phoenix-laser presets BOOK.cfg [--lang CODE]
phoenix-laser text OUT.nc --text TEXT [--height-mm 10] [--spacing 25] [--line-spacing 50] [--power 20] [--feed 1000] [--passes 1] [--max-minutes 240] [--align left|center|right] [--mirror] [--origin X,Y] [--bed X,Y] [--smax 1000] [--max-power 100] [--max-speed MM/MIN] [--config FILE] [--profile NAME]
phoenix-laser qr OUT.nc --text DATA [--ecc L|M|Q|H] [--module-mm 1] [--line-mm 0.1] [--power 20] [--feed 1000] [--passes 1] [--max-minutes 240] [--invert] [--border-modules 0] [--overscan-mm 0] [--mirror] [--origin X,Y] [--bed X,Y] [--smax 1000] [--max-power 100] [--max-speed MM/MIN] [--print] [--config FILE] [--profile NAME]
phoenix-laser barcode OUT.nc --text DATA [--module-mm 0.4] [--height-mm 10] [--line-mm 0.1] [--power 20] [--feed 1000] [--passes 1] [--max-minutes 240] [--mirror] [--origin X,Y] [--bed X,Y] [--smax 1000] [--max-power 100] [--max-speed MM/MIN] [--config FILE] [--profile NAME]
phoenix-laser marks OUT.nc --size W,H [--origin X,Y] [--arm-mm 5] [--margin-mm 0] [--power 10] [--feed 1000] [--passes 1] [--centre-cross|--no-centre-cross] [--bed X,Y] [--smax 1000] [--max-power 100] [--max-speed MM/MIN] [--config FILE] [--profile NAME]
phoenix-laser info IMAGE
phoenix-laser cut DRAWING OUT.nc [--config FILE] [--project-layers] [--power P] [--speed MM/MIN] [--passes N] [--focus-step MM] [--kerf MM] [--kerf-side none|auto|inside|outside]
              [--tabs N] [--tab-width MM] [--smax N] [--overscan MM] [--origin X,Y] [--bed X,Y] [--max-power P] [--max-speed MM/MIN] [--profile NAME] [--material NAME --materials FILE] [--dpi N] [--tolerance MM] [--dry-run]
phoenix-laser vector DRAWING OUT.nc [the options of cut] [--angle DEG] [--spacing MM] [--cross|--no-cross]
phoenix-laser layers DRAWING [--config FILE] [--project-layers] [--dpi N] [--accel MM/S2]
phoenix-laser convert DRAWING OUT.svg|dxf [--config FILE] [--dpi N] [--tolerance MM]
phoenix-laser trace IMAGE OUT.nc --width-mm W [--threshold 128] [--invert] [--simplify MM] [--min-area MM2] [--engrave] [the options of cut]
phoenix-laser materials FILE
phoenix-laser focus OUT.nc --z-step MM [--z-start MM] [--steps 9] [--length 10] [--spacing 2] [--feed 1000] [--power 30 | --powers 10,20,30] [--smax 1000] [--max-power P] [--max-speed MM/MIN] [--config FILE] [--profile NAME]
phoenix-laser frame OUT.nc --size W,H | --gcode PROGRAM.nc [--origin X,Y] [--margin MM] [--feed 3000] [--laps 1] [--bed X,Y] [--smax N]
              [--pilot P --confirm-laser-on] [--max-power P] [--max-speed MM/MIN] [--config FILE] [--profile NAME]
phoenix-laser verify PROGRAM.nc [--config FILE] [--profile NAME] [--smax N] [--max-power P] [--max-speed MM/MIN] [--bed X,Y] [--overscan-mm MM] [--accel 1000] [--expect-sha256 HASH] [--json]
phoenix-laser report PROGRAM.nc [--config FILE] [--profile NAME] [--lang CODE] [--material NAME --materials FILE]
phoenix-laser settings [--json]
phoenix-laser project list FILE.eh3d [--json]
phoenix-laser project save FILE.eh3d [--drawing D] [--config FILE] [--materials FILE] [--program P.nc] [--name NAME] [--id ID]
phoenix-laser project import FILE.eh3d DRAWING [--name NAME] [--id ID]
phoenix-laser project open FILE.eh3d [--into DIR] | take FILE.eh3d BRANCH NAME | extract FILE.eh3d BRANCH DIR
phoenix-laser preview PROGRAM.nc [OUT.svg] [--accel 1000] [--rapid 6000] [--smax N] [--laser-watts W] [--bed X,Y] [--width-px 900] [--config FILE] [--profile NAME]
phoenix-laser align PICTURE.pgm|png --size W,H --px-per-mm S [--origin X,Y] [--arm-mm 5] [--margin-mm 0] [--centre-cross|--no-centre-cross] [--threshold 128] [--max-shift-mm 20] [--max-angle-deg 5]
              [--config FILE] [--profile NAME] | --simulate OUT.pgm --size W,H --px-per-mm S [--shift DX,DY] [--turn DEG] [--picture-px W,H]
phoenix-laser server status|release [--address HOST:PORT] [--machine ID] [--config FILE] [--profile NAME]
phoenix-laser server send PROGRAM.nc [--name TEXT] [--follow] | job|pause|resume|stop JOB [--address HOST:PORT] [--machine ID] [--config FILE] [--profile NAME]
phoenix-laser server vector DRAWING.svg|dxf [--name TEXT] [--follow] [--address HOST:PORT] [--machine ID] [--config FILE] [--profile NAME] [--material NAME] [--power P] [--speed MM/MIN] [--passes N]
phoenix-laser server test-fire --power-percent P [--seconds S] --confirm-fire [--address HOST:PORT] [--machine ID] [--config FILE] [--profile NAME]
phoenix-laser [gui] [FILE]                       the window (a build with --features gui); with no arguments it opens
```

`DRAWING` è un file SVG o DXF. `cut` segue i contorni (prima quelli interni); `vector` riempie di linee le forme chiuse e segue quelle aperte; entrambi prendono le impostazioni di ogni livello o colore da `--config FILE` (vedi [docs/vector-settings.md](docs/vector-settings.md)) e un'opzione della riga di comando le cambia per tutti i livelli. `layers` elenca i livelli di un disegno con ciò che si farebbe di ciascuno. `convert` riscrive un disegno come SVG o DXF semplice (in base all'estensione dell'uscita, mai sopra il disegno letto), così un altro programma può aprire ciò che questo ha capito.

## ⚙️ Configurazione

L'incisione di un'immagine si imposta con argomenti della riga di comando (vedi Interfacce) e ogni valore predefinito è scritto nella guida del comando (`phoenix-laser help`). Il taglio e l'incisione di un disegno si impostano con un file di impostazioni (`--config FILE`, o «Carica impostazioni» nella finestra): i limiti dei lettori, come si legge un file, l'ordine dei tagli, la macchina e, per ogni livello o colore, la modalità (tagliare, incidere o saltare), la potenza, la velocità, le passate, il passo del fuoco, la larghezza del fascio e il suo lato, i ponticelli e il tratteggio. Ogni chiave, il suo valore predefinito e il suo intervallo sono in [docs/vector-settings.md](docs/vector-settings.md) e un esempio è in `examples/plate.cfg`; una chiave sconosciuta o un valore fuori intervallo è un errore che nomina la sua riga. I limiti fissi del lettore di immagini sono costanti con nome nel codice e sono coperti da test. `phoenix-laser settings` elenca ogni impostazione (sezione, chiave, tipo di valore, valore predefinito e se è un'impostazione di sicurezza) come testo, o come JSON con `--json`, perché un altro programma possa leggerla.

## 🚀 Avvio rapido

Serve una toolchain Rust stabile (1.88 o più recente). Il repository EH3D-COMMON deve essere clonato accanto a questo: il lettore di immagini viene preso da lì tramite percorso.

```bash
cargo build --release
cargo test
cargo run --release -- engrave examples/gradient.pgm examples/gradient.nc --width-mm 16 --dot-mm 0.5 --overscan auto
cargo run --release -- cut examples/plate.svg examples/plate.nc --config examples/plate.cfg
cargo run --release --features gui   # the window
cargo run --release --features gui -- --shot screen.png examples/plate.svg   # a picture of the window
```

Per fare un'immagine della finestra senza aprirla a mano, eseguila con `--shot FILE.png` (anche `--size WxH`, `--theme forge|dark|light`, `--wait N`, `--window NAME`, `--lang CODE`, `--open FILE`, `--replace`): si apre con quella dimensione, salva il PNG, non conserva impostazioni e si chiude da sola (codice di uscita 0 se il file è stato scritto, 1 se no, 2 per argomenti errati). `--window NOME` mostra una finestra del programma: `engrave`, `calibrate`, `cut`, `focus`, `frame`, `preview`, `trace`, `text`, `qr`, `marks`, `barcode` (viene scelta quella modalità) o `server` (la sezione del server è aperta), così le immagini del README possono mostrare ogni modalità. `--window` accetta anche `prepare`, `run`, `review` e `compact`, i quattro spazi di lavoro dei pannelli.

## 🧪 Test

La suite ha 1055 test (`cargo test`, eseguiti su Windows e Linux nella CI). Verificano i casi normali, i casi limite e gli input danneggiati: file vuoti, spazzatura e file tagliati non causano mai un panico. La CI esegue anche `cargo fmt --check`, `cargo clippy` con gli avvisi come errori e `python -I tools/ci_validate.py`.

## 🏗️ Compilazione e rilascio

La versione sta in `Cargo.toml` e in `eh3d.project.json`; ogni versione ha la sua voce in [CHANGELOG.md](CHANGELOG.md). `build.bat` / `build.sh` compilano il programma, `test.bat` / `test.sh` eseguono i test e `run.bat` / `run.sh` lo eseguono con un esempio. Le versioni sono pubblicate su GitHub.

**Download:** Con ogni versione di questo repository vengono pubblicati programmi già pronti per ogni sistema: Windows (64 e 32 bit), Linux (64 bit) e macOS (Apple silicon e Intel). Ogni pacchetto, `phoenix-laser-<versione>-<sistema>.zip` o `.tar.gz`, contiene il programma, la licenza, questo README, il registro delle modifiche, l'icona e i file di esempio, e viene con il suo file `.sha256`; la versione elenca anche ogni file in `SHA256SUMS` e `build-manifest.json`. Le versioni sono elencate nella pagina Releases del repository.

**Firmati e verificabili:** i pacchetti di una versione sono firmati con la chiave dei rilasci di EH3D, e il file `.sig` di ogni pacchetto è pubblicato accanto (una firma OpenSSH, namespace `eh3d-release`). Verifica un download prima di eseguirlo: confronta il suo SHA-256 (`sha256sum -c phoenix-laser-<versione>-<sistema>.tar.gz.sha256`, oppure `Get-FileHash` su Windows) e verifica la firma con `ssh-keygen -Y verify -f allowed_signers -I eh3d-release -n eh3d-release -s <pacchetto>.sig < <pacchetto>`, dove `allowed_signers` contiene la riga `eh3d-release <tipo di chiave> <chiave pubblica dei rilasci di EH3D>`.

## 📂 Struttura del repository

<p align="center">
  <img src="images/repository-structure.svg" alt="Struttura del repository EH3D-PHOENIX-LASER" width="100%">
</p>

<details>
<summary>Versione testuale dell'albero</summary>

```text
EH3D-PHOENIX-LASER/
├── src/
│   ├── lib.rs                                   la libreria: ciò che usa ogni comando
│   ├── main.rs                                  la riga di comando: engrave, calibrate, cut, vector, verify, report, convert, settings e altro
│   ├── calibrate.rs                             le griglie di prova: potenza contro avanzamento, scala di fuoco e griglia di un materiale
│   ├── cli_args.rs                              gli argomenti della riga di comando, lo stesso file in ogni piccola utilità
│   ├── dither.rs                                dai livelli di grigio ai punti: soglia, Floyd-Steinberg, Atkinson e Bayer
│   ├── materials.rs                             la libreria dei materiali: potenza, velocità, passate, toni
│   ├── commonmat.rs                             punti di partenza della base comune dei materiali, così come sono
│   ├── preview.rs                               l'anteprima di un G-code: movimenti, tempo, pericoli e un SVG del percorso
│   ├── tone.rs                                  la curva dei toni di un'immagine: livelli, contrasto, luminosità, gamma e inversione prima dei punti
│   ├── trace.rs                                 i contorni di un'immagine: soglia, marching squares e anelli semplificati, con limiti
│   ├── text.rs                                  una riga di testo incisa con un carattere a tratti proprio, entro limiti
│   ├── qr.rs                                    un codice QR proprio, versioni da 1 a 4 con correzione degli errori, inciso riempiendo i moduli
│   ├── barcode.rs                               un codice a barre Code 128 proprio, inciso riempiendo le barre di linee
│   ├── gcode_out.rs                             l'unico modo di scrivere un numero G-code e l'intestazione standard
│   ├── raster.rs                                l'incisione riga per riga: dai punti ai movimenti, extracorsa e potenza
│   ├── safety.rs                                il limite di potenza e le regole che tengono spento il laser mentre si mostra il lavoro
│   ├── guard.rs                                 le zone vietate del piano e gli interblocchi: ciò che un programma non può raggiungere
│   ├── server.rs                                la strada verso il server EH3D: verificare, inviare, seguire, arresto di emergenza
│   ├── linkpane.rs                              il pannello del server nella finestra: le caselle, il collegamento e l'accesso
│   ├── shot.rs                                  la modalità immagine della finestra (--shot): i suoi argomenti, letti e verificati senza schermo
│   ├── vector.rs                                un disegno letto: forme in mm, livelli, limiti e geometria di base
│   ├── verify.rs                                il controllo di un programma: il suo SHA-256, ciò che fa e un verdetto rispetto ai limiti della macchina
│   ├── report.rs                                il rapporto che si legge prima di un lavoro, nelle otto lingue
│   ├── flatten.rs                               curve in tratti rettilinei entro una tolleranza (Bézier, archi)
│   ├── frame.rs                                 la cornice del lavoro (il rettangolo percorso con il laser spento) e i segni ai suoi angoli
│   ├── register.rs                              dove si trova il lavoro: i segni trovati in un'immagine e la telecamera simulata
│   ├── svg.rs                                   il lettore SVG: un sottoinsieme sicuro, senza script, immagini o entità
│   ├── testfiles.rs                             la matrice potenza contro velocità e la prova dell'asse Y, come progetti
│   ├── dxf.rs                                   il lettore DXF: linee, polilinee, archi, cerchi, ellissi, spline e blocchi, con unità
│   ├── eh3dfile.rs                              il ramo laser di un progetto .eh3d: salvare, importare, prendere, aprire, estrarre
│   ├── projcli.rs                               il comando project della riga di comando
│   ├── suitecore.rs                             lo stato di un progetto .eh3d per la finestra: non aggiornato, firmato, cosa chiedere prima di scrivere
│   ├── suitetext.rs                             i testi delle voci di progetto della finestra in otto lingue
│   ├── suitewin.rs                              il menu File della finestra per un progetto .eh3d: aprire, salvare, importare, prendere, estrarre
│   ├── lbrn2.rs                                 il lettore dei progetti LBRN2: livelli, forme, gruppi e curve
│   ├── lbrn2_pic.rs                             le immagini di un progetto LBRN2: base64, PNG, limiti, ruotate come posizionate
│   ├── vartext.rs                               il testo variabile: un contatore e le colonne di un CSV, un testo per riga
│   ├── sublayers.rs                             passate extra su un livello: impostazioni proprie, limiti e ordine del lavoro
│   ├── job.rs                                   dove inizia il lavoro, la cornice che lo mostra e il tempo come intervallo
│   ├── live.rs                                  la regolazione in tempo reale di potenza e velocità, con quattro limiti (nessuna macchina toccata)
│   ├── presets.rs                               preimpostazioni di materiale: assegnare (copia) o collegare (segue), con avviso dei valori non provati
│   ├── export.rs                                un disegno riscritto come SVG o DXF semplice
│   ├── energy.rs                                l'energia di una passata per millimetro o millimetro quadrato, dalla potenza ottica
│   ├── offset.rs                                spostare un contorno di lato della larghezza del fascio, verificato spigolo per spigolo
│   ├── order.rs                                 l'ordine dei tagli: linee unite, prima l'interno, poco spostamento
│   ├── cut.rs                                   dal disegno al G-code: taglio, ponticelli, tratteggio, passate e verifica di sicurezza
│   ├── vconfig.rs                               il file di impostazioni del lavoro vettoriale: limiti, macchina e ogni livello
│   ├── contract.rs                              ogni impostazione come elenco: sezione, chiave, tipo, valore predefinito e se è di sicurezza
│   ├── browse.rs                                l'elenco dei file della finestra che apre un file
│   ├── gui.rs                                   la finestra (egui, con la funzione gui): disegna lo stato e inoltra ciò che fa la persona
│   ├── panels.rs                                i pannelli, gli spazi di lavoro e i menu della finestra come dati
│   ├── lang.rs                                  le parole della finestra nelle otto lingue
│   ├── viewport.rs                              zoom e spostamento dell'immagine, in semplici numeri
│   └── window.rs                                lo stato della finestra senza schermo: immagine o disegno, impostazioni, livelli, passate ed esportazione
├── tests/
│   ├── fuzz.rs                                  test con seme fisso: file danneggiati, tagli e numeri strani
│   ├── dry_run.rs                               test di --dry-run: conta lo stesso e non scrive nulla
│   ├── laser.rs                                 test della libreria e della riga di comando
│   ├── laser_tools.rs                           test degli strumenti attorno all'incisione, dalla riga di comando
│   ├── readme.rs                                test che eseguono ciò che promettono il README e gli script run
│   ├── strict.rs                                test della riga di comando: opzioni e parole che non conosce
│   ├── vector.rs                                test del taglio da disegni: comandi, esempi, file ostili e verifica del G-code
│   ├── lbrn2.rs                                 test del lettore LBRN2 su progetti scritti qui, file ostili e fuzz
│   ├── lbrn2_more.rs                            test delle immagini LBRN2, dell'asse y, ellisse, poligono, ponti e fuzz
│   ├── lbrn_v1.rs                               prove del vecchio formato di progetto, dei suoi testi e dei livelli di un progetto sulla riga di comando
│   ├── dxf_hatch.rs                             prove dei tratteggi e dei solidi di un DXF, con fuzz a seme fisso
│   ├── vartext.rs                               test del testo variabile: CSV, contatori, segnaposto, serie e fuzz
│   ├── sublayers.rs                             test delle passate extra: ordine, limiti, file danneggiati con seme fisso
│   ├── joborigin.rs                             test dei nove ancoraggi, della cornice a laser spento e del tempo come intervallo
│   ├── livelimits.rs                            test della regolazione in tempo reale: quattro limiti, otto lingue, seme fisso
│   ├── presets.rs                               test delle preimpostazioni: assegnare, collegare, modifiche rifiutate, alternative e serie casuale
│   ├── presetbook.rs                            prove del libro dei preset conservato come testo
│   ├── matrix.rs                                prove degli intervalli della griglia di prova e della sua matrice
│   ├── testfiles.rs                             prove dei progetti di matrice e di asse Y che il programma scrive
│   ├── samples.rs                               i lettori su una cartella di campioni, quando ne è indicata una
│   ├── guard.rs                                 test delle zone, degli interblocchi, dei tetti dei materiali, del rapporto e dell'elenco delle impostazioni
│   ├── server_cli.rs                            il comando server contro un sostituto del plugin laser
│   ├── window_cli.rs                            test che una compilazione senza la funzione gui resti una riga di comando
│   ├── dockwindow.rs                            test del modello della finestra: livelli di un progetto, rapporto, interblocchi, disposizione e materiali
│   ├── eh3dfile.rs                              test del ramo del progetto: andata e ritorno, altri rami, copie superate, file danneggiati
│   ├── suitewindow.rs                           test del menu di progetto della finestra, senza schermo
│   ├── reqs.rs                                  test del registro dei requisiti
│   ├── more.rs                                  il G-code riletto, i numeri e la griglia
│   ├── more2.rs                                 l'incisione al limite di ogni valore
│   └── rapid_power.rs                           nessun movimento rapido con potenza impostata
├── examples/
│   ├── gradient.pgm                             un esempio per provare il programma
│   ├── plate.svg                                una piastra con un foro e un pannello da incidere (SVG)
│   ├── plate.dxf                                la stessa piastra in DXF
│   ├── plate.cfg                                le impostazioni dei suoi livelli: potenza, velocità, passate, larghezza del fascio, ponticelli e tratteggio
│   ├── materials.cfg                            una libreria di materiali: betulla e ardesia, taglio, incisione e fotografie
│   ├── safety.cfg                               zone vietate e interblocchi da provare con verify
│   ├── materials-starter.cfg                    una libreria iniziale di materiali, tutti segnati come non verificati
│   └── lbrn2_survey.rs                          legge tutti i file LBRN2 di una cartella e dice cosa si legge e cosa no
├── docs/
│   ├── architecture.json                        dati dell'immagine di Architettura
│   ├── NOT_YET.md                               buone idee fuori da questa versione, con il perché e il quando
│   ├── REQUIREMENTS.md                          il registro dei requisiti: stato, evidenza e test di ciascuno
│   ├── SCOPE.md                                 a cosa serve, cosa non farà e cosa non decide mai
│   ├── STATUS.md                                cosa esiste, cosa manca e cosa è ancora aperto
│   ├── structure.json                           dati dell'immagine di Struttura
│   ├── vector-settings.md                       il file di impostazioni del lavoro vettoriale, chiave per chiave
│   ├── materials.md                             la libreria dei materiali, chiave per chiave
│   ├── eh3d-project.md                          il progetto .eh3d e il ramo laser
│   └── lbrn2.md                                 cosa legge il lettore LBRN2, cosa lascia fuori e come è stato misurato
├── tools/                                       controllo della CI e del README e strumenti per le immagini
├── images/                                      banner, immagini del README e schermate della finestra
├── .github/                                     workflow di CI e di rilascio, modelli di issue e pull request
├── eh3d.project.json                            manifesto: versione, ruolo, maturità e comando di compilazione
├── Cargo.toml Cargo.lock                        il workspace e le sue dipendenze bloccate
├── rust-toolchain.toml                          la toolchain Rust fissata
├── CHANGELOG.md                                 cosa è cambiato in ogni versione
├── AGENTS.md                                    le regole per chi modifica il repository
├── SECURITY.md SUPPORT.md                       come segnalare una vulnerabilità e dove chiedere
└── CONTRIBUTING.md CODE_OF_CONDUCT.md LICENSE   come contribuire, codice di condotta e licenza
```

</details>

## 🗺️ Roadmap e limiti noti

**Limiti noti oggi**

* Non è stato provato su una macchina né con una raccolta di file reali e non ha avuto revisione indipendente: nulla è stato bruciato, i controlli leggono il programma come testo e il server è un sostituto.
* Il lettore dei progetti LBRN2 è stato misurato su dodici progetti reali di un produttore di laser e su nient'altro: le immagini dentro un progetto, le forme di un tipo assente da quei progetti, il verso dell'asse y e l'angolo arrotondato di un rettangolo non sono confermati.
* Un disegno viene letto in un sottoinsieme sicuro (niente `use` di un altro file, testo, sfumature o stili con selettori; niente tratteggi, testo o quote in un DXF); un contorno più stretto del fascio non viene tagliato e il programma lo dice; il laser è quello di un controller GRBL con `M4`.
* Il codice QR e il codice a barre sono riletti da lettori propri del programma; nessuno scanner ha letto un simbolo bruciato, e la finestra non è stata vista su uno schermo da una persona.
* I sotto-livelli, i collegamenti delle preimpostazioni e la regolazione in tempo reale non hanno ancora posto nella finestra; i collegamenti delle preimpostazioni sono tenuti in memoria e non nel file delle impostazioni, e nulla della regolazione in tempo reale è inviato a una macchina.

**Prossimi passi**

* Una prima prova su un laser e un materiale reali, con una persona alla macchina e l'arresto di emergenza a portata di mano.
* Mostrare la densità di potenza sulla scheda prima di bruciare (`layers` stampa già l'energia di una passata quando `laser_watts` è impostato) e mantenere costante la potenza negli angoli di un taglio.
* Vedere la finestra su uno schermo e finirla.
* Provarlo con disegni reali e con gli altri programmi della suite.
* Le idee lasciate fuori di proposito sono in [Non ancora](docs/NOT_YET.md).

## 🌍 Lingue

Il README e i testi esistono in inglese, spagnolo, francese, italiano, tedesco, cinese, giapponese e portoghese europeo.

## 🔗 Progetti correlati

* [EH3D-COMMON](https://github.com/JuanenRac/EH3D-COMMON): i crate condivisi, tra cui il lettore di immagini
* [EH3D-STENCILCAM](https://github.com/JuanenRac/EH3D-STENCILCAM): gli stencil per pasta saldante
* [EH3D-CAM-DEBUGGER](https://github.com/JuanenRac/EH3D-CAM-DEBUGGER): il controllo del G-code prima che arrivi a una macchina
* [EH3D-TOOL-MANAGER](https://github.com/JuanenRac/EH3D-TOOL-MANAGER): il magazzino utensili condiviso dai programmi CAM
* [EH3D-MOTIONCNC](https://github.com/JuanenRac/EH3D-MOTIONCNC): il programma di controllo della macchina
* [EH3D-SUITE-LAUNCHER](https://github.com/JuanenRac/EH3D-SUITE-LAUNCHER): il centro che trova e avvia i programmi della suite

## 📚 Documentazione

* [Changelog](CHANGELOG.md)
* [Ambito](docs/SCOPE.md): a cosa serve il programma, cosa non farà e cosa non decide mai.
* [Requisiti](docs/REQUIREMENTS.md): il registro delle capacità, lo stato di ciascuna e il test che la mostra.
* [Stato](docs/STATUS.md): cosa esiste, cosa manca e cosa è ancora aperto.
* [Non ancora](docs/NOT_YET.md): buone idee lasciate fuori da questa versione.
* [Regole per modificare il repository](AGENTS.md): le regole per chi modifica il repository.
* [Dati di architettura e struttura](docs/architecture.json): i dati da cui si disegnano le due immagini.
* [Impostazioni del lavoro vettoriale](docs/vector-settings.md): ogni chiave del file di impostazioni con il suo valore predefinito e il suo intervallo.
* [La libreria dei materiali](docs/materials.md): ogni chiave di un materiale e come si applica

## 🤝 Contribuire e comunità

* Prima di una pull request eseguite `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` e `python -I tools/ci_validate.py`: tutto deve passare. Vedi [CONTRIBUTING.md](CONTRIBUTING.md) e il [codice di condotta](CODE_OF_CONDUCT.md).
* Il README esiste in otto lingue: una modifica al testo si fa in tutti e otto i file, e le due immagini sono disegnate da `docs/architecture.json` e `docs/structure.json` con `tools/make_architecture.py` e `tools/make_structure.py`.
* Segnalate una vulnerabilità in privato: [SECURITY.md](SECURITY.md). Domande, idee e segnalazioni: electrohobby3d@gmail.com ([SUPPORT.md](SUPPORT.md)).

## 👤 AUTORE

**JuanenRac (Electro Hobby 3D)** · electrohobby3d@gmail.com

## 📜 LICENZA

Software: GPL-3.0-or-later. Vedi [LICENSE](LICENSE). I progetti hardware della suite usano la CERN OHL-S.

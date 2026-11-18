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
  <img src="https://img.shields.io/badge/Version-0.0.8-0f6cbd.svg" alt="Version">
  <img src="https://img.shields.io/badge/Maturity-functional-4CAF50.svg" alt="Maturity">
</p>

---

**Verifica di onestà - cosa funziona oggi:** una libreria Rust e un programma a riga di comando (`phoenix-laser`) con 473 test che girano su un computer: trasforma un'immagine in punti con la retinatura e scrive il G-code di un laser che brucia riga per riga con potenza dinamica ed extracorsa, legge disegni SVG e DXF e scrive il G-code che ne taglia i contorni (prima quelli interni, con la larghezza del fascio, ponticelli e più passate) e ne incide le forme con linee, con le impostazioni di ogni livello in un file, e una griglia di prova potenza contro avanzamento. Ha una finestra (compilata con `--features gui`) che apre un'immagine o un disegno, mostra il percorso del laser, permette di cambiare le impostazioni ed esporta il G-code, ma la finestra non è ancora stata vista su uno schermo; non è stato provato su una macchina e non è stato verificato con una raccolta di file reali.

---

> **Stato di sicurezza:** questo programma non controlla alcuna macchina e non è un sistema di sicurezza. Ciò che produce deve essere controllato da una persona prima di eseguire qualsiasi cosa su una macchina; nulla di questo ha girato su hardware reale.


## 📋 In breve

| | |
|---|---|
| Versione | `0.0.8` |
| Maturità | Funzionale |
| Test | 473 |
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
| Test | 🟡 | 473 test passano (`cargo test`), con casi normali, casi limite e input danneggiati. |
| Sicurezza | 🟡 | Rust sicuro e semplice (`forbid(unsafe_code)`): nulla va in panico con i file ricevuti e un valore errato è un errore con il suo motivo; nessuna revisione indipendente. |
| Finestra | 🟠 | Scritta (egui, `--features gui`): aprire o trascinare un'immagine o un disegno SVG o DXF, vedere il percorso del laser (il taglio in un colore, lo spostamento attenuato), modificare la tabella dei livelli, una griglia di calibrazione, zoom e spostamento, stimare il tempo, esportare; la sua logica è testata, ma non è ancora stata compilata né vista su uno schermo. |
| Hardware reale | 🔴 | Non provato: nulla di questo è stato eseguito su una macchina. |
| Documentazione | 🟡 | README in otto lingue, il changelog e le due immagini disegnate da `docs/`. |

## 🎯 Panoramica

* **L'immagine** (`eh3d-image`): PNG (grigio, colore, trasparente, 8 o 16 bit) e PGM (testo e binario) diventano livelli di grigio, con le parti trasparenti come bianco e la dimensione limitata perché un file ostile non possa chiedere gigabyte; `resize` fa la media dei pixel così che un punto sia grande quanto il fascio.
* **I punti** (`src/dither.rs`): una soglia o la diffusione dell'errore di Floyd-Steinberg o Atkinson; i test verificano che l'oscurità di un grigio si conservi (un grigio medio brucia circa metà dei punti) e che il bianco non bruci nulla e il nero tutto.
* **Il laser** (`src/raster.rs`, `src/calibrate.rs`): una riga dopo l'altra, da sinistra a destra e poi da destra a sinistra, `M4` con `S0` durante lo spostamento e la potenza scelta dove brucia, un overscan fisso o calcolato dall'accelerazione (v al quadrato su 2a) e una griglia di quadrati con una potenza per colonna e un avanzamento per riga per trovare ciò che si adatta a un materiale.
* **L'intestazione e i numeri di ogni file G-code** (`src/gcode_out.rs`): il file si apre con righe di commento che dicono il generatore e la sua versione, le unità (mm, G21), l'immagine da cui è nato con il suo SHA-256 e gli utensili (il laser e la sua potenza); una griglia di calibrazione dice di non avere origine. Un numero si scrive in un solo modo (al massimo tre decimali, niente `-0`) e un valore non finito viene rifiutato invece di essere scritto come `NaN` o `inf`.
* **`--dry-run`:** `engrave` calcola il risultato e lo conta (punti, righe, tratti, lunghezza bruciata, tempo) senza scrivere il file di uscita, che allora può essere omesso.
* **I disegni** (`src/svg.rs`, `src/dxf.rs`, `src/flatten.rs`, `src/vector.rs`): un sottoinsieme sicuro di SVG (tracciati con tutti i comandi, forme, trasformazioni, gruppi, unità) e le entità di un DXF (linee, polilinee con archi, archi, cerchi, spline, unità) diventano forme in millimetri; script, immagini, `use`, entità dichiarate e tutto ciò che è sconosciuto vengono saltati o rifiutati, la dimensione, la profondità e il numero di elementi e di punti sono limitati dalle impostazioni, e le curve diventano tratti rettilinei entro una tolleranza.
* **Taglio e incisione** (`src/order.rs`, `src/offset.rs`, `src/cut.rs`): le forme sono raggruppate per livello (DXF) o colore (SVG); ogni gruppo viene tagliato (linee sciolte unite, prima i contorni dentro un altro, il minor spostamento, la larghezza del fascio tolta dal lato giusto, ponticelli lasciati, più passate con il fuoco che avanza) o inciso (forme chiuse riempite di linee con un angolo, con la regola pari-dispari) con le sue impostazioni.
* **Il file di impostazioni** (`src/vconfig.rs`, `docs/vector-settings.md`): ogni valore da regolare (i limiti dei lettori, la macchina, le impostazioni di ogni livello) è in un file di testo semplice controllato riga per riga; nulla di tutto ciò è nel codice.
* **La prova di fuoco** (`src/calibrate.rs`, `[focus]` nel file delle impostazioni): `phoenix-laser focus` brucia una scala di linee corte, ciascuna con la testa a un'altra altezza, e la linea più nitida mostra dove il fascio è a fuoco; il passo di altezza non ha un valore predefinito (cosa sia giù dipende dalla macchina), la potenza rispetta il limite di potenza e la testa torna alla prima altezza.
* **La cornice del lavoro** (`src/frame.rs`, `[frame]` nel file delle impostazioni): `phoenix-laser frame` scrive un programma che percorre il rettangolo del lavoro con il laser spento, da una dimensione o dal riquadro di ciò che un programma G-code brucia, per vedere dove cadrà; una debole potenza pilota richiede ogni volta `--confirm-laser-on`, è limitata al 5 % (`max_pilot_percent` di `[frame]` cambia il tetto) e rispetta il limite di potenza, e una cornice fuori dal piano viene rifiutata.
* **L'anteprima** (`src/preview.rs`): `phoenix-laser preview` legge un programma G-code (proprio o altrui), stampa i movimenti, la lunghezza bruciata e percorsa, il riquadro, le potenze e un tempo che conta la frenata negli angoli, avverte di ciò che sembra pericoloso (laser acceso in uno spostamento rapido, nessun avanzamento, potenza oltre il massimo, lavoro fuori dal piano, nessuna fine) e disegna il percorso come SVG; i test rileggono ciò che scrive l'incisione e verificano la stessa lunghezza.
* **La libreria dei materiali** (`src/materials.rs`, `examples/materials.cfg`): ciò che si addice a ogni materiale e modo di lavorare si scrive una volta in un file di testo (`[material NOME]`: potenza, velocità, passate, passo di fuoco, larghezza del fascio, tratteggio, dimensione del punto, dithering e curva dei toni) e si sceglie con `--material NOME --materials FILE`; le opzioni della riga di comando prevalgono e il limite di potenza vale su entrambi. `phoenix-laser materials FILE` elenca la libreria.
* **Il limite di potenza** (`src/safety.rs`, `max_power_percent` in `[machine]`, `--max-power`): un programma la cui potenza supera il limite viene rifiutato con un errore che nomina il livello o l'opzione, mai abbassato senza dirlo; il limite è 100 % finché un file o un'opzione non dice meno.
* **Profili di macchina** (`[profile NAME]` nel file delle impostazioni, `--profile NAME`): chi ha due macchine, o sostituisce il modulo laser con un altro, tiene un profilo per ciascuna (la corsa del piano, il limite di potenza, il limite di velocità, il valore di piena potenza e i watt); il profilo scelto è la macchina, e `--max-speed` affianca `--max-power`: una velocità oltre il limite viene rifiutata, mai abbassata.
* **Contorni da un'immagine** (`src/trace.rs`, `[trace]` nel file delle impostazioni): `phoenix-laser trace` segue il contorno di un logo, di una sagoma o di una maschera invece di bruciarlo punto per punto; i livelli di grigio si separano con una soglia, il bordo si trova con marching squares a precisione sub-pixel, si semplifica finché nessun punto dista più di `simplify_mm` dalla linea e le macchie si scartano; il risultato è un livello di un disegno, tagliato (o riempito di linee) con le stesse impostazioni e gli stessi controlli, e la dimensione dell'immagine, i pezzi di bordo, i contorni e i punti sono tutti limitati.
* **Toni, materiali e limiti nella finestra** (`src/window.rs`): le caselle della curva dei toni (gamma, contrasto, luminosità, inversione e livelli automatici) lavorano l'immagine con la stessa funzione della riga di comando; una libreria di materiali si carica da un file (o da quello indicato da `PHOENIX_LASER_MATERIALS`) e scegliere un materiale riempie le caselle di potenza, velocità, punto, dithering, toni e ogni livello di un disegno; i limiti di potenza e di velocità sono anch'essi caselle e un valore oltre il limite viene rifiutato, mai abbassato. I valori iniziali vengono dal file delle impostazioni (`[tone]`, `[machine]`).
* **Il test di messa a fuoco nella finestra** (`src/window.rs`): una modalità con le caselle della sezione `[focus]` (l'altezza della prima linea, il passo, il numero e la lunghezza delle linee, la distanza, la velocità e la potenza); il test si calcola con la stessa funzione di `phoenix-laser focus`, potenza e velocità rispettano i limiti, non c'è un passo predefinito perché dov'è il basso dipende dalla macchina, e un avviso che il test muove Z resta nelle note.
* **La cornice nella finestra** (`src/window.rs`, `[frame]` nel file delle impostazioni): una modalità che inquadra l'immagine o il disegno aperto con il laser spento; margine, giri e velocità sono caselle. Una potenza pilota maggiore di 0 accende il laser, quindi la finestra chiede e non si calcola né si scrive nulla finché la persona non risponde sì, ogni volta (cambiare una casella, cambiare modalità o scrivere il file ritira la risposta), sotto il tetto di `max_pilot_percent`, il limite di potenza e il piano.
* **L'anteprima nella finestra** (`src/window.rs`): ogni G-code calcolato dalla finestra è contato dalla stessa anteprima di `phoenix-laser preview` (movimenti, lunghezza bruciata e percorsa, un tempo che conta gli angoli con l'accelerazione della casella e ciò che sembra pericoloso, come un lavoro fuori dal piano del file delle impostazioni), e un programma di qualsiasi origine (`.nc`, `.ngc`, `.gcode`, `.gc`, `.tap`) può essere aperto o trascinato sulla finestra per vederne il percorso e le note. Si legge come testo, non si esegue mai e non si riscrive.
* **Il controllo di un programma** (`src/verify.rs`): `phoenix-laser verify PROGRAMMA.nc` legge un programma G-code (di questo programma o di un altro) come testo, non lo esegue mai, e dà lo SHA-256 del file esatto, ciò che fa e un verdetto: passa solo sotto il limite di potenza e di velocità della macchina, dentro il piano, con il laser spento in ogni movimento rapido e un avanzamento in ogni movimento che brucia. `--json` dà il rapporto che un altro programma può conservare accanto a un'autorizzazione legata a quell'hash, e il codice di uscita è 1 quando fallisce.
* **Dithering ordinato** (`src/dither.rs`): `--dither bayer` (anche nella finestra) decide ogni punto in base al suo posto e al suo grigio con una matrice di Bayer 8 x 8, quindi il motivo è regolare e lo stesso grigio brucia sempre gli stessi punti; i test verificano che ogni livello della matrice compaia una volta e che la scurezza di un grigio si conservi.
* **La curva dei toni** (`src/tone.rs`): prima di fare i punti i grigi si possono stirare su tutta la gamma (`--auto-level`), cambiare di contrasto e luminosità, curvare con un gamma o invertire, con un'unica tabella di 256 voci; con i valori predefiniti l'immagine non cambia affatto.
* **Un G-code verificato:** il programma di un disegno viene verificato riga per riga prima di essere consegnato: nessun laser acceso durante un movimento rapido o senza velocità, nessuna `S` oltre la piena potenza, nessun movimento fuori dal piano, `M5` alla fine; il file non viene mai scritto sopra il disegno né sopra le impostazioni da cui deriva.

## 🧩 Ruolo in EH3D

EH3D-PHOENIX-LASER è un programma della EH3D Fabrication Suite. Funziona come libreria e riga di comando e, compilato con `--features gui`, come finestra con l'aspetto comune agli altri programmi della suite. Legge e scrive file semplici, quindi può essere usato da solo o tra gli altri programmi.

## 🏛️ Architettura

<p align="center">
  <img src="images/architecture.svg" alt="Architettura di EH3D-PHOENIX-LASER: chi lo usa, che cosa contiene e da che cosa dipende" width="100%">
</p>


Come si incastrano le parti di questo repository: chi lo usa, cosa contiene e da cosa dipende. L'immagine è disegnata da `docs/architecture.json` e verificata con il codice.

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
| Il file di uscita scelto sopra il disegno o le impostazioni da cui deriva | rifiutato, nella finestra e nei comandi |

**Cosa non è difeso:** un risultato prodotto senza errori non è un risultato verificato. Nulla qui conosce i limiti di una macchina e il programma non ha avuto revisione indipendente. Segnalate una vulnerabilità in privato (vedi [SECURITY.md](SECURITY.md)).

## 🔌 Interfacce

La libreria è `eh3d_phoenix_laser` (`use eh3d_phoenix_laser::...`); la riga di comando è `phoenix-laser`. I comandi sono questi; ciascuno stampa il risultato sull'output standard e segnala un errore sull'output di errore con un codice di uscita diverso da zero. Non c'è accesso alla rete.

```text
phoenix-laser engrave IMAGE OUT.nc --width-mm 50 [--dot-mm 0.1] [--feed 3000] [--power 50] [--smax 1000]
              [--dither fs|atkinson|threshold|bayer] [--threshold 128] [--overscan MM|auto] [--accel 1000] [--origin X,Y] [--dry-run]
              [--gamma 1] [--contrast 0] [--brightness 0] [--invert] [--auto-level] [--max-power 100] [--max-speed MM/MIN] [--config FILE] [--profile NAME]
              [--material NAME --materials FILE]
phoenix-laser calibrate OUT.nc --powers 20,40,60,80 --feeds 1000,2000,3000 [--smax 1000] [--max-power 100] [--max-speed MM/MIN] [--config FILE] [--profile NAME]
phoenix-laser info IMAGE
phoenix-laser cut DRAWING OUT.nc [--config FILE] [--power P] [--speed MM/MIN] [--passes N] [--focus-step MM] [--kerf MM] [--kerf-side none|auto|inside|outside]
              [--tabs N] [--tab-width MM] [--smax N] [--overscan MM] [--origin X,Y] [--bed X,Y] [--max-power P] [--max-speed MM/MIN] [--profile NAME] [--material NAME --materials FILE] [--dpi N] [--tolerance MM] [--dry-run]
phoenix-laser vector DRAWING OUT.nc [the options of cut] [--angle DEG] [--spacing MM]
phoenix-laser layers DRAWING [--config FILE] [--dpi N]
phoenix-laser trace IMAGE OUT.nc --width-mm W [--threshold 128] [--invert] [--simplify MM] [--min-area MM2] [--engrave] [the options of cut]
phoenix-laser materials FILE
phoenix-laser focus OUT.nc --z-step MM [--z-start MM] [--steps 9] [--length 10] [--spacing 2] [--feed 1000] [--power 30] [--smax 1000] [--max-power P] [--max-speed MM/MIN] [--config FILE] [--profile NAME]
phoenix-laser frame OUT.nc --size W,H | --gcode PROGRAM.nc [--origin X,Y] [--margin MM] [--feed 3000] [--laps 1] [--bed X,Y] [--smax N]
              [--pilot P --confirm-laser-on] [--max-power P] [--max-speed MM/MIN] [--config FILE] [--profile NAME]
phoenix-laser verify PROGRAM.nc [--config FILE] [--profile NAME] [--smax N] [--max-power P] [--max-speed MM/MIN] [--bed X,Y] [--accel 1000] [--json]
phoenix-laser preview PROGRAM.nc [OUT.svg] [--accel 1000] [--rapid 6000] [--smax N] [--bed X,Y] [--width-px 900] [--config FILE] [--profile NAME]
phoenix-laser [gui] [FILE]                       the window (a build with --features gui); with no arguments it opens
```

`DRAWING` è un file SVG o DXF. `cut` segue i contorni (prima quelli interni); `vector` riempie di linee le forme chiuse e segue quelle aperte; entrambi prendono le impostazioni di ogni livello o colore da `--config FILE` (vedi [docs/vector-settings.md](docs/vector-settings.md)) e un'opzione della riga di comando le cambia per tutti i livelli. `layers` elenca i livelli di un disegno con ciò che si farebbe di ciascuno.

## ⚙️ Configurazione

L'incisione di un'immagine si imposta con argomenti della riga di comando (vedi Interfacce) e ogni valore predefinito è scritto nella guida del comando (`phoenix-laser help`). Il taglio e l'incisione di un disegno si impostano con un file di impostazioni (`--config FILE`, o «Carica impostazioni» nella finestra): i limiti dei lettori, come si legge un file, l'ordine dei tagli, la macchina e, per ogni livello o colore, la modalità (tagliare, incidere o saltare), la potenza, la velocità, le passate, il passo del fuoco, la larghezza del fascio e il suo lato, i ponticelli e il tratteggio. Ogni chiave, il suo valore predefinito e il suo intervallo sono in [docs/vector-settings.md](docs/vector-settings.md) e un esempio è in `examples/plate.cfg`; una chiave sconosciuta o un valore fuori intervallo è un errore che nomina la sua riga. I limiti fissi del lettore di immagini sono costanti con nome nel codice e sono coperti da test.

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

Per fare un'immagine della finestra senza aprirla a mano, eseguila con `--shot FILE.png` (anche `--size WxH`, `--theme forge|dark|light`, `--wait N`, `--lang CODE`, `--open FILE`, `--replace`): si apre con quella dimensione, salva il PNG, non conserva impostazioni e si chiude da sola (codice di uscita 0 se il file è stato scritto, 1 se no, 2 per argomenti errati).

## 🧪 Test

La suite ha 473 test (`cargo test`, eseguiti su Windows e Linux nella CI). Verificano i casi normali, i casi limite e gli input danneggiati: file vuoti, spazzatura e file tagliati non causano mai un panico. La CI esegue anche `cargo fmt --check`, `cargo clippy` con gli avvisi come errori e `python -I tools/ci_validate.py`.

## 🏗️ Compilazione e rilascio

La versione sta in `Cargo.toml` e in `eh3d.project.json`; ogni versione ha la sua voce in [CHANGELOG.md](CHANGELOG.md). `build.bat` / `build.sh` compilano il programma, `test.bat` / `test.sh` eseguono i test e `run.bat` / `run.sh` lo eseguono con un esempio. Le versioni sono pubblicate su GitHub.

**Download:** Con ogni versione di questo repository vengono pubblicati programmi già pronti per ogni sistema: Windows (64 e 32 bit), Linux (64 bit) e macOS (Apple silicon e Intel). Ogni pacchetto, `phoenix-laser-<versione>-<sistema>.zip` o `.tar.gz`, contiene il programma, la licenza, questo README, il registro delle modifiche, l'icona e i file di esempio, e viene con il suo file `.sha256`; la versione elenca anche ogni file in `SHA256SUMS` e `build-manifest.json`. Non è ancora stata pubblicata nessuna versione.

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
│   ├── main.rs                                  la riga di comando: engrave, calibrate, info, cut, vector, trace, verify e layers
│   ├── calibrate.rs                             una parte della logica
│   ├── cli_args.rs                              gli argomenti della riga di comando, lo stesso file in ogni piccola utilità
│   ├── dither.rs                                una parte della logica
│   ├── materials.rs                             la libreria dei materiali: potenza, velocità, passate, toni
│   ├── preview.rs                               l'anteprima di un G-code: movimenti, tempo, pericoli e un SVG del percorso
│   ├── tone.rs                                  la curva dei toni di un'immagine: livelli, contrasto, luminosità, gamma e inversione prima dei punti
│   ├── trace.rs                                 i contorni di un'immagine: soglia, marching squares e anelli semplificati, con limiti
│   ├── gcode_out.rs                             l'unico modo di scrivere un numero G-code e l'intestazione standard
│   ├── raster.rs                                una parte della logica
│   ├── safety.rs                                il limite di potenza e le regole che tengono spento il laser mentre si mostra il lavoro
│   ├── shot.rs                                  la modalità immagine della finestra (--shot): i suoi argomenti, letti e verificati senza schermo
│   ├── vector.rs                                un disegno letto: forme in mm, livelli, limiti e geometria di base
│   ├── verify.rs                                il controllo di un programma: il suo SHA-256, ciò che fa e un verdetto rispetto ai limiti della macchina
│   ├── flatten.rs                               curve in tratti rettilinei entro una tolleranza (Bézier, archi)
│   ├── frame.rs                                 la cornice del lavoro: il rettangolo percorso con il laser spento
│   ├── svg.rs                                   il lettore SVG: un sottoinsieme sicuro, senza script, immagini o entità
│   ├── dxf.rs                                   il lettore DXF: linee, polilinee, archi, cerchi e spline, con unità
│   ├── energy.rs                                l'energia di una passata per millimetro o millimetro quadrato, dalla potenza ottica
│   ├── offset.rs                                spostare un contorno di lato della larghezza del fascio, verificato spigolo per spigolo
│   ├── order.rs                                 l'ordine dei tagli: linee unite, prima l'interno, poco spostamento
│   ├── cut.rs                                   dal disegno al G-code: taglio, ponticelli, tratteggio, passate e verifica di sicurezza
│   ├── vconfig.rs                               il file di impostazioni del lavoro vettoriale: limiti, macchina e ogni livello
│   ├── browse.rs                                l'elenco dei file della finestra che apre un file
│   ├── gui.rs                                   la finestra (egui, con la funzione gui): disegna lo stato e inoltra ciò che fa la persona
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
│   └── window_cli.rs                            test che una compilazione senza la funzione gui resti una riga di comando
├── examples/
│   ├── gradient.pgm                             un esempio per provare il programma
│   ├── plate.svg                                una piastra con un foro e un pannello da incidere (SVG)
│   ├── plate.dxf                                la stessa piastra in DXF
│   ├── plate.cfg                                le impostazioni dei suoi livelli: potenza, velocità, passate, larghezza del fascio, ponticelli e tratteggio
│   └── materials.cfg                            una libreria di materiali: betulla e ardesia, taglio, incisione e fotografie
├── docs/
│   ├── architecture.json                        dati dell'immagine di Architettura
│   ├── structure.json                           dati dell'immagine di Struttura
│   ├── vector-settings.md                       il file di impostazioni del lavoro vettoriale, chiave per chiave
│   └── materials.md                             la libreria dei materiali, chiave per chiave
├── tools/                                       controllo della CI e del README e strumenti per le immagini
├── images/                                      banner e le immagini di Architettura e Struttura del repository
├── .github/                                     workflow di CI e di rilascio, modelli di issue e pull request
├── eh3d.project.json                            manifesto: versione, ruolo, maturità e comando di compilazione
├── Cargo.toml Cargo.lock                        il workspace e le sue dipendenze bloccate
├── rust-toolchain.toml                          la toolchain Rust fissata
├── CHANGELOG.md                                 cosa è cambiato in ogni versione
├── SECURITY.md SUPPORT.md                       come segnalare una vulnerabilità e dove chiedere
└── CONTRIBUTING.md CODE_OF_CONDUCT.md LICENSE   come contribuire, codice di condotta e licenza
```

</details>

## 🗺️ Roadmap e limiti noti

**Limiti noti oggi**

* La finestra non è ancora stata vista su uno schermo.
* Non è stato provato su una macchina né con una raccolta di file reali e non ha avuto revisione indipendente.
* Un disegno viene letto in un sottoinsieme sicuro (niente `use`, testo, sfumature o stili con selettori; niente ellissi, tratteggi o blocchi in un DXF); un contorno più stretto del fascio non viene tagliato e il programma lo dice; il laser è quello di un controller GRBL con `M4` e nulla è stato provato su un laser o un materiale reali.

**Prossimi passi**

* Mostrare la densità di potenza sulla scheda prima di bruciare (`layers` stampa già l'energia di una passata quando `laser_watts` è impostato) e mantenere costante la potenza negli angoli di un taglio.
* Vedere la finestra su uno schermo e finirla.
* Provarlo con disegni reali e con gli altri programmi della suite.

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

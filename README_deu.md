<p align="center">
  <img src="images/EH3D_PHOENIX_LASER_BANNER.svg" alt="EH3D-PHOENIX-LASER banner" width="100%">
</p>

# <img src="images/brand/icon-64.png" alt="" width="40" height="40" align="absmiddle"> EH3D-PHOENIX-LASER
<p align="center"><a href="README.md">🇺🇸 English</a> | <a href="README_spa.md">🇪🇸 Español</a> | <a href="README_fra.md">🇫🇷 Français</a> | <a href="README_ita.md">🇮🇹 Italiano</a> | 🇩🇪 <b>Deutsch</b> | <a href="README_zho.md">🇨🇳 中文</a> | <a href="README_jpn.md">🇯🇵 日本語</a> | <a href="README_por.md">🇵🇹 Português</a></p>

### Lasergravur aus einem Bild und Schneiden aus einer Zeichnung: Dithering, Raster-G-Code, SVG- und DXF-Konturen und Kalibrierraster

<p align="center">
  <img src="https://img.shields.io/badge/License-GPL%203.0--or--later-blue.svg" alt="GPL 3.0 or later">
  <img src="https://img.shields.io/badge/Stack-Rust-DEA584.svg" alt="Rust">
  <img src="https://img.shields.io/badge/Languages-8-00E5FF.svg" alt="Languages">
  <img src="https://img.shields.io/badge/Version-0.0.8-0f6cbd.svg" alt="Version">
  <img src="https://img.shields.io/badge/Maturity-functional-4CAF50.svg" alt="Maturity">
</p>

---

**Ehrlichkeitscheck - was heute wirklich läuft:** eine Rust-Bibliothek und ein Kommandozeilenprogramm (`phoenix-laser`) mit 506 Tests, die auf einem Computer laufen: wandelt ein Bild per Dithering in Punkte um und schreibt den G-Code eines Lasers, der Zeile für Zeile mit dynamischer Leistung und Überlauf brennt, liest SVG- und DXF-Zeichnungen und schreibt den G-Code, der ihre Konturen schneidet (die inneren zuerst, mit Strahlbreite, Stegen und mehreren Durchgängen) und ihre Formen mit Linien graviert, mit den Einstellungen jeder Ebene in einer Datei, sowie ein Testraster Leistung gegen Vorschub. Es hat ein Fenster (mit `--features gui` gebaut), das ein Bild oder eine Zeichnung öffnet, den Weg des Lasers zeigt, die Einstellungen ändern lässt und den G-Code exportiert, aber das Fenster wurde noch nicht auf einem Bildschirm angesehen; es wurde nicht an einer Maschine erprobt und nicht mit einer Sammlung echter Dateien geprüft.

---

> **Sicherheitsstatus:** Dieses Programm steuert keine Maschine und ist kein Sicherheitssystem. Was es erzeugt, muss ein Mensch prüfen, bevor etwas an einer Maschine läuft; nichts davon lief auf echter Hardware.


## 📋 Auf einen Blick

| | |
|---|---|
| Version | `0.0.8` |
| Reifegrad | Funktionsfähig |
| Tests | 506 |
| Technologie-Stack | Rust |
| Auf echter Hardware | Nicht erprobt: nichts davon lief an einer Maschine |

**Springe zu:** [Überblick](#-überblick) · [Architektur](#-architektur) · [Schnellstart](#-schnellstart) · [Tests](#-tests) · [Struktur](#-struktur-des-repositorys) · [Verwandte](#-verwandte-projekte)

## 🚦 Projektstatus

Jede Zeile ist ein Teil dieses Repositorys, und die Ampel sagt, wie weit er heute ist. Sie beruht auf dem, was das Repository zeigen kann (Tests, Builds, Läufe), nicht auf dem, was geplant ist.

🟢 Fertig und überprüft · 🟡 Funktioniert, mit bekannten Grenzen · 🟠 Teilweise fertig oder noch nicht überprüft · 🔴 Nicht gemacht oder funktioniert nicht

| Bereich | Stand | Was das hier bedeutet |
|---|---|---|
| Kern | 🟡 | wandelt ein Bild per Dithering in Punkte um und schreibt den G-Code eines Lasers, der Zeile für Zeile mit dynamischer Leistung und Überlauf brennt, liest SVG- und DXF-Zeichnungen und schneidet oder graviert sie ebenenweise (geordnete Konturen, Strahlbreite, Stege, Durchgänge, Schraffur) sowie ein Testraster Leistung gegen Vorschub; auf einem Computer mit normalen und beschädigten Eingaben getestet, nicht mit einer Sammlung echter Dateien. |
| Kommandozeile | 🟡 | `phoenix-laser` (`engrave`, `calibrate`, `info`, `cut`, `vector`, `layers`) führt dieselben Funktionen wie die Bibliothek aus und gibt das Ergebnis aus; jeder Fehler kommt als Meldung und mit einem Exit-Code ungleich null zurück. |
| Tests | 🟡 | 506 Tests bestehen (`cargo test`), mit Normalfällen, Grenzfällen und beschädigten Eingaben. |
| Sicherheit | 🟡 | Reines sicheres Rust (`forbid(unsafe_code)`): nichts gerät bei den übergebenen Dateien in Panik und ein schlechter Wert ist ein Fehler mit Begründung; keine unabhängige Prüfung. |
| Fenster | 🟠 | Geschrieben (egui, `--features gui`): ein Bild oder eine SVG- oder DXF-Zeichnung öffnen oder ablegen, den Weg des Lasers sehen (der Schnitt in einer Farbe, der Leerweg abgeschwächt), die Ebenentabelle bearbeiten, ein Kalibrierraster, Zoom und Verschieben, die Zeit schätzen, exportieren; die Logik ist getestet, aber das Fenster wurde noch nicht gebaut oder auf einem Bildschirm angesehen. |
| Echte Hardware | 🔴 | Nicht erprobt: nichts davon wurde an einer Maschine ausgeführt. |
| Dokumentation | 🟡 | README in acht Sprachen, das Changelog und die zwei aus `docs/` gezeichneten Bilder. |

## 🎯 Überblick

* **Das Bild** (`eh3d-image`): PNG (Grau, Farbe, transparent, 8 oder 16 Bit) und PGM (Text und binär) werden zu Graustufen, transparente Teile als Weiß, die Größe ist begrenzt, damit eine feindliche Datei keine Gigabytes verlangen kann; `resize` mittelt die Pixel, sodass ein Punkt so groß wie der Strahl ist.
* **Die Punkte** (`src/dither.rs`): ein Schwellwert oder Fehlerdiffusion nach Floyd-Steinberg oder Atkinson; die Tests prüfen, dass die Dunkelheit eines Grautons erhalten bleibt (ein mittleres Grau brennt etwa die Hälfte der Punkte) und dass Weiß nichts und Schwarz alles brennt.
* **Der Laser** (`src/raster.rs`, `src/calibrate.rs`): Zeile für Zeile, von links nach rechts und dann von rechts nach links, `M4` mit `S0` beim Verfahren und der gewählten Leistung beim Brennen, ein fester oder aus der Beschleunigung berechneter Überlauf (v zum Quadrat durch 2a) und ein Raster aus Quadraten mit einer Leistung je Spalte und einem Vorschub je Zeile, um das Passende für ein Material zu finden.
* **Der Kopf und die Zahlen jeder G-Code-Datei** (`src/gcode_out.rs`): Die Datei beginnt mit Kommentarzeilen, die den Generator und seine Version, die Einheiten (mm, G21), das Bild, aus dem sie entstand, mit seinem SHA-256 und die Werkzeuge (den Laser und seine Leistung) nennen; ein Kalibrierraster gibt an, dass es keine Quelle hat. Eine Zahl wird nur auf eine Weise geschrieben (höchstens drei Dezimalstellen, kein `-0`), und ein Wert, der nicht endlich ist, wird abgelehnt, statt als `NaN` oder `inf` geschrieben zu werden.
* **`--dry-run`:** `engrave` berechnet das Ergebnis und zählt es (Punkte, Zeilen, Läufe, gebrannte Länge, Zeit), ohne die Ausgabedatei zu schreiben, die dann entfallen darf.
* **Die Zeichnungen** (`src/svg.rs`, `src/dxf.rs`, `src/flatten.rs`, `src/vector.rs`): eine sichere Teilmenge von SVG (Pfade mit allen Befehlen, Formen, Transformationen, Gruppen, Einheiten und ein `use` eines Elements derselben Datei) und die Elemente einer DXF (Linien, Polylinien mit Bögen, Bögen, Kreise, Splines, Einheiten) werden zu Formen in Millimetern; Skripte, Bilder, ein `use` einer anderen Datei, deklarierte Entitäten und alles Unbekannte werden übersprungen oder abgelehnt, Größe, Tiefe sowie Zahl der Elemente (was ein `use` hereinholt, zählt mit) und Punkte sind durch die Einstellungen begrenzt, ein `use`, das auf sich selbst zurückführt, ist ein Fehler, und Kurven werden innerhalb einer Toleranz zu geraden Stücken.
* **Schneiden und Gravieren** (`src/order.rs`, `src/offset.rs`, `src/cut.rs`): Die Formen werden nach Ebene (DXF) oder Farbe (SVG) gruppiert; jede Gruppe wird mit ihren eigenen Einstellungen geschnitten (lose Linien verbunden, die Konturen innerhalb einer anderen zuerst, kürzester Leerweg, die Strahlbreite auf der richtigen Seite abgezogen, Stege ungeschnitten, mehrere Durchgänge mit nachgeführtem Fokus) oder graviert (geschlossene Formen mit Linien in einem Winkel gefüllt, nach der Gerade-Ungerade-Regel).
* **Die Einstellungsdatei** (`src/vconfig.rs`, `docs/vector-settings.md`): Jeder abgestimmte Wert (die Grenzen der Leser, die Maschine, die Einstellungen jeder Ebene) steht in einer einfachen Textdatei, die Zeile für Zeile geprüft wird; nichts davon steckt im Code.
* **Der Fokustest** (`src/calibrate.rs`, `[focus]` in der Einstellungsdatei): `phoenix-laser focus` brennt eine Leiter kurzer Linien, jede mit dem Kopf in anderer Höhe; die schärfste Linie zeigt, wo der Strahl im Fokus ist; die Höhenschrittweite hat keinen Standardwert (was unten ist, hängt von der Maschine ab), die Leistung folgt der Leistungsgrenze und der Kopf kehrt auf die erste Höhe zurück.
* **Das Testraster eines Materials** (`src/calibrate.rs`, `[calibrate]` in der Einstellungsdatei): `phoenix-laser calibrate --material NAME --materials DATEI` verteilt die Leistungen und Vorschübe des Rasters um die Werte des Materials der Bibliothek, innerhalb der Grenzen und ohne Wiederholungen, und `--labels` brennt die Leistung jeder Spalte und den Vorschub jeder Zeile mit einer Strichschrift neben die Quadrate, sodass das Raster später auf dem Material lesbar ist; ein Material über einer Grenze wird abgelehnt und nie gesenkt.
* **Der Rahmen der Arbeit** (`src/frame.rs`, `[frame]` in der Einstellungsdatei): `phoenix-laser frame` schreibt ein Programm, das das Rechteck der Arbeit bei ausgeschaltetem Laser abfährt, aus einer Größe oder aus dem Kasten dessen, was ein G-Code-Programm brennt, damit man sieht, wo es landet; eine schwache Pilotleistung braucht jedes Mal `--confirm-laser-on`, ist auf 5 % begrenzt (`max_pilot_percent` in `[frame]` ändert die Grenze) und folgt der Leistungsgrenze, und ein Rahmen außerhalb des Betts wird abgelehnt.
* **Die Vorschau** (`src/preview.rs`): `phoenix-laser preview` liest ein G-Code-Programm (das eigene oder ein fremdes), gibt die Bewegungen, die gebrannte und die gefahrene Länge, den Kasten, die Leistungen und eine Zeit aus, die das Bremsen in den Ecken zählt, warnt vor Gefährlichem (Laser an bei einer Eilbewegung, kein Vorschub, Leistung über Vollleistung, Arbeit außerhalb des Betts, kein Ende) und zeichnet den Weg als SVG; die Tests lesen zurück, was das Gravieren schreibt, und prüfen dieselbe Länge.
* **Die Materialbibliothek** (`src/materials.rs`, `examples/materials.cfg`): Was zu einem Material und einer Arbeitsweise passt, steht einmal in einer Textdatei (`[material NAME]`: Leistung, Geschwindigkeit, Durchgänge, Fokusschritt, Strahlbreite, Schraffur, Punktgröße, Dithering und Tonkurve) und wird mit `--material NAME --materials DATEI` gewählt; die Optionen der Kommandozeile gehen vor, und die Leistungsgrenze gilt über beiden. `phoenix-laser materials DATEI` listet die Bibliothek.
* **Die Leistungsgrenze** (`src/safety.rs`, `max_power_percent` in `[machine]`, `--max-power`): Ein Programm, dessen Leistung über der Grenze liegt, wird mit einem Fehler abgelehnt, der die Ebene oder die Option nennt, und nie stillschweigend gesenkt; die Grenze ist 100 %, bis eine Datei oder eine Option weniger sagt.
* **Maschinenprofile** (`[profile NAME]` in der Einstellungsdatei, `--profile NAME`): Wer zwei Maschinen hat oder ein Lasermodul gegen ein anderes tauscht, führt für jede ein Profil (Verfahrweg des Tischs, Leistungsgrenze, Geschwindigkeitsgrenze, Wert der vollen Leistung und Watt); das gewählte Profil ist die Maschine, und `--max-speed` steht neben `--max-power`: Eine Geschwindigkeit über der Grenze wird abgelehnt, nie gesenkt.
* **Konturen aus einem Bild** (`src/trace.rs`, `[trace]` in der Einstellungsdatei): `phoenix-laser trace` folgt dem Umriss eines Logos, einer Silhouette oder einer Schablone, statt ihn Punkt für Punkt zu brennen; die Graustufen werden an einer Schwelle getrennt, der Rand wird mit Marching Squares auf Subpixel-Genauigkeit gefunden, vereinfacht, bis kein Punkt weiter als `simplify_mm` von der Linie liegt, und Flecken werden verworfen; das Ergebnis ist eine Ebene einer Zeichnung, die mit denselben Einstellungen und Prüfungen geschnitten (oder mit Linien gefüllt) wird, und Bildgröße, Randstücke, Konturen und Punkte sind alle begrenzt.
* **Töne, Materialien und Grenzen im Fenster** (`src/window.rs`): Die Felder der Tonkurve (Gamma, Kontrast, Helligkeit, Invertieren und Auto-Level) bearbeiten das Bild mit derselben Funktion wie die Befehlszeile; eine Materialbibliothek wird aus einer Datei geladen (oder über `PHOENIX_LASER_MATERIALS` benannt), und die Wahl eines Materials füllt die Felder für Leistung, Geschwindigkeit, Punkt, Dithering, Ton und jede Ebene einer Zeichnung; die Grenzen für Leistung und Geschwindigkeit sind ebenfalls Felder, und ein Wert darüber wird abgelehnt, nie gesenkt. Die Startwerte kommen aus der Einstellungsdatei (`[tone]`, `[machine]`).
* **Der Fokustest im Fenster** (`src/window.rs`): ein Modus mit den Feldern des Abschnitts `[focus]` (Höhe der ersten Linie, Schritt, Anzahl und Länge der Linien, Abstand, Geschwindigkeit und Leistung); der Test wird mit derselben Funktion wie `phoenix-laser focus` berechnet, Leistung und Geschwindigkeit folgen den Grenzen, es gibt keinen Standardschritt, weil von der Maschine abhängt, wo unten ist, und ein Hinweis, dass der Test Z bewegt, bleibt in den Notizen.
* **Der Rahmen im Fenster** (`src/window.rs`, `[frame]` in der Einstellungsdatei): ein Modus, der das geöffnete Bild oder die geöffnete Zeichnung bei ausgeschaltetem Laser einrahmt; Rand, Runden und Geschwindigkeit sind Felder. Eine Pilotleistung über 0 schaltet den Laser ein, deshalb fragt das Fenster, und nichts wird berechnet oder geschrieben, bis die Person mit Ja antwortet, jedes Mal (ein geändertes Feld, ein Moduswechsel oder das Schreiben der Datei nimmt die Antwort zurück), unter der Grenze von `max_pilot_percent`, der Leistungsgrenze und dem Tisch.
* **Die Vorschau im Fenster** (`src/window.rs`): Jeder vom Fenster berechnete G-Code wird von derselben Vorschau wie `phoenix-laser preview` gezählt (Bewegungen, gebrannte und gefahrene Länge, eine Zeit, die die Ecken mit der Beschleunigung des Feldes zählt, und was gefährlich aussieht, etwa Arbeit außerhalb des Tisches der Einstellungsdatei), und ein Programm beliebiger Herkunft (`.nc`, `.ngc`, `.gcode`, `.gc`, `.tap`) kann geöffnet oder auf das Fenster gezogen werden, um seinen Weg und seine Notizen zu sehen. Es wird als Text gelesen, nie ausgeführt und nie wieder geschrieben.
* **Die Kontur im Fenster** (`src/window.rs`, `[trace]` in der Einstellungsdatei): ein Modus, der die Kontur des geöffneten Bildes mit derselben Funktion wie `phoenix-laser trace` nachfährt und sie schneidet oder ihre Formen mit Linien füllt, als Ebene namens `trace`; die Felder sind Schwelle, Vereinfachung und kleinstes Teil, dazu ein Schalter für helle Tinte; Leistung und Geschwindigkeit sind die der Laser-Felder und halten die Grenzen ein, und die Grenzen des Bildes (`max_pixels`, `max_contours`, `max_segments`) stammen aus der Einstellungsdatei, sodass ein zu detailreiches Bild ein Fehler ist und kein langsames Programm.
* **Das Maschinenprofil im Fenster** (`src/window.rs`, `[profile NAME]` in der Einstellungsdatei): eine Liste im Seitenfeld mit den Maschinen der Datei; die gewählte ist die Maschine, so wie `--profile` sie macht, also übernehmen die Felder für den Vollleistungswert und die Grenzen von Leistung und Geschwindigkeit ihre Werte, und das Bett des Profils ist das, das Rahmen und Vorschau prüfen. Was ein Profil auslässt, kommt aus `[machine]`, ein Profil, das nicht in der Datei steht, ändert nichts, und die Grenzen weisen weiterhin eine Leistung oder Geschwindigkeit darüber zurück.
* **Die Prüfung eines Programms** (`src/verify.rs`): `phoenix-laser verify PROGRAMM.nc` liest ein G-Code-Programm (dieses oder ein fremdes) als Text, führt es nie aus und liefert den SHA-256 der genauen Datei, was sie tut und ein Urteil: Es besteht nur unter der Leistungs- und Geschwindigkeitsgrenze der Maschine, innerhalb des Tisches (Arbeit außerhalb fällt durch; `--overscan-mm` oder `overscan_mm` in `[verify]` erlaubt dem Kopf, mit ausgeschaltetem Laser so weit über den Tisch hinauszufahren, wie es der Überlauf eines anderen Programms tut, und Brennen außerhalb des Tisches fällt immer durch), mit ausgeschaltetem Laser bei jeder Eilbewegung und einem Vorschub bei jeder brennenden Bewegung. `--json` liefert den Bericht, den ein anderes Programm neben einer an diesen Hash gebundenen Freigabe aufbewahren kann, und der Exit-Code ist 1, wenn es durchfällt.
* **Geordnetes Dithering** (`src/dither.rs`): `--dither bayer` (auch im Fenster) entscheidet jeden Punkt nach seinem Ort und seinem Grau mit einer 8-x-8-Bayer-Matrix, das Muster ist also regelmäßig und dasselbe Grau brennt immer dieselben Punkte; die Tests prüfen, dass jede Stufe der Matrix einmal vorkommt und die Dunkelheit eines Grautons erhalten bleibt.
* **Die Tonkurve** (`src/tone.rs`): Vor der Punkterzeugung lassen sich die Graustufen auf den vollen Bereich dehnen (`--auto-level`), Kontrast und Helligkeit ändern, mit einem Gamma krümmen oder invertieren, mit einer einzigen Tabelle von 256 Einträgen; mit den Standardwerten bleibt das Bild völlig unverändert.
* **Ein geprüfter G-Code:** Das Programm einer Zeichnung wird vor der Ausgabe Zeile für Zeile geprüft: kein Laser an bei einer Eilbewegung oder ohne Geschwindigkeit, kein `S` über voller Leistung, keine Bewegung außerhalb des Bettes, `M5` am Ende; die Datei wird nie über die Zeichnung oder die Einstellungen geschrieben, aus denen sie stammt.

## 🧩 Rolle in EH3D

EH3D-PHOENIX-LASER ist ein Programm der EH3D Fabrication Suite. Es arbeitet als Bibliothek und Kommandozeile und, mit `--features gui` gebaut, als Fenster im gemeinsamen Aussehen der Suite. Es liest und schreibt einfache Dateien und lässt sich allein oder zwischen den anderen Programmen nutzen.

## 🏛️ Architektur

<p align="center">
  <img src="images/architecture.svg" alt="Architektur von EH3D-PHOENIX-LASER: wer es nutzt, was es enthält und wovon es abhängt" width="100%">
</p>


Wie die Teile dieses Repositorys zusammenpassen: wer es nutzt, was es enthält und wovon es abhängt. Das Bild wird aus `docs/architecture.json` gezeichnet und gegen den Code geprüft.

## ✨ Funktionen

* **Bibliothek und Kommandozeile:** Jeder Befehl von `phoenix-laser` ist eine Funktion der Bibliothek, sodass ein anderes Programm der Suite ihn ohne Prozessstart aufrufen kann.
* **Strenge Kommandozeile:** Eine falsch geschriebene Option (`--powr`) oder ein überzähliges Wort ist ein Fehler, der sie nennt, und wird nicht ignoriert; sonst würde das Programm seine Ausgabe mit den Standardwerten erzeugen.
* **Tests mit beschädigten Eingaben:** Die Tests füttern auch leere Dateien, Datenmüll, riesige Zahlen und abgeschnittene Dateien; keiner darf in Panik geraten.
* **Kein Raten:** Ein unsinniger Wert ist ein Fehler, der sagt welcher und warum, nie ein Ergebnis, das nur richtig aussieht.
* **Kein unsafe-Code:** `#![forbid(unsafe_code)]` in Bibliothek und Programm.

## 🔒 Sicherheit und Schutz

| Bedrohung | Schutz |
|---|---|
| Eine beschädigte, abgeschnittene oder leere Datei | keine Panik: die Tests schneiden gültige Eingaben an jeder Stelle ab und füttern Datenmüll |
| Eine unsinnige Zahl (negativ, nicht endlich, absurd groß) | wird mit einem Fehler abgelehnt, der den Wert nennt; nichts wird zu etwas Plausiblem gerundet |
| Unsichere Speichernutzung | reines sicheres Rust mit `forbid(unsafe_code)` |
| Eine feindliche Zeichnung (Skripte, deklarierte Entitäten, tiefe Verschachtelung, Millionen Punkte, riesige Zahlen) | Skripte, Bilder und Links werden nie ausgeführt oder geladen; eine deklarierte Entität ist ein Fehler; Größe, Tiefe, Elemente, Formen, Punkte und die Stücke einer Kurve sind begrenzt; eine Form mit einem Punkt außerhalb des Bereichs wird weggelassen |
| Ein Programm, das den Laser ohne Geschwindigkeit oder über voller Leistung einschalten könnte | jedes Programm wird vor der Ausgabe Zeile für Zeile geprüft, und eines, das durchfällt, wird nicht geschrieben |
| Eine höhere Leistung, als Maschine oder Material bekommen sollen | die Leistungsgrenze (`max_power_percent`, `--max-power`) lehnt ein Programm darüber mit einem Fehler ab, der die Ebene oder die Option nennt; sie wird nie stillschweigend gesenkt |
| Eine kaputte oder riesige Materialbibliothek | die Datei ist in Größe und Zahl der Materialien begrenzt; ein Wert außerhalb des Bereichs, ein unbekannter Schlüssel oder ein doppelter Name ist ein Fehler, der die Zeile nennt; die Ausgabe wird nie darüber geschrieben |
| Ein fremdes G-Code-Programm, das vor dem Ausführen gelesen wird | die Vorschau liest nur Text, mit Größenlimit, führt nie etwas aus und lädt nichts, ignoriert absurde Zahlen und warnt bei Laser an während einer Eilbewegung, fehlendem Vorschub, Leistung über Vollleistung, Arbeit außerhalb des Betts und fehlendem Ende |
| Der Laser schaltet sich nur zum Positionieren der Arbeit ein | der Rahmen wird bei ausgeschaltetem Laser abgefahren (keine Leistung, die Spindel nie an); eine Pilotleistung braucht bei jedem Lauf `--confirm-laser-on`, auch wenn die Einstellungsdatei sie verlangt, bleibt höchstens bei der Obergrenze (5 %, wenn `max_pilot_percent` nichts anderes sagt) unter der Leistungsgrenze und wird vor dem Programmende ausgeschaltet |
| Ein Kopf, den ein Fokustest in Z in die falsche Richtung schickt | die Höhenschrittweite hat keinen Standardwert und der Test wird abgelehnt, bis die Person sie angibt; das Programm sagt, dass es Z bewegt, der Bereich ist begrenzt (insgesamt 50 mm), der Kopf kehrt auf die erste Höhe zurück und die Leistung folgt der Leistungsgrenze |
| Die Ausgabedatei, über die Zeichnung oder die Einstellungen gewählt, aus denen sie entsteht | wird abgelehnt, im Fenster und in den Befehlen |

**Was nicht abgesichert ist:** Ein fehlerfrei erzeugtes Ergebnis ist kein geprüftes Ergebnis. Nichts hier kennt die Grenzen einer Maschine, und das Programm hatte keine unabhängige Prüfung. Eine Schwachstelle bitte privat melden (siehe [SECURITY.md](SECURITY.md)).

## 🔌 Schnittstellen

Die Bibliothek ist `eh3d_phoenix_laser` (`use eh3d_phoenix_laser::...`); die Kommandozeile ist `phoenix-laser`. Die Befehle sind diese; jeder gibt sein Ergebnis auf der Standardausgabe aus und meldet einen Fehler auf der Fehlerausgabe mit einem Exit-Code ungleich null. Es gibt keinen Netzwerkzugriff.

```text
phoenix-laser engrave IMAGE OUT.nc --width-mm 50 [--dot-mm 0.1] [--feed 3000] [--power 50] [--smax 1000]
              [--dither fs|atkinson|threshold|bayer] [--threshold 128] [--overscan MM|auto] [--accel 1000] [--origin X,Y] [--dry-run]
              [--gamma 1] [--contrast 0] [--brightness 0] [--invert] [--auto-level] [--max-power 100] [--max-speed MM/MIN] [--config FILE] [--profile NAME]
              [--material NAME --materials FILE]
phoenix-laser calibrate OUT.nc --powers 20,40,60,80 --feeds 1000,2000,3000 [--smax 1000] [--max-power 100] [--max-speed MM/MIN] [--config FILE] [--profile NAME]
phoenix-laser calibrate OUT.nc --material NAME --materials FILE [--steps 5] [--spread 40] [--labels|--no-labels] [--label-mm 3] [--smax 1000] [--max-power 100] [--max-speed MM/MIN] [--config FILE] [--profile NAME]
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
phoenix-laser verify PROGRAM.nc [--config FILE] [--profile NAME] [--smax N] [--max-power P] [--max-speed MM/MIN] [--bed X,Y] [--overscan-mm MM] [--accel 1000] [--json]
phoenix-laser preview PROGRAM.nc [OUT.svg] [--accel 1000] [--rapid 6000] [--smax N] [--bed X,Y] [--width-px 900] [--config FILE] [--profile NAME]
phoenix-laser [gui] [FILE]                       the window (a build with --features gui); with no arguments it opens
```

`DRAWING` ist eine SVG- oder DXF-Datei. `cut` folgt den Konturen (die inneren zuerst); `vector` füllt die geschlossenen Formen mit Linien und folgt den offenen; beide nehmen die Einstellungen jeder Ebene oder Farbe aus `--config FILE` (siehe [docs/vector-settings.md](docs/vector-settings.md)), und eine Option der Kommandozeile ändert sie für alle Ebenen. `layers` listet die Ebenen einer Zeichnung mit dem auf, was mit jeder geschehen würde.

## ⚙️ Konfiguration

Die Gravur eines Bildes wird mit Argumenten der Kommandozeile eingestellt (siehe Schnittstellen), und jeder Standardwert steht in der Hilfe des Befehls (`phoenix-laser help`). Das Schneiden und Gravieren einer Zeichnung wird mit einer Einstellungsdatei eingestellt (`--config FILE` oder „Einstellungen laden“ im Fenster): die Grenzen der Leser, wie eine Datei gelesen wird, die Reihenfolge der Schnitte, die Maschine und für jede Ebene oder Farbe der Modus (schneiden, gravieren oder auslassen), die Leistung, die Geschwindigkeit, die Durchgänge, der Fokusschritt, die Strahlbreite und ihre Seite, die Stege und die Schraffur. Jeder Schlüssel, sein Standardwert und sein Bereich stehen in [docs/vector-settings.md](docs/vector-settings.md), ein Beispiel in `examples/plate.cfg`; ein unbekannter Schlüssel oder ein Wert außerhalb des Bereichs ist ein Fehler, der seine Zeile nennt. Die festen Grenzen des Bildlesers stehen als benannte Konstanten im Code und sind durch Tests abgedeckt.

## 🚀 Schnellstart

Sie brauchen eine stabile Rust-Toolchain (1.88 oder neuer). Das Repository EH3D-COMMON muss neben diesem geklont sein: der Bildleser wird daraus per Pfad übernommen.

```bash
cargo build --release
cargo test
cargo run --release -- engrave examples/gradient.pgm examples/gradient.nc --width-mm 16 --dot-mm 0.5 --overscan auto
cargo run --release -- cut examples/plate.svg examples/plate.nc --config examples/plate.cfg
cargo run --release --features gui   # the window
cargo run --release --features gui -- --shot screen.png examples/plate.svg   # a picture of the window
```

Um ein Bild des Fensters aufzunehmen, ohne es von Hand zu öffnen, starten Sie es mit `--shot DATEI.png` (auch `--size WxH`, `--theme forge|dark|light`, `--wait N`, `--lang CODE`, `--open FILE`, `--replace`): Es öffnet sich in dieser Größe, speichert das PNG, behält keine Einstellungen und schließt sich selbst (Exit-Code 0, wenn die Datei geschrieben wurde, 1, wenn nicht, 2 bei falschen Argumenten).

## 🧪 Tests

Die Suite hat 506 Tests (`cargo test`, in der CI unter Windows und Linux). Sie prüfen Normalfälle, Grenzfälle und beschädigte Eingaben: leere Dateien, Datenmüll und abgeschnittene Dateien lösen nie eine Panik aus. Die CI führt außerdem `cargo fmt --check`, `cargo clippy` mit Warnungen als Fehlern und `python -I tools/ci_validate.py` aus.

## 🏗️ Build und Veröffentlichung

Die Version steht in `Cargo.toml` und in `eh3d.project.json`; jede Version hat ihren Eintrag in [CHANGELOG.md](CHANGELOG.md). `build.bat` / `build.sh` bauen das Programm, `test.bat` / `test.sh` führen die Tests aus und `run.bat` / `run.sh` starten es mit einem Beispiel. Releases erscheinen auf GitHub.

**Downloads:** Mit jeder Version dieses Repositorys werden fertige Programme für jedes System veröffentlicht: Windows (64 und 32 Bit), Linux (64 Bit) und macOS (Apple Silicon und Intel). Jedes Paket, `phoenix-laser-<Version>-<System>.zip` oder `.tar.gz`, enthält das Programm, die Lizenz, diese README, das Änderungsprotokoll, das Symbol und die Beispieldateien und kommt mit seiner `.sha256`-Datei; die Version listet außerdem jede Datei in `SHA256SUMS` und `build-manifest.json` auf. Bisher wurde noch keine Version veröffentlicht.

**Signiert und prüfbar:** Die Pakete einer Version werden mit dem Release-Schlüssel von EH3D signiert, und die `.sig`-Datei jedes Pakets liegt daneben (eine OpenSSH-Signatur, Namespace `eh3d-release`). Prüfe einen Download, bevor du ihn startest: Vergleiche seine SHA-256 (`sha256sum -c phoenix-laser-<Version>-<System>.tar.gz.sha256`, unter Windows `Get-FileHash`) und prüfe die Signatur mit `ssh-keygen -Y verify -f allowed_signers -I eh3d-release -n eh3d-release -s <Paket>.sig < <Paket>`; `allowed_signers` enthält die Zeile `eh3d-release <Schlüsseltyp> <öffentlicher Schlüssel der EH3D-Releases>`.

## 📂 Struktur des Repositorys

<p align="center">
  <img src="images/repository-structure.svg" alt="Struktur des Repositorys EH3D-PHOENIX-LASER" width="100%">
</p>

<details>
<summary>Textversion des Baums</summary>

```text
EH3D-PHOENIX-LASER/
├── src/
│   ├── lib.rs                                   die Bibliothek: was jeder Befehl nutzt
│   ├── main.rs                                  die Befehlszeile: engrave, calibrate, info, cut, vector, trace, verify und layers
│   ├── calibrate.rs                             ein Teil der Logik
│   ├── cli_args.rs                              die Argumente der Kommandozeile, dieselbe Datei in jedem kleinen Werkzeug
│   ├── dither.rs                                ein Teil der Logik
│   ├── materials.rs                             die Materialbibliothek: Leistung, Geschwindigkeit, Durchgänge, Töne
│   ├── preview.rs                               die Vorschau eines G-Codes: Bewegungen, Zeit, Gefahren und ein SVG des Weges
│   ├── tone.rs                                  die Tonkurve eines Bildes: Stufen, Kontrast, Helligkeit, Gamma und Inversion vor den Punkten
│   ├── trace.rs                                 die Konturen eines Bildes: Schwelle, Marching Squares und vereinfachte Ringe, mit Grenzen
│   ├── gcode_out.rs                             die einzige Schreibweise einer G-Code-Zahl und der Standardkopf
│   ├── raster.rs                                ein Teil der Logik
│   ├── safety.rs                                die Leistungsgrenze und die Regeln, die den Laser beim Zeigen der Arbeit aus lassen
│   ├── shot.rs                                  der Bildmodus des Fensters (--shot): seine Argumente, ohne Bildschirm gelesen und geprüft
│   ├── vector.rs                                eine gelesene Zeichnung: Formen in mm, Ebenen, Grenzen und einfache Geometrie
│   ├── verify.rs                                die Prüfung eines Programms: sein SHA-256, was es tut und ein Urteil gegen die Grenzen der Maschine
│   ├── flatten.rs                               Kurven in gerade Stücke innerhalb einer Toleranz (Bézier, Bögen)
│   ├── frame.rs                                 der Rahmen der Arbeit: das Rechteck bei ausgeschaltetem Laser
│   ├── svg.rs                                   der SVG-Leser: eine sichere Teilmenge, ohne Skripte, Bilder oder Entitäten
│   ├── dxf.rs                                   der DXF-Leser: Linien, Polylinien, Bögen, Kreise und Splines, mit Einheiten
│   ├── energy.rs                                die Energie eines Durchgangs je Millimeter oder Quadratmillimeter aus der optischen Leistung
│   ├── offset.rs                                Kontur um die Strahlbreite seitlich versetzen, Kante für Kante geprüft
│   ├── order.rs                                 die Reihenfolge der Schnitte: lose Linien verbunden, Inneres zuerst, kurzer Leerweg
│   ├── cut.rs                                   von der Zeichnung zum G-Code: Schnitt, Stege, Schraffur, Durchgänge und Sicherheitsprüfung
│   ├── vconfig.rs                               die Einstellungsdatei der Vektorarbeit: Grenzen, Maschine und jede Ebene
│   ├── browse.rs                                die Dateiliste des Fensters zum Öffnen einer Datei
│   ├── gui.rs                                   das Fenster (egui, mit dem Feature gui): zeichnet den Zustand und gibt weiter, was die Person tut
│   ├── lang.rs                                  die Wörter des Fensters in den acht Sprachen
│   ├── viewport.rs                              Zoom und Verschiebung des Bildes, in einfachen Zahlen
│   └── window.rs                                der Zustand des Fensters ohne Bildschirm: Bild oder Zeichnung, Einstellungen, Ebenen, Durchgänge und Export
├── tests/
│   ├── fuzz.rs                                  Tests mit festem Startwert: beschädigte Dateien, Abschnitte und seltsame Zahlen
│   ├── dry_run.rs                               Tests von --dry-run: zählt dasselbe und schreibt nichts
│   ├── laser.rs                                 Tests von Bibliothek und Kommandozeile
│   ├── laser_tools.rs                           Tests der Werkzeuge rund um das Gravieren über die Kommandozeile
│   ├── readme.rs                                Tests, die ausführen, was README und run-Skripte versprechen
│   ├── strict.rs                                Tests der Kommandozeile: Optionen und Wörter, die sie nicht kennt
│   ├── vector.rs                                Tests des Schneidens aus Zeichnungen: Befehle, Beispiele, feindliche Dateien und die Prüfung des G-Codes
│   └── window_cli.rs                            Tests, dass ein Build ohne das Feature gui eine reine Kommandozeile bleibt
├── examples/
│   ├── gradient.pgm                             ein Beispiel zum Ausprobieren
│   ├── plate.svg                                eine Platte mit Loch und einer Fläche zum Gravieren (SVG)
│   ├── plate.dxf                                dieselbe Platte als DXF
│   ├── plate.cfg                                die Einstellungen seiner Ebenen: Leistung, Geschwindigkeit, Durchgänge, Strahlbreite, Stege und Schraffur
│   └── materials.cfg                            eine Materialbibliothek: Birke und Schiefer, Schneiden, Gravieren und Fotos
├── docs/
│   ├── architecture.json                        Daten des Architektur-Bildes
│   ├── structure.json                           Daten des Struktur-Bildes
│   ├── vector-settings.md                       die Einstellungsdatei der Vektorarbeit, Schlüssel für Schlüssel
│   └── materials.md                             die Materialbibliothek, Schlüssel für Schlüssel
├── tools/                                       CI-Prüfung, README-Prüfung und die Bildwerkzeuge
├── images/                                      Banner und die Bilder von Architektur und Repository-Struktur
├── .github/                                     CI- und Release-Workflows, Vorlagen für Issues und Pull Requests
├── eh3d.project.json                            Manifest: Version, Rolle, Reifegrad und Build-Befehl
├── Cargo.toml Cargo.lock                        der Workspace und seine festgelegten Abhängigkeiten
├── rust-toolchain.toml                          die festgelegte Rust-Toolchain
├── CHANGELOG.md                                 was sich in jeder Version geändert hat
├── SECURITY.md SUPPORT.md                       Schwachstellen melden und Fragen stellen
└── CONTRIBUTING.md CODE_OF_CONDUCT.md LICENSE   Mitwirken, Verhaltenskodex und Lizenz
```

</details>

## 🗺️ Roadmap und bekannte Grenzen

**Bekannte Grenzen heute**

* Das Fenster wurde noch nicht auf einem Bildschirm angesehen.
* Es wurde nicht an einer Maschine und nicht mit einer Sammlung echter Dateien erprobt und hatte keine unabhängige Prüfung.
* Eine Zeichnung wird in einer sicheren Teilmenge gelesen (kein `use` einer anderen Datei, kein Text, keine Verläufe oder Stile mit Selektoren; keine Ellipsen, Schraffuren oder Blöcke in einer DXF); eine Kontur, die schmaler als der Strahl ist, wird nicht geschnitten, und das Programm sagt es; der Laser ist der eines GRBL-Controllers mit `M4`, und nichts wurde an einem echten Laser oder Material erprobt.

**Als Nächstes**

* Die Leistungsdichte vor dem Brennen (`layers` gibt die Energie eines Durchgangs schon aus, wenn `laser_watts` gesetzt ist) auf der Platine zeigen und die Leistung in den Ecken eines Schnitts konstant halten.
* Das Fenster auf einem Bildschirm ansehen und fertigstellen.
* Es mit echten Zeichnungen und mit den anderen Programmen der Suite erproben.

## 🌍 Sprachen

README und Texte gibt es auf Englisch, Spanisch, Französisch, Italienisch, Deutsch, Chinesisch, Japanisch und europäischem Portugiesisch.

## 🔗 Verwandte Projekte

* [EH3D-COMMON](https://github.com/JuanenRac/EH3D-COMMON): die gemeinsamen Crates, darunter der Bildleser
* [EH3D-STENCILCAM](https://github.com/JuanenRac/EH3D-STENCILCAM): Lotpasten-Schablonen
* [EH3D-CAM-DEBUGGER](https://github.com/JuanenRac/EH3D-CAM-DEBUGGER): die Prüfung von G-Code, bevor er eine Maschine erreicht
* [EH3D-TOOL-MANAGER](https://github.com/JuanenRac/EH3D-TOOL-MANAGER): das Werkzeuglager der CAM-Programme
* [EH3D-MOTIONCNC](https://github.com/JuanenRac/EH3D-MOTIONCNC): das Maschinensteuerungsprogramm
* [EH3D-SUITE-LAUNCHER](https://github.com/JuanenRac/EH3D-SUITE-LAUNCHER): die Zentrale, die die Programme der Suite findet und startet

## 📚 Dokumentation

* [Changelog](CHANGELOG.md)
* [Architektur- und Strukturdaten](docs/architecture.json): die Daten, aus denen die beiden Bilder gezeichnet werden.
* [Einstellungen der Vektorarbeit](docs/vector-settings.md): jeder Schlüssel der Einstellungsdatei mit Standardwert und Bereich.
* [Die Materialbibliothek](docs/materials.md): jeder Schlüssel eines Materials und wie ein Material angewendet wird

## 🤝 Mitwirken und Community

* Vor einem Pull Request `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` und `python -I tools/ci_validate.py` ausführen: alles muss bestehen. Siehe [CONTRIBUTING.md](CONTRIBUTING.md) und den [Verhaltenskodex](CODE_OF_CONDUCT.md).
* Die README gibt es in acht Sprachen: Eine Textänderung erfolgt in allen acht Dateien, und die zwei Bilder werden aus `docs/architecture.json` und `docs/structure.json` mit `tools/make_architecture.py` und `tools/make_structure.py` gezeichnet.
* Eine Schwachstelle privat melden: [SECURITY.md](SECURITY.md). Fragen, Ideen und Berichte: electrohobby3d@gmail.com ([SUPPORT.md](SUPPORT.md)).

## 👤 AUTOR

**JuanenRac (Electro Hobby 3D)** · electrohobby3d@gmail.com

## 📜 LIZENZ

Software: GPL-3.0-or-later. Siehe [LICENSE](LICENSE). Die Hardware-Entwürfe der Suite verwenden die CERN OHL-S.

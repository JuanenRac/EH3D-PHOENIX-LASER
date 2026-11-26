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
  <img src="https://img.shields.io/badge/Version-0.1.6-0f6cbd.svg" alt="Version">
  <img src="https://img.shields.io/badge/Maturity-functional-4CAF50.svg" alt="Maturity">
</p>

---

**Ehrlichkeitscheck - was heute wirklich läuft:** eine Rust-Bibliothek und ein Kommandozeilenprogramm (`phoenix-laser`) mit 1044 Tests, die auf einem Computer laufen: es wandelt ein Bild per Dithering in Punkte um und schreibt den G-Code eines Lasers, der Zeile für Zeile mit dynamischer Leistung und Überlauf brennt, liest SVG-, DXF- und LBRN2-Zeichnungen und schreibt den G-Code, der ihre Konturen schneidet (die inneren zuerst, mit Strahlbreite, Stegen und mehreren Durchgängen) und ihre Formen mit Linien graviert, mit den Einstellungen jeder Ebene in einer Datei, brennt Text, QR-Codes und Strichcodes, prüft ein Programm gegen die Grenzen der Maschine und schreibt ein Testraster Leistung gegen Vorschub. Es hat ein Fenster (mit `--features gui` gebaut), den Panel-Rahmen der Suite mit zwölf Panels und vier Arbeitsbereichen, das ein Bild oder eine Zeichnung öffnet, den Weg des Lasers zeigt, die Einstellungen ändern lässt und den G-Code exportiert; das Fenster wurde von keiner Person auf einem Bildschirm angesehen. Der Leser für LBRN2-Projekte wurde an zwölf echten Projekten eines Laserherstellers gemessen (alle zwölf werden gelesen, und das einzige Bild darunter wird dekodiert und punktweise graviert; die Richtung ihrer y-Achse wurde nie an einem echten Teil bestätigt). Es kann außerdem ein Programm an das Laser-Plugin des EH3D-Servers senden und den Auftrag verfolgen, anhalten oder stoppen (`phoenix-laser server`), nur gegen einen Ersatz des Servers getestet. **Nichts davon lief auf einem echten Laser**: nichts, was daraus oder aus etwas anderem entstand, wurde an einer Maschine geschnitten oder gebrannt.

---

> **Sicherheitsstatus:** Dieses Programm steuert selbst keine Maschine und ist kein Sicherheitssystem: es schreibt G-Code, prüft ihn und kann ihn an das Laser-Plugin des EH3D-Servers senden, das bisher ein Simulator ist. Was es erzeugt, muss ein Mensch prüfen, bevor etwas an einer Maschine läuft; nichts davon lief auf echter Hardware.


## 📋 Auf einen Blick

| | |
|---|---|
| Version | `0.1.6` |
| Reifegrad | Funktionsfähig |
| Tests | 1044 |
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
| Tests | 🟡 | 1044 Tests bestehen (`cargo test`), mit Normalfällen, Grenzfällen und beschädigten Eingaben. |
| Sicherheit | 🟡 | Reines sicheres Rust (`forbid(unsafe_code)`): nichts gerät bei den übergebenen Dateien in Panik und ein schlechter Wert ist ein Fehler mit Begründung; keine unabhängige Prüfung. |
| Fenster | 🟠 | Geschrieben (egui, `--features gui`) als Panel-Rahmen der Suite: ein Bild oder eine SVG-, DXF- oder LBRN2-Zeichnung öffnen oder ablegen, den Weg des Lasers sehen (der Schnitt in einer Farbe, der Leerweg abgeschwächt), die Ebenentabelle bearbeiten, die Modi für Text, QR-Code, Strichcode, Marken, Rahmen und Fokus, ein Kalibrierraster, Zoom und Verschieben, die Zeit schätzen, prüfen, Bericht, an den Server senden, exportieren; die Logik ist getestet, aber das Fenster wurde von keiner Person auf einem Bildschirm angesehen. |
| Echte Hardware | 🔴 | Nicht erprobt: nichts davon wurde an einer Maschine ausgeführt. |
| Dokumentation | 🟡 | README in acht Sprachen, das Changelog und die zwei aus `docs/` gezeichneten Bilder. |

## 🎯 Überblick

* **Das Bild** (`eh3d-image`): PNG (Grau, Farbe, transparent, 8 oder 16 Bit) und PGM (Text und binär) werden zu Graustufen, transparente Teile als Weiß, die Größe ist begrenzt, damit eine feindliche Datei keine Gigabytes verlangen kann; `resize` mittelt die Pixel, sodass ein Punkt so groß wie der Strahl ist.
* **Die Punkte** (`src/dither.rs`): ein Schwellwert oder Fehlerdiffusion nach Floyd-Steinberg oder Atkinson; die Tests prüfen, dass die Dunkelheit eines Grautons erhalten bleibt (ein mittleres Grau brennt etwa die Hälfte der Punkte) und dass Weiß nichts und Schwarz alles brennt.
* **Der Laser** (`src/raster.rs`, `src/calibrate.rs`): Zeile für Zeile, von links nach rechts und dann von rechts nach links, `M4` mit `S0` beim Verfahren und der gewählten Leistung beim Brennen, ein fester oder aus der Beschleunigung berechneter Überlauf (v zum Quadrat durch 2a) und ein Raster aus Quadraten mit einer Leistung je Spalte und einem Vorschub je Zeile, um das Passende für ein Material zu finden.
* **Der Kopf und die Zahlen jeder G-Code-Datei** (`src/gcode_out.rs`): Die Datei beginnt mit Kommentarzeilen, die den Generator und seine Version, die Einheiten (mm, G21), das Bild, aus dem sie entstand, mit seinem SHA-256 und die Werkzeuge (den Laser und seine Leistung) nennen; ein Kalibrierraster gibt an, dass es keine Quelle hat. Eine Zahl wird nur auf eine Weise geschrieben (höchstens drei Dezimalstellen, kein `-0`), und ein Wert, der nicht endlich ist, wird abgelehnt, statt als `NaN` oder `inf` geschrieben zu werden.
* **`--dry-run`:** `engrave` berechnet das Ergebnis und zählt es (Punkte, Zeilen, Läufe, gebrannte Länge, Zeit), ohne die Ausgabedatei zu schreiben, die dann entfallen darf.
* **Die Zeichnungen** (`src/svg.rs`, `src/dxf.rs`, `src/flatten.rs`, `src/vector.rs`): eine sichere Teilmenge von SVG (Pfade mit allen Befehlen, Formen, Transformationen, Gruppen, Einheiten und ein `use` eines Elements derselben Datei) und die Elemente einer DXF (Linien, Polylinien mit Bögen, Bögen, Kreise, Ellipsen, Splines, Blöcke mit ihren Einfügungen, Einheiten) werden zu Formen in Millimetern; Skripte, Bilder, ein `use` einer anderen Datei, deklarierte Entitäten und alles Unbekannte werden übersprungen oder abgelehnt, Größe, Tiefe sowie Zahl der Elemente (was ein `use` hereinholt, zählt mit) und Punkte sind durch die Einstellungen begrenzt, ein `use`, das auf sich selbst zurückführt, ist ein Fehler, und Kurven werden innerhalb einer Toleranz zu geraden Stücken.
* **Schneiden und Gravieren** (`src/order.rs`, `src/offset.rs`, `src/cut.rs`): Die Formen werden nach Ebene (DXF) oder Farbe (SVG) gruppiert; jede Gruppe wird mit ihren eigenen Einstellungen geschnitten (lose Linien verbunden, die Konturen innerhalb einer anderen zuerst, kürzester Leerweg, die Strahlbreite auf der richtigen Seite abgezogen, Stege ungeschnitten, mehrere Durchgänge mit nachgeführtem Fokus) oder graviert (geschlossene Formen mit Linien in einem Winkel gefüllt, nach der Gerade-Ungerade-Regel). `phoenix-laser layers DRAWING --accel A` nennt außerdem die Zeit jeder Ebene, wenn der Kopf an jeder Ecke mit `A` mm/s2 abbremst und beschleunigt (und wie viel davon der Laser brennt), gezählt von der Vorschau des Programms.
* **Die Einstellungsdatei** (`src/vconfig.rs`, `docs/vector-settings.md`): Jeder abgestimmte Wert (die Grenzen der Leser, die Maschine, die Einstellungen jeder Ebene) steht in einer einfachen Textdatei, die Zeile für Zeile geprüft wird; nichts davon steckt im Code.
* **Der Fokustest** (`src/calibrate.rs`, `[focus]` in der Einstellungsdatei): `phoenix-laser focus` brennt eine Leiter kurzer Linien, jede mit dem Kopf in anderer Höhe; die schärfste Linie zeigt, wo der Strahl im Fokus ist; die Höhenschrittweite hat keinen Standardwert (was unten ist, hängt von der Maschine ab), die Leistung folgt der Leistungsgrenze und der Kopf kehrt auf die erste Höhe zurück. Mit `--powers 10,20,30` (bis zu 10 Leistungen) brennt es stattdessen ein Fokusraster: dieselben Linien einmal für jede Leistung, mit einer Spalte je Leistung nebeneinander, sodass man ablesen kann, welche Leistung in der Höhe des Fokus die dünnste Linie brennt; der Kopf fährt jede Höhe nur einmal an und brennt alle Spalten dieser Zeile, und jede Leistung hält die Leistungsgrenze ein.
* **Das Testraster eines Materials** (`src/calibrate.rs`, `[calibrate]` in der Einstellungsdatei): `phoenix-laser calibrate --material NAME --materials DATEI` verteilt die Leistungen und Vorschübe des Rasters um die Werte des Materials der Bibliothek, innerhalb der Grenzen und ohne Wiederholungen, und `--labels` brennt die Leistung jeder Spalte und den Vorschub jeder Zeile mit einer Strichschrift neben die Quadrate, sodass das Raster später auf dem Material lesbar ist; ein Material über einer Grenze wird abgelehnt und nie gesenkt. Mit einem Material kommen die Zahlen standardmäßig (`--no-labels` nimmt sie weg), und die niedrigste Leistung und der niedrigste Vorschub der Leiter sind die Schlüssel `min_power_percent` und `min_feed_mm_min`.
* **Text** (`src/text.rs`, `[text]` in der Einstellungsdatei): `phoenix-laser text OUT.nc --text TEXT` brennt einen Namen, eine Seriennummer oder eine Marke mit einer Strichschrift des Programms (Großbuchstaben, Ziffern, Leerzeichen und `. , - + / : ! ? ( ) = _ '`; Kleinbuchstaben werden als Großbuchstaben gebrannt, ein Zeilenumbruch beginnt eine Zeile) und ohne Schriftdatei. Die Höhe eines Großbuchstabens, die Abstände zwischen Buchstaben und Zeilen, die Leistung, der Vorschub und die Durchgänge sind Einstellungen; ein Zeichen, das die Schrift nicht hat, ein Text mit mehr als `max_chars` Zeichen, eine Höhe über `max_height_mm`, eine Leistung oder ein Vorschub über der Grenze oder eine Arbeit, die das Bett verlassen würde, werden abgelehnt und nie gekürzt, und der Text wird nie in einen Kommentar des Programms geschrieben. Die Zeilen eines mehrzeiligen Textes lassen sich links, mittig oder rechts ausrichten (`--align`), der Text kann als Spiegelbild für die Rückseite von Glas oder Acryl gebrannt werden (`--mirror`), und ein Brennvorgang, der länger als `max_minutes` dauern würde, wird abgelehnt.
* **QR-Code** (`src/qr.rs`, `[qr]` in der Einstellungsdatei): `phoenix-laser qr OUT.nc --text DATEN` brennt Daten als eigenen QR-Code des Programms (Versionen 1 bis 4, 21 bis 33 Module je Seite, im Byte-Modus, mit den vier Stufen der Reed-Solomon-Fehlerkorrektur und der Maske mit der geringsten Strafe), indem jedes dunkle Modul mit Linien gefüllt wird; das Symbol wurde mit einem unabhängigen Decoder in allen Versionen und Stufen zurückgelesen, und die Tests lesen es mit einem eigenen Leser zurück. Die Seite eines Moduls, der Abstand der Linien, die Stufe, die Leistung, die Geschwindigkeit, die Durchgänge und die Grenzen sind Einstellungen; Daten über `max_bytes`, eine Version über `max_version`, ein Symbol größer als `max_size_mm`, eine Füllung mit mehr als `max_lines` Bewegungen, eine Leistung oder Geschwindigkeit über der Grenze oder Arbeit außerhalb des Bettes werden abgelehnt, nie beschnitten oder verkleinert, und die Daten werden nie in einen Kommentar des Programms geschrieben. `--print` zeigt das Symbol als Text. Standardmäßig aus: `--invert` brennt stattdessen die hellen Module (mit `--border-modules` ein gebrannter Rand darum), `--mirror` brennt das Spiegelbild für die Rückseite von Glas oder Acryl, `--overscan-mm` fährt den Kopf bei ausgeschaltetem Laser an jede Linie heran und darüber hinaus (abgelehnt, wenn er das Bett verlassen würde), und ein Brennvorgang, der länger als `max_minutes` dauern würde, wird abgelehnt.
* **Strichcode** (`src/barcode.rs`, `[barcode]` in der Einstellungsdatei): `phoenix-laser barcode OUT.nc --text DATEN` brennt Daten als eigenen Code-128-Strichcode des Programms (die Zeichen von Leerzeichen bis `~` im Zeichensatz B und ein Text aus einer geraden Anzahl Ziffern im Zeichensatz C, halb so breit, mit Prüfzeichen), wobei jeder Balken entlang seiner Höhe mit Linien gefüllt wird; die Tests lesen ihn mit einem eigenen Leser zurück, er wurde aber noch nicht mit einem Scanner gelesen. Die Breite des schmalsten Balkens, die Höhe, der Abstand der Linien, die Leistung, die Geschwindigkeit, die Durchgänge und die Grenzen sind Einstellungen; ein Byte, das kein druckbares Zeichen ist, mehr als `max_chars` Zeichen, ein Strichcode, der breiter als `max_width_mm` ist, eine Füllung mit mehr als `max_lines` Bewegungen, ein Brennvorgang, der länger als `max_minutes` dauern würde, eine Leistung oder Geschwindigkeit über der Grenze oder Arbeit außerhalb des Bettes werden abgelehnt, nie beschnitten oder verkleinert, und die Daten werden nie in einen Kommentar des Programms geschrieben. `--mirror` brennt das Spiegelbild für die Rückseite von Glas oder Acryl.
* **Kreuzschraffur** (`src/cut.rs`, `hatch_cross` in einer Ebene): eine `engrave`-Ebene mit `hatch_cross = on` (oder `--cross` bei `vector` und `trace`) wird ein zweites Mal mit Linien im rechten Winkel zu den ersten gefüllt, mit demselben Abstand, in jedem Durchgang, und die Grenze der Linien gilt für jeden Satz; die Energie eines Quadratmillimeters, die die Liste der Ebenen angibt, verdoppelt sich, und die Vorschau und die Prüfung des Programms sehen es wie jedes andere Programm.
* **Der Rahmen der Arbeit** (`src/frame.rs`, `[frame]` in der Einstellungsdatei): `phoenix-laser frame` schreibt ein Programm, das das Rechteck der Arbeit bei ausgeschaltetem Laser abfährt, aus einer Größe oder aus dem Kasten dessen, was ein G-Code-Programm brennt, damit man sieht, wo es landet; eine schwache Pilotleistung braucht jedes Mal `--confirm-laser-on`, ist auf 5 % begrenzt (`max_pilot_percent` in `[frame]` ändert die Grenze) und folgt der Leistungsgrenze, und ein Rahmen außerhalb des Betts wird abgelehnt.
* **Positioniermarken** (`src/frame.rs`, `[marks]` in der Einstellungsdatei): `phoenix-laser marks OUT.nc --size W,H` brennt mit geringer Leistung ein L an jede Ecke des Rechtecks der Arbeit (auf seinen Kanten oder `margin_mm` außerhalb) und mit `--centre-cross` ein Kreuz in der Mitte, damit das Material an dieselbe Stelle zurückgelegt und eine zweite Arbeit an der ersten ausgerichtet werden kann. Anders als der Rahmen werden die Marken gebrannt: Leistung und Geschwindigkeit halten die Grenzen der Maschine ein, und ein Arm, der länger als eine halbe Seite des Rechtecks ist, oder Marken, die das Bett verlassen würden, werden abgelehnt, nie gekürzt. Die Länge der Arme, der Rand, die Leistung, die Geschwindigkeit und die Durchgänge sind Einstellungen. `phoenix-laser align PICTURE.pgm --size W,H --px-per-mm S` findet die Marken in einem von oben aufgenommenen Bild des Betts und sagt, wie weit sie von ihrem Soll entfernt liegen (die Verschiebung, die zum Ursprung des Auftrags addiert wird, und die Drehung); ein Bild, in dem eine Marke fehlt, zu weit abweicht oder zu stark gedreht ist, wird abgelehnt; es wurde nur an Bildern erprobt, die `--simulate` zeichnet, nie an einer echten Kamera.
* **Die Vorschau** (`src/preview.rs`): `phoenix-laser preview` liest ein G-Code-Programm (das eigene oder ein fremdes), gibt die Bewegungen, die gebrannte und die gefahrene Länge, den Kasten, die Leistungen und eine Zeit aus, die das Bremsen in den Ecken zählt, warnt vor Gefährlichem (Laser an bei einer Eilbewegung, kein Vorschub, Leistung über Vollleistung, Arbeit außerhalb des Betts, kein Ende) und zeichnet den Weg als SVG; die Tests lesen zurück, was das Gravieren schreibt, und prüfen dieselbe Länge. Mit `--laser-watts W` (die Leistung des Lasers bei voller Leistung) nennt es außerdem die Energie, die das Programm einbrennt, aus der Leistung jeder Bewegung als Anteil von `--smax` über die Zeit, in der sie brennt, und seine mittlere Leistung.
* **Die Materialbibliothek** (`src/materials.rs`, `examples/materials.cfg`): Was zu einem Material und einer Arbeitsweise passt, steht einmal in einer Textdatei (`[material NAME]`: Leistung, Geschwindigkeit, Durchgänge, Fokusschritt, Strahlbreite, Schraffur, Punktgröße, Dithering und Tonkurve) und wird mit `--material NAME --materials DATEI` gewählt; die Optionen der Kommandozeile gehen vor, und die Leistungsgrenze gilt über beiden. `phoenix-laser materials DATEI` listet die Bibliothek.
* **Die Leistungsgrenze** (`src/safety.rs`, `max_power_percent` in `[machine]`, `--max-power`): Ein Programm, dessen Leistung über der Grenze liegt, wird mit einem Fehler abgelehnt, der die Ebene oder die Option nennt, und nie stillschweigend gesenkt; die Grenze ist 100 %, bis eine Datei oder eine Option weniger sagt.
* **Maschinenprofile** (`[profile NAME]` in der Einstellungsdatei, `--profile NAME`): Wer zwei Maschinen hat oder ein Lasermodul gegen ein anderes tauscht, führt für jede ein Profil (Verfahrweg des Tischs, Leistungsgrenze, Geschwindigkeitsgrenze, Wert der vollen Leistung und Watt); das gewählte Profil ist die Maschine, und `--max-speed` steht neben `--max-power`: Eine Geschwindigkeit über der Grenze wird abgelehnt, nie gesenkt.
* **Konturen aus einem Bild** (`src/trace.rs`, `[trace]` in der Einstellungsdatei): `phoenix-laser trace` folgt dem Umriss eines Logos, einer Silhouette oder einer Schablone, statt ihn Punkt für Punkt zu brennen; die Graustufen werden an einer Schwelle getrennt, der Rand wird mit Marching Squares auf Subpixel-Genauigkeit gefunden, vereinfacht, bis kein Punkt weiter als `simplify_mm` von der Linie liegt, und Flecken werden verworfen; das Ergebnis ist eine Ebene einer Zeichnung, die mit denselben Einstellungen und Prüfungen geschnitten (oder mit Linien gefüllt) wird, und Bildgröße, Randstücke, Konturen und Punkte sind alle begrenzt.
* **Töne, Materialien und Grenzen im Fenster** (`src/window.rs`): Die Felder der Tonkurve (Gamma, Kontrast, Helligkeit, Invertieren und Auto-Level) bearbeiten das Bild mit derselben Funktion wie die Befehlszeile; eine Materialbibliothek wird aus einer Datei geladen (oder über `PHOENIX_LASER_MATERIALS` benannt), und die Wahl eines Materials füllt die Felder für Leistung, Geschwindigkeit, Punkt, Dithering, Ton und jede Ebene einer Zeichnung; die Grenzen für Leistung und Geschwindigkeit sind ebenfalls Felder, und ein Wert darüber wird abgelehnt, nie gesenkt. Die Startwerte kommen aus der Einstellungsdatei (`[tone]`, `[machine]`).
* **Materialien im Fenster bearbeitet** (`src/window.rs`, `src/materials.rs`): Der Materialabschnitt kann ein Material bearbeiten, kopieren, aus den Feldern erzeugen (Leistung, Geschwindigkeit, Punkt, Überlauf, Rasterung und Tonkurve) und löschen sowie eine neue Bibliothek anlegen; ein Feld für jeden Schlüssel der Datei nimmt die Werte auf, und beim Speichern wird der Abschnitt dieses Materials an seiner Stelle neu geschrieben (oder am Ende angefügt), ohne die Kommentare, die Leerzeilen, die Reihenfolge, die Zeilenenden oder die anderen Materialien anzutasten. Ein Wert außerhalb seines Bereichs, ein leerer, zu langer oder schon benutzter Name, eine Datei, die nicht UTF-8 oder keine gültige Bibliothek ist, wird mit dem genannten Schlüssel und unberührter Datei abgelehnt, nie gekürzt; Löschen braucht einen zweiten Klick, und eine neue Bibliothek wird nie über eine vorhandene Datei geschrieben.
* **Der Fokustest im Fenster** (`src/window.rs`): ein Modus mit den Feldern des Abschnitts `[focus]` (Höhe der ersten Linie, Schritt, Anzahl und Länge der Linien, Abstand, Geschwindigkeit und Leistung); der Test wird mit derselben Funktion wie `phoenix-laser focus` berechnet, Leistung und Geschwindigkeit folgen den Grenzen, es gibt keinen Standardschritt, weil von der Maschine abhängt, wo unten ist, und ein Hinweis, dass der Test Z bewegt, bleibt in den Notizen. Ein Feld für die Leistungen des Rasters (bis zu 10, durch Kommas getrennt) lässt das Fenster das Fokusraster von `focus --powers` brennen: die Linien einmal für jede Leistung, eine Spalte für jede; das Feld der einzelnen Leistung wird dann nicht benutzt, und eine Leistung, die nicht über 0 bis 100 liegt, über der Grenze liegt oder zu viele sind, werden abgelehnt.
* **Text im Fenster** (`src/window.rs`): ein Modus mit einem Feld für die Wörter und den Feldern des Abschnitts `[text]` (Höhe eines Großbuchstabens, höchster Buchstabe, Abstände zwischen Buchstaben und Zeilen, Leistung, Geschwindigkeit, Durchgänge und Höchstzahl der Zeichen); der Text wird mit derselben Funktion wie `phoenix-laser text` vom Ursprung der Maschine aus berechnet, sodass ein Buchstabe, den die Schrift nicht hat, ein Text über einer Grenze, eine Leistung oder Geschwindigkeit über der Grenze der Maschine oder Arbeit außerhalb des Bettes (wenn die Einstellungsdatei es angibt) abgelehnt wird und die Statuszeile den Grund nennt; er wird als `text.nc` mit dem Kopf der Befehlszeile geschrieben. Die Ausrichtung der Zeilen, der Spiegel-Schalter und die längste Dauer gibt es auch im Fenster.
* **QR-Code im Fenster** (`src/window.rs`): ein Modus mit einem Feld für die Daten, der Wahl der Fehlerkorrekturstufe und den Feldern des Abschnitts `[qr]` (Modulseite, Abstand der Linien, Leistung, Geschwindigkeit, Durchgänge, Höchstzahl der Bytes, größte Version und Seite sowie Höchstzahl der Brennbewegungen); das Symbol wird mit derselben Funktion wie `phoenix-laser qr` vom Ursprung der Maschine aus berechnet, sodass Daten, die nicht passen, ein Symbol über einer Grenze, eine Leistung oder Geschwindigkeit über der Grenze der Maschine oder Arbeit außerhalb des Bettes (wenn die Einstellungsdatei es angibt) abgelehnt wird und die Statuszeile den Grund nennt; es wird als `qr.nc` mit dem Kopf der Befehlszeile geschrieben. Die Schalter für invertiert und Spiegelbild sowie die Felder für Rand, Anlauf und längste Dauer gibt es auch im Fenster.
* **Positionsmarken im Fenster** (`src/window.rs`, `[marks]` in der Einstellungsdatei): ein Modus mit Feldern für die Größe des Rechtecks, den Schenkel jedes L, den Rand, die Leistung, die Geschwindigkeit und die Durchgänge sowie einem Schalter für das Kreuz in der Mitte (eine Einstellungsdatei füllt die Felder von `[marks]`); die Marken werden mit derselben Funktion wie `phoenix-laser marks` vom Ursprung der Maschine aus berechnet, und ein Schenkel länger als eine halbe Seite, eine Leistung oder Geschwindigkeit über der Grenze oder Marken außerhalb des Betts werden abgelehnt, mit dem Feld oder dem Grund in der Statuszeile. Es wird als `marks.nc` mit dem üblichen Kopf geschrieben.
* **Strichcode Code 128 im Fenster** (`src/window.rs`, `[barcode]` in der Einstellungsdatei): ein Modus mit einem Feld für die Daten, einem Schalter für das Spiegelbild und den Feldern des Abschnitts `[barcode]` (die Breite des schmalsten Balkens, die Höhe, der Abstand der Linien, die Leistung, die Geschwindigkeit, die Durchgänge und die Grenzen; eine Einstellungsdatei füllt sie); der Strichcode wird mit derselben Funktion wie `phoenix-laser barcode` vom Ursprung der Maschine aus berechnet, und Daten, die kein druckbarer Text oder zu lang sind, ein Strichcode über einer Grenze, eine Leistung oder Geschwindigkeit über der Grenze der Maschine oder Arbeit außerhalb des Betts werden abgelehnt, mit dem Feld oder dem Grund in der Statuszeile. Er wird als `barcode.nc` mit dem üblichen Kopf geschrieben.
* **Der Rahmen im Fenster** (`src/window.rs`, `[frame]` in der Einstellungsdatei): ein Modus, der das geöffnete Bild oder die geöffnete Zeichnung bei ausgeschaltetem Laser einrahmt; Rand, Runden und Geschwindigkeit sind Felder. Eine Pilotleistung über 0 schaltet den Laser ein, deshalb fragt das Fenster, und nichts wird berechnet oder geschrieben, bis die Person mit Ja antwortet, jedes Mal (ein geändertes Feld, ein Moduswechsel oder das Schreiben der Datei nimmt die Antwort zurück), unter der Grenze von `max_pilot_percent`, der Leistungsgrenze und dem Tisch.
* **Die Vorschau im Fenster** (`src/window.rs`): Jeder vom Fenster berechnete G-Code wird von derselben Vorschau wie `phoenix-laser preview` gezählt (Bewegungen, gebrannte und gefahrene Länge, eine Zeit, die die Ecken mit der Beschleunigung des Feldes zählt, und was gefährlich aussieht, etwa Arbeit außerhalb des Tisches der Einstellungsdatei), und ein Programm beliebiger Herkunft (`.nc`, `.ngc`, `.gcode`, `.gc`, `.tap`) kann geöffnet oder auf das Fenster gezogen werden, um seinen Weg und seine Notizen zu sehen. Es wird als Text gelesen, nie ausgeführt und nie wieder geschrieben. Die Leistung des Lasers kann in einem Feld eingegeben werden (die Einstellungsdatei und ein Maschinenprofil füllen es aus `laser_watts`; 0 bedeutet unbekannt): Die Vorschau nennt dann auch die Energie, die das Programm brennt, und seine mittlere Leistung, wie es `phoenix-laser preview --laser-watts` tut; ein Wert, der keine Zahl von 0 bis 1000 ist, wird abgelehnt.
* **Die Kontur im Fenster** (`src/window.rs`, `[trace]` in der Einstellungsdatei): ein Modus, der die Kontur des geöffneten Bildes mit derselben Funktion wie `phoenix-laser trace` nachfährt und sie schneidet oder ihre Formen mit Linien füllt, als Ebene namens `trace`; die Felder sind Schwelle, Vereinfachung und kleinstes Teil, dazu ein Schalter für helle Tinte; Leistung und Geschwindigkeit sind die der Laser-Felder und halten die Grenzen ein, und die Grenzen des Bildes (`max_pixels`, `max_contours`, `max_segments`) stammen aus der Einstellungsdatei, sodass ein zu detailreiches Bild ein Fehler ist und kein langsames Programm.
* **Das Maschinenprofil im Fenster** (`src/window.rs`, `[profile NAME]` in der Einstellungsdatei): eine Liste im Seitenfeld mit den Maschinen der Datei; die gewählte ist die Maschine, so wie `--profile` sie macht, also übernehmen die Felder für den Vollleistungswert und die Grenzen von Leistung und Geschwindigkeit ihre Werte, und das Bett des Profils ist das, das Rahmen und Vorschau prüfen. Was ein Profil auslässt, kommt aus `[machine]`, ein Profil, das nicht in der Datei steht, ändert nichts, und die Grenzen weisen weiterhin eine Leistung oder Geschwindigkeit darüber zurück.
* **Das Raster aus einem Material im Fenster** (`src/window.rs`, `[calibrate]` in der Einstellungsdatei): der Kalibriermodus listet die Materialien der Bibliothek auf; ist eines gewählt, wird das Raster mit derselben Leiter wie bei `phoenix-laser calibrate --material` um seine Leistung und seinen Vorschub gelegt (Felder für die Stufen, die Spreizung und die niedrigste Leistung und den niedrigsten Vorschub, zwischen diesen und den Grenzen gehalten), und ein Schalter brennt die Zahlen mit der Höhe ihres Feldes neben die Quadrate. Ohne Auswahl ist es das Raster der beiden Listen wie bisher.
* **Die Prüfung des Programms im Fenster** (`src/window.rs`, `[verify]` in der Einstellungsdatei): ein G-Code-Programm, das zum Ansehen geöffnet wird, wird auch von `phoenix-laser verify` mit den Grenzen des Fensters und dem Bett der Einstellungsdatei geprüft, und die Seitenleiste sagt, ob es besteht (mit dem Anfang seines SHA-256) oder was falsch ist; ein Feld hält den Überlauf, der mit ausgeschaltetem Laser über das Bett hinaus erlaubt ist (standardmäßig 0, bis 100 mm), und ein Programm, das außerhalb des Betts brennt, besteht nicht, was auch immer das Feld sagt.
* **Die Prüfung eines Programms** (`src/verify.rs`): `phoenix-laser verify PROGRAMM.nc` liest ein G-Code-Programm (dieses oder ein fremdes) als Text, führt es nie aus und liefert den SHA-256 der genauen Datei, was sie tut und ein Urteil: Es besteht nur unter der Leistungs- und Geschwindigkeitsgrenze der Maschine, innerhalb des Tisches (Arbeit außerhalb fällt durch; `--overscan-mm` oder `overscan_mm` in `[verify]` erlaubt dem Kopf, mit ausgeschaltetem Laser so weit über den Tisch hinauszufahren, wie es der Überlauf eines anderen Programms tut, und Brennen außerhalb des Tisches fällt immer durch), mit ausgeschaltetem Laser bei jeder Eilbewegung und einem Vorschub bei jeder brennenden Bewegung. `--json` liefert den Bericht, den ein anderes Programm neben einer an diesen Hash gebundenen Freigabe aufbewahren kann, mit dem Maschinenprofil, dem SHA-256 der Einstellungsdatei, aus der die Grenzen stammen, und den Namen der durchgeführten Prüfungen, und der Exit-Code ist 1, wenn es durchfällt. `report` schreibt diese Prüfung als etwas zum Lesen vor einem Auftrag, in der Sprache von `--lang` (Englisch, Spanisch, Deutsch, Französisch, Italienisch, Japanisch, Chinesisch oder Portugiesisch aus Portugal): SHA-256, Bewegungen und Zeit, höchste Leistung und Geschwindigkeit gegen die Grenzen, Energie, ob das Material geprüft war, Sperrzonen und Verriegelungen sowie ein Ergebnis.
* **Eine Prüfung, die an das genehmigte Programm gebunden ist** (`src/verify.rs`): `phoenix-laser verify PROGRAMM.nc --expect-sha256 HASH` lässt das Programm durchfallen, wenn sein SHA-256 nicht die angegebenen 64 Hexziffern sind, und nennt den, den es hat, sodass ein nach der Ansicht geändertes Programm nicht versehentlich ausgeführt wird; der Fehler steht auch im JSON-Bericht.
* **Sperrzonen, Verriegelungen, der Bericht und die Einstellungsliste** (`src/guard.rs`, `src/report.rs`, `src/contract.rs`): Rechtecke des Betts, die ein Programm meiden muss (`[zone NAME]`), und der Zustand von Not-Aus, Gehäuse und Luftunterstützung, wie ihn die Einstellungsdatei angibt (`[interlocks]`), lassen die Prüfung scheitern und verhindern das Senden eines Programms, eines Vektorauftrags und eines Testschusses; was an den Server geht, wird auf 40 % der Leistung begrenzt, sofern `[send]` keine andere Grenze nennt; `phoenix-laser report` schreibt die Prüfung zum Lesen vor einem Auftrag, in acht Sprachen; `phoenix-laser settings` listet jede Einstellung zum Lesen für ein anderes Programm auf. Weder ein Sensor noch ein echter Not-Aus wurde erprobt.
* **Senden an den EH3D-Server** (`src/server.rs`, `[server]` in der Einstellungsdatei): `phoenix-laser server send PROGRAMM.nc` sendet ein G-Code-Programm als Auftrag des Plugins `laser` von EH3D-SERVER, über den gemeinsamen Client der Suite (`eh3d-serverclient` aus EH3D-COMMON: die Schlüssel `server_address`, `server_token`, `server_machine`, `server_timeout_s`, `server_retries` und `server_mdns`, Wiederholungen mit wachsenden Pausen, eine Anfrage, die etwas anlegt, wird nie zweimal gesendet, die Ablehnungen des Servers werden mit ihrem Code in der Sprache des Systems genannt). Bevor es hinausgeht, wird das Programm gegen die Grenzen der Maschine der Einstellungen geprüft (Grenze für Leistung und Geschwindigkeit, das Bett), und eines, das durchfällt, wird nicht gesendet; `--follow` bleibt, bis der Auftrag endet; `status`, `job`, `pause` und `resume` fragen nach Maschine und Auftrag und halten ihn an oder lassen ihn weiterlaufen; `stop` ist der NOT-HALT der Lasermaschine: der Auftrag endet sofort und die Maschine wird verriegelt, sodass sie kein Programm annimmt und ihr keines gesendet wird, bis ein Administrator `release` ausführt. Das Token wird nie ausgegeben (es kann in der Variablen `PHOENIX_LASER_SERVER_TOKEN` stehen) und läuft im Klartext über einfaches HTTP. Die lokalen Befehle bleiben unberührt. Getestet gegen einen Ersatz des Laser-Plugins, im Speicher und an einem Socket dieses Rechners; das Plugin des Servers ist selbst ein Simulator, und nichts davon ist mit einem echten Laser gelaufen. `phoenix-laser server vector DRAWING.svg|dxf` sendet eine Zeichnung als Vektorauftrag (die Pfade und eine Operation je Ebene, zuvor gegen die Grenzen der Einstellungen geprüft), und `phoenix-laser server test-fire --power-percent P --confirm-fire` feuert den Laser einen Augenblick lang, um den Fokus zu finden (der Server lehnt das ab, solange ein Auftrag läuft, solange ein Not-Halt die Maschine hält, bei offenem Gehäuse oder ausgeschalteter Luftunterstützung und über seiner eigenen Grenze).
* **Senden an den Server aus dem Fenster** (`src/window.rs`, `src/linkpane.rs`, `[server]` in der Einstellungsdatei): Der Serverbereich des Fensters zeigt das Feld der Suite (Adresse, Token, Maschine, Grenzen der Verbindung, Zustand der Verbindung und eine Anmeldung mit Name und Passwort), und seine Schaltfläche sendet das berechnete Programm als Auftrag der Lasermaschine, zeigt den Fortschritt und lässt den Auftrag anhalten, weiterlaufen oder mit einer NOT-HALT-Schaltfläche beenden (der Auftrag endet sofort, und die Maschine bleibt verriegelt, bis ein Administrator sie freigibt); ein Rahmen, der den Laser einschaltet und nicht bestätigt wurde, und ein Programm, das die Prüfung gegen die Grenzen nicht besteht, werden nicht gesendet, und ein nur zum Ansehen geöffnetes Programm wird erst gesendet, wenn es die Prüfung besteht und die Person das Kästchen dafür ankreuzt, und dann unverändert. Das Schließen des Fensters lässt den Auftrag auf dem Server, wo er ist.
* **LBRN2-Projekte** (`src/lbrn2.rs`, `docs/lbrn2.md`, `examples/lbrn2_survey.rs`): `cut`, `vector`, `layers` und `convert` (und das Fenster) lesen auch `.lbrn2`-Projekte. Die Rechtecke, die Pfade (gerade Stücke und Bézier-Kurven), die Gruppen mit ihren Transformationen und die Umrisse eines Textes werden zu einer Zeichnung in Millimetern; jede Ebene des Projekts wird zu einer Zeile der Tabelle mit Leistung, Geschwindigkeit (das Projekt hat mm/s, die Tabelle mm/min), Durchgängen, Füllabstand und Reihenfolge als Vorschläge, über denen die Grenzen der Maschine weiter gelten, und eine Ebene, die nicht zum Laser geschickt wird, wird übersprungen. Bilder in einem Projekt (ein PNG oder PGM in Base64) werden innerhalb von Grenzen dekodiert und mit `phoenix-laser engrave PROJEKT.lbrn2 OUT.nc` punktweise graviert (Leistung und Geschwindigkeit der Ebene des Bildes werden genommen, wenn die Kommandozeile keine gibt, und ein Wert über der Grenze wird abgelehnt, nie gesenkt); auch eine Ellipse und ein Vieleck (nur nach den Namen ihrer Attribute geschrieben, ein Prototyp), die Stege einer Ebene und der Zähler des variablen Textes werden gelesen, und ein Bild über einer Grenze oder in einem anderen Format sowie Formen eines Typs, der in den echten Beispielen nicht vorkam, bleiben draußen und werden in den Hinweisen mit einem stabilen Code gezählt. Das Format wird nach dem Verhalten von zwölf echten Projekten gelesen, ohne Spezifikation, und die y-Achse wird wie die eines SVG genommen, außer `lbrn2_flip_y = off` sagt etwas anderes: die Richtung wurde nie an einem echten Teil bestätigt, und jedes Lesen sagt das in seinen Hinweisen. `phoenix-laser text OUT.nc --text 'N{counter:4} {name}' --csv DATEI.csv` (oder `--counter START,SCHRITT,ANZAHL` oder `--counter-project PROJEKT.lbrn2`) schreibt für jede Zeile einer CSV-Datei ein Programm, alle geprüft, bevor das erste geschrieben wird, und `--preview` zeigt den Text jeder Zeile, ohne zu schreiben. `cargo run --release --example lbrn2_survey -- ORDNER` liest jede `.lbrn2`-Datei eines Ordners und sagt, welche gelesen werden, welche nicht und warum, und welche Elemente und Attribute die Dateien haben, die der Leser nicht nutzt.
* **Testdateien, das ältere Projektformat und Voreinstellungen als Text** (`src/testfiles.rs`, `src/presets.rs`, `src/dxf.rs`, `src/lbrn2.rs`): der DXF-Leser zeichnet den Umriss eines `HATCH` und eines `SOLID`; `.lbrn`-Projekte (das ältere Format, dessen Pfade als `V`- und `P`-Elemente stehen) werden wie `.lbrn2` gelesen und ihre Texte mit der Strichschrift gezeichnet; `--project-layers` gibt den Ebenen eines Projekts auf der Kommandozeile ihre Leistung, Geschwindigkeit und Durchgänge; `calibrate` nimmt Bereiche (`--powers 20:80:20`); `phoenix-laser matrix` schreibt die Matrix Leistung gegen Geschwindigkeit als Projekt mit einer Ebene je Paar; `phoenix-laser ytest` schreibt das Projekt, das zeigt, wohin die Y-Achse zeigt; ein Buch von Voreinstellungen wird als Text gespeichert und von `phoenix-laser presets` aufgelistet. Es wurde nichts gebrannt: die Matrix und der Test der Y-Achse wurden in keinem anderen Programm geöffnet.
* **Das Fenster aus Panels** (`src/panels.rs`, `src/gui.rs`): das Fenster ist der Panel-Rahmen der Suite (`eh3d-dock`): zwölf Panels (Ansicht, Ebenen, Einstellungen der Arbeit, Eigenschaften, Bibliothek, Materialien, Umrandung, Maschine, Simulation und Zeit, Auftrag, Bericht und Konsole), die sich entfernen, wieder einsetzen, verschieben, als Tabs bündeln, schweben lassen, einklappen und maximieren lassen; vier Arbeitsbereiche nach Aufgaben, nicht nach Erfahrungsstufen (Vorbereiten, Ausführen, Prüfen und Kompakt); eine Menüleiste mit Datei, Bearbeiten, Ansicht, Werkzeuge, Fenster und Hilfe; und, außerhalb jedes Panels, damit kein Arbeitsbereich sie verbergen kann, der Stopp des Auftrags und der Zustand der Verriegelungen. Die Anordnung steht in der Einstellungsdatei als Zeilen, die mit `dock.` beginnen und die der Leser der Einstellungen in Ruhe lässt. Panels, Arbeitsbereiche und Menüs sind Daten mit Tests; das Fenster wurde von keiner Person auf einem Bildschirm angesehen.
* **Startwerte aus der gemeinsamen Materialdatenbank** (`src/commonmat.rs`, Feature `common-machining`): ein Material, eine Laserart (Diode oder CO2), Schneiden oder Gravieren, die Leistung des Lasers in Watt und die Dicke ergeben Leistung, Geschwindigkeit und Durchgänge, die die gemeinsame Datenbank hat, genau so, wie sie sie hat, mit dem stets sichtbaren Hinweis, dass sie an keiner Maschine erprobt wurden, und den Gründen, wenn das Material nicht unter einen Laser darf (dann gibt es keine Antwort). Außerhalb des Bereichs, den die Datenbank kennt, gibt es ebenfalls keine Antwort, nie eine Vermutung. Das Feature ist standardmäßig an (der Build ist auf ein Release von EH3D-COMMON mit dem Modul `machining` festgelegt); `--no-default-features` lässt es weg.
* **Unterebenen** (`src/sublayers.rs`, `[sublayer EBENE/BEZEICHNUNG]` in der Einstellungsdatei): Eine Ebene kann mehr als einmal bearbeitet werden, zum Beispiel erst eine Füllung und dann eine Kontur, jeder Durchgang mit eigenem Modus, eigener Leistung, Geschwindigkeit, Durchgangszahl, Fokusschritt und Schraffur. Eine Unterebene geht von den Einstellungen ihrer Ebene aus und wird wie eine Ebene von den Grenzen der Maschine gehalten (ein Fehler nennt Ebene und Bezeichnung, nichts wird stillschweigend gesenkt); die Arbeit läuft erst gravierend und zuletzt schneidend, jede Art nach ihrem `order`, und `layers` listet die Durchgänge mit ihrer Zeit. Der Server erhält nur die Ebenen und sagt das.
* **Wohin die Arbeit kommt, ihr Rahmen und ihre Zeit** (`src/job.rs`, `[job]` in der Einstellungsdatei): Die Arbeit wird an einem der neun Punkte ihrer Box (`anchor`) angelegt oder mit `align = page` dort belassen, wo sie gezeichnet wurde; der Rahmen der Arbeit ist ihre Box mit dem Rand von `[frame]`, bei ausgeschaltetem Laser gefahren und abgelehnt, wenn er das Bett verließe; die Zeit ist eine Spanne, von jeder Bewegung mit eigenem Vorschub ohne Bremsen bis zur Zeit der Vorschau mit Bremsen an jeder Ecke bei `accel_mm_s2`, und die Durchgänge sind in beiden enthalten.
* **Voreinstellungen und Live-Anpassung** (`src/presets.rs`, `src/live.rs`, `[live]` in der Einstellungsdatei): Eine Voreinstellung für ein Material und eine Dicke wird einer Ebene zugewiesen (eine Kopie) oder mit ihr verknüpft (die Ebene folgt der Voreinstellung; eine Änderung nennt die erreichten Ebenen und wird ganz abgelehnt, wenn eine sie nicht annehmen kann). Die Werte der gemeinsamen Datenbank werden unverändert verwendet, als an keiner Maschine erprobt gekennzeichnet, und die Zweitwerte anderer Quellen werden nur auf Wunsch verwendet. Die Live-Anpassung von Leistung und Geschwindigkeit ist eine Rechnung mit vier Grenzen (eigener Bereich, der standardmäßig nie einen Wert erhöht, Maschine, Server und Pilot), die sagt, welche Grenze einen Wert hielt; nichts wird an eine Maschine gesendet.
* **Der Bericht im Fenster** (`src/window.rs`): der Bericht von `phoenix-laser report` lässt sich aus dem vom Fenster berechneten Programm in der Sprache des Fensters erstellen und wird verworfen, sobald sich etwas ändert.
* **Geordnetes Dithering** (`src/dither.rs`): `--dither bayer` (auch im Fenster) entscheidet jeden Punkt nach seinem Ort und seinem Grau mit einer 8-x-8-Bayer-Matrix, das Muster ist also regelmäßig und dasselbe Grau brennt immer dieselben Punkte; die Tests prüfen, dass jede Stufe der Matrix einmal vorkommt und die Dunkelheit eines Grautons erhalten bleibt.
* **Die Tonkurve** (`src/tone.rs`): Vor der Punkterzeugung lassen sich die Graustufen auf den vollen Bereich dehnen (`--auto-level`), Kontrast und Helligkeit ändern, mit einem Gamma krümmen oder invertieren, mit einer einzigen Tabelle von 256 Einträgen; mit den Standardwerten bleibt das Bild völlig unverändert.
* **Ein geprüfter G-Code:** Das Programm einer Zeichnung wird vor der Ausgabe Zeile für Zeile geprüft: kein Laser an bei einer Eilbewegung oder ohne Geschwindigkeit, kein `S` über voller Leistung, keine Bewegung außerhalb des Bettes, `M5` am Ende; die Datei wird nie über die Zeichnung oder die Einstellungen geschrieben, aus denen sie stammt.
* **Projekte der Suite** (`src/eh3dfile.rs`, `docs/eh3d-project.md`): `phoenix-laser project` hält die Arbeit im Zweig `laser/` eines `.eh3d`-Projekts, einer einzigen Datei mit einem Zweig je Programm: `save` legt Zeichnung, Einstellungen, Materialbibliothek und Programme dort ab; `import` bewahrt die Zeichnung eines anderen Programms (LBRN2, LBRN, SVG oder DXF) Byte für Byte mit einem Bericht darüber, was gelesen wurde, was nicht und was beim Export verloren geht; `take` kopiert einen Umriss oder ein Netz aus einem anderen Zweig, etwa `cad/`, mit Hash und Format der Quelle, sodass `list` und `open` melden, wenn die Kopie veraltet ist; `open` und `extract` schreiben Dateien, ohne eine zu überschreiben. Geschrieben wird nur `laser/`: alle anderen Zweige, bekannt oder nicht, behalten ihre Bytes. Erprobt wurde das nur mit erfundenen Projekten.

## 🧩 Rolle in EH3D

EH3D-PHOENIX-LASER ist ein Programm der EH3D Fabrication Suite. Es arbeitet als Bibliothek und Kommandozeile und, mit `--features gui` gebaut, als Fenster im gemeinsamen Aussehen der Suite. Es liest und schreibt einfache Dateien und lässt sich allein oder zwischen den anderen Programmen nutzen.

## 🏛️ Architektur

<p align="center">
  <img src="images/architecture.svg" alt="Architektur von EH3D-PHOENIX-LASER: wer es nutzt, was es enthält und wovon es abhängt" width="100%">
</p>


Wie die Teile dieses Repositorys zusammenpassen: wer es nutzt, was es enthält und wovon es abhängt. Das Bild wird aus `docs/architecture.json` gezeichnet und gegen den Code geprüft.

## 🖼️ Screenshots

Diese Bilder nimmt das Programm selbst mit `--shot` auf (siehe Schnellstart).

<p align="center">
  <img src="images/screens/window-vector.png" alt="Schneiden einer Zeichnung: der Weg des Lasers über der Platte der Beispiele, darunter die Tabelle der Ebenen" width="100%">
</p>

<p align="center">
  <img src="images/screens/window-engrave.png" alt="Gravieren eines Bildes: die Punkte des Verlaufs der Beispiele, nach ihrer Leistung gefärbt" width="100%">
</p>

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
| Ein Text, der zu lang, zu hoch oder zu stark ist oder außerhalb des Bettes brennen würde | nur die Zeichen der Schrift werden angenommen (ein Buchstabe, den sie nicht hat, wird abgelehnt, nie übersprungen), Zeichenzahl, Höhe, Leistung und Geschwindigkeit bleiben zwischen Grenzen, die die Einstellungen enger ziehen können, Arbeit außerhalb des Bettes wird abgelehnt statt beschnitten, und der Text wird nie in einen Kommentar des Programms geschrieben |
| Daten, die nicht in den QR-Code passen, ein Symbol, das zu groß, zu dicht oder zu stark ist, oder eines, das außerhalb des Bettes brennen würde | Version, Bytezahl, Größe, Zahl der Brennbewegungen, Leistung und Geschwindigkeit bleiben zwischen Grenzen, die die Einstellungen enger ziehen können, und was darüber liegt, wird abgelehnt, statt beschnitten oder verkleinert zu werden; die Daten werden nie in einen Kommentar des Programms geschrieben |
| Die Ausgabedatei, über die Zeichnung oder die Einstellungen gewählt, aus denen sie entsteht | wird abgelehnt, im Fenster und in den Befehlen |
| Ein an einen Server gesendetes Programm, ein Token, das gesehen werden könnte, oder eine Maschine, die angehalten und wieder gestartet wurde | das Programm wird vor dem Senden gegen die Grenzen der Maschine geprüft und eines, das durchfällt, wird nicht gesendet; das Token wird nie ausgegeben (nicht in einer Meldung, nicht in einem `Debug`), kommt aus der Einstellungsdatei oder der Umgebung und läuft im Klartext über einfaches HTTP, gilt also für ein vertrauenswürdiges Netz oder einen Tunnel (das Programm sagt es, wenn die Adresse öffentlich ist); der Halt ist der Not-Halt, der die Maschine verriegelt, bis ein Administrator sie freigibt, und einer verriegelten Maschine wird kein Programm gesendet |

**Was nicht abgesichert ist:** Ein fehlerfrei erzeugtes Ergebnis ist kein geprüftes Ergebnis. Nichts hier kennt die Grenzen einer Maschine, und das Programm hatte keine unabhängige Prüfung. Eine Schwachstelle bitte privat melden (siehe [SECURITY.md](SECURITY.md)).

## 🔌 Schnittstellen

Die Bibliothek ist `eh3d_phoenix_laser` (`use eh3d_phoenix_laser::...`); die Kommandozeile ist `phoenix-laser`. Die Befehle sind diese; jeder gibt sein Ergebnis auf der Standardausgabe aus und meldet einen Fehler auf der Fehlerausgabe mit einem Exit-Code ungleich null. Es gibt keinen Netzwerkzugriff.

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

`DRAWING` ist eine SVG- oder DXF-Datei. `cut` folgt den Konturen (die inneren zuerst); `vector` füllt die geschlossenen Formen mit Linien und folgt den offenen; beide nehmen die Einstellungen jeder Ebene oder Farbe aus `--config FILE` (siehe [docs/vector-settings.md](docs/vector-settings.md)), und eine Option der Kommandozeile ändert sie für alle Ebenen. `layers` listet die Ebenen einer Zeichnung mit dem auf, was mit jeder geschehen würde. `convert` schreibt eine Zeichnung erneut als schlichte SVG- oder DXF-Datei (nach der Endung der Ausgabe, nie über die gelesene Zeichnung), damit ein anderes Programm öffnen kann, was dieses verstanden hat.

## ⚙️ Konfiguration

Die Gravur eines Bildes wird mit Argumenten der Kommandozeile eingestellt (siehe Schnittstellen), und jeder Standardwert steht in der Hilfe des Befehls (`phoenix-laser help`). Das Schneiden und Gravieren einer Zeichnung wird mit einer Einstellungsdatei eingestellt (`--config FILE` oder „Einstellungen laden“ im Fenster): die Grenzen der Leser, wie eine Datei gelesen wird, die Reihenfolge der Schnitte, die Maschine und für jede Ebene oder Farbe der Modus (schneiden, gravieren oder auslassen), die Leistung, die Geschwindigkeit, die Durchgänge, der Fokusschritt, die Strahlbreite und ihre Seite, die Stege und die Schraffur. Jeder Schlüssel, sein Standardwert und sein Bereich stehen in [docs/vector-settings.md](docs/vector-settings.md), ein Beispiel in `examples/plate.cfg`; ein unbekannter Schlüssel oder ein Wert außerhalb des Bereichs ist ein Fehler, der seine Zeile nennt. Die festen Grenzen des Bildlesers stehen als benannte Konstanten im Code und sind durch Tests abgedeckt. `phoenix-laser settings` listet jede Einstellung (Abschnitt, Schlüssel, Art des Werts, Standardwert und ob sie eine Sicherheitseinstellung ist) als Text oder mit `--json` als JSON auf, damit ein anderes Programm sie lesen kann.

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

Um ein Bild des Fensters aufzunehmen, ohne es von Hand zu öffnen, starten Sie es mit `--shot DATEI.png` (auch `--size WxH`, `--theme forge|dark|light`, `--wait N`, `--window NAME`, `--lang CODE`, `--open FILE`, `--replace`): Es öffnet sich in dieser Größe, speichert das PNG, behält keine Einstellungen und schließt sich selbst (Exit-Code 0, wenn die Datei geschrieben wurde, 1, wenn nicht, 2 bei falschen Argumenten). `--window NAME` zeigt ein Fenster des Programms: `engrave`, `calibrate`, `cut`, `focus`, `frame`, `preview`, `trace`, `text`, `qr`, `marks`, `barcode` (dieser Modus wird gewählt) oder `server` (der Abschnitt für den Server ist geöffnet), so können die Bilder der README jeden Modus zeigen. `--window` akzeptiert außerdem `prepare`, `run`, `review` und `compact`, die vier Arbeitsbereiche der Panels.

## 🧪 Tests

Die Suite hat 1044 Tests (`cargo test`, in der CI unter Windows und Linux). Sie prüfen Normalfälle, Grenzfälle und beschädigte Eingaben: leere Dateien, Datenmüll und abgeschnittene Dateien lösen nie eine Panik aus. Die CI führt außerdem `cargo fmt --check`, `cargo clippy` mit Warnungen als Fehlern und `python -I tools/ci_validate.py` aus.

## 🏗️ Build und Veröffentlichung

Die Version steht in `Cargo.toml` und in `eh3d.project.json`; jede Version hat ihren Eintrag in [CHANGELOG.md](CHANGELOG.md). `build.bat` / `build.sh` bauen das Programm, `test.bat` / `test.sh` führen die Tests aus und `run.bat` / `run.sh` starten es mit einem Beispiel. Releases erscheinen auf GitHub.

**Downloads:** Mit jeder Version dieses Repositorys werden fertige Programme für jedes System veröffentlicht: Windows (64 und 32 Bit), Linux (64 Bit) und macOS (Apple Silicon und Intel). Jedes Paket, `phoenix-laser-<Version>-<System>.zip` oder `.tar.gz`, enthält das Programm, die Lizenz, diese README, das Änderungsprotokoll, das Symbol und die Beispieldateien und kommt mit seiner `.sha256`-Datei; die Version listet außerdem jede Datei in `SHA256SUMS` und `build-manifest.json` auf. Die Versionen stehen auf der Seite Releases des Repositorys.

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
│   ├── main.rs                                  die Befehlszeile: engrave, calibrate, cut, vector, verify, report, convert, settings und mehr
│   ├── calibrate.rs                             die Testraster: Leistung gegen Vorschub, Fokusleiter und das Raster eines Materials
│   ├── cli_args.rs                              die Argumente der Kommandozeile, dieselbe Datei in jedem kleinen Werkzeug
│   ├── dither.rs                                von Graustufen zu Punkten: Schwellwert, Floyd-Steinberg, Atkinson und Bayer
│   ├── materials.rs                             die Materialbibliothek: Leistung, Geschwindigkeit, Durchgänge, Töne
│   ├── commonmat.rs                             Startwerte der gemeinsamen Materialdatenbank, so wie sie sind
│   ├── preview.rs                               die Vorschau eines G-Codes: Bewegungen, Zeit, Gefahren und ein SVG des Weges
│   ├── tone.rs                                  die Tonkurve eines Bildes: Stufen, Kontrast, Helligkeit, Gamma und Inversion vor den Punkten
│   ├── trace.rs                                 die Konturen eines Bildes: Schwelle, Marching Squares und vereinfachte Ringe, mit Grenzen
│   ├── text.rs                                  eine Textzeile, mit einer eigenen Strichschrift gebrannt, innerhalb von Grenzen
│   ├── qr.rs                                    ein eigener QR-Code, Versionen 1 bis 4 mit Fehlerkorrektur, durch Füllen der Module gebrannt
│   ├── barcode.rs                               ein eigener Code-128-Strichcode, durch Füllen der Balken mit Linien gebrannt
│   ├── gcode_out.rs                             die einzige Schreibweise einer G-Code-Zahl und der Standardkopf
│   ├── raster.rs                                die Gravur Zeile für Zeile: Punkte zu Bewegungen, Überlauf und Leistung
│   ├── safety.rs                                die Leistungsgrenze und die Regeln, die den Laser beim Zeigen der Arbeit aus lassen
│   ├── guard.rs                                 die Sperrzonen des Betts und die Verriegelungen: was ein Programm nicht erreichen darf
│   ├── server.rs                                der Weg zum EH3D-Server: prüfen, senden, verfolgen, Not-Halt
│   ├── linkpane.rs                              das Serverfeld im Fenster: die Felder, die Verbindung und die Anmeldung
│   ├── shot.rs                                  der Bildmodus des Fensters (--shot): seine Argumente, ohne Bildschirm gelesen und geprüft
│   ├── vector.rs                                eine gelesene Zeichnung: Formen in mm, Ebenen, Grenzen und einfache Geometrie
│   ├── verify.rs                                die Prüfung eines Programms: sein SHA-256, was es tut und ein Urteil gegen die Grenzen der Maschine
│   ├── report.rs                                der Bericht, der vor einem Auftrag gelesen wird, in den acht Sprachen
│   ├── flatten.rs                               Kurven in gerade Stücke innerhalb einer Toleranz (Bézier, Bögen)
│   ├── frame.rs                                 der Rahmen der Arbeit (das Rechteck bei ausgeschaltetem Laser) und die Marken an seinen Ecken
│   ├── register.rs                              wo der Auftrag liegt: die in einem Bild gefundenen Marken und die simulierte Kamera
│   ├── svg.rs                                   der SVG-Leser: eine sichere Teilmenge, ohne Skripte, Bilder oder Entitäten
│   ├── testfiles.rs                             die Matrix Leistung gegen Geschwindigkeit und der Y-Achsen-Test als Projekte
│   ├── dxf.rs                                   der DXF-Leser: Linien, Polylinien, Bögen, Kreise, Ellipsen, Splines und Blöcke, mit Einheiten
│   ├── eh3dfile.rs                              der Zweig laser eines .eh3d-Projekts: speichern, importieren, übernehmen, öffnen, entnehmen
│   ├── projcli.rs                               der Befehl project der Kommandozeile
│   ├── lbrn2.rs                                 der Leser von LBRN2-Projekten: Ebenen, Formen, Gruppen und Kurven
│   ├── lbrn2_pic.rs                             die Bilder eines LBRN2-Projekts: Base64, PNG, Grenzen, wie platziert gedreht
│   ├── vartext.rs                               der variable Text: ein Zähler und die Spalten einer CSV-Datei, ein Text je Zeile
│   ├── sublayers.rs                             zusätzliche Durchgänge über einer Ebene: eigene Werte, Grenzen, Reihenfolge
│   ├── job.rs                                   wo die Arbeit beginnt, der Rahmen dazu und die Zeit als Spanne
│   ├── live.rs                                  die Live-Anpassung von Leistung und Tempo mit vier Grenzen (keine Maschine wird berührt)
│   ├── presets.rs                               Voreinstellungen je Material: zuweisen (Kopie) oder verknüpfen (folgt), mit Warnung bei Ungeprüftem
│   ├── export.rs                                eine Zeichnung, erneut als schlichte SVG- oder DXF-Datei geschrieben
│   ├── energy.rs                                die Energie eines Durchgangs je Millimeter oder Quadratmillimeter aus der optischen Leistung
│   ├── offset.rs                                Kontur um die Strahlbreite seitlich versetzen, Kante für Kante geprüft
│   ├── order.rs                                 die Reihenfolge der Schnitte: lose Linien verbunden, Inneres zuerst, kurzer Leerweg
│   ├── cut.rs                                   von der Zeichnung zum G-Code: Schnitt, Stege, Schraffur, Durchgänge und Sicherheitsprüfung
│   ├── vconfig.rs                               die Einstellungsdatei der Vektorarbeit: Grenzen, Maschine und jede Ebene
│   ├── contract.rs                              jede Einstellung als Liste: Abschnitt, Schlüssel, Art, Standardwert und ob sie der Sicherheit dient
│   ├── browse.rs                                die Dateiliste des Fensters zum Öffnen einer Datei
│   ├── gui.rs                                   das Fenster (egui, mit dem Feature gui): zeichnet den Zustand und gibt weiter, was die Person tut
│   ├── panels.rs                                die Panels, Arbeitsbereiche und Menüs des Fensters als Daten
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
│   ├── lbrn2.rs                                 Tests des LBRN2-Lesers mit hier geschriebenen Projekten, feindlichen Dateien und Fuzz
│   ├── lbrn2_more.rs                            Tests der LBRN2-Bilder, der y-Achse, Ellipse, Vieleck, Stege und Fuzz
│   ├── lbrn_v1.rs                               Tests des älteren Projektformats, seiner Texte und der Ebenen eines Projekts auf der Kommandozeile
│   ├── dxf_hatch.rs                             Tests der Schraffuren und Solids eines DXF, mit Fuzz mit festem Seed
│   ├── vartext.rs                               Tests des variablen Textes: CSV, Zähler, Platzhalter, Serien und Fuzz
│   ├── sublayers.rs                             Tests der zusätzlichen Durchgänge: Reihenfolge, Grenzen, beschädigte Dateien mit festem Seed
│   ├── joborigin.rs                             Tests der neun Anker, des Rahmens bei ausgeschaltetem Laser und der Zeit als Spanne
│   ├── livelimits.rs                            Tests der Live-Anpassung: vier Grenzen, acht Sprachen, fester Seed
│   ├── presets.rs                               Tests der Voreinstellungen: zuweisen, verknüpfen, abgelehnte Änderungen, Alternativen, Zufallsserie
│   ├── presetbook.rs                            Tests des als Text gespeicherten Buchs der Voreinstellungen
│   ├── matrix.rs                                Tests der Bereiche des Testrasters und seiner Matrix
│   ├── testfiles.rs                             Tests der Matrix- und Y-Achsen-Projekte, die das Programm schreibt
│   ├── samples.rs                               die Leser über einen Ordner mit Beispielen, wenn einer genannt wird
│   ├── guard.rs                                 Tests der Zonen, Verriegelungen, Materialgrenzen, des Berichts und der Einstellungsliste
│   ├── server_cli.rs                            der Befehl server gegen einen Ersatz des Laser-Plugins
│   ├── window_cli.rs                            Tests, dass ein Build ohne das Feature gui eine reine Kommandozeile bleibt
│   ├── dockwindow.rs                            Tests des Fenstermodells: Projektebenen, Bericht, Verriegelungen, Anordnung und Materialien
│   ├── eh3dfile.rs                              Tests des Projektzweigs: Hin und zurück, andere Zweige, veraltete Kopien, beschädigte Dateien
│   ├── reqs.rs                                  Tests des Anforderungsregisters
│   ├── more.rs                                  der G-Code zurückgelesen, die Zahlen, das Raster
│   ├── more2.rs                                 die Gravur am Rand jedes Werts
│   └── rapid_power.rs                           keine Eilbewegung mit gesetzter Leistung
├── examples/
│   ├── gradient.pgm                             ein Beispiel zum Ausprobieren
│   ├── plate.svg                                eine Platte mit Loch und einer Fläche zum Gravieren (SVG)
│   ├── plate.dxf                                dieselbe Platte als DXF
│   ├── plate.cfg                                die Einstellungen seiner Ebenen: Leistung, Geschwindigkeit, Durchgänge, Strahlbreite, Stege und Schraffur
│   ├── materials.cfg                            eine Materialbibliothek: Birke und Schiefer, Schneiden, Gravieren und Fotos
│   ├── safety.cfg                               Sperrzonen und Verriegelungen zum Ausprobieren mit verify
│   ├── materials-starter.cfg                    eine Start-Bibliothek von Materialien, alle als ungeprüft markiert
│   └── lbrn2_survey.rs                          liest alle LBRN2-Dateien eines Ordners und meldet, was gelesen wird und was nicht
├── docs/
│   ├── architecture.json                        Daten des Architektur-Bildes
│   ├── NOT_YET.md                               gute Ideen außerhalb dieser Version, mit Grund und Zeitpunkt
│   ├── REQUIREMENTS.md                          das Register der Anforderungen: Stand, Nachweis und Test jeder einzelnen
│   ├── SCOPE.md                                 wofür es da ist, was es nicht tut und was es nie entscheidet
│   ├── STATUS.md                                was es gibt, was fehlt und was offen ist
│   ├── structure.json                           Daten des Struktur-Bildes
│   ├── vector-settings.md                       die Einstellungsdatei der Vektorarbeit, Schlüssel für Schlüssel
│   ├── materials.md                             die Materialbibliothek, Schlüssel für Schlüssel
│   ├── eh3d-project.md                          das .eh3d-Projekt und der Zweig laser
│   └── lbrn2.md                                 was der LBRN2-Leser liest, was er auslässt und wie er gemessen wurde
├── tools/                                       CI-Prüfung, README-Prüfung und die Bildwerkzeuge
├── images/                                      Banner, README-Bilder und Fensteraufnahmen
├── .github/                                     CI- und Release-Workflows, Vorlagen für Issues und Pull Requests
├── eh3d.project.json                            Manifest: Version, Rolle, Reifegrad und Build-Befehl
├── Cargo.toml Cargo.lock                        der Workspace und seine festgelegten Abhängigkeiten
├── rust-toolchain.toml                          die festgelegte Rust-Toolchain
├── CHANGELOG.md                                 was sich in jeder Version geändert hat
├── AGENTS.md                                    die Regeln für alle, die das Repository ändern
├── SECURITY.md SUPPORT.md                       Schwachstellen melden und Fragen stellen
└── CONTRIBUTING.md CODE_OF_CONDUCT.md LICENSE   Mitwirken, Verhaltenskodex und Lizenz
```

</details>

## 🗺️ Roadmap und bekannte Grenzen

**Bekannte Grenzen heute**

* Es wurde nicht an einer Maschine und nicht mit einer Sammlung echter Dateien erprobt und hatte keine unabhängige Prüfung: nichts wurde gebrannt, die Prüfungen lesen das Programm als Text und der Server ist ein Ersatz.
* Der Leser für LBRN2-Projekte wurde an zwölf echten Projekten eines Laserherstellers gemessen und an nichts sonst: Bilder in einem Projekt, Formen eines Typs, der nicht darunter war, die Richtung der y-Achse und die abgerundete Ecke eines Rechtecks sind nicht bestätigt.
* Eine Zeichnung wird in einer sicheren Teilmenge gelesen (kein `use` einer anderen Datei, kein Text, keine Verläufe oder Stile mit Selektoren; keine Schraffuren, kein Text und keine Bemaßungen in einer DXF); eine Kontur, die schmaler als der Strahl ist, wird nicht geschnitten, und das Programm sagt es; der Laser ist der eines GRBL-Controllers mit `M4`.
* Der QR-Code und der Strichcode werden von eigenen Lesern des Programms zurückgelesen; kein Scanner hat ein gebranntes Symbol gelesen, und das Fenster wurde von keiner Person auf einem Bildschirm angesehen.
* Die Unterebenen, die Verknüpfungen der Voreinstellungen und die Live-Anpassung haben noch keinen Platz im Fenster; die Verknüpfungen der Voreinstellungen liegen im Speicher und nicht in der Einstellungsdatei, und nichts von der Live-Anpassung wird an eine Maschine gesendet.

**Als Nächstes**

* Ein erster Beweis an einem echten Laser und Material, mit einer Person an der Maschine und dem Not-Halt in Reichweite.
* Die Leistungsdichte vor dem Brennen (`layers` gibt die Energie eines Durchgangs schon aus, wenn `laser_watts` gesetzt ist) auf der Platine zeigen und die Leistung in den Ecken eines Schnitts konstant halten.
* Das Fenster auf einem Bildschirm ansehen und fertigstellen.
* Es mit echten Zeichnungen und mit den anderen Programmen der Suite erproben.
* Die bewusst zurückgestellten Ideen stehen unter [Noch nicht](docs/NOT_YET.md).

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
* [Umfang](docs/SCOPE.md): wofür das Programm da ist, was es nicht tut und was es nie entscheidet.
* [Anforderungen](docs/REQUIREMENTS.md): das Register der Fähigkeiten, der Stand jeder einzelnen und der Test, der sie zeigt.
* [Stand](docs/STATUS.md): was es gibt, was fehlt und was noch offen ist.
* [Noch nicht](docs/NOT_YET.md): gute Ideen, die in dieser Version fehlen.
* [Regeln für Änderungen am Repository](AGENTS.md): die Regeln für alle, die das Repository ändern.
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

<p align="center">
  <img src="images/EH3D_PHOENIX_LASER_BANNER.svg" alt="EH3D-PHOENIX-LASER banner" width="100%">
</p>

# <img src="images/brand/icon-64.png" alt="" width="40" height="40" align="absmiddle"> EH3D-PHOENIX-LASER
<p align="center"><a href="README.md">🇺🇸 English</a> | <a href="README_spa.md">🇪🇸 Español</a> | 🇫🇷 <b>Français</b> | <a href="README_ita.md">🇮🇹 Italiano</a> | <a href="README_deu.md">🇩🇪 Deutsch</a> | <a href="README_zho.md">🇨🇳 中文</a> | <a href="README_jpn.md">🇯🇵 日本語</a> | <a href="README_por.md">🇵🇹 Português</a></p>

### Gravure laser à partir d'une image et découpe à partir d'un dessin : tramage, G-code raster, contours SVG et DXF et grille d'étalonnage

<p align="center">
  <img src="https://img.shields.io/badge/License-GPL%203.0--or--later-blue.svg" alt="GPL 3.0 or later">
  <img src="https://img.shields.io/badge/Stack-Rust-DEA584.svg" alt="Rust">
  <img src="https://img.shields.io/badge/Languages-8-00E5FF.svg" alt="Languages">
  <img src="https://img.shields.io/badge/Version-0.1.1-0f6cbd.svg" alt="Version">
  <img src="https://img.shields.io/badge/Maturity-functional-4CAF50.svg" alt="Maturity">
</p>

---

**Vérification d'honnêteté - ce qui fonctionne aujourd'hui :** une bibliothèque Rust et un programme en ligne de commande (`phoenix-laser`) avec 591 tests qui s'exécutent sur un ordinateur : il transforme une image en points par tramage et écrit le G-code d'un laser qui brûle ligne par ligne avec une puissance dynamique et un dépassement, lit des dessins SVG et DXF et écrit le G-code qui découpe leurs contours (ceux de l'intérieur d'abord, avec la largeur du faisceau, des ponts et plusieurs passes) et grave leurs formes avec des lignes, avec les réglages de chaque calque dans un fichier, et une grille d'essai puissance contre avance. Il a une fenêtre (compilée avec `--features gui`) qui ouvre une image ou un dessin, montre le trajet du laser, permet de changer les réglages et exporte le G-code, mais la fenêtre n'a pas encore été vue sur un écran ; il n'a pas été essayé sur une machine et n'a pas été comparé à une collection de fichiers réels.

---

> **État de sécurité :** ce programme ne commande aucune machine et n'est pas un système de sécurité. Ce qu'il produit doit être vérifié par une personne avant tout essai sur une machine ; rien ici n'a fonctionné sur du matériel réel.


## 📋 En un coup d'œil

| | |
|---|---|
| Version | `0.1.1` |
| Maturité | Fonctionnel |
| Tests | 591 |
| Pile technique | Rust |
| Sur du matériel réel | Non essayé : rien ici n'a fonctionné sur une machine |

**Aller à  :** [Présentation](#-présentation) · [Architecture](#-architecture) · [Démarrage rapide](#-démarrage-rapide) · [Tests](#-tests) · [Structure](#-structure-du-dépôt) · [Liés](#-projets-liés)

## 🚦 État du projet

Chaque ligne est une partie de ce dépôt et le feu dit où elle en est aujourd'hui. Elle est écrite d'après ce que le dépôt peut montrer (tests, compilations, exécutions), pas d'après ce qui est prévu.

🟢 Fait et vérifié · 🟡 Fonctionne, avec des limites connues · 🟠 À moitié fait ou pas encore vérifié · 🔴 Pas fait ou ne fonctionne pas

| Domaine | État | Ce que cela veut dire ici |
|---|---|---|
| Cœur | 🟡 | transforme une image en points par tramage et écrit le G-code d'un laser qui brûle ligne par ligne avec une puissance dynamique et un dépassement, lit des dessins SVG et DXF et les découpe ou les grave par calque (contours ordonnés, largeur du faisceau, ponts, passes, hachures), et une grille d'essai puissance contre avance ; testé sur un ordinateur avec des entrées normales et abîmées, pas avec une collection de fichiers réels. |
| Ligne de commande | 🟡 | `phoenix-laser` (`engrave`, `calibrate`, `info`, `cut`, `vector`, `layers`) exécute les mêmes fonctions que la bibliothèque et affiche le résultat ; chaque erreur revient sous forme de message et d'un code de sortie non nul. |
| Tests | 🟡 | 591 tests passent (`cargo test`), avec des cas normaux, des cas limites et des entrées abîmées. |
| Sécurité | 🟡 | Rust sûr et simple (`forbid(unsafe_code)`) : rien ne panique sur les fichiers reçus et une mauvaise valeur est une erreur avec sa raison ; pas de revue indépendante. |
| Fenêtre | 🟠 | Écrite (egui, `--features gui`) : ouvrir ou déposer une image ou un dessin SVG ou DXF, voir le trajet du laser (la découpe d'une couleur, le déplacement atténué), modifier le tableau des calques, une grille d'étalonnage, zoom et déplacement, estimer la durée, exporter ; sa logique est testée, mais elle n'a pas encore été compilée ni vue sur un écran. |
| Matériel réel | 🔴 | Non essayé : rien ici n'a été exécuté sur une machine. |
| Documentation | 🟡 | README en huit langues, le journal des modifications et les deux images dessinées depuis `docs/`. |

## 🎯 Présentation

* **L'image** (`eh3d-image`) : PNG (gris, couleur, transparent, 8 ou 16 bits) et PGM (texte et binaire) deviennent des niveaux de gris, les parties transparentes comptant comme blanc et la taille étant limitée pour qu'un fichier hostile ne puisse pas réclamer des gigaoctets ; `resize` moyenne les pixels pour qu'un point soit aussi gros que le faisceau.
* **Les points** (`src/dither.rs`) : un seuil ou une diffusion d'erreur de Floyd-Steinberg ou d'Atkinson ; les tests vérifient que l'obscurité d'un gris est conservée (un gris moyen brûle environ la moitié des points) et que le blanc ne brûle rien et le noir tout.
* **Le laser** (`src/raster.rs`, `src/calibrate.rs`) : une ligne après l'autre, de gauche à droite puis de droite à gauche, `M4` avec `S0` pendant le déplacement et la puissance choisie là où il brûle, un dépassement fixe ou calculé avec l'accélération (v au carré sur 2a) et une grille de carrés avec une puissance par colonne et une avance par ligne pour trouver ce qui convient à un matériau.
* **L'en-tête et les nombres de tout fichier de G-code** (`src/gcode_out.rs`) : le fichier s'ouvre sur des lignes de commentaire qui disent le générateur et sa version, les unités (mm, G21), l'image dont il vient avec son SHA-256 et les outils (le laser et sa puissance) ; une grille d'étalonnage dit qu'elle n'a pas de source. Un nombre s'écrit d'une seule façon (trois décimales au plus, pas de `-0`) et une valeur non finie est refusée au lieu d'être écrite `NaN` ou `inf`.
* **`--dry-run` :** `engrave` calcule le résultat et le compte (points, lignes, passages, longueur brûlée, temps) sans écrire le fichier de sortie, qui peut alors être omis.
* **Les dessins** (`src/svg.rs`, `src/dxf.rs`, `src/flatten.rs`, `src/vector.rs`) : un sous-ensemble sûr de SVG (tracés avec toutes leurs commandes, formes, transformations, groupes, unités et un `use` d'un élément du même fichier) et les entités d'un DXF (lignes, polylignes avec arcs, arcs, cercles, splines, unités) deviennent des formes en millimètres ; les scripts, les images, un `use` d'un autre fichier, les entités déclarées et tout ce qui est inconnu sont ignorés ou refusés, la taille, la profondeur et le nombre d'éléments (ce qu'apporte un `use` compte) et de points sont limités par les réglages, un `use` qui revient sur lui-même est une erreur, et les courbes deviennent des segments droits dans une tolérance.
* **Découpe et gravure** (`src/order.rs`, `src/offset.rs`, `src/cut.rs`) : les formes sont groupées par calque (DXF) ou par couleur (SVG) ; chaque groupe est découpé (lignes libres jointes, les contours à l'intérieur d'un autre d'abord, le plus court déplacement, la largeur du faisceau retirée du bon côté, ponts laissés, plusieurs passes avec la mise au point qui avance) ou gravé (formes fermées remplies de lignes selon un angle, par la règle pair-impair) avec ses propres réglages.
* **Le fichier de réglages** (`src/vconfig.rs`, `docs/vector-settings.md`) : toute valeur ajustée (les limites des lecteurs, la machine, les réglages de chaque calque) est dans un fichier texte simple vérifié ligne par ligne ; rien de cela n'est dans le code.
* **Le test de mise au point** (`src/calibrate.rs`, `[focus]` dans le fichier de réglages) : `phoenix-laser focus` brûle une échelle de courtes lignes, chacune avec la tête à une autre hauteur, et la ligne la plus nette montre où le faisceau est net ; le pas de hauteur n'a pas de valeur par défaut (le bas dépend de la machine), la puissance obéit à la limite de puissance et la tête revient à sa première hauteur.
* **La grille d'essai d'un matériau** (`src/calibrate.rs`, `[calibrate]` dans le fichier de réglages) : `phoenix-laser calibrate --material NOM --materials FICHIER` répartit les puissances et les vitesses de la grille autour de ce que dit le matériau de la bibliothèque, dans les limites et sans répéter de valeur, et `--labels` grave la puissance de chaque colonne et la vitesse de chaque ligne à côté des carrés avec une police de traits, de sorte que la grille reste lisible sur le matériau ; un matériau au-dessus d'une limite est refusé, jamais abaissé. Avec un matériau, les chiffres sont gravés par défaut (`--no-labels` les retire) et la puissance et la vitesse minimales de l'échelle sont les clés `min_power_percent` et `min_feed_mm_min`.
* **Texte** (`src/text.rs`, `[text]` dans le fichier de réglages) : `phoenix-laser text OUT.nc --text TEXTE` grave un nom, un numéro de série ou une marque avec une police de traits du programme (majuscules, chiffres, espace et `. , - + / : ! ? ( ) = _ '` ; une minuscule est gravée en majuscule, un saut de ligne commence une rangée) et sans fichier de police. La hauteur d'une majuscule, les écarts entre lettres et rangées, la puissance, la vitesse et les passes sont des réglages ; un caractère que la police n'a pas, un texte de plus de `max_chars` caractères, une hauteur au-dessus de `max_height_mm`, une puissance ou une vitesse au-dessus de la limite, ou un travail qui sortirait du plateau sont refusés, jamais coupés, et le texte n'est jamais écrit dans un commentaire du programme. Les lignes d'un texte de plusieurs lignes peuvent être alignées à gauche, au centre ou à droite (`--align`), le texte peut être gravé en image miroir pour l'envers du verre ou de l'acrylique (`--mirror`), et une gravure qui durerait plus de `max_minutes` est refusée.
* **Code QR** (`src/qr.rs`, `[qr]` dans le fichier de réglages) : `phoenix-laser qr OUT.nc --text DONNÉES` grave des données sous forme d'un code QR propre au programme (versions 1 à 4, de 21 à 33 modules de côté, en mode octet, avec les quatre niveaux de correction d'erreurs de Reed-Solomon et le masque de plus faible pénalité), en remplissant chaque module sombre avec des lignes ; le symbole a été relu avec un décodeur indépendant dans toutes ses versions et tous ses niveaux, et les tests le relisent avec leur propre lecteur. Le côté d'un module, la distance entre les lignes, le niveau, la puissance, la vitesse, les passes et les limites sont des réglages ; des données au-delà de `max_bytes`, une version au-delà de `max_version`, un symbole plus grand que `max_size_mm`, un remplissage de plus de `max_lines` déplacements, une puissance ou une vitesse au-delà de la limite ou un travail hors du plateau sont refusés, jamais rognés ni réduits, et les données ne sont jamais écrites dans un commentaire du programme. `--print` montre le symbole sous forme de texte. Désactivé par défaut : `--invert` grave les modules clairs à la place (avec `--border-modules`, une bordure gravée autour), `--mirror` grave l'image miroir pour l'envers du verre ou de l'acrylique, `--overscan-mm` amène la tête jusqu'à chaque ligne et au-delà, laser éteint (refusé si elle sortait du plateau), et une gravure qui durerait plus de `max_minutes` est refusée.
* **Code-barres** (`src/barcode.rs`, `[barcode]` dans le fichier de réglages) : `phoenix-laser barcode OUT.nc --text DONNÉES` grave des données sous forme d'un code-barres Code 128 propre au programme (les caractères de l'espace à `~` dans le jeu B, et un texte d'un nombre pair de chiffres dans le jeu C, deux fois moins large, avec son symbole de contrôle), en remplissant chaque barre de lignes dans le sens de sa hauteur ; les tests le relisent avec leur propre lecteur, mais il n'a pas été lu avec un lecteur de codes-barres. La largeur de la barre la plus étroite, la hauteur, la distance entre les lignes, la puissance, la vitesse, les passes et les limites sont des réglages ; un octet qui n'est pas un caractère imprimable, plus de `max_chars` caractères, un code-barres plus large que `max_width_mm`, un remplissage de plus de `max_lines` déplacements, une gravure qui durerait plus de `max_minutes`, une puissance ou une vitesse au-delà de la limite ou un travail hors du plateau sont refusés, jamais rognés ni réduits, et les données ne sont jamais écrites dans un commentaire du programme. `--mirror` grave l'image miroir pour l'envers du verre ou de l'acrylique.
* **Hachures croisées** (`src/cut.rs`, `hatch_cross` dans un calque) : un calque `engrave` avec `hatch_cross = on` (ou `--cross` dans `vector` et `trace`) est rempli une seconde fois avec des lignes à angle droit des premières, avec le même espacement, à chaque passe, et la limite de lignes vaut pour chaque jeu ; l'énergie d'un millimètre carré que donne la liste des calques est doublée, et l'aperçu et la vérification du programme le voient comme n'importe quel autre programme.
* **Le cadre du travail** (`src/frame.rs`, `[frame]` dans le fichier de réglages) : `phoenix-laser frame` écrit un programme qui parcourt le rectangle du travail laser éteint, à partir d'une taille ou de la boîte de ce qu'un programme G-code brûle, pour voir où il tombera ; une faible puissance pilote exige `--confirm-laser-on` à chaque fois, est plafonnée à 5 % (`max_pilot_percent` de `[frame]` change le plafond) et obéit à la limite de puissance, et un cadre hors du plateau est refusé.
* **Repères de positionnement** (`src/frame.rs`, `[marks]` dans le fichier de réglages) : `phoenix-laser marks OUT.nc --size W,H` grave à faible puissance un L à chaque coin du rectangle du travail (sur ses bords, ou à `margin_mm` à l'extérieur) et, avec `--centre-cross`, une croix au milieu, pour pouvoir remettre le matériau au même endroit et aligner un second travail sur le premier. Contrairement au cadre, les repères sont gravés : la puissance et la vitesse respectent les limites de la machine, et un bras plus long que la moitié d'un côté du rectangle ou des repères qui sortiraient du plateau sont refusés, jamais rognés. La longueur des bras, la marge, la puissance, la vitesse et les passes sont des réglages.
* **L'aperçu** (`src/preview.rs`) : `phoenix-laser preview` lit un programme G-code (le sien ou un autre), affiche les mouvements, la longueur brûlée et parcourue, la boîte, les puissances et un temps qui compte le freinage dans les coins, avertit de ce qui semble dangereux (laser allumé pendant un déplacement rapide, pas d'avance, puissance au-dessus du plein, travail hors du plateau, pas de fin) et dessine le parcours en SVG ; les tests relisent ce que la gravure écrit et vérifient la même longueur. Avec `--laser-watts W` (la puissance du laser à pleine puissance), elle indique aussi l'énergie que le programme grave, d'après la puissance de chaque déplacement comme part de `--smax` sur la durée de gravure, et sa puissance moyenne.
* **La bibliothèque de matériaux** (`src/materials.rs`, `examples/materials.cfg`) : ce qui convient à chaque matériau et manière de travailler s'écrit une fois dans un fichier texte (`[material NOM]` : puissance, vitesse, passes, pas de mise au point, largeur du faisceau, hachures, taille du point, tramage et courbe de tons) et se choisit avec `--material NOM --materials FICHIER` ; les options de la ligne de commande l'emportent et la limite de puissance vaut sur les deux. `phoenix-laser materials FICHIER` liste la bibliothèque.
* **La limite de puissance** (`src/safety.rs`, `max_power_percent` dans `[machine]`, `--max-power`) : un programme dont la puissance dépasse la limite est refusé avec une erreur qui nomme le calque ou l'option, jamais abaissé sans le dire ; la limite est de 100 % tant qu'un fichier ou une option n'en dit pas moins.
* **Profils de machine** (`[profile NAME]` dans le fichier de réglages, `--profile NAME`) : qui a deux machines, ou change le module laser pour un autre, garde un profil pour chacune (la course du plateau, la limite de puissance, la limite de vitesse, la valeur de pleine puissance et les watts) ; le profil choisi est la machine, et `--max-speed` accompagne `--max-power` : une vitesse au-delà de la limite est refusée, jamais abaissée.
* **Contours à partir d'une image** (`src/trace.rs`, `[trace]` dans le fichier de réglages) : `phoenix-laser trace` suit le contour d'un logo, d'une silhouette ou d'un pochoir au lieu de le brûler point par point ; les niveaux de gris sont séparés par un seuil, le bord est trouvé par marching squares à précision sous-pixel, simplifié jusqu'à ce qu'aucun point ne soit à plus de `simplify_mm` de la ligne, et les taches sont écartées ; le résultat est un calque de dessin, découpé (ou rempli de lignes) avec les mêmes réglages et les mêmes vérifications, et la taille de l'image, les morceaux de bord, les contours et les points sont tous limités.
* **Tons, matériaux et limites dans la fenêtre** (`src/window.rs`) : les cases de la courbe de tons (gamma, contraste, luminosité, inversion et niveaux auto) travaillent l'image avec la même fonction que la ligne de commande ; une bibliothèque de matériaux se charge depuis un fichier (ou celui que nomme `PHOENIX_LASER_MATERIALS`) et choisir un matériau remplit les cases de la puissance, de la vitesse, du point, du tramage, des tons et chaque calque d'un dessin ; les limites de puissance et de vitesse sont aussi des cases et une valeur au-delà est refusée, jamais abaissée. Les valeurs de départ viennent du fichier de réglages (`[tone]`, `[machine]`).
* **Le test de mise au point dans la fenêtre** (`src/window.rs`) : un mode avec les cases de la section `[focus]` (la hauteur de la première ligne, le pas, le nombre et la longueur des lignes, leur espacement, la vitesse et la puissance) ; le test est calculé avec la même fonction que `phoenix-laser focus`, sa puissance et sa vitesse obéissent aux limites, il n'y a pas de pas par défaut car le sens du bas dépend de la machine, et un avertissement que le test déplace Z reste dans les notes.
* **Texte dans la fenêtre** (`src/window.rs`) : un mode avec une case pour les mots et les cases de la section `[text]` (la hauteur d'une majuscule, la lettre la plus haute, les espaces entre lettres et lignes, la puissance, la vitesse, les passes et le nombre maximal de caractères) ; le texte est calculé avec la même fonction que `phoenix-laser text`, depuis l'origine de la machine, si bien qu'une lettre que la police n'a pas, un texte au-delà d'une limite, une puissance ou une vitesse au-delà de la limite de la machine ou un travail hors du plateau (quand le fichier de réglages le donne) est refusé et la ligne d'état dit pourquoi ; il s'écrit sous le nom `text.nc` avec l'en-tête de la ligne de commande. L'alignement des lignes, l'interrupteur miroir et la durée maximale sont aussi dans la fenêtre.
* **Code QR dans la fenêtre** (`src/window.rs`) : un mode avec une case pour les données, le choix du niveau de correction d'erreurs et les cases de la section `[qr]` (le côté du module, la distance entre les lignes, la puissance, la vitesse, les passes, le nombre maximal d'octets, la version et le côté maximaux, et le nombre maximal de déplacements de gravure) ; le symbole est calculé avec la même fonction que `phoenix-laser qr`, depuis l'origine de la machine, si bien que des données qui ne tiennent pas, un symbole au-delà d'une limite, une puissance ou une vitesse au-delà de la limite de la machine ou un travail hors du plateau (quand le fichier de réglages le donne) est refusé et la ligne d'état dit pourquoi ; il s'écrit sous le nom `qr.nc` avec l'en-tête de la ligne de commande. Les interrupteurs inversé et miroir et les cases de la bordure, de l'élan et de la durée maximale sont aussi dans la fenêtre.
* **Le cadre dans la fenêtre** (`src/window.rs`, `[frame]` dans le fichier de réglages) : un mode qui encadre l'image ou le dessin ouvert laser éteint ; la marge, les tours et la vitesse sont des cases. Une puissance pilote supérieure à 0 allume le laser, donc la fenêtre demande et rien n'est calculé ni écrit tant que la personne n'a pas répondu oui, à chaque fois (changer une case, changer de mode ou écrire le fichier retire la réponse), sous le plafond de `max_pilot_percent`, la limite de puissance et le plateau.
* **L'aperçu dans la fenêtre** (`src/window.rs`) : tout G-code que calcule la fenêtre est compté par le même aperçu que `phoenix-laser preview` (déplacements, longueur brûlée et parcourue, un temps qui compte les coins avec l'accélération de la case, et ce qui semble dangereux, comme un travail hors du plateau du fichier de réglages), et un programme de n'importe quelle origine (`.nc`, `.ngc`, `.gcode`, `.gc`, `.tap`) peut être ouvert ou déposé sur la fenêtre pour voir son tracé et ses notes. Il est lu comme du texte, jamais exécuté et jamais réécrit.
* **Le contour dans la fenêtre** (`src/window.rs`, `[trace]` dans le fichier de réglages) : un mode qui suit le contour de l'image ouverte avec la même fonction que `phoenix-laser trace` et le coupe, ou remplit ses formes de lignes, comme une couche appelée `trace` ; les cases sont le seuil, la simplification et la plus petite pièce, avec un interrupteur pour une encre claire ; la puissance et la vitesse sont celles des cases du laser et respectent les limites, et les limites de l'image (`max_pixels`, `max_contours`, `max_segments`) sont celles du fichier de réglages, si bien qu'une image trop détaillée est une erreur et non un programme lent.
* **Le profil de machine dans la fenêtre** (`src/window.rs`, `[profile NAME]` dans le fichier de réglages) : une liste dans le panneau latéral avec les machines du fichier ; celle qui est choisie est la machine, comme `--profile` la fait, de sorte que les cases de la valeur de pleine puissance et des limites de puissance et de vitesse prennent ses valeurs et que le lit du profil est celui que vérifient le cadre et l'aperçu. Ce qu'un profil omet vient de `[machine]`, un profil absent du fichier ne change rien, et les limites refusent toujours une puissance ou une vitesse au-dessus.
* **La grille à partir d'un matériau dans la fenêtre** (`src/window.rs`, `[calibrate]` dans le fichier de réglages) : le mode d'étalonnage liste les matériaux de la bibliothèque ; avec l'un d'eux choisi, la grille est répartie autour de sa puissance et de sa vitesse avec la même échelle que `phoenix-laser calibrate --material` (cases pour les étapes, l'écart et la puissance et la vitesse minimales, tenues entre elles et les limites), et un interrupteur grave les chiffres à côté des carrés avec la hauteur de sa case. Sans choix, c'est la grille des deux listes, comme avant.
* **La vérification du programme dans la fenêtre** (`src/window.rs`, `[verify]` dans le fichier de réglages) : un programme G-code ouvert pour être vu est aussi vérifié par `phoenix-laser verify` avec les limites de la fenêtre et le plateau du fichier de réglages, et le panneau latéral dit s'il réussit (avec le début de son SHA-256) ou ce qui ne va pas ; une case tient le dépassement autorisé hors du plateau laser éteint (0 par défaut, jusqu'à 100 mm), et un programme qui grave hors du plateau échoue quelle que soit la case.
* **La vérification d'un programme** (`src/verify.rs`) : `phoenix-laser verify PROGRAMME.nc` lit un programme G-code (de ce programme ou d'un autre) comme du texte, ne l'exécute jamais, et donne le SHA-256 du fichier exact, ce qu'il fait et un verdict : il ne passe que sous la limite de puissance et de vitesse de la machine, dans le plateau (le travail en dehors échoue ; `--overscan-mm` ou `overscan_mm` dans `[verify]` laisse la tête dépasser le plateau de cette distance, laser éteint, comme le dépassement d'un autre programme, et brûler en dehors du plateau échoue toujours), avec le laser éteint pendant chaque déplacement rapide et une avance sur chaque déplacement qui brûle. `--json` donne le rapport qu'un autre programme peut conserver à côté d'une autorisation liée à ce hash, avec le profil de machine, le SHA-256 du fichier de réglages d'où viennent les limites et les noms des contrôles faits, et le code de sortie est 1 en cas d'échec.
* **Une vérification liée au programme qui a été autorisé** (`src/verify.rs`) : `phoenix-laser verify PROGRAMME.nc --expect-sha256 HASH` fait échouer le programme quand son SHA-256 n'est pas les 64 chiffres hexadécimaux donnés, et dit lequel il a, de sorte qu'un programme modifié après avoir été examiné n'est pas exécuté par erreur ; l'échec figure aussi dans le rapport JSON.
* **Tramage ordonné** (`src/dither.rs`) : `--dither bayer` (aussi dans la fenêtre) décide chaque point selon sa place et son gris avec une matrice de Bayer 8 x 8, donc le motif est régulier et le même gris brûle toujours les mêmes points ; les tests vérifient que chaque niveau de la matrice apparaît une fois et que le noircissement d'un gris est conservé.
* **La courbe de tons** (`src/tone.rs`) : avant de fabriquer les points, les niveaux de gris peuvent être étirés sur toute la plage (`--auto-level`), recevoir plus ou moins de contraste et de luminosité, être courbés par un gamma ou inversés, avec une seule table de 256 entrées ; avec les valeurs par défaut l'image n'est pas modifiée du tout.
* **Un G-code vérifié :** le programme d'un dessin est vérifié ligne par ligne avant d'être livré : aucun laser allumé pendant un déplacement rapide ou sans vitesse, aucun `S` au-dessus de la pleine puissance, aucun déplacement hors du plateau, `M5` à la fin ; le fichier n'est jamais écrit sur le dessin ni sur les réglages dont il provient.

## 🧩 Rôle dans EH3D

EH3D-PHOENIX-LASER est un programme de l'EH3D Fabrication Suite. Il fonctionne comme bibliothèque et ligne de commande et, compilé avec `--features gui`, comme fenêtre avec l'aspect commun aux autres programmes de la suite. Il lit et écrit des fichiers simples et peut donc s'utiliser seul ou entre les autres programmes.

## 🏛️ Architecture

<p align="center">
  <img src="images/architecture.svg" alt="Architecture de EH3D-PHOENIX-LASER : qui l'utilise, ce qu'il contient et de quoi il dépend" width="100%">
</p>


Comment les parties de ce dépôt s'articulent : qui l'utilise, ce qu'il contient et de quoi il dépend. L'image est dessinée depuis `docs/architecture.json` et vérifiée avec le code.

## 🖼️ Captures d'écran

Ces images sont prises par le programme lui-même avec `--shot` (voir Démarrage rapide).

<p align="center">
  <img src="images/screens/window-vector.png" alt="Découpe d'un dessin : le trajet du laser sur la plaque des exemples, avec le tableau des calques dessous" width="100%">
</p>

<p align="center">
  <img src="images/screens/window-engrave.png" alt="Gravure d'une image : les points du dégradé des exemples colorés selon leur puissance" width="100%">
</p>

## ✨ Fonctionnalités

* **Une bibliothèque et une ligne de commande :** chaque commande de `phoenix-laser` est une fonction de la bibliothèque ; un autre programme de la suite peut l'appeler sans lancer de processus.
* **Ligne de commande stricte :** une option mal écrite (`--powr`) ou un mot en trop est une erreur qui la nomme, et non quelque chose d'ignoré ; sinon le programme ferait sa sortie avec les valeurs par défaut.
* **Tests avec entrées abîmées :** les tests fournissent aussi des fichiers vides, du bruit, des nombres énormes et des fichiers coupés, et aucun ne doit paniquer.
* **Sans deviner :** une valeur absurde est une erreur qui dit laquelle et pourquoi, jamais un résultat qui n'en a que l'air.
* **Pas de code unsafe :** `#![forbid(unsafe_code)]` dans la bibliothèque et le programme.

## 🔒 Sécurité et protection

| Menace | Défense |
|---|---|
| Un fichier abîmé, coupé ou vide | pas de panique : les tests coupent une entrée valide à chaque position et fournissent du bruit |
| Un nombre absurde (négatif, non fini, démesuré) | refusé avec une erreur qui nomme la valeur ; rien n'est arrondi en quelque chose de plausible |
| Usage non sûr de la mémoire | Rust sûr et simple avec `forbid(unsafe_code)` |
| Un dessin hostile (scripts, entités déclarées, imbrication profonde, millions de points, nombres énormes) | les scripts, les images et les liens ne sont jamais exécutés ni téléchargés ; une entité déclarée est une erreur ; la taille, la profondeur, les éléments, les formes, les points et les morceaux d'une courbe sont limités ; une forme avec un point hors limites est laissée de côté |
| Un programme qui pourrait allumer le laser sans vitesse ou au-dessus de la pleine puissance | chaque programme est vérifié ligne par ligne avant d'être livré, et celui qui échoue n'est pas écrit |
| Une puissance supérieure à ce que la machine ou le matériau doit recevoir | la limite de puissance (`max_power_percent`, `--max-power`) refuse un programme qui la dépasse avec une erreur qui nomme le calque ou l'option ; elle n'est jamais abaissée sans le dire |
| Une bibliothèque de matériaux cassée ou énorme | le fichier est limité en taille et en nombre de matériaux ; une valeur hors plage, une clé inconnue ou un nom répété est une erreur qui nomme la ligne ; la sortie n'est jamais écrite par-dessus |
| Un programme G-code venu d'ailleurs que l'on lit avant de l'exécuter | l'aperçu ne lit que du texte, avec une limite de taille, n'exécute ni ne télécharge rien, ignore les nombres absurdes et avertit d'un laser allumé pendant un déplacement rapide, d'une avance absente, d'une puissance au-dessus du plein, d'un travail hors du plateau et d'une fin absente |
| Le laser qui s'allume juste pour positionner le travail | le cadre est parcouru laser éteint (aucune puissance, broche jamais allumée) ; une puissance pilote exige `--confirm-laser-on` à chaque exécution, même si le fichier de réglages la demande, reste au plafond ou en dessous (5 % si `max_pilot_percent` n'en dit pas un autre), sous la limite de puissance et est coupée avant la fin du programme |
| Une tête envoyée dans le mauvais sens en Z par un test de mise au point | le pas de hauteur n'a pas de valeur par défaut et le test est refusé tant que la personne ne le donne pas ; le programme dit qu'il déplace Z, la course est limitée (50 mm en tout), la tête revient à la première hauteur et la puissance obéit à la limite de puissance |
| Un texte trop long, trop haut ou trop fort, ou qui graverait hors du plateau | seuls les caractères de la police sont acceptés (une lettre qu'elle n'a pas est refusée, jamais sautée), le nombre de caractères, la hauteur, la puissance et la vitesse restent entre des limites que les réglages peuvent resserrer, un travail hors du plateau est refusé au lieu d'être rogné, et le texte n'est jamais écrit dans un commentaire du programme |
| Des données qui ne tiennent pas dans le code QR, un symbole trop grand, trop dense ou trop fort, ou qui graverait hors du plateau | la version, le nombre d'octets, la taille, le nombre de déplacements de gravure, la puissance et la vitesse restent entre des limites que les réglages peuvent resserrer, et ce qui les dépasse est refusé au lieu d'être rogné ou réduit ; les données ne sont jamais écrites dans un commentaire du programme |
| Le fichier de sortie choisi sur le dessin ou sur les réglages dont il provient | refusé, dans la fenêtre et dans les commandes |

**Ce qui n'est pas défendu :** un résultat produit sans erreur n'est pas un résultat vérifié. Rien ici ne connaît les limites d'une machine et le programme n'a pas eu de revue indépendante. Signalez une vulnérabilité en privé (voir [SECURITY.md](SECURITY.md)).

## 🔌 Interfaces

La bibliothèque est `eh3d_phoenix_laser` (`use eh3d_phoenix_laser::...`) ; la ligne de commande est `phoenix-laser`. Les commandes sont celles-ci ; chacune affiche son résultat sur la sortie standard et signale une erreur sur la sortie d'erreur avec un code de sortie non nul. Il n'y a pas d'accès réseau.

```text
phoenix-laser engrave IMAGE OUT.nc --width-mm 50 [--dot-mm 0.1] [--feed 3000] [--power 50] [--smax 1000]
              [--dither fs|atkinson|threshold|bayer] [--threshold 128] [--overscan MM|auto] [--accel 1000] [--origin X,Y] [--dry-run]
              [--gamma 1] [--contrast 0] [--brightness 0] [--invert] [--auto-level] [--max-power 100] [--max-speed MM/MIN] [--config FILE] [--profile NAME]
              [--material NAME --materials FILE]
phoenix-laser calibrate OUT.nc --powers 20,40,60,80 --feeds 1000,2000,3000 [--smax 1000] [--max-power 100] [--max-speed MM/MIN] [--config FILE] [--profile NAME]
phoenix-laser calibrate OUT.nc --material NAME --materials FILE [--steps 5] [--spread 40] [--labels|--no-labels] [--label-mm 3] [--smax 1000] [--max-power 100] [--max-speed MM/MIN] [--config FILE] [--profile NAME]
phoenix-laser text OUT.nc --text TEXT [--height-mm 10] [--spacing 25] [--line-spacing 50] [--power 20] [--feed 1000] [--passes 1] [--max-minutes 240] [--align left|center|right] [--mirror] [--origin X,Y] [--bed X,Y] [--smax 1000] [--max-power 100] [--max-speed MM/MIN] [--config FILE] [--profile NAME]
phoenix-laser qr OUT.nc --text DATA [--ecc L|M|Q|H] [--module-mm 1] [--line-mm 0.1] [--power 20] [--feed 1000] [--passes 1] [--max-minutes 240] [--invert] [--border-modules 0] [--overscan-mm 0] [--mirror] [--origin X,Y] [--bed X,Y] [--smax 1000] [--max-power 100] [--max-speed MM/MIN] [--print] [--config FILE] [--profile NAME]
phoenix-laser barcode OUT.nc --text DATA [--module-mm 0.4] [--height-mm 10] [--line-mm 0.1] [--power 20] [--feed 1000] [--passes 1] [--max-minutes 240] [--mirror] [--origin X,Y] [--bed X,Y] [--smax 1000] [--max-power 100] [--max-speed MM/MIN] [--config FILE] [--profile NAME]
phoenix-laser marks OUT.nc --size W,H [--origin X,Y] [--arm-mm 5] [--margin-mm 0] [--power 10] [--feed 1000] [--passes 1] [--centre-cross|--no-centre-cross] [--bed X,Y] [--smax 1000] [--max-power 100] [--max-speed MM/MIN] [--config FILE] [--profile NAME]
phoenix-laser info IMAGE
phoenix-laser cut DRAWING OUT.nc [--config FILE] [--power P] [--speed MM/MIN] [--passes N] [--focus-step MM] [--kerf MM] [--kerf-side none|auto|inside|outside]
              [--tabs N] [--tab-width MM] [--smax N] [--overscan MM] [--origin X,Y] [--bed X,Y] [--max-power P] [--max-speed MM/MIN] [--profile NAME] [--material NAME --materials FILE] [--dpi N] [--tolerance MM] [--dry-run]
phoenix-laser vector DRAWING OUT.nc [the options of cut] [--angle DEG] [--spacing MM] [--cross|--no-cross]
phoenix-laser layers DRAWING [--config FILE] [--dpi N]
phoenix-laser trace IMAGE OUT.nc --width-mm W [--threshold 128] [--invert] [--simplify MM] [--min-area MM2] [--engrave] [the options of cut]
phoenix-laser materials FILE
phoenix-laser focus OUT.nc --z-step MM [--z-start MM] [--steps 9] [--length 10] [--spacing 2] [--feed 1000] [--power 30] [--smax 1000] [--max-power P] [--max-speed MM/MIN] [--config FILE] [--profile NAME]
phoenix-laser frame OUT.nc --size W,H | --gcode PROGRAM.nc [--origin X,Y] [--margin MM] [--feed 3000] [--laps 1] [--bed X,Y] [--smax N]
              [--pilot P --confirm-laser-on] [--max-power P] [--max-speed MM/MIN] [--config FILE] [--profile NAME]
phoenix-laser verify PROGRAM.nc [--config FILE] [--profile NAME] [--smax N] [--max-power P] [--max-speed MM/MIN] [--bed X,Y] [--overscan-mm MM] [--accel 1000] [--expect-sha256 HASH] [--json]
phoenix-laser preview PROGRAM.nc [OUT.svg] [--accel 1000] [--rapid 6000] [--smax N] [--laser-watts W] [--bed X,Y] [--width-px 900] [--config FILE] [--profile NAME]
phoenix-laser [gui] [FILE]                       the window (a build with --features gui); with no arguments it opens
```

`DRAWING` est un fichier SVG ou DXF. `cut` suit les contours (ceux de l'intérieur d'abord) ; `vector` remplit les formes fermées de lignes et suit les ouvertes ; les deux prennent les réglages de chaque calque ou couleur dans `--config FILE` (voir [docs/vector-settings.md](docs/vector-settings.md)) et une option de la ligne de commande les change pour tous les calques. `layers` liste les calques d'un dessin avec ce qui serait fait de chacun.

## ⚙️ Configuration

La gravure d'une image se règle avec des arguments de la ligne de commande (voir Interfaces) et chaque valeur par défaut est écrite dans l'aide de la commande (`phoenix-laser help`). La découpe et la gravure d'un dessin se règlent avec un fichier de réglages (`--config FILE`, ou « Charger les réglages » dans la fenêtre) : les limites des lecteurs, la façon de lire un fichier, l'ordre des découpes, la machine et, pour chaque calque ou couleur, le mode (découper, graver ou ignorer), la puissance, la vitesse, les passes, le pas de la mise au point, la largeur du faisceau et son côté, les ponts et les hachures. Chaque clé, sa valeur par défaut et sa plage sont dans [docs/vector-settings.md](docs/vector-settings.md) et un exemple est dans `examples/plate.cfg` ; une clé inconnue ou une valeur hors plage est une erreur qui nomme sa ligne. Les limites fixes du lecteur d'images sont des constantes nommées dans le code, couvertes par des tests.

## 🚀 Démarrage rapide

Il faut une chaîne d'outils Rust stable (1.88 ou plus récente). Le dépôt EH3D-COMMON doit être cloné à côté de celui-ci : le lecteur d'images en est pris par chemin.

```bash
cargo build --release
cargo test
cargo run --release -- engrave examples/gradient.pgm examples/gradient.nc --width-mm 16 --dot-mm 0.5 --overscan auto
cargo run --release -- cut examples/plate.svg examples/plate.nc --config examples/plate.cfg
cargo run --release --features gui   # the window
cargo run --release --features gui -- --shot screen.png examples/plate.svg   # a picture of the window
```

Pour prendre une capture de la fenêtre sans l'ouvrir à la main, lancez-la avec `--shot FICHIER.png` (aussi `--size WxH`, `--theme forge|dark|light`, `--wait N`, `--lang CODE`, `--open FILE`, `--replace`) : elle s'ouvre à cette taille, enregistre le PNG, ne garde aucun réglage et se ferme toute seule (code de sortie 0 si le fichier a été écrit, 1 sinon, 2 si les arguments sont erronés).

## 🧪 Tests

La suite compte 591 tests (`cargo test`, exécutés sous Windows et Linux dans le CI). Ils vérifient les cas normaux, les cas limites et les entrées abîmées : fichiers vides, bruit et fichiers coupés ne font jamais paniquer. Le CI exécute aussi `cargo fmt --check`, `cargo clippy` avec les avertissements comme erreurs et `python -I tools/ci_validate.py`.

## 🏗️ Compilation et publication

La version se trouve dans `Cargo.toml` et `eh3d.project.json` ; chaque version a son entrée dans [CHANGELOG.md](CHANGELOG.md). `build.bat` / `build.sh` compilent le programme, `test.bat` / `test.sh` lancent les tests et `run.bat` / `run.sh` l'exécutent avec un exemple. Les versions sont publiées sur GitHub.

**Téléchargements :** Avec chaque version de ce dépôt sont publiés des programmes prêts à l'emploi pour chaque système : Windows (64 et 32 bits), Linux (64 bits) et macOS (Apple silicon et Intel). Chaque paquet, `phoenix-laser-<version>-<système>.zip` ou `.tar.gz`, contient le programme, la licence, ce README, le journal des modifications, l'icône et les fichiers d'exemple, et vient avec son fichier `.sha256` ; la version liste aussi tous les fichiers dans `SHA256SUMS` et `build-manifest.json`. Aucune version n'a encore été publiée.

**Signés et vérifiables :** les paquets d'une version sont signés avec la clé de publication d'EH3D, et le fichier `.sig` de chaque paquet est publié à côté (une signature OpenSSH, espace de noms `eh3d-release`). Vérifiez un téléchargement avant de l'exécuter : comparez son SHA-256 (`sha256sum -c phoenix-laser-<version>-<système>.tar.gz.sha256`, ou `Get-FileHash` sous Windows) et vérifiez la signature avec `ssh-keygen -Y verify -f allowed_signers -I eh3d-release -n eh3d-release -s <paquet>.sig < <paquet>`, où `allowed_signers` contient la ligne `eh3d-release <type de clé> <clé publique des versions d'EH3D>`.

## 📂 Structure du dépôt

<p align="center">
  <img src="images/repository-structure.svg" alt="Structure du dépôt EH3D-PHOENIX-LASER" width="100%">
</p>

<details>
<summary>Version texte de l'arborescence</summary>

```text
EH3D-PHOENIX-LASER/
├── src/
│   ├── lib.rs                                   la bibliothèque : ce que chaque commande utilise
│   ├── main.rs                                  la ligne de commande : engrave, calibrate, info, cut, vector, trace, verify et layers
│   ├── calibrate.rs                             une partie de la logique
│   ├── cli_args.rs                              les arguments de la ligne de commande, le même fichier dans chaque petit utilitaire
│   ├── dither.rs                                une partie de la logique
│   ├── materials.rs                             la bibliothèque de matériaux : puissance, vitesse, passes, tons
│   ├── preview.rs                               l'aperçu d'un G-code : mouvements, temps, dangers et un SVG du parcours
│   ├── tone.rs                                  la courbe de tons d'une image : niveaux, contraste, luminosité, gamma et inversion avant les points
│   ├── trace.rs                                 les contours d'une image : seuil, marching squares et anneaux simplifiés, avec limites
│   ├── text.rs                                  une ligne de texte gravée avec une police de traits propre, dans des limites
│   ├── qr.rs                                    un code QR propre, versions 1 à 4 avec correction d'erreurs, gravé en remplissant ses modules
│   ├── barcode.rs                               un code-barres Code 128 propre, gravé en remplissant ses barres de lignes
│   ├── gcode_out.rs                             la seule façon d'écrire un nombre de G-code et l'en-tête standard
│   ├── raster.rs                                une partie de la logique
│   ├── safety.rs                                la limite de puissance et les règles qui gardent le laser éteint pendant qu'on montre le travail
│   ├── shot.rs                                  le mode capture de la fenêtre (--shot) : ses arguments, lus et vérifiés sans écran
│   ├── vector.rs                                un dessin lu : formes en mm, calques, limites et géométrie de base
│   ├── verify.rs                                la vérification d'un programme : son SHA-256, ce qu'il fait et un verdict face aux limites de la machine
│   ├── flatten.rs                               courbes en segments droits dans une tolérance (Bézier, arcs)
│   ├── frame.rs                                 le cadre du travail (le rectangle parcouru laser éteint) et les repères de ses coins
│   ├── svg.rs                                   le lecteur SVG : un sous-ensemble sûr, sans scripts, images ni entités
│   ├── dxf.rs                                   le lecteur DXF : lignes, polylignes, arcs, cercles et splines, avec unités
│   ├── energy.rs                                l'énergie d'une passe par millimètre ou millimètre carré, d'après la puissance optique
│   ├── offset.rs                                décaler un contour de la largeur du faisceau, vérifié arête par arête
│   ├── order.rs                                 l'ordre des découpes : lignes jointes, l'intérieur d'abord, peu de déplacement
│   ├── cut.rs                                   du dessin au G-code : découpe, ponts, hachures, passes et vérification de sécurité
│   ├── vconfig.rs                               le fichier de réglages du travail vectoriel : limites, machine et chaque calque
│   ├── browse.rs                                la liste des fichiers de la fenêtre d'ouverture d'un fichier
│   ├── gui.rs                                   la fenêtre (egui, avec la fonctionnalité gui) : dessine l'état et transmet ce que fait la personne
│   ├── lang.rs                                  les mots de la fenêtre dans les huit langues
│   ├── viewport.rs                              zoom et déplacement de l'image, en simples nombres
│   └── window.rs                                l'état de la fenêtre sans écran : image ou dessin, réglages, calques, passes et export
├── tests/
│   ├── fuzz.rs                                  tests à graine fixe : fichiers abîmés, coupures et nombres étranges
│   ├── dry_run.rs                               tests de --dry-run : compte la même chose et n'écrit rien
│   ├── laser.rs                                 tests de la bibliothèque et de la ligne de commande
│   ├── laser_tools.rs                           tests des outils autour de la gravure, par la ligne de commande
│   ├── readme.rs                                tests qui exécutent ce que promettent le README et les scripts run
│   ├── strict.rs                                tests de la ligne de commande : options et mots qu'elle ne connaît pas
│   ├── vector.rs                                tests de la découpe depuis des dessins : commandes, exemples, fichiers hostiles et vérification du G-code
│   └── window_cli.rs                            tests qu'une compilation sans la fonctionnalité gui reste une ligne de commande
├── examples/
│   ├── gradient.pgm                             un exemple pour essayer le programme
│   ├── plate.svg                                une plaque avec un trou et un panneau à graver (SVG)
│   ├── plate.dxf                                la même plaque en DXF
│   ├── plate.cfg                                les réglages de ses calques : puissance, vitesse, passes, largeur du faisceau, ponts et hachures
│   └── materials.cfg                            une bibliothèque de matériaux : bouleau et ardoise, découpe, gravure et photographies
├── docs/
│   ├── architecture.json                        données de l'image d'Architecture
│   ├── structure.json                           données de l'image de Structure
│   ├── vector-settings.md                       le fichier de réglages du travail vectoriel, clé par clé
│   └── materials.md                             la bibliothèque de matériaux, clé par clé
├── tools/                                       contrôle de la CI et du README et outils d'images
├── images/                                      bannière, images du README et captures de la fenêtre
├── .github/                                     workflows de CI et de publication, modèles d'issues et de pull requests
├── eh3d.project.json                            manifeste : version, rôle, maturité et commande de compilation
├── Cargo.toml Cargo.lock                        le workspace et ses dépendances verrouillées
├── rust-toolchain.toml                          la toolchain Rust épinglée
├── CHANGELOG.md                                 ce qui a changé dans chaque version
├── SECURITY.md SUPPORT.md                       signaler une vulnérabilité et poser des questions
└── CONTRIBUTING.md CODE_OF_CONDUCT.md LICENSE   contribuer, code de conduite et licence
```

</details>

## 🗺️ Feuille de route et limites connues

**Limites connues aujourd'hui**

* La fenêtre n'a pas encore été vue sur un écran.
* Il n'a pas été essayé sur une machine ni avec une collection de fichiers réels et n'a pas eu de revue indépendante.
* Un dessin est lu dans un sous-ensemble sûr (pas de `use` d'un autre fichier, de texte, de dégradés ni de styles à sélecteurs ; pas d'ellipses, de hachures ni de blocs dans un DXF) ; un contour plus étroit que le faisceau n'est pas découpé et le programme le dit ; le laser est celui d'un contrôleur GRBL avec `M4`, et rien n'a été essayé sur un vrai laser ni un vrai matériau.

**Ensuite**

* Montrer la densité de puissance sur la carte avant de brûler (`layers` affiche déjà l'énergie d'une passe quand `laser_watts` est défini) et garder la puissance constante dans les angles d'une découpe.
* Voir la fenêtre sur un écran et la terminer.
* L'essayer avec de vrais dessins et avec les autres programmes de la suite.

## 🌍 Langues

Le README et les textes existent en anglais, espagnol, français, italien, allemand, chinois, japonais et portugais européen.

## 🔗 Projets liés

* [EH3D-COMMON](https://github.com/JuanenRac/EH3D-COMMON) : les crates partagés, dont le lecteur d'images
* [EH3D-STENCILCAM](https://github.com/JuanenRac/EH3D-STENCILCAM): les pochoirs de pâte à braser
* [EH3D-CAM-DEBUGGER](https://github.com/JuanenRac/EH3D-CAM-DEBUGGER): la vérification du G-code avant qu'il n'atteigne une machine
* [EH3D-TOOL-MANAGER](https://github.com/JuanenRac/EH3D-TOOL-MANAGER): le magasin d'outils partagé par les programmes CAM
* [EH3D-MOTIONCNC](https://github.com/JuanenRac/EH3D-MOTIONCNC): le programme de commande de machine
* [EH3D-SUITE-LAUNCHER](https://github.com/JuanenRac/EH3D-SUITE-LAUNCHER): le centre qui trouve et lance les programmes de la suite

## 📚 Documentation

* [Journal des modifications](CHANGELOG.md)
* [Données d'architecture et de structure](docs/architecture.json) : les données qui servent à dessiner les deux images.
* [Réglages du travail vectoriel](docs/vector-settings.md) : chaque clé du fichier de réglages avec sa valeur par défaut et sa plage.
* [La bibliothèque de matériaux](docs/materials.md) : chaque clé d'un matériau et la façon de l'appliquer

## 🤝 Contribuer et communauté

* Avant une pull request, lancez `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` et `python -I tools/ci_validate.py` : tout doit passer. Voir [CONTRIBUTING.md](CONTRIBUTING.md) et le [code de conduite](CODE_OF_CONDUCT.md).
* Le README existe en huit langues : un changement de texte se fait dans les huit fichiers, et les deux images sont dessinées depuis `docs/architecture.json` et `docs/structure.json` par `tools/make_architecture.py` et `tools/make_structure.py`.
* Signalez une vulnérabilité en privé : [SECURITY.md](SECURITY.md). Questions, idées et rapports : electrohobby3d@gmail.com ([SUPPORT.md](SUPPORT.md)).

## 👤 AUTEUR

**JuanenRac (Electro Hobby 3D)** · electrohobby3d@gmail.com

## 📜 LICENCE

Logiciel : GPL-3.0-or-later. Voir [LICENSE](LICENSE). Les conceptions matérielles de la suite utilisent la CERN OHL-S.

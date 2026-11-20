<p align="center">
  <img src="images/EH3D_PHOENIX_LASER_BANNER.svg" alt="EH3D-PHOENIX-LASER banner" width="100%">
</p>

# <img src="images/brand/icon-64.png" alt="" width="40" height="40" align="absmiddle"> EH3D-PHOENIX-LASER
<p align="center">🇺🇸 <b>English</b> | <a href="README_spa.md">🇪🇸 Español</a> | <a href="README_fra.md">🇫🇷 Français</a> | <a href="README_ita.md">🇮🇹 Italiano</a> | <a href="README_deu.md">🇩🇪 Deutsch</a> | <a href="README_zho.md">🇨🇳 中文</a> | <a href="README_jpn.md">🇯🇵 日本語</a> | <a href="README_por.md">🇵🇹 Português</a></p>

### Laser engraving from an image and cutting from a drawing: dithering, raster G-code, SVG and DXF contours and a calibration grid

<p align="center">
  <img src="https://img.shields.io/badge/License-GPL%203.0--or--later-blue.svg" alt="GPL 3.0 or later">
  <img src="https://img.shields.io/badge/Stack-Rust-DEA584.svg" alt="Rust">
  <img src="https://img.shields.io/badge/Languages-8-00E5FF.svg" alt="Languages">
  <img src="https://img.shields.io/badge/Version-0.1.3-0f6cbd.svg" alt="Version">
  <img src="https://img.shields.io/badge/Maturity-functional-4CAF50.svg" alt="Maturity">
</p>

---

**Honesty check - what actually runs today:** a Rust library and a command line program (`phoenix-laser`) with 671 tests that run on a computer: turns an image into dots by dithering and writes the G-code of a laser that burns line by line with dynamic power and overscan, reads SVG and DXF drawings and writes the G-code that cuts their contours (the inside ones first, the width of the beam taken into account, bridges, several passes) and engraves their shapes with lines, with the settings of each layer in a file, and a power-against-feed test grid. It has a window (built with `--features gui`) that opens a picture or a drawing, shows the path of the laser, lets the settings be changed and exports the G-code, but the window has not been looked at on a screen yet; it has not been tried on a machine and it has not been checked against a collection of real files. It can also send a program to the laser plugin of the EH3D server and follow, hold or stop the job (`phoenix-laser server`), tested against a stand-in of the server only.

---

> **Safety and security status:** this program does not control any machine and is not a safety system. What it produces must be checked by a person before anything is run on a machine; nothing here has run on real hardware.


## 📋 At a glance

| | |
|---|---|
| Version | `0.1.3` |
| Maturity | Functional |
| Tests | 671 |
| Stack | Rust |
| On real hardware | Not tried: nothing here has run on a machine |

**Jump to:** [Overview](#-overview) · [Architecture](#-architecture) · [Quick start](#-quick-start) · [Testing](#-testing) · [Structure](#-repository-structure) · [Related](#-related-projects)

## 🚦 Project status

Each line is one part of this repository and the light says how far it is today. It is written from what the repository can show (tests, builds, runs), not from what is planned.

🟢 Done and verified · 🟡 Works, with known limits · 🟠 Partly done or not verified yet · 🔴 Not done or not working

| Area | State | What it means here |
|---|---|---|
| Core | 🟡 | turns an image into dots by dithering and writes the G-code of a laser that burns line by line with dynamic power and overscan, reads SVG and DXF drawings and cuts or engraves them by layer (ordered contours, beam width, bridges, passes, hatch), and a power-against-feed test grid; tested on a computer with normal and damaged input, not with a collection of real files. |
| Command line | 🟡 | `phoenix-laser` (`engrave`, `calibrate`, `info`, `cut`, `vector`, `layers`) runs the same functions as the library and prints the result; every error comes back as a message and a non-zero exit code. |
| Tests | 🟡 | 671 tests pass (`cargo test`), with normal cases, edge cases and damaged input. |
| Security | 🟡 | Plain safe Rust (`forbid(unsafe_code)`): nothing panics on the files it is given and a bad value is an error with its reason; no independent review. |
| Window | 🟠 | Written (egui, `--features gui`): open or drop a picture or an SVG or DXF drawing, see the path of the laser (the cut in one colour, the travel dim), edit the table of layers, a calibration grid, zoom and pan, estimate the time, export; its logic is tested, but it has not been built or looked at on a screen yet. |
| Real hardware | 🔴 | Not tried: nothing here has been run on a machine. |
| Documentation | 🟡 | README in eight languages, the changelog and the two pictures drawn from `docs/`. |

## 🎯 Overview

* **The image** (`eh3d-image`): PNG (grey, colour, transparent, 8 or 16 bits) and PGM (text and binary) become grey levels, with transparent parts as white and the size limited so that a hostile file cannot ask for gigabytes; `resize` averages the pixels so that one dot is as big as the beam.
* **The dots** (`src/dither.rs`): a threshold, Floyd-Steinberg or Atkinson error diffusion; the tests check that the darkness of a grey is kept (a mid grey burns about half of the dots) and that white burns nothing and black burns everything.
* **The laser** (`src/raster.rs`, `src/calibrate.rs`): one line after another, left to right and then right to left, `M4` with `S0` while it travels and the chosen power where it burns, an overscan fixed or worked out from the acceleration (v squared over 2a), and a grid of squares with a power for each column and a feed for each row to find what suits a material.
* **The header and the numbers of every G-code file** (`src/gcode_out.rs`): the file opens with comment lines that say the generator and its version, the units (mm, G21), the picture it was made from with its SHA-256, and the tools (the laser and its power); a calibration grid says it has no source. A number is written one way only (at most three decimals, no `-0`) and a value that is not finite is refused instead of being written as `NaN` or `inf`.
* **`--dry-run`:** `engrave` works the result out and counts it (dots, lines, runs, burnt length, time) without writing the output file, which may then be left out.
* **The drawings** (`src/svg.rs`, `src/dxf.rs`, `src/flatten.rs`, `src/vector.rs`): a safe subset of SVG (paths with all their commands, shapes, transforms, groups, units, and a `use` of an element of the same file) and the entities of a DXF (lines, polylines with arcs, arcs, circles, splines, units) become shapes in millimetres; scripts, images, a `use` of another file, declared entities and anything unknown are skipped or refused, the size, the depth, the number of elements (what a `use` brings in counts) and of points are limited by the settings, a `use` that leads back to itself is an error, and curves become straight pieces within a tolerance.
* **Cutting and engraving** (`src/order.rs`, `src/offset.rs`, `src/cut.rs`): the shapes are grouped by layer (DXF) or colour (SVG); each group is cut (loose lines joined, the contours inside another one first, the shortest travel, the width of the beam taken off on the right side, bridges left uncut, several passes with the focus moving on) or engraved (closed shapes filled with lines at an angle, by the even-odd rule) with its own settings.
* **The settings file** (`src/vconfig.rs`, `docs/vector-settings.md`): every value that is tuned (the limits of the readers, the machine, the settings of each layer) is in a plain text file that is checked line by line; none of it is in the code.
* **The focus test** (`src/calibrate.rs`, `[focus]` in the settings file): `phoenix-laser focus` burns a ladder of short lines, each with the head at another height, so the sharpest line shows where the beam is in focus; the step of the height has no default (which way is down depends on the machine), the power obeys the limit of power and the head returns to its first height. With `--powers 10,20,30` (up to 10 powers) it burns a focus grid instead: the same lines once for each power, side by side with a column for each, so the power that burns the thinnest line at the height of the focus can be read off; the head goes to each height once and burns every column of that row, and each power obeys the limit of power.
* **The test grid of a material** (`src/calibrate.rs`, `[calibrate]` in the settings file): `phoenix-laser calibrate --material NAME --materials FILE` spreads the powers and the feeds of the grid around what the material of the library says, held between the limits and given once each, and `--labels` burns the power of each column and the feed of each row next to the squares with a font of strokes, so the grid can be read on the material afterwards; a material over a limit is refused, never lowered. With a material the figures come by default (`--no-labels` takes them away) and the lowest power and feed of the ladder are the keys `min_power_percent` and `min_feed_mm_min`.
* **Text** (`src/text.rs`, `[text]` in the settings file): `phoenix-laser text OUT.nc --text TEXT` burns a name, a serial number or a mark with a font of strokes of the program (capitals, figures, space and `. , - + / : ! ? ( ) = _ '`; lower case is burnt as capitals, a new line starts a row) and no font file. The height of a capital, the gaps between letters and rows, the power, the speed and the passes are settings; a character the font has not, a text of more than `max_chars` characters, a height over `max_height_mm`, a power or a speed over the limit, or work that would go outside of the bed is refused, never cut down, and the text is never written into a comment of the program. The rows of a text of several lines can be aligned at the left, in the middle or at the right (`--align`), the text can be burnt as its mirror image for the back of glass or acrylic (`--mirror`), and a burning that would take more than `max_minutes` is refused.
* **QR code** (`src/qr.rs`, `[qr]` in the settings file): `phoenix-laser qr OUT.nc --text DATA` burns some data as a QR code of the program's own (versions 1 to 4, from 21 to 33 modules a side, in byte mode, with the four levels of error correction by Reed-Solomon and the mask of the lowest penalty), filling each dark module with lines; the symbol was read back with an independent decoder in all its versions and levels, and the tests read it back with their own reader. The side of a module, the distance between the lines, the level, the power, the speed, the passes and the limits are settings; data over `max_bytes`, a version over `max_version`, a symbol larger than `max_size_mm`, a fill of more than `max_lines` moves, a power or a speed over the limit or work outside of the bed is refused, never cut down or shrunk, and the data is never written into a comment of the program. `--print` shows the symbol as text. Off by default: `--invert` burns the light modules instead (with `--border-modules` a burnt border around it), `--mirror` burns the mirror image for the back of glass or acrylic, `--overscan-mm` runs the head up to each line and out past it with the laser off (refused when it would leave the bed), and a burning that would take more than `max_minutes` is refused.
* **Barcode** (`src/barcode.rs`, `[barcode]` in the settings file): `phoenix-laser barcode OUT.nc --text DATA` burns some data as a Code 128 barcode of the program's own (the characters from space to `~` in set B, and a text of an even number of figures in set C, half as wide, with its check symbol), filling each bar with lines along its height; the tests read it back with their own reader, but it has not been read with a scanner. The width of the narrowest bar, the height, the distance between the lines, the power, the speed, the passes and the limits are settings; a byte that is not a printable character, more than `max_chars` characters, a barcode wider than `max_width_mm`, a fill of more than `max_lines` moves, a burning that would take more than `max_minutes`, a power or a speed over the limit or work outside of the bed is refused, never cut down or shrunk, and the data is never written into a comment of the program. `--mirror` burns the mirror image for the back of glass or acrylic.
* **Cross hatch** (`src/cut.rs`, `hatch_cross` in a layer): an `engrave` layer with `hatch_cross = on` (or `--cross` in `vector` and `trace`) is filled a second time with lines at right angles to the first ones, with the same spacing, in every pass, and the limit of lines holds over each set; the energy of a square millimetre that the list of layers gives is doubled, and the preview and the check of the program see it as any other program.
* **The frame of the work** (`src/frame.rs`, `[frame]` in the settings file): `phoenix-laser frame` writes a program that traces the rectangle of the work with the laser off, from a size or from the box of what a G-code program burns, so the person sees where it will land; a weak pilot power needs `--confirm-laser-on` every time, is capped at 5 % (`max_pilot_percent` of `[frame]` changes the cap) and obeys the limit of power, and a frame outside the bed is refused.
* **Positioning marks** (`src/frame.rs`, `[marks]` in the settings file): `phoenix-laser marks OUT.nc --size W,H` burns an L at each corner of the rectangle of the work (on its edges, or `margin_mm` outside of them) and, with `--centre-cross`, a cross in the middle, at a low power, so that the material can be put back in the same place and a second job lined up with the first. Unlike the frame, the marks are burnt: the power and the speed obey the limits of the machine, and an arm longer than half a side of the rectangle or marks that would go outside of the bed are refused, never cut down. The length of the arms, the margin, the power, the speed and the passes are settings.
* **The preview** (`src/preview.rs`): `phoenix-laser preview` reads a G-code program (its own or another's), prints the moves, the burnt and travelled length, the box, the powers and a time that counts the braking at the corners, warns of what looks dangerous (laser on during a rapid move, no feed, power above full, work outside the bed, no end) and draws the path as an SVG; the tests read back what the engraving writes and check the same length. With `--laser-watts W` (the power of the laser at full power) it also tells the energy that the program burns, from the power of each move as a part of `--smax` over the time it burns, and its mean power.
* **The library of materials** (`src/materials.rs`, `examples/materials.cfg`): what suits each material and way of working is written once in a plain text file (`[material NAME]`: power, speed, passes, focus step, beam width, hatch, dot size, dithering and tone curve) and chosen with `--material NAME --materials FILE`; the options of the command line win over it and the limit of power holds over both. `phoenix-laser materials FILE` lists the library.
* **The limit of power** (`src/safety.rs`, `max_power_percent` in `[machine]`, `--max-power`): a program whose power is over the limit is refused with an error that names the layer or the option, never lowered without a word; the limit is 100 % until a file or an option says less.
* **Machine profiles** (`[profile NAME]` in the settings file, `--profile NAME`): a person with two machines, or a laser module that is changed for another, keeps one profile for each (the travel of the bed, the limit of power, the limit of speed, the value of full power and the watts); the chosen profile is the machine, and `--max-speed` joins `--max-power`: a speed over the limit is refused, never lowered.
* **Contours from a picture** (`src/trace.rs`, `[trace]` in the settings file): `phoenix-laser trace` follows the outline of a logo, a silhouette or a stencil instead of burning it dot by dot; the grey levels are split at a threshold, the border is found with sub-pixel marching squares, simplified until no point is farther than `simplify_mm` from the line, and the specks are dropped; the result is a layer of a drawing, cut (or filled with lines) with the same settings and the same checks, and the size of the picture, the pieces of border, the contours and the points are all limited.
* **Tone, materials and limits in the window** (`src/window.rs`): the boxes of the tone curve (gamma, contrast, brightness, invert and auto level) work the picture out with the same function as the command line; a library of materials is loaded from a file (or named by `PHOENIX_LASER_MATERIALS`), and choosing a material fills the boxes of the power, the speed, the dot, the dithering, the tone and every layer of a drawing; the limits of power and of speed are boxes too and a value over them is refused, never lowered. The starting values come from the settings file (`[tone]`, `[machine]`).
* **Materials edited in the window** (`src/window.rs`, `src/materials.rs`): the material section can edit, copy, make from the boxes (power, speed, dot, overscan, dithering and tone curve) and delete a material, and create a new library; a box for each key of the file takes the values, and saving rewrites the section of that material in its place (or adds it at the end) without touching the comments, the blank lines, the order, the line ends or the other materials. A value out of its range, a name that is empty, too long or already used, a file that is not UTF-8 or not a valid library is refused with the key named and the file untouched, never cut down; deleting needs a second press, and a new library is never written over a file that is there.
* **The focus test in the window** (`src/window.rs`): a mode with the boxes of the `[focus]` section (the height of the first line, the step, the number and length of the lines, their spacing, the speed and the power); the test is worked out with the same function as `phoenix-laser focus`, its power and speed obey the limits, there is no default step because which way is down depends on the machine, and a warning that the test moves Z stays in the notes. A box for the powers of the grid (up to 10, separated by commas) makes the window burn the focus grid of `focus --powers`: the lines once for each power, a column for each; the box of the single power is not used then, and a power that is not more than 0 up to 100, over the limit or too many is refused.
* **Text in the window** (`src/window.rs`): a mode with a box for the words and the boxes of the `[text]` section (the height of a capital, the tallest letter, the gaps between letters and rows, the power, the speed, the passes and the most characters); the text is worked out with the same function as `phoenix-laser text`, from the origin of the machine, so a letter that the font has not, a text over a limit, a power or a speed over the limit of the machine or work outside of the bed (when the settings file gives it) is refused and the status line says why; it is written as `text.nc` with the header of the command line. The alignment of the rows, the mirror switch and the longest time are in the window too.
* **QR code in the window** (`src/window.rs`): a mode with a box for the data, the choice of the level of error correction and the boxes of the `[qr]` section (the side of a module, the distance between the lines, the power, the speed, the passes, the most bytes, the largest version and side, and the most burning moves); the symbol is worked out with the same function as `phoenix-laser qr`, from the origin of the machine, so data that does not fit, a symbol over a limit, a power or a speed over the limit of the machine or work outside of the bed (when the settings file gives it) is refused and the status line says why; it is written as `qr.nc` with the header of the command line. The switches for the inverted and the mirror image and the boxes of the border, the run-up and the longest time are in the window too.
* **Positioning marks in the window** (`src/window.rs`, `[marks]` in the settings file): a mode with boxes for the size of the rectangle, the arm of each L, the margin, the power, the speed and the passes and a switch for the cross in the middle (a settings file fills the boxes of `[marks]`); the marks are worked out with the same function as `phoenix-laser marks` from the origin of the machine, and an arm longer than half a side, a power or a speed over the limit or marks outside of the bed are refused, with the box or the reason named in the status line. It is written as `marks.nc` with the usual header.
* **Code 128 barcode in the window** (`src/window.rs`, `[barcode]` in the settings file): a mode with a box for the data, a switch for the mirror image and the boxes of the `[barcode]` section (the width of the narrowest bar, the height, the distance between the lines, the power, the speed, the passes and the limits; a settings file fills them); the barcode is worked out with the same function as `phoenix-laser barcode` from the origin of the machine, and data that is not printable text or is too long, a barcode over a limit, a power or a speed over the limit of the machine or work outside of the bed is refused, with the box or the reason named in the status line. It is written as `barcode.nc` with the usual header.
* **The frame in the window** (`src/window.rs`, `[frame]` in the settings file): a mode that frames the picture or the drawing that is open with the laser off; the margin, the laps and the speed are boxes. A pilot power above 0 turns the laser on, so the window asks and nothing is worked out or written until the person answers yes, every time (changing a box, changing the mode or writing the file takes the answer back), under the cap of `max_pilot_percent`, the limit of power and the bed.
* **The preview in the window** (`src/window.rs`): every G-code the window works out is counted by the same preview as `phoenix-laser preview` (moves, burnt and travelled length, a time that counts the corners with the acceleration of the box, and what looks dangerous, such as work outside the bed of the settings file), and a program of any origin (`.nc`, `.ngc`, `.gcode`, `.gc`, `.tap`) can be opened or dropped on the window to see its path and its notes. It is read as text, never run, and never written again. The power of the laser can be typed in a box (the settings file and a machine profile fill it from `laser_watts`; 0 means it is not known): the preview then also tells the energy that the program burns and its mean power, as `phoenix-laser preview --laser-watts` does, and a value that is not a number from 0 to 1000 is refused.
* **The outline in the window** (`src/window.rs`, `[trace]` in the settings file): a mode that follows the outline of the picture that is open with the same function as `phoenix-laser trace` and cuts it, or fills its shapes with lines, as a layer called `trace`; the boxes are the threshold, the simplification and the smallest piece, with a switch for a light ink; the power and the speed are the ones of the laser boxes and obey the limits, and the limits of the picture (`max_pixels`, `max_contours`, `max_segments`) are the ones of the settings file, so a picture that is too detailed is an error and not a slow program.
* **The machine profile in the window** (`src/window.rs`, `[profile NAME]` in the settings file): a list in the side panel with the machines of the file; the one that is picked is the machine, as `--profile` makes it, so the boxes of the full-power value and of the limits of power and of speed take its values and the bed of the profile is the one the frame and the preview check. What a profile leaves out comes from `[machine]`, a profile that is not in the file changes nothing, and the limits still refuse a power or a speed over them.
* **The grid from a material in the window** (`src/window.rs`, `[calibrate]` in the settings file): the calibration mode lists the materials of the library; with one picked, the grid is spread around its power and its speed with the same ladder as `phoenix-laser calibrate --material` (boxes for the steps, the spread and the lowest power and feed, held between them and the limits), and a switch burns the figures next to the squares with the height of its box. With none picked it is the grid of the two lists, as before.
* **The check of the program in the window** (`src/window.rs`, `[verify]` in the settings file): a G-code program that is opened to be seen is also checked by `phoenix-laser verify` with the limits of the window and the bed of the settings file, and the side panel says whether it passes (with the start of its SHA-256) or what is wrong; a box holds the overscan that is allowed past the bed with the laser off (0 by default, up to 100 mm), and a program that burns outside of the bed fails whatever the box says.
* **The check of a program** (`src/verify.rs`): `phoenix-laser verify PROGRAM.nc` reads a G-code program (this program's or another's) as text, never runs it, and gives the SHA-256 of the exact file, what it does and a verdict: it passes only under the limit of power and of speed of the machine, inside the bed (work outside it fails; `--overscan-mm` or `overscan_mm` in `[verify]` lets the head go that far past the bed with the laser off, as the overscan of another program does, and burning outside the bed always fails), with the laser off during every rapid move and a feed on every burning move. `--json` gives the report for another program to keep next to an authorisation bound to that hash, with the machine profile, the SHA-256 of the settings file the limits came from and the names of the checks that were made, and the exit code is 1 when it fails.
* **A check that is tied to the program that was authorised** (`src/verify.rs`): `phoenix-laser verify PROGRAM.nc --expect-sha256 HASH` fails the program when its SHA-256 is not the 64 hex digits that were given, and says which one it has, so a program that was changed after it was looked at is not run by mistake; the failure is in the JSON report too.
* **Sending to the EH3D server** (`src/server.rs`, `[server]` in the settings file): `phoenix-laser server send PROGRAM.nc` sends a G-code program as a job of the `laser` plugin of EH3D-SERVER, through the shared client of the suite (`eh3d-serverclient` of EH3D-COMMON: the keys `server_address`, `server_token`, `server_machine`, `server_timeout_s`, `server_retries` and `server_mdns`, retries with growing pauses, a request that makes something never sent twice, the refusals of the server told by their code in the language of the system). Before it leaves, the program is checked against the limits of the machine of the settings (the limit of power and of speed, the bed) and one that fails is not sent; `--follow` stays until the job ends; `status`, `job`, `pause` and `resume` ask after the machine and the job and hold it or let it go on; `stop` is the EMERGENCY STOP of the laser machine: the job ends at once and the machine is latched, so no program is taken, and none is sent, until an administrator runs `release`. The token is never printed (it may be in the variable `PHOENIX_LASER_SERVER_TOKEN`) and travels in the clear over plain HTTP. The local commands are not touched. Tested against a stand-in of the laser plugin, in memory and on a socket of this computer; the plugin of the server is a simulator itself, and nothing here has run against a real laser. `phoenix-laser server vector DRAWING.svg|dxf` sends a drawing as a vector job (the paths and one operation for each layer, checked against the limits of the settings first), and `phoenix-laser server test-fire --power-percent P --confirm-fire` fires the laser for a moment to find the focus (the server refuses it while a job runs, while an emergency stop holds the machine, with the enclosure open or the air assist off, and above its own limit). The section of the window for the server shows the panel of the suite (address, token, machine, link and a login with a name and a password), sends a program that was opened to be seen once it passes the check and the person confirms it, and the marks mode can take its size from the work.
* **Send to the server from the window** (`src/window.rs`, `[server]` in the settings file): with a server in the settings, a section of the window sends the program that was worked out as a job of the laser machine, shows how far it is and holds it, lets it go on or stops it with an EMERGENCY STOP button (the job ends at once and the machine is latched until an administrator releases it); a program opened to be seen, a frame that lights the laser and was not confirmed, and a program that fails the check against the limits are not sent. Closing the window leaves the job where it is on the server.
* **Ordered dithering** (`src/dither.rs`): `--dither bayer` (also in the window) decides each dot by its own place and grey with an 8 x 8 Bayer matrix, so the pattern is regular and the same grey always burns the same dots; the tests check that each level of the matrix appears once and that the darkness of a grey is kept.
* **The tone curve** (`src/tone.rs`): before the dots are made the grey levels can be stretched to the full range (`--auto-level`), given more or less contrast and brightness, bent with a gamma or inverted, with one table of 256 entries; with the default values the picture is not changed at all.
* **A checked G-code:** the program of a drawing is checked line by line before it is given: no laser on during a rapid move or at no speed, no `S` above full power, no move outside of the bed, `M5` at the end; the file is never written over the drawing or the settings it comes from.

## 🧩 Role in EH3D

EH3D-PHOENIX-LASER is one program of the EH3D Fabrication Suite. It works as a library and a command line and, built with `--features gui`, as a window in the look the other programs of the suite share. It reads and writes plain files, so it can be used on its own or between the other programs.

## 🏛️ Architecture

<p align="center">
  <img src="images/architecture.svg" alt="EH3D-PHOENIX-LASER architecture: who uses it, what it holds and what it depends on" width="100%">
</p>


How the parts of this repository fit together: who uses it, what it holds and what it depends on. The picture is drawn from `docs/architecture.json` and checked against the code.

## 🖼️ Screenshots

These pictures are taken by the program itself with `--shot` (see Quick start).

<p align="center">
  <img src="images/screens/window-vector.png" alt="Cutting a drawing: the path of the laser over the plate of the examples, with the table of layers under it" width="100%">
</p>

<p align="center">
  <img src="images/screens/window-engrave.png" alt="Engraving a picture: the dots of the gradient of the examples coloured by their power" width="100%">
</p>

## ✨ Features

* **A library and a command line:** every command of `phoenix-laser` is a function of the library, so another program of the suite can call it without starting a process.
* **Strict command line:** an option that is written wrong (`--powr`) or a word too many is an error that names it, not something to ignore; the program would otherwise make its output with the default values.
* **Tests with damaged input:** the tests also feed empty files, garbage, huge numbers and cut files, and none of them may panic.
* **No guessing:** a value that does not make sense is an error that says which value and why, never a result that only looks right.
* **No unsafe code:** `#![forbid(unsafe_code)]` in the library and the program.

## 🔒 Security & Safety

| Threat | Defense |
|---|---|
| A damaged, truncated or empty file | no panic: the tests cut valid input at every position and feed garbage |
| A number that makes no sense (negative, not finite, absurdly large) | refused with an error that names the value; nothing is rounded into something plausible |
| Unsafe memory use | plain safe Rust with `forbid(unsafe_code)` |
| A hostile drawing (scripts, declared entities, deep nesting, millions of points, huge numbers) | scripts, images and links are never run or fetched; a declared entity is an error; the size, the depth, the elements, the shapes, the points and the pieces of a curve are limited; a shape with a point out of range is left out |
| A program that could turn the laser on at no speed or above full power | every program is checked line by line before it is given, and one that fails is not written |
| A power higher than the machine or the material should get | the limit of power (`max_power_percent`, `--max-power`) refuses a program over it with an error that names the layer or the option; it is never lowered without a word |
| A library of materials that is broken or huge | the file is limited in size and in number of materials; a value out of range, an unknown key or a name used twice is an error that names the line; the output is never written over it |
| A G-code program from somewhere else that is read before it is run | the preview only reads text, with a limit on its size, never runs or fetches anything, ignores numbers that are absurd and warns of the laser on during a rapid move, no feed, power above full, work outside the bed and no end |
| The laser turning on just to position the work | the frame is traced with the laser off (no power set, the spindle never on); a pilot power needs `--confirm-laser-on` on every run, even when the settings file asks for it, stays at or under the cap (5 % unless `max_pilot_percent` says another), under the limit of power and is turned off before the program ends |
| A head sent the wrong way in Z by a focus test | the step of the height has no default and the test is refused until the person gives it; the program says that it moves Z, the range is limited (50 mm in all), the head returns to the first height and the power obeys the limit of power |
| A text that is too long, too tall or too strong, or that would burn outside of the bed | only the characters of the font are accepted (a letter it has not is refused, never skipped), the number of characters, the height, the power and the speed are held between limits that the settings can narrow, work outside of the bed is refused instead of cut down, and the text is never written into a comment of the program |
| Data that does not fit in the QR code, a symbol that is too big, too dense or too strong, or one that would burn outside of the bed | the version, the number of bytes, the size, the number of burning moves, the power and the speed are held between limits that the settings can narrow, and anything over them is refused instead of cut down or shrunk; the data is never written into a comment of the program |
| The output file chosen over the drawing or the settings it is made from | refused, in the window and in the commands |
| A program sent to a server, a token that could be seen, or a machine that was stopped and started again | the program is checked against the limits of the machine before it leaves and one that fails is not sent; the token is never printed (not in a message, not in a `Debug`), comes from the settings file or the environment and travels in the clear over plain HTTP, so it is for a network you trust or a tunnel (the program says so when the address is public); the stop is the emergency stop, which latches the machine until an administrator releases it, and a latched machine is not sent a program |

**What is not defended:** a result that was produced cleanly is not a verified result. Nothing here knows the limits of a machine, and the program has had no independent review. Report a vulnerability privately (see [SECURITY.md](SECURITY.md)).

## 🔌 Interfaces

The library is `eh3d_phoenix_laser` (`use eh3d_phoenix_laser::...`); the command line is `phoenix-laser`. The commands are these; each prints its result on the standard output and reports an error on the standard error with a non-zero exit code. There is no network access.

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
phoenix-laser focus OUT.nc --z-step MM [--z-start MM] [--steps 9] [--length 10] [--spacing 2] [--feed 1000] [--power 30 | --powers 10,20,30] [--smax 1000] [--max-power P] [--max-speed MM/MIN] [--config FILE] [--profile NAME]
phoenix-laser frame OUT.nc --size W,H | --gcode PROGRAM.nc [--origin X,Y] [--margin MM] [--feed 3000] [--laps 1] [--bed X,Y] [--smax N]
              [--pilot P --confirm-laser-on] [--max-power P] [--max-speed MM/MIN] [--config FILE] [--profile NAME]
phoenix-laser verify PROGRAM.nc [--config FILE] [--profile NAME] [--smax N] [--max-power P] [--max-speed MM/MIN] [--bed X,Y] [--overscan-mm MM] [--accel 1000] [--expect-sha256 HASH] [--json]
phoenix-laser preview PROGRAM.nc [OUT.svg] [--accel 1000] [--rapid 6000] [--smax N] [--laser-watts W] [--bed X,Y] [--width-px 900] [--config FILE] [--profile NAME]
phoenix-laser server status|release [--address HOST:PORT] [--machine ID] [--config FILE] [--profile NAME]
phoenix-laser server send PROGRAM.nc [--name TEXT] [--follow] | job|pause|resume|stop JOB [--address HOST:PORT] [--machine ID] [--config FILE] [--profile NAME]
phoenix-laser server vector DRAWING.svg|dxf [--name TEXT] [--follow] [--address HOST:PORT] [--machine ID] [--config FILE] [--profile NAME] [--material NAME] [--power P] [--speed MM/MIN] [--passes N]
phoenix-laser server test-fire --power-percent P [--seconds S] --confirm-fire [--address HOST:PORT] [--machine ID] [--config FILE] [--profile NAME]
phoenix-laser [gui] [FILE]                       the window (a build with --features gui); with no arguments it opens
```

`DRAWING` is an SVG or a DXF file. `cut` follows the contours (the inside ones first); `vector` fills the closed shapes with lines and follows the open ones; both take the settings of every layer or colour from `--config FILE` (see [docs/vector-settings.md](docs/vector-settings.md)) and an option of the command line changes them for all the layers. `layers` lists the layers of a drawing with what would be done with each.

## ⚙️ Configuration

The engraving of a picture is set with arguments of the command line (see Interfaces) and every default is written in the help of the command (`phoenix-laser help`). Cutting and engraving a drawing is set with a settings file (`--config FILE`, or "Load settings" in the window): the limits of the readers, how a file is read, the order of the cuts, the machine and, for each layer or colour, the mode (cut, engrave or skip), the power, the speed, the passes, the step of the focus, the width of the beam and its side, the bridges and the hatch. Every key, its default and its range are in [docs/vector-settings.md](docs/vector-settings.md) and an example is in `examples/plate.cfg`; a key that is not known or a value out of range is an error that names its line. The fixed limits of the picture reader live in the code as named constants and are covered by tests.

## 🚀 Quick start

You need a stable Rust toolchain (1.88 or newer). The repository EH3D-COMMON must be cloned beside this one: the reader of pictures is taken from it by path.

```bash
cargo build --release
cargo test
cargo run --release -- engrave examples/gradient.pgm examples/gradient.nc --width-mm 16 --dot-mm 0.5 --overscan auto
cargo run --release -- cut examples/plate.svg examples/plate.nc --config examples/plate.cfg
cargo run --release --features gui   # the window
cargo run --release --features gui -- --shot screen.png examples/plate.svg   # a picture of the window
```

To take a picture of the window without opening it by hand, run it with `--shot FILE.png` (also `--size WxH`, `--theme forge|dark|light`, `--wait N`, `--lang CODE`, `--open FILE`, `--replace`): it opens at that size, saves the PNG, keeps no settings and closes by itself (exit code 0 when the file was written, 1 when it was not, 2 for wrong arguments).

## 🧪 Testing

The suite has 671 tests (`cargo test`, run on Windows and Linux in the CI). They check the normal cases, the edge cases and damaged input: empty files, garbage and cut files never panic. The CI also runs `cargo fmt --check`, `cargo clippy` with warnings as errors and `python -I tools/ci_validate.py`.

## 🏗️ Build & Release

The version lives in `Cargo.toml` and in `eh3d.project.json`; each version has its entry in [CHANGELOG.md](CHANGELOG.md). `build.bat` / `build.sh` build the program, `test.bat` / `test.sh` run the tests and `run.bat` / `run.sh` run it with a sample. Releases are published on GitHub.

**Downloads:** Ready-made programs for every system are published with each release of this repository: Windows (64 and 32 bits), Linux (64 bits) and macOS (Apple silicon and Intel). Each package, `phoenix-laser-<version>-<system>.zip` or `.tar.gz`, holds the program, the licence, this README, the changelog, the icon and the example files, and comes with its `.sha256` file; the release also lists every file in `SHA256SUMS` and `build-manifest.json`. No release has been published yet.

**Signed and checked:** the packages of a release are signed with the release key of EH3D, and the `.sig` file of each package is published next to it (an OpenSSH signature, namespace `eh3d-release`). Check a download before running it: compare its SHA-256 (`sha256sum -c phoenix-laser-<version>-<system>.tar.gz.sha256`, or `Get-FileHash` on Windows) and verify the signature with `ssh-keygen -Y verify -f allowed_signers -I eh3d-release -n eh3d-release -s <package>.sig < <package>`, where `allowed_signers` holds the line `eh3d-release <key type> <public key of the releases of EH3D>`.

## 📂 Repository Structure

<p align="center">
  <img src="images/repository-structure.svg" alt="Repository structure of EH3D-PHOENIX-LASER" width="100%">
</p>

<details>
<summary>Text version of the tree</summary>

```text
EH3D-PHOENIX-LASER/
├── src/
│   ├── lib.rs                                   the library: what every command uses
│   ├── main.rs                                  the command line: engrave, calibrate, info, cut, vector, trace, verify and layers
│   ├── calibrate.rs                             a part of the logic
│   ├── cli_args.rs                              the arguments of the command line, the same file in every small utility
│   ├── dither.rs                                a part of the logic
│   ├── materials.rs                             the library of materials: power, speed, passes, dots and tone
│   ├── preview.rs                               the preview of a G-code program: its moves, time, dangers and an SVG of the path
│   ├── tone.rs                                  the tone curve of a picture: levels, contrast, brightness, gamma and inversion before the dots
│   ├── trace.rs                                 the contours of a picture: threshold, marching squares and simplified rings, with limits
│   ├── text.rs                                  a line of text burnt with a font of strokes of its own, inside limits
│   ├── qr.rs                                    a QR code of its own, versions 1 to 4 with error correction, burnt by filling its modules
│   ├── barcode.rs                               a Code 128 barcode of its own, burnt by filling its bars with lines
│   ├── gcode_out.rs                             the one way a G-code number is written and the standard header
│   ├── raster.rs                                a part of the logic
│   ├── safety.rs                                the limit of power, and the rules that keep the laser off while the work is shown
│   ├── server.rs                                the way to the EH3D server: check, send, follow, hold, emergency stop
│   ├── linkpane.rs                              the panel of the server in the window: the boxes, the link and the login
│   ├── shot.rs                                  the picture mode of the window (--shot): its arguments, read and checked without a screen
│   ├── vector.rs                                a drawing once read: shapes in mm, layers, limits and the small geometry
│   ├── verify.rs                                the check of a program: its SHA-256, what it does and a verdict against the limits of the machine
│   ├── flatten.rs                               curves cut into straight pieces within a tolerance (Bezier, arcs)
│   ├── frame.rs                                 the frame of the work (the rectangle traced with the laser off) and the marks at its corners
│   ├── svg.rs                                   the SVG reader: a safe subset, no scripts, images or entities
│   ├── dxf.rs                                   the DXF reader: lines, polylines, arcs, circles and splines, with units
│   ├── energy.rs                                the energy of a pass for each millimetre or square millimetre, from the optical power
│   ├── offset.rs                                moving a contour sideways by the width of the beam, checked edge by edge
│   ├── order.rs                                 the order of the cuts: loose lines joined, inside first, short travel
│   ├── cut.rs                                   from a drawing to G-code: cut, tabs, hatch, passes and the safety check
│   ├── vconfig.rs                               the settings file of the vector work: limits, machine and each layer
│   ├── browse.rs                                the list of files of the window that opens a file
│   ├── gui.rs                                   the window (egui, with the gui feature): draws the state and passes on what the person does
│   ├── lang.rs                                  the words of the window in the eight languages
│   ├── viewport.rs                              zoom and pan of the picture, in plain numbers
│   └── window.rs                                the state of the window without a screen: picture or drawing, settings, layers, passes and export
├── tests/
│   ├── fuzz.rs                                  fuzz tests with a fixed seed: damaged files, cuts and odd numbers
│   ├── dry_run.rs                               tests of --dry-run: it counts the same and writes nothing
│   ├── laser.rs                                 tests of the library and the command line
│   ├── laser_tools.rs                           tests of the tools around the engraving, through the command line
│   ├── readme.rs                                tests that run what the README and the run scripts promise
│   ├── strict.rs                                tests of the command line: options and words it does not know
│   ├── vector.rs                                tests of cutting from drawings: commands, examples, hostile files and the check of the G-code
│   ├── server_cli.rs                            the server command against a stand-in of the laser plugin
│   └── window_cli.rs                            tests that a build without the gui feature stays a command line
├── examples/
│   ├── gradient.pgm                             a sample to try the program with
│   ├── plate.svg                                a plate with a hole and a panel to engrave (SVG)
│   ├── plate.dxf                                the same plate as a DXF
│   ├── plate.cfg                                the settings of its layers: power, speed, passes, beam width, tabs and hatch
│   └── materials.cfg                            a library of materials: birch and slate, cutting, engraving and photographs
├── docs/
│   ├── architecture.json                        data of the Architecture picture
│   ├── structure.json                           data of the Repository Structure picture
│   ├── vector-settings.md                       the settings file of the vector work, key by key
│   └── materials.md                             the library of materials, key by key
├── tools/                                       CI check, README check and the picture tools
├── images/                                      banner, README pictures and window screenshots
├── .github/                                     CI and release workflows, issue and pull-request templates
├── eh3d.project.json                            manifest: version, role, maturity and build command
├── Cargo.toml Cargo.lock                        the workspace and its locked dependencies
├── rust-toolchain.toml                          the pinned Rust toolchain
├── CHANGELOG.md                                 what changed in each version
├── SECURITY.md SUPPORT.md                       how to report a vulnerability and where to ask
└── CONTRIBUTING.md CODE_OF_CONDUCT.md LICENSE   how to contribute, the code of conduct and the licence
```

</details>

## 🗺️ Roadmap & Known limits

**Known limits today**

* The window has not been looked at on a screen yet.
* It has not been tried on a machine nor with a collection of real files, and it has had no independent review.
* A drawing is read in a safe subset (no `use` of another file, text, gradients or styles with selectors; no ellipses, hatches or blocks in a DXF); a contour narrower than the beam is not cut and the program says so; the laser is the one of a GRBL controller with `M4`, and nothing has been tried on a real laser or material.

**Next**

* Show the power density on the board before burning (`layers` already prints the energy of a pass when `laser_watts` is set) and keep the power constant in the corners of a cut.
* Look at the window on a screen and finish it.
* Try it with real drawings and with the other programs of the suite.

## 🌍 Languages

README and texts exist in English, Spanish, French, Italian, German, Chinese, Japanese and European Portuguese.

## 🔗 Related Projects

* [EH3D-COMMON](https://github.com/JuanenRac/EH3D-COMMON): the shared crates, among them the reader of pictures
* [EH3D-STENCILCAM](https://github.com/JuanenRac/EH3D-STENCILCAM): solder paste stencils
* [EH3D-CAM-DEBUGGER](https://github.com/JuanenRac/EH3D-CAM-DEBUGGER): the checking of G-code before it reaches a machine
* [EH3D-TOOL-MANAGER](https://github.com/JuanenRac/EH3D-TOOL-MANAGER): the tool crib shared by the CAM programs
* [EH3D-MOTIONCNC](https://github.com/JuanenRac/EH3D-MOTIONCNC): the machine control program
* [EH3D-SUITE-LAUNCHER](https://github.com/JuanenRac/EH3D-SUITE-LAUNCHER): the hub that finds and starts the programs of the suite

## 📚 Documentation

* [Changelog](CHANGELOG.md)
* [Architecture and structure data](docs/architecture.json): the data the two pictures are drawn from.
* [Settings of the vector work](docs/vector-settings.md): every key of the settings file with its default and its range.
* [The library of materials](docs/materials.md): every key of a material and how a material is applied

## 🤝 Contributing & Community

* Before a pull request run `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` and `python -I tools/ci_validate.py`: all must pass. See [CONTRIBUTING.md](CONTRIBUTING.md) and the [code of conduct](CODE_OF_CONDUCT.md).
* The README exists in eight languages: a change to the text is made in all eight files, and the two pictures are drawn from `docs/architecture.json` and `docs/structure.json` by `tools/make_architecture.py` and `tools/make_structure.py`.
* Report a vulnerability privately: [SECURITY.md](SECURITY.md). Questions, ideas and reports: electrohobby3d@gmail.com ([SUPPORT.md](SUPPORT.md)).

## 👤 AUTHOR

**JuanenRac (Electro Hobby 3D)** · electrohobby3d@gmail.com

## 📜 LICENSE

Software: GPL-3.0-or-later. See [LICENSE](LICENSE). Hardware designs of the suite use CERN OHL-S.

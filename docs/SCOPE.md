# EH3D-PHOENIX-LASER: scope

Status: **functional**. EH3D-PHOENIX-LASER is a library, a command line (`phoenix-laser`) and a window (built with `--features gui`) that turn a picture or a drawing into the G-code of a diode or CO2 laser, check a program before it is given, and send it to the laser plugin of the EH3D server. It has run on a computer only; it does not control a machine. This page says what it is for, what it will not do and where it sits among the other programs of the suite.

## Purpose

The laser program of the PHOENIX family: it engraves a picture dot by dot, cuts and engraves the shapes of a drawing (SVG, DXF and LBRN2 projects) layer by layer, burns text, QR codes and barcodes, helps to find the focus and the settings of a material, and checks any G-code program against the limits of power and speed, the bed, the forbidden zones and the interlocks. A program that fails a check is not written and not sent.

## What it does

* Turns a PNG or PGM picture into dots (threshold, error diffusion or ordered dithering, with a tone curve) and writes the G-code of a laser that burns line by line with dynamic power and overscan.
* Reads SVG, DXF and LBRN2 drawings in a safe subset and writes the G-code that cuts their contours (inside ones first, the width of the beam taken into account, bridges, several passes with the focus moving on) and engraves their shapes with lines, with the settings of each layer in a file.
* Burns text with a font of strokes, QR codes and Code 128 barcodes, traces the outline of a picture, burns positioning marks, frames the work with the laser off and burns test grids and focus ladders.
* Checks a G-code program, its own or another's, as text and never runs it: limit of power and of speed, laser on during a rapid move, work outside the bed, forbidden zones, the emergency stop and the interlocks, and gives the SHA-256 of the exact file.
* Compares a saved listing of the settings of the controller with the machine and explains its alarms and errors, from text only: it opens no port and writes nothing to a controller.
* Writes test programs for the spacing of the lines and the width of the beam.
* Keeps every tuned value in a settings file, a library of materials (all unverified until a person verifies them) and profiles of machines, and offers the starting points of the common database of materials with the note that nobody tried them.
* Sends a program, a vector job or a test fire to the laser plugin of the EH3D server and follows, holds or stops the job; the stop is the emergency stop.
* Shows the work in a docking window of thirteen panels and four workspaces for tasks, in eight languages.

## What it will not do

* It does not control a machine and is not a safety system: what it produces must be checked by a person before anything is run.
* It never lowers a power or a speed over the limit without a word: a program over a limit is refused with the name of the layer or the option.
* It never writes an output file over the picture, the drawing or the settings it is made from.
* It never turns the laser on to position the work without the confirmation of the person every time.
* It does not claim that anything was proven on a real laser or material: the evidence of every capability is `simulator` or `host-test`.

## Where it sits

* **It reads:** pictures (PNG, PGM), drawings (SVG, DXF, LBRN2), G-code programs to check, the settings file, the library of materials, the common database of materials when it is built in, and the answers of the server.
* **It writes:** G-code programs, SVG and DXF conversions, reports and previews that the person asks for, the library of materials the person edits and the arrangement of the window, never over a source; and orders to the laser plugin of the server when the person sends a job.
* **It is read by:** the laser plugin of the EH3D server receives its jobs; no other program reads it.
* **It depends on:** EH3D-COMMON (the docking frame, the server client and the image reader) as a sibling folder, and the common database of materials (feature `common-machining`, on by default; `--no-default-features` leaves it out).

## What it never decides

Whether a program is safe to burn, the limits, the interlocks and the emergency stop of the laser, the enclosure and the air assist, and what a material does under a beam belong to the machine, to the person at it and to the standards that apply. The interlocks and the emergency stop of the settings only ever stop more; a passing check is not a statement that burning is safe. Nothing here replaces the emergency stop of the machine.

The requirements of the program, with the state of each and the test that shows it, are in `REQUIREMENTS.md`; the state in a paragraph is in `STATUS.md`.

# EH3D-PHOENIX-LASER: status

## Current state

EH3D-PHOENIX-LASER works on a computer: the command line engraves pictures, cuts and engraves drawings, burns text, QR codes and barcodes, checks programs and sends them to a stand-in of the server; a docking window of twelve panels does the same with boxes. The reader of LBRN2 projects was measured on twelve real projects of one laser maker (all twelve are read; the one picture inside them, a PNG, is decoded and engraved by dots) and nothing made from them was cut or engraved on a machine; the direction of their y axis was never confirmed with a real part. A series of texts from a CSV file or a counter writes one program for each row. Nothing has run on a real laser.

## Recent work

The window became the docking frame of the suite with four workspaces for tasks, the reader of LBRN2 projects was added with the layers of a project filling the table of layers, a tool that reads a folder of projects and says what the reader leaves out was added, and the common database of materials now gives starting points that are shown as untried. The README files, this page and the register of requirements agree on what exists and what is missing; the register is the one that says it.

## Capabilities

The official state of each capability is in `REQUIREMENTS.md`. The register has 105 requirements: 91 implemented, 9 prototype, 5 planned and 0 validated. By progress: 91 done, 9 partial, 1 pending and 4 blocked. This page never states a state that `REQUIREMENTS.md` does not.

## Open questions

* Which real laser is the first one, diode or CO2, and what its controller is.
* Which way the y axis of an LBRN2 project points, to be settled with a real part (the setting `lbrn2_flip_y` holds the answer).
* How the power should follow the speed in the corners of a cut.

## What it needs from other repositories

* EH3D-COMMON: the docking frame, the server client and the image reader.
* EH3D-SERVER: the laser plugin that receives the jobs.
* A real laser, a material and a person at the machine, for every proof that a check cannot give.

## Known technical debt and omitted tests

* 895 tests (`#[test]` lines of `src/` and `tests/`; nine of them keep this register true), all on a computer, with fixed seeds where chance is involved.
* Nothing has been burnt: the checks read the program as text and the server is a stand-in.
* The window has not been looked at on a screen by a person.
* The QR code and the barcode are read back by readers of the program's own; no scanner has read a burnt symbol.
* The program has had no independent review.

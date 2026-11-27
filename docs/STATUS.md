# EH3D-PHOENIX-LASER: status

## Current state

EH3D-PHOENIX-LASER works on a computer: the command line engraves pictures, cuts and engraves drawings, burns text, QR codes and barcodes, checks programs and sends them to a stand-in of the server; a docking window of thirteen panels does the same with boxes. The reader of LBRN2 projects was measured on twelve real projects of one laser maker (all twelve are read; the one picture inside them, a PNG, is decoded and engraved by dots) and nothing made from them was cut or engraved on a machine; the direction of their y axis was never confirmed with a real part. A series of texts from a CSV file or a counter writes one program for each row. Nothing has run on a real laser.

## Recent work

The check of a program reads a line as the controller does and fails what it cannot judge: spaces inside a number, constant power left on while the head stands, commands of the controller inside a program, parts that cannot be read, outputs of other controllers and a program too long to be read whole; a danger is no longer pushed out of the report by other notes. A saved listing of the settings of the controller can be compared with the machine (`controller`, `verify --controller`), alarms are explained (`explain`), two further test programs find the spacing of the lines and the width of the beam (`interval`, `kerf`), four more ways of dithering were added and a line that would be cut twice is told.

The window has a panel «Attachments» (the shared `eh3d-attachpanel`): the data sheets, manuals and other files of the project are listed, attached, linked to a part, extracted and removed after a question, and shown beside the list (`eh3d-project.md`). A file can be dropped on the panel and its limits come from the settings file (`attachments.max_mb`, `viewer.max_*_mb`). The panel itself is tested in EH3D-COMMON; it has not been tried on a screen by a person.

The menu File of the window opens, saves, imports, takes and extracts an `.eh3d` project and asks before it writes one that is not its own. The command `project` keeps the work in the branch `laser/` of an `.eh3d` project (`eh3d-project.md`): the other branches keep their bytes, copies taken from them are told as out of date and imported files keep their original with a report. The window became the docking frame of the suite with four workspaces for tasks, the reader of LBRN2 projects was added with the layers of a project filling the table of layers, a tool that reads a folder of projects and says what the reader leaves out was added, and the common database of materials now gives starting points that are shown as untried. The DXF reader draws the outline of hatches and solids, projects of the older format (`.lbrn`) are read with their texts, the layers of a project can give the command line their power and speed, and the program writes the power-against-speed matrix and the test of the y axis as projects. A book of presets can be kept as text. A layer can now be worked in several passes with their own settings (sub-layers), the work can be placed by any of nine points of its box, framed with the laser off and timed as a range, the presets of a material can be assigned or linked to a layer, and the live adjustment of power and speed is a calculation held by four limits. The README files, this page and the register of requirements agree on what exists and what is missing; the register is the one that says it. It does not write over a project another program saved since it was read, and offers to read it again or to save under another name.

## Capabilities

The official state of each capability is in `REQUIREMENTS.md`. The register has 156 requirements: 134 implemented, 18 prototype, 4 planned and 0 validated. By progress: 134 done, 18 partial, 0 pending and 4 blocked. This page never states a state that `REQUIREMENTS.md` does not.

## Open questions

* Which real laser is the first one, diode or CO2, and what its controller is.
* Which way the y axis of an LBRN2 project points, to be settled with a real part (the setting `lbrn2_flip_y` holds the answer).
* How the power should follow the speed in the corners of a cut.

## What it needs from other repositories

* EH3D-COMMON: the docking frame, the server client, the image reader and the project container. The workflows take its tag v0.3.7.
* EH3D-SERVER: the laser plugin that receives the jobs.
* A real laser, a material and a person at the machine, for every proof that a check cannot give.

## Known technical debt and omitted tests

* 1060 tests (`#[test]` lines of `src/` and `tests/`; nine of them keep this register true), all on a computer, with fixed seeds where chance is involved.
* Nothing has been burnt: the checks read the program as text and the server is a stand-in.
* The window has not been looked at on a screen by a person.
* The QR code and the barcode are read back by readers of the program's own; no scanner has read a burnt symbol.
* The program has had no independent review.

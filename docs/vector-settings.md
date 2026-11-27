# Cutting and engraving from a drawing: the settings file

`phoenix-laser cut`, `phoenix-laser vector`, `phoenix-laser layers` and the window read an SVG or a DXF drawing and make the G-code of a laser from it. Everything that is tuned lives in one plain text
file, given with `--config FILE` (or chosen in the window with "Load settings", or named by `PHOENIX_LASER_CONFIG`, or `phoenix-laser.cfg` in the folder the window runs in). The file in
`examples/plate.cfg` shows every kind of setting at work.

Nothing is guessed: a line that is not a setting, a key that is not known and a value outside its range are errors that name the line, and a program is never written from a file that has one.
The options of the command line (`--power`, `--speed`, `--kerf`...) are applied over the file, to every layer.

## The file

```text
# a note (also ; at the start of a line)
[section]
key = value
[layer NAME]
key = value
```

Sections: `[limits]`, `[read]`, `[order]`, `[offset]`, `[hatch]`, `[machine]`, `[frame]`, `[focus]`, `[trace]`, `[tone]`, `[verify]`, `[calibrate]`, `[text]`, `[qr]`, `[send]`, `[interlocks]`, `[job]`, `[live]`, `[defaults]`, one `[layer NAME]` for each layer or colour, one `[sublayer LAYER/LABEL]` for each extra pass over a layer, one `[profile NAME]` for each machine and one `[zone NAME]` for each forbidden zone of the bed. `[defaults]` is what a layer gets that the file
does not name, and also what every `[layer NAME]` starts from (wherever in the file `[defaults]` is written). The switches are `on` and `off`.

The window keeps the arrangement of its panels in the same file, as lines that start with `dock.` (for example `dock.version=1`). The reader of the settings leaves those lines alone wherever they are, the window writes them back without touching any other line, and a line that is not a setting and does not start with `dock.` is still an error.

The limits of the panel «Attachments» are lines of their own that may be written anywhere in the file, as in every program of the suite. Each value is a whole number from 1 to 4096; another key that starts with `attachments.` or `viewer.`, or another value, is an error. The settings file of a project never changes them.

| Key | Default | Meaning |
|---|---|---|
| `attachments.max_mb` | 1024 | largest file that is attached or taken out |
| `viewer.max_attachment_mb` | 1024 | largest attachment shown, opened or saved |
| `viewer.max_pdf_mb` | 256 | largest PDF shown inside |
| `viewer.max_image_mb` | 64 | largest picture file shown |
| `viewer.max_text_mb` | 2 | bytes of a text file shown |
| `viewer.max_office_mb` | 128 | largest office document previewed |
| `viewer.max_preview_mb` | 16 | text a whole preview keeps; the rest is cut and said |

`phoenix-laser settings` lists every key of the sections of this file and of a library of materials (section, key, kind of value, default and whether it is a safety setting), as text or as JSON with `--json`; a test keeps that list equal to the keys the reader takes and to the defaults of the tables below.

## What a layer is

The shapes of a drawing are grouped, and each group has its own settings:

* **an SVG:** by colour, `#rrggbb` in lower case: the stroke, or the fill of a shape that has no stroke (names such as `red` and `rgb(255, 0, 0)` are turned into `#ff0000`). A shape with neither
  has no colour and is not drawn. A shape without a style is black, `#000000`. A group of Inkscape that is a layer gives its name to the shapes inside it, which is used when a shape has no colour.
* **a DXF:** by layer name, in lower case. The colour of the entity is kept but not used unless `group_by = color`.
* `group_by` (in `[read]`) is `auto` (what is said above), `layer` or `color`.

`phoenix-laser layers DRAWING` lists the groups with their size and what would be done with each. With `--accel A` (1 to 100000 mm/s2) it also gives, under each layer that is burnt, its time with the head braking and speeding up at every corner at that acceleration and how much of it the laser burns, and the total of all the layers: the whole drawing is planned once, the program is cut at the comment that opens each layer and each piece is read by the preview of the program (`phoenix-laser preview`), so the burning times add up to the one of the whole program; each piece starts with the head at the origin, so the rapid move to its first contour is counted from there.

## What the program reads

| | SVG | DXF (text form) |
|---|---|---|
| Lines and curves | `path` (`M L H V C S Q T A Z`, absolute and relative), `rect` (with round corners), `circle`, `ellipse`, `line`, `polyline`, `polygon` | `LINE`, `LWPOLYLINE` and `POLYLINE` with bulges, `ARC`, `CIRCLE`, `ELLIPSE` (whole or an arc), `SPLINE`, `INSERT` of a block, `SOLID` and `TRACE` (their outline), `HATCH` (the outline of its boundary paths: polyline paths and edges that are lines and arcs of a circle; an ellipse or a spline edge is left out, and the fill pattern is never drawn) |
| Placing | `transform` (`translate scale rotate skewX skewY matrix`), groups, `viewBox`, `use` of an element of the same file (`href="#id"`, with `x` and `y`; a `symbol` with a `viewBox` is fitted to the `width` and `height` of the `use`): what it brings in counts against `max_nodes` and `max_depth`, and a `use` that leads back to itself is an error | the extrusion of an arc, a circle, an ellipse or a light polyline pointing down; `INSERT` with its insertion point, scale, rotation and the base point of its block, as an array of columns and rows too, a block inside a block (it counts against `max_nodes` and `max_depth`, and a block that draws itself is an error); an entity of a block on layer 0, or with no colour, takes the layer and the colour of the `INSERT` |
| Units | `mm cm in pt pc px`; a `px` is `25.4 / dpi` mm (`dpi` is 96 unless the file has a size and a view box) | `$INSUNITS` (inch, foot, mile, millimetre, centimetre, metre, mil, yard, micrometre, decimetre); a drawing with no unit uses `dxf_unit_mm` |
| Never read | scripts, images, a `use` of another file or an address, text, styles with selectors, filters, gradients, masks, and any entity declared in the file (an external entity or a declared one is an error) | text, dimensions, points, the fill pattern of a hatch, meshes, binary files |

A project of LBRN2 kind (`.lbrn2`, and the older `.lbrn`) is read too: see [lbrn2.md](lbrn2.md). `--project-layers` of `cut`, `vector` and `layers` gives the layers of such a project the power, the speed and the passes of their rows (the layers that the file of `--config` names keep what it says; `--material` and the other options still win; the limits of the machine hold over all of them). What is left out is listed as a note (in the window and on the standard error of the commands). Curves become straight pieces that are never farther than `tolerance_mm` from the true curve.

## Order of the work

1. The layers that are `engrave` are done first, then the ones that are `cut`; inside each kind by their `order` (the lowest first) and then by name. A layer with sub-layers is done as several operations: the layer itself and then its sub-layers, each one placed by the same rule, so a fill (engrave) goes before an outline (cut) of the same layer, and two passes of the same kind and order keep the order of the file.
2. Open paths whose ends are closer than `join_tolerance_mm` are joined into one path; one that comes back to its start becomes a closed contour.
3. A contour that lies inside another one is cut before it (a hole before the piece it is a hole of), and among the orders that respect this the head travels as little as possible with the laser
   off: the nearest next contour, then a 2-opt improvement that never breaks the rule. A closed contour is entered at the vertex nearest to the head; an open one from its nearer end.

## Cutting

For each pass of a `cut` layer the program writes `F` (the speed), moves the focus when `focus_step` is not 0, and cuts every contour with `S` (the power) turned on only while it cuts.

* **Width of the beam (kerf):** with `kerf` more than 0 a closed contour is moved half of it sideways, with round corners where the offset line goes round a corner. `kerf_side`: `auto` puts the
  beam outside the outline of a piece and inside a hole (the odd levels of nesting), so that both keep their size; `outside` and `inside` put it always on one side; `none` follows the line. Open
  paths are never moved. A contour that is too small or too narrow for the beam, or whose offset would have to lose a part of it, is left out with a note: nothing is better than a line that is wrong.
* **Tabs:** `tabs` bridges of `tab_width` mm are left uncut on the outline of a piece, the first at `1 / (tabs + 1)` of its length from its first point, and so on, evenly. Holes have none. A contour
  too short for them (the bridges would be more than half of it) is cut whole, with a note. Bridges are left on every pass.
* **Several passes:** `passes` times, with the focus (the Z axis) at `focus_z_mm + focus_step * pass`; it goes back to `focus_z_mm` after the layer. A negative step goes down.

## Engraving

A closed shape of an `engrave` layer is filled with lines `hatch_spacing` mm apart at `hatch_angle` degrees (the even-odd rule: a shape inside another is a hole), one way and then the other. The
head runs `overscan_mm` past each end of a line so that it is at speed when the laser is on. Open paths are followed once. `passes` and `focus_step` work as for cutting.

## The G-code and its check

The program starts with `G21`, `G90` and `M4 S0` (dynamic power, as the engraving of a picture), moves with `G0 ... S0` while it travels, writes `S` on the first move of every cut, puts the laser off
with `S0` after it and ends with `M5` and `M2`. Before it leaves the program, every line is checked, and a program that fails is never given:

* only the words the program writes are allowed (`G0 G1 G21 G90 M4 M5 M2 X Y Z F S`); a number that is not finite is refused;
* `S` is never more than `s_max` and never less than 0, and a power that rounds to `S0` is refused (it would not cut);
* the laser is never on during a `G0` move or a move of the focus, and a `G1` move with the laser on needs a speed (`F` more than 0);
* no move goes outside of the bed when `bed_x_mm` and `bed_y_mm` are given (the overscan counts);
* the program ends with `M5` and `M2`.

The file written by the commands and by the window has the same header as the one of a picture (generator, units, source with its SHA-256, tools) and is never written over the drawing or
the settings file it is made from.

## Keys

### `[limits]`: what a hostile or broken file may ask for

| Key | Default | Meaning |
|---|---|---|
| `max_file_bytes` | 33554432 | the largest file that is read |
| `max_depth` | 64 | how deep the elements of an SVG may be nested |
| `max_nodes` | 200000 | the most elements (SVG) or entities (DXF) |
| `max_shapes` | 100000 | the most shapes |
| `max_points` | 2000000 | the most points in all the shapes |
| `max_curve_segments` | 2000 | the most pieces one curve is cut into |
| `max_coordinate_mm` | 100000 | a point farther than this from the origin is a mistake of the file; its shape is left out |
| `max_bitmaps` | 16 | the most pictures of an LBRN2 project that are read; the rest are left out with a note |
| `max_bitmap_bytes` | 16777216 | the longest text (base64) one picture of an LBRN2 project may have; a longer one is refused with `PHOENIX-LASER-LBRN2-BITMAP-SIZE` |
| `max_bitmap_pixels` | 20000000 | the most pixels one picture of an LBRN2 project may have; more is refused with `PHOENIX-LASER-LBRN2-BITMAP-PIXELS` |
| `max_csv_bytes` | 1048576 | the largest CSV file of the variable text that is read |
| `max_csv_rows` | 10000 | the most rows of a CSV file of the variable text (one program for each) |
| `max_csv_columns` | 64 | the most columns of a CSV file of the variable text |

### `[read]`

| Key | Default | Meaning |
|---|---|---|
| `dpi` | 96 | pixels in an inch, from 1 to 2400 (an SVG whose size is in pixels) |
| `tolerance_mm` | 0.01 | how far a flattened curve, or a round corner of the offset, may be from the true one, from 0.001 to 1 |
| `dxf_unit_mm` | 1 | the millimetres of a unit of a DXF that does not say its unit |
| `lbrn2_flip_y` | on | `on`: the y axis of an LBRN2 project points down like the one of an SVG, so the drawing is turned over when it is cut; `off`: it points up like the one of the machine. The direction was never confirmed with a real part, so the reading says so in its notes whichever it is |
| `group_by` | auto | `auto`, `layer` or `color` |
| `align` | bbox | `bbox`: the lower left corner of the work (the layers that are not skipped) is the origin, like a picture; `page`: an SVG keeps its page (the lower left corner of the page is the origin when the file has a size), a DXF keeps its own origin |

### `[order]`

| Key | Default | Meaning |
|---|---|---|
| `join_tolerance_mm` | 0.01 | ends closer than this are joined (0 to 1; 0 joins nothing) |
| `join_max_paths` | 5000 | nothing is joined when a layer has more open paths than this |
| `order_max_items` | 20000 | with more contours than this they are cut inside ones first but not in the shortest order |
| `containment_max_items` | 1500 | with more closed contours than this holes are not told from pieces |
| `two_opt` | on | the improvement of the order |
| `two_opt_budget` | 20000000 | the most moves it tries |
| `two_opt_max_items` | 2000 | it is not tried for more contours than this |

### `[offset]` and `[hatch]`

| Key | Default | Meaning |
|---|---|---|
| `max_points` (`[offset]`) | 5000 | a contour with more points than this is not offset (it is left out with a note) |
| `max_lines` (`[hatch]`) | 100000 | a hatch with more lines than this is an error |

### `[machine]`

| Key | Default | Meaning |
|---|---|---|
| `s_max` | 1000 | the spindle value that is full power for the controller (`$30` of GRBL), from 1 to 100000 |
| `overscan_mm` | 2 | the run past the end of a hatch line, from 0 to 50 |
| `rapid_mm_min` | 6000 | the speed of the moves with the laser off, only to estimate the time |
| `laser_watts` | 0 | the optical power of the laser at full power, in watts, from 0 to 1000; 0 means it is not known and `layers` then prints no energy |
| `origin_x`, `origin_y` | 0 | where the lower left corner of the work goes |
| `focus_z_mm` | 0 | where the focus (Z) is for the first pass |
| `bed_x_mm`, `bed_y_mm` | 0 | the size of the bed; 0 for both means it is not known; the work may not go outside of it |
| `max_passes` | 50 | the most passes a layer may ask for, up to 1000 |
| `max_power_percent` | 100 | the limit of power, from 0.1 to 100 percent of full power: a layer (or `[defaults]`) that asks for more is an error that names it, never lowered; `--max-power` of the commands overrides it |
| `max_speed_mm_min` | 30000 | the limit of speed, from 1 to 30000 mm/min: a layer (or `[defaults]`), a `--feed`, a `--feeds`, the frame or the focus test that asks for more is an error that names it, never lowered; `--max-speed` of the commands overrides it |
| `profile` | none | the name of the `[profile NAME]` that is the machine (see below); `--profile` of the commands chooses another |

The keys of `[machine]` are `s_max`, `overscan_mm`, `rapid_mm_min`, `laser_watts`, `origin_x`, `origin_y`, `focus_z_mm`, `bed_x_mm`, `bed_y_mm`, `max_passes`, `max_power_percent`, `max_speed_mm_min` and `profile`.

In the window, a box holds `laser_watts` (the settings file and a machine profile fill it, and 0 means it is not known): the preview of what the window works out or opens then tells the energy that the program burns and its mean power, as `phoenix-laser preview --laser-watts` does.

### `[profile NAME]`

A person with two machines, or a laser module that is changed for another, keeps one profile for each in the same file. A profile holds the travel of the machine, its limits and its full-power value; what it leaves out stays as `[machine]` says. The profile that is chosen (`profile = NAME` in `[machine]`, or `--profile NAME` on the command line, which wins) is the machine: its values win over `[machine]`, and the options of the command line (`--smax`, `--bed`, `--max-power`...) win over both. The name is not case sensitive and may have spaces. The limits of a profile hold over everything, so a `[defaults]` or a `[layer NAME]` that asks for more power or speed than the profile allows is an error.

| Key | Default | Meaning |
|---|---|---|
| `travel_x_mm`, `travel_y_mm` | not given | the travel of the machine, the size of its bed (1 to 100000 mm); both or none; the work may not go outside of it |
| `max_power_percent` | not given | the limit of power of this laser, from 0.1 to 100 |
| `max_speed_mm_min` | not given | the limit of speed of this machine, from 1 to 30000 |
| `s_max` | not given | the spindle value that is full power for its controller |
| `laser_watts` | not given | the optical power of its laser, from 0 to 1000 |

The keys of `[profile NAME]` are `travel_x_mm`, `travel_y_mm`, `max_power_percent`, `max_speed_mm_min`, `s_max` and `laser_watts`. A file may describe up to 64 profiles.

### `[frame]`

The frame of the work (`phoenix-laser frame`): the rectangle the head traces to show where the work will land. The laser stays off unless `pilot_percent` is more than 0 **and** the command is given `--confirm-laser-on`; the file alone never turns it on.

| Key | Default | Meaning |
|---|---|---|
| `feed_mm_min` | 3000 | the speed of the trace, from 1 to 30000 |
| `laps` | 1 | how many times the rectangle is traced, from 1 to 20 |
| `margin_mm` | 0 | how far outside of the work the rectangle goes, from 0 to 100 |
| `pilot_percent` | 0 | a weak power to make the frame visible, from 0 to `max_pilot_percent` and no more than `max_power_percent`; 0 keeps the laser off |
| `max_pilot_percent` | 5 | the cap of `pilot_percent`, from 0 to 100 percent of full power; 0 allows no pilot power at all. The limit of power of the machine still holds over it, and `--confirm-laser-on` is still needed on every run |

The keys of `[frame]` are `feed_mm_min`, `laps`, `margin_mm`, `pilot_percent` and `max_pilot_percent`.

### `[focus]`

The focus test (`phoenix-laser focus`): short lines side by side, each burnt with the head at another height (Z), to see where the beam is sharpest. `z_step_mm` is 0 until it is given (here or with `--z-step`) and the test is refused while it is 0: which way is down depends on the machine, and a head sent the wrong way can hit the material. The power of the test must not be over `max_power_percent`.

| Key | Default | Meaning |
|---|---|---|
| `z_start_mm` | 0 | the height of the first line, from -100 to 100 |
| `z_step_mm` | 0 | the change of height from a line to the next, from -20 to 20, and over all the lines at most 50 mm; 0 means it is not given |
| `steps` | 9 | how many lines, from 2 to 30 |
| `length_mm` | 10 | the length of each line, from 1 to 100 |
| `spacing_mm` | 2 | the distance between two lines, from 0.5 to 20 |
| `feed_mm_min` | 1000 | the speed of the lines, from 1 to 30000 |
| `power_percent` | 30 | the power of the lines, from 0.001 to 100 percent of full power |

The keys of `[focus]` are `z_start_mm`, `z_step_mm`, `steps`, `length_mm`, `spacing_mm`, `feed_mm_min` and `power_percent`.

`--powers P1,P2,...` (from 1 to 10 powers, as percents of full power) makes a focus grid instead of the test of one power: the same lines once for each power, side by side with a column for each (`spacing_mm` apart, in the order given) and a row for each height, so the power that burns the thinnest line at the height of the focus can be read off. Each power must be more than 0 up to 100, at least one step of `S` and at most the limit of power of the machine, and `--powers` cannot be given together with `--power`; the other keys of `[focus]` work as for the test.

In the window, the focus mode has a box for the powers of the grid: with powers in it the grid is burnt and the box of the single `power_percent` is not used, and the powers are held to the same rules as in the command (more than 0 up to 100, under the limit of power, from 1 to 10 of them).

### `[trace]`

The contours of a picture (`phoenix-laser trace IMAGE OUT.nc --width-mm W`): the grey levels are split at `threshold` into ink and paper, the border between them is found with marching squares (the crossing of every edge is placed by the grey levels on both sides, so the line does not follow the steps of the pixels), the pieces of border are joined into closed rings and each ring is simplified until no point is farther than `simplify_mm` from the line. The rings become the shapes of one layer called `trace`, which is cut (or, with `--engrave`, filled with lines) with the settings of `[layer trace]` or `[defaults]`, the options of `cut` and every check of a drawing. A picture that asks for more than a limit of this section is an error that names the limit; nothing is left to run for a long time.

| Key | Default | Meaning |
|---|---|---|
| `threshold` | 128 | from 0 to 255: a grey level under it is ink (over it with `invert`); `--threshold` overrides it |
| `invert` | off | the light parts are ink and the dark parts are paper; `--invert` turns it on |
| `simplify_mm` | 0.02 | a point is dropped when the line stays within this distance (0 to 5 mm) without it; 0 keeps them all; `--simplify` overrides it |
| `min_area_mm2` | 0.05 | a contour that encloses less than this is a speck and is left out with a note (0 to 1000000); `--min-area` overrides it |
| `max_pixels` | 4000000 | a picture with more pixels is reduced to fit before it is followed (1 to 25000000), with a note |
| `max_contours` | 20000 | a picture with more contours is an error (1 to 1000000) |
| `max_segments` | 2000000 | a picture with more pieces of border is an error (1 to 50000000): it is too detailed to follow its outline |

The keys of `[trace]` are `threshold`, `invert`, `simplify_mm`, `min_area_mm2`, `max_pixels`, `max_contours` and `max_segments`.

### `[tone]`

The tone curve of a picture before it becomes dots. It is the starting point of `phoenix-laser engrave` when `--config` is given (a material over it, the options `--gamma`, `--contrast`, `--brightness`, `--invert` and `--auto-level` over both) and the starting values of the boxes of the window when the file is loaded. With the default values the picture is not changed at all.

| Key | Default | Meaning |
|---|---|---|
| `gamma` | 1 | from 0.1 to 10: more than 1 lightens the middle greys, less than 1 darkens them; black and white stay where they are |
| `contrast` | 0 | from -100 to 100: negative flattens the picture, positive pushes the greys away from the middle |
| `brightness` | 0 | from -100 to 100: a percent of the full range added to every level (positive makes the picture lighter, so it burns less) |
| `invert` | off | black becomes white and white becomes black |
| `auto_level` | off | the darkest level of the picture becomes black and the lightest becomes white |

The keys of `[tone]` are `gamma`, `contrast`, `brightness`, `invert` and `auto_level`.

### `[verify]`

The check of a program (`phoenix-laser verify`). Work outside the bed is a failure. A program made by another program often runs the head a little past the bed with the laser off, which is the overscan of its hatch lines, so the person can say how far that is allowed; the option `--overscan-mm` of the command takes the place of this key. In the window, a G-code program that is opened to be seen is checked the same way and the key is the box "Overscan allowed past the bed (mm)" of that mode.

| Key | Default | Meaning |
|---|---|---|
| `overscan_mm` | 0 | how far (mm) the head may go past the bed with the laser off, from 0 to 100; 0 allows none. A move that burns outside the bed fails whatever this says |
| `arc_tolerance_mm` | 0.01 | how far (mm) the straight pieces that an arc (`G2`, `G3`) of the program is cut into may lie from the true arc, from 0.0005 to 1; a finer tolerance gives more pieces. The length, the time, the box and the check against the bed follow the curve |

The keys of `[verify]` are `overscan_mm` and `arc_tolerance_mm`.

### `[send]`: what is held back before something goes to the server

The laser plugin of the EH3D server is a simulator that refuses more than 40 % of full power until its own limit is changed, so a program, a vector job and a test fire are held here first to the lower of the limit of the machine (`max_power_percent` of `[machine]`) and this one, and the refusal says which limit it was. Nothing local uses it: the engraving, the cutting and the other commands keep the limit of the machine alone.

| Key | Default | Meaning |
|---|---|---|
| `max_power_percent` | 40 | the most power (a percent of full power) that is sent, from 0.1 to 100 |

### `[zone NAME]`: a forbidden zone of the bed

A rectangle of the bed, in millimetres of the machine, that a program must keep out of. The name is read in any case and a zone written twice is one zone. A program that reaches a zone fails `phoenix-laser verify`, the check of the window and the check before a program is sent to the server (the zone, how many moves reach it and the line of the first are told), and a drawing sent as a vector job is refused when one of its paths reaches a zone of either kind. The edges of a zone count as inside, so a move along an edge is a hit, and the arcs of a program are followed along the curve. At most 64 zones.

| Key | Default | Meaning |
|---|---|---|
| `x_mm`, `y_mm` | 0 | the lower left corner, from -100000 to 100000 |
| `width_mm`, `height_mm` | none | the size, more than 0 and up to 100000; a zone with no size is refused |
| `kind` | `no_go` | `no_burn`: the laser must be off inside it, the head may cross it (a clamp lower than the head); `no_go`: the head must not enter it at all (a fixture, a camera, the end of a rail) |

`examples/safety.cfg` shows a zone of each kind and the interlocks.

### `[interlocks]`

The state of the emergency stop, of the enclosure and of the air assist as the file says it. This program reads no sensor: a simulator, a gateway or the person writes the state here, and nothing in it has been tried against a real sensor or a real emergency stop. A pressed emergency stop, and a required interlock that is not in its safe state, fail `verify` and stop a program, a vector job and a test fire from being sent; a state that is not known counts as unsafe. With the defaults nothing is required and nothing changes.

| Key | Default | Meaning |
|---|---|---|
| `estop` | `released` | `released` or `pressed`; pressed stops everything |
| `enclosure` | `unknown` | `closed`, `open` or `unknown` |
| `air_assist` | `unknown` | `on`, `off` or `unknown` |
| `require_enclosure` | off | nothing is sent or fired unless the enclosure is `closed` |
| `require_air_assist` | off | nothing is sent or fired unless the air assist is `on` |

### `[calibrate]`

The test grid made from a material (`phoenix-laser calibrate --material NAME --materials FILE`): a grid of squares with the powers and the feeds spread around the power and the speed of the material, and, when asked, the power of each column and the feed of each row burnt next to the squares. The powers are held between `min_power_percent` (1 %) and `max_power_percent`, the feeds between `min_feed_mm_min` (10 mm/min) and `max_speed_mm_min`, and a value that comes out twice is given once, so the grid never asks for more than the limits allow; the material itself over a limit is refused. `--powers` and `--feeds` give a list and win over the ladder, and the options `--steps`, `--spread`, `--labels`, `--no-labels` and `--label-mm` take the place of these keys. With a material the figures come by default and `--no-labels` takes them away; without a material the output is the same as ever, with figures only when `--labels` asks for them.

| Key | Default | Meaning |
|---|---|---|
| `steps` | 5 | how many powers and how many feeds the ladder has, from 1 to 12 |
| `spread_percent` | 40 | how far the ladder goes below and above what the material says, as a percent of it, from 0 to 90 |
| `labels` | on | with a material, burn the power of each column and the feed of each row next to the squares, at the power and the speed of the material (`--labels` asks for them with lists too) |
| `label_mm` | 3 | the height of the figures, from 1 to 20 mm |
| `min_power_percent` | 1 | the lowest power of the ladder, more than 0 up to 100 % of full power: the ladder is held at it and never goes under |
| `min_feed_mm_min` | 10 | the lowest feed of the ladder, more than 0 up to 30000 mm/min |

In the window, the calibration mode lists the materials of the library: with one picked the grid is made with this ladder and the keys are boxes of the side panel (the figures are a switch), and with none picked it is the grid of the two lists of boxes.

The keys of `[calibrate]` are `steps`, `spread_percent`, `labels`, `label_mm`, `min_power_percent` and `min_feed_mm_min`.

### `[text]`

A line of text burnt with the font of strokes of the program (`phoenix-laser text OUT.nc --text TEXT`): capitals, figures, space and `. , - + / : ! ? ( ) = _ '`, each a few strokes on a lattice 4 wide and 6 high. Lower case is burnt as capitals and a new line starts a row; a character that the font has not is refused. The lower left corner of the text is at `--origin X,Y` (the machine origin otherwise), and a text that would go outside of the bed (`--bed X,Y` or the machine of the file) is refused. The power and the speed obey the limits of the machine (`--max-power`, `--max-speed`), and the options `--height-mm`, `--spacing`, `--line-spacing`, `--power`, `--feed` and `--passes` take the place of these keys.

| Key | Default | Meaning |
|---|---|---|
| `height_mm` | 10 | the height of a capital, from 1 up to `max_height_mm` |
| `max_height_mm` | 100 | the tallest letter that is accepted, from 1 to 1000 mm: a bigger one is refused, never cut down |
| `spacing_percent` | 25 | the gap between two letters, as a percent of the height, from 0 to 200 |
| `line_spacing_percent` | 50 | the gap between two rows, as a percent of the height, from 0 to 300 |
| `power_percent` | 20 | more than 0 up to 100, a percent of `s_max` |
| `speed_mm_min` | 1000 | from 1 to 30000 |
| `passes` | 1 | how many times the whole text is burnt, from 1 to 20 |
| `max_chars` | 200 | the most characters a text may have, from 1 to 5000 |
| `max_minutes` | 240 | the longest time that the burning itself may take (the length of every stroke over the speed, all the passes together), from 1 to 1440 minutes: a text that would take longer is refused, never cut down (`--max-minutes`) |
| `align` | left | where the rows of a text of several lines start: `left`, `center` (or `centre`) or `right` (`--align`); the box of the text is the same |
| `mirror` | off | `on` burns the text as its mirror image, for the back of glass or acrylic read from the front (`--mirror`, and `--no-mirror` to undo it) |

The keys of `[text]` are `height_mm`, `max_height_mm`, `spacing_percent`, `line_spacing_percent`, `power_percent`, `speed_mm_min`, `passes`, `max_chars`, `max_minutes`, `align` and `mirror`.

In the window, the text mode has a box for the words and one box for each of these keys (a file of settings fills them); the text is worked out with the same function as the command, from the origin of the machine, and is refused for the same reasons, with the status line naming the box that is wrong. The bed is the one of the machine of the file.

#### A series of texts: a counter and a CSV file

`phoenix-laser text OUT.nc --text 'N{counter:4} {name}' --csv FILE.csv` writes one program for each row of the CSV file, named `OUT_001.nc`, `OUT_002.nc` and so on (three figures, more when there are more than 999). The text holds places: `{counter}` (or `{counter:5}` for five figures with zeros in front, up to 12), `{row}` (the number of the row, from 1) and `{NAME}` (the column of the file that has that name in its first row); `{{` and `}}` are the braces themselves. A place that names nothing, a column that is not there, `{counter}` with no counter and a text longer than 4096 characters are errors with the code `PHOENIX-LASER-VARTEXT-FIELD`; nothing is guessed. `--counter START[,STEP[,COUNT]]` gives the counter (the step is 1 when left out; the count is the number of rows of the file, and is needed when there is no file), and `--counter-project PROJECT.lbrn2` takes it from the variable text of an LBRN2 project, from its current value to its end. `--preview` shows the text of each row and writes nothing.

The CSV file has the names of its columns in the first row; the separator is a comma (a semicolon when the first row has none), a field in double quotes may hold the separator, new lines and `""` for a quote, blank lines are skipped and a row with another number of fields than the first is an error (`PHOENIX-LASER-VARTEXT-CSV`, with the number of the row). Nothing in the file is ever run: a field is only text, and a character the font of strokes has not stops the series. The file is held to `max_csv_bytes`, `max_csv_rows` (also the longest series a counter may make) and `max_csv_columns` of `[limits]`, and a field of more than 1024 characters is refused. Every program is worked out and checked, with the limits of the machine and the bed of the settings, before the first file is written, so a row that cannot be burnt writes nothing; the header of each file names the CSV file and its SHA-256, and the series never writes over the CSV file, the settings or the project it is made from.

### `[qr]`

A QR code burnt by filling its dark modules with lines (`phoenix-laser qr OUT.nc --text DATA`): versions 1 to 4 (21 to 33 modules a side) in byte mode, with the four levels of error correction and the mask with the lowest penalty. A module is a square of `module_mm`, filled with lines `line_mm` apart, so the symbol is dark where the laser burns; leave a light margin of four modules around it on the material. The lower left corner is at `--origin X,Y` (the machine origin otherwise). The options `--ecc`, `--module-mm`, `--line-mm`, `--power`, `--feed` and `--passes` take the place of the keys with the same meaning, and `--print` shows the symbol as text.

Data that does not fit is refused, never cut down: more than `max_bytes` bytes, a version over `max_version` at the level that is asked for, a symbol larger than `max_size_mm`, a fill of more than `max_lines` burning moves, a power or a speed over the limits of the machine (`--max-power`, `--max-speed`) or a symbol that would go outside of the bed (`--bed X,Y` or the machine of the file). The data is never written into a comment of the program.

| Key | Default | Meaning |
|---|---|---|
| `module_mm` | 1 | the side of a module, from 0.2 to 10 mm |
| `line_mm` | 0.1 | the distance between the lines that fill a module, from 0.02 mm up to `module_mm` |
| `power_percent` | 20 | more than 0 up to 100, a percent of `s_max` |
| `speed_mm_min` | 1000 | from 1 to 30000 |
| `passes` | 1 | how many times the whole symbol is burnt, from 1 to 20 |
| `max_bytes` | 40 | the most bytes of data, from 1 to 78 |
| `max_version` | 4 | the largest version of the symbol, from 1 to 4 (a version holds 14, 26, 42 or 62 bytes at level M) |
| `ecc` | M | the level of error correction: `L`, `M`, `Q` or `H` |
| `max_size_mm` | 100 | the largest side of the symbol, from 1 to 1000 mm |
| `max_lines` | 100000 | the most burning moves of the program, all the passes together, from 1 to 1000000 |
| `border_modules` | 0 | the width in modules, from 0 to 8, of a burnt border around an inverted symbol, so it is left bare in a burnt plate (`--border-modules`, only with `invert`); the square with its border counts as the symbol for the origin, the bed and `max_size_mm` |
| `max_minutes` | 240 | the longest time that the burning itself may take (the length of every burnt line over the speed, all the passes together), from 1 to 1440 minutes: a symbol that would take longer is refused, never cut down (`--max-minutes`) |
| `mirror` | off | `on` burns the symbol as its mirror image, for the back of glass or acrylic read from the front (`--mirror`, and `--no-mirror` to undo it) |
| `overscan_mm` | 0 | the run-up at each end of a burnt line, from 0 to 20 mm: the head runs up to the line and out past its end with the laser off, so it is at speed all along the burnt part (`--overscan-mm`); a run-up that would leave the bed is refused |
| `invert` | off | `on` burns the light modules instead of the dark ones, so the symbol is left bare in a burnt plate (`--invert`, and `--no-invert` to undo it); the limits hold over the light modules too |

The keys of `[qr]` are `module_mm`, `line_mm`, `power_percent`, `speed_mm_min`, `passes`, `max_bytes`, `max_version`, `ecc`, `max_size_mm`, `max_lines`, `max_minutes`, `overscan_mm`, `mirror`, `invert` and `border_modules`.

In the window, the QR mode has a box for the data, a choice of `ecc`, a switch for `invert` and one box for each of the other numbers (a file of settings fills them); the symbol is worked out with the same function as the command, from the origin of the machine, and is refused for the same reasons, with the status line naming the box that is wrong. The bed is the one of the machine of the file.

### `[barcode]`

A Code 128 barcode burnt by filling its bars with lines along their height (`phoenix-laser barcode OUT.nc --text DATA`): the characters from space to `~` are written in set B, and a text of an even number of figures only is written in set C (two figures to a symbol), which is half as wide. A symbol is 11 modules wide and the stop is 13, and a module is `module_mm` wide, so the barcode is dark where the laser burns; leave a light margin of ten modules at each side on the material. The lower left corner is at `--origin X,Y` (the machine origin otherwise). The options `--module-mm`, `--height-mm`, `--line-mm`, `--power`, `--feed` and `--passes` take the place of the keys with the same meaning.

Data that cannot be written is refused, never cut down: a byte that is not a printable character, more than `max_chars` characters, a barcode wider than `max_width_mm`, a fill of more than `max_lines` burning moves, a burning that would take more than `max_minutes`, a power or a speed over the limits of the machine (`--max-power`, `--max-speed`) or a barcode that would go outside of the bed (`--bed X,Y` or the machine of the file). The data is never written into a comment of the program.

| Key | Default | Meaning |
|---|---|---|
| `module_mm` | 0.4 | the width of the narrowest bar, from 0.1 to 5 mm |
| `height_mm` | 10 | the height of the bars, from 1 to 100 mm |
| `line_mm` | 0.1 | the distance between the lines that fill a bar, from 0.02 mm up to `module_mm` |
| `power_percent` | 20 | more than 0 up to 100, a percent of `s_max` |
| `speed_mm_min` | 1000 | from 1 to 30000 |
| `passes` | 1 | how many times the whole barcode is burnt, from 1 to 20 |
| `max_chars` | 20 | the most characters of data, from 1 to 80 |
| `max_width_mm` | 150 | the widest barcode, from 1 to 1000 mm |
| `max_lines` | 100000 | the most burning moves of the program, all the passes together, from 1 to 1000000 |
| `max_minutes` | 240 | the longest time that the burning itself may take (the length of every line over the speed, all the passes together), from 1 to 1440 minutes (`--max-minutes`) |
| `mirror` | off | `on` burns the barcode as its mirror image, for the back of glass or acrylic read from the front (`--mirror`, and `--no-mirror` to undo it) |

The keys of `[barcode]` are `module_mm`, `height_mm`, `line_mm`, `power_percent`, `speed_mm_min`, `passes`, `max_chars`, `max_width_mm`, `max_lines`, `max_minutes` and `mirror`.

In the window, the barcode mode has a box for the data, a switch for `mirror` and one box for each of the other keys (a file of settings fills them); the barcode is worked out with the same function as the command, from the origin of the machine, and is refused for the same reasons, with the status line naming the box or the reason.

### `[marks]`

Positioning marks burnt with a low power (`phoenix-laser marks OUT.nc --size W,H`): an L at each corner of the rectangle of the work, and a cross in the middle when `centre_cross` is on, so that the material can be put back in the same place after it is taken out and a second job can be lined up with the first. The rectangle starts at `--origin X,Y` (the machine origin otherwise) and is `W` by `H` mm; the marks lie on its edges, or `margin_mm` outside of them. Unlike the frame, which is traced with the laser off, the marks are burnt: they obey the limits of power and of speed of the machine (`--max-power`, `--max-speed`) and are refused when they would go outside of the bed (`--bed X,Y` or the machine of the file) or when an arm is longer than half a side of the rectangle that the marks go round. The options `--arm-mm`, `--margin-mm`, `--power`, `--feed`, `--passes` and `--centre-cross` (and `--no-centre-cross`) take the place of the keys with the same meaning.

| Key | Default | Meaning |
|---|---|---|
| `arm_mm` | 5 | the length of each arm of an L, from 1 to 100 mm; at most half a side of the rectangle the marks go round |
| `margin_mm` | 0 | how far outside of the rectangle of the work the marks are burnt, from 0 to 50 mm |
| `power_percent` | 10 | more than 0 up to 100, a percent of `s_max` |
| `speed_mm_min` | 1000 | from 1 to 30000 |
| `passes` | 1 | how many times the marks are burnt, from 1 to 20 |
| `centre_cross` | off | `on` burns a cross in the middle of the rectangle too, with arms half as long as an arm of an L |

The keys of `[marks]` are `arm_mm`, `margin_mm`, `power_percent`, `speed_mm_min`, `passes` and `centre_cross`.

In the window, the marks mode has boxes for the size of the rectangle (50 x 50 mm until they are changed), one box for each key and a switch for `centre_cross`; the lower left corner of the rectangle is the origin of the machine, and the marks are refused for the same reasons as by the command. A button, `Size from the work`, puts the width and the height of what the open picture, drawing or outline burns in the two boxes (the box the frame takes, rounded up to a hundredth of a millimetre and at least 1 mm), so that the marks go round exactly the work; the marks are placed from the origin, where the work is placed too.

Where the material lies, from a picture (`phoenix-laser align`): `align PICTURE.pgm|png --size W,H --px-per-mm S` finds the marks that `marks` burns (the same `--size`, `--origin`, `--arm-mm`, `--margin-mm` and `--centre-cross`, or the `[marks]` section of `--config`) in a picture of the bed taken from straight above. The picture has the machine axes along its rows and columns, the origin of the machine at its lower left corner and `S` pixels to the millimetre; a dark pixel is one at or below `--threshold` (128 by default), and a dark spot of fewer than 4 pixels is dust. Each mark that should be there takes the nearest spot within `--max-shift-mm` (20 by default), and the turn and the shift that fit them best are printed: the shift (`dx`, `dy`, in mm, to add to the origin of the work, with the `--origin` to use next) and the turn in degrees about the origin of the machine, with how far the marks are from lying as one piece. A picture in which a mark is missing, a turn over `--max-angle-deg` (5 by default), marks that are over 1 mm from the best fit and a picture with more than 256 dark spots are refused with the reason. `align --simulate OUT.pgm --size W,H --px-per-mm S [--shift DX,DY] [--turn DEG] [--picture-px W,H]` draws the picture a simulated camera would take of the marks moved and turned by a known amount (the picture covers the bed of the settings, or the marks and 10 mm round them); the tests read it back to a fraction of a millimetre and of a degree. The command reads a picture file, not a camera, and it has only been tried on simulated pictures: the lens, the light and the dust of a real bed, and a mark cut by the edge of the picture, are still to be tried on a real camera. It writes nothing but the simulated picture, and never over the settings it is made from.

### `[server]`

The way to the EH3D server (`phoenix-laser server`): a G-code program goes to the `laser` plugin of EH3D-SERVER as a job, and the job is followed, held, let go on and stopped. The link is the shared client of the suite (`eh3d-serverclient` of EH3D-COMMON), so the keys have the names and the ranges every program of the suite reads. Nothing is sent while `server_address` is empty.

| Key | Default | Meaning |
|---|---|---|
| `server_address` | empty | `host:port` of the server (for example `192.168.0.180:18200`); empty means there is no server; no space or control character |
| `server_token` | empty | the token of this program; it is never printed, and the variable `PHOENIX_LASER_SERVER_TOKEN` is used when the file does not give one; no space or control character |
| `server_machine` | empty | the id of the laser machine on the server: letters, digits, `_` and `-`, up to 64 |
| `server_timeout_s` | 15 | the seconds one try may take, from 1 to 120 |
| `server_retries` | 2 | the tries more after a failure on the network, from 0 to 5; a request that makes something is never sent twice |
| `server_mdns` | off | `on` looks for the server on the local network; the program has no resolver of its own yet, so `on` finds nothing |

The keys of `[server]` are `server_address`, `server_token`, `server_machine`, `server_timeout_s`, `server_retries` and `server_mdns`. `--address HOST:PORT` and `--machine ID` of the command win over the file.

In the window, the section for the server shows the panel that every program of the suite shows (`eh3d-view` of EH3D-COMMON): the boxes for the address, the token of the program, the id of the machine and the limits of the link (`server_timeout_s`, `server_retries` and `server_mdns`), a button that tests the connection, the state of the link and a login with a name and a password for the application `laser`. The login keeps its token in memory only and the job that is sent uses it when the settings give no token; what is applied in the panel is used from then on, and the settings file is not rewritten. Below the panel, a button sends the program that the window has worked out (after the guards of the export: a frame that lights the laser and was not confirmed is not sent), shows how far the job is, and holds it, lets it go on or stops it with the emergency stop. A G-code program that was opened to be seen is sent as it is only when the check of the window passes for exactly the bytes that are open, the check against the limits of `[machine]` passes too and the box `I checked this program and send it as it is` is ticked (it is asked again for every program that is opened). Closing the window leaves the job where it is on the server.

`phoenix-laser server vector DRAWING.svg|dxf` sends a drawing as a `vector` job instead of a program: the paths of the layers that are not skipped, in the millimetres of the machine with the y axis up (the origin and the alignment of the settings apply), and one operation for each layer from the settings (`cut` or `engrave`, the power, the speed and the passes, with the options of the drawing commands over them). The limit of power and of speed and the bed of `[machine]` are checked first, as they are for a program, and at most 400000 points go. The plugin of the server makes its own program from the shapes, so the kerf, the tabs and the hatch of a layer are not applied; the command says so for each layer that asked for one (write the program with `phoenix-laser vector` and use `send` to apply them). `--follow` stays until the job ends.

`phoenix-laser server test-fire --power-percent P [--seconds S] --confirm-fire` fires the laser for a moment with no job, to find the focus. The power is above 0 and up to the limit of power of `[machine]`, the time is from 0.01 to 10 seconds (0.05 when it is not given), and because it lights the laser it is not sent without `--confirm-fire`. The server refuses it while a job runs, while an emergency stop holds the machine, with the enclosure open or the air assist off when the machine asks for them, and above its own limit of a test fire (`test_fire_max_pct` and `test_fire_max_s`, lower than the ones of this program); the refusal is told by its code in the language of the system and nothing is fired. A test fire is never sent twice.

The token travels in the clear over plain HTTP: use it on a network you trust or through a tunnel; the program says so when the address is not a private one. `send` checks the program against the limits of `[machine]` (the limit of power and of speed, the bed) as `verify` does, and a program that fails, or one of more than 200000 lines, is not sent; the server holds the job to its own limits too. `stop` is the emergency stop of the laser machine: the job ends at once and the machine is latched, so that no program is taken until an administrator runs `release`; a machine that says it is latched is not sent a program. Nothing here has run against a real laser, and the `laser` plugin of the server is a simulator so far.

### `[job]`: where the work starts and how long it takes

The work is placed by one of the nine points of its box, and that point goes to the origin of the machine (`origin_x` and `origin_y` of `[machine]`). With `align = page` in `[read]` the drawing is not moved at all (an absolute start: the coordinates are the ones it was drawn with, plus the origin) and `anchor` has no effect. The acceleration is the one the time of the work counts at every corner; the time is given as a range, from every move at its own feed with no braking at all to the time of the preview of the program, which brakes at every corner (the passes are in both, because the program holds each of them).

| Key | Default | Meaning |
|---|---|---|
| `anchor` | bottom_left | the point of the box of the work that goes to the origin: `bottom_left`, `bottom`, `bottom_right`, `left`, `center`, `right`, `top_left`, `top` or `top_right` |
| `accel_mm_s2` | 1000 | the acceleration of the head for the time of the work, from 1 to 100000 mm/s2 |

The keys of `[job]` are `anchor` and `accel_mm_s2`.

### `[live]`: the range of the live adjustment

While a job runs, the power and the speed can be adjusted as a percent of what the program has. The adjustment is only a calculation here, held by the range below, by the limit of power and of speed of `[machine]`, by the limit sent to the server (`[send]`) and, for the low power of the pilot, by `max_pilot_percent` of `[frame]`; the lowest limit wins, and a value that was held is told with the limit that held it. Nothing is sent to a machine. By default the adjustment can lower a value to a tenth and never raise it.

| Key | Default | Meaning |
|---|---|---|
| `power_min_percent` | 10 | the lowest percent of the power of the program that can be asked, from 1 to 200 and no more than `power_max_percent` |
| `power_max_percent` | 100 | the highest percent of the power of the program that can be asked, from 1 to 200; the limits of the machine and of the server still hold over it |
| `speed_min_percent` | 10 | the lowest percent of the speed of the program, from 1 to 200 and no more than `speed_max_percent` |
| `speed_max_percent` | 100 | the highest percent of the speed of the program, from 1 to 200; the limit of speed of the machine still holds over it |

The keys of `[live]` are `power_min_percent`, `power_max_percent`, `speed_min_percent` and `speed_max_percent`.

### `[defaults]` and `[layer NAME]`

| Key | Default | Range | Meaning |
|---|---|---|---|
| `mode` | cut | `cut`, `engrave`, `skip` | what is done with the layer; a skipped layer is not even used to place the work |
| `power` | 50 | more than 0 up to 100 | percent of `s_max` |
| `speed` | 600 | 1 to 30000 | mm/min |
| `passes` | 1 | 1 to `max_passes` | how many times |
| `focus_step` | 0 | -20 to 20 | mm the focus moves on at each pass (the total may not pass 100 mm) |
| `kerf` | 0 | 0 to 5 | width of the beam in mm |
| `kerf_side` | none | `none`, `auto`, `inside`, `outside` | on which side of the line the beam goes |
| `tabs` | 0 | 0 to 100 | bridges left on the outline of a piece |
| `tab_width` | 1 | 0 to 100 | width of a bridge in mm |
| `hatch_angle` | 0 | -360 to 360 | degrees |
| `hatch_spacing` | 0.1 | 0.01 to 5 | mm between the lines |
| `hatch_cross` | off | `on`, `off` | fill a second time with lines at right angles to the first ones (`hatch_angle` + 90 degrees), in every pass; the limit of lines holds over each set |
| `order` | 0 | any whole number | the layers of a kind are done from the lowest |

### `[sublayer LAYER/LABEL]`: an extra pass over a layer

A layer can be worked more than once with other settings: a fill and then an outline, a light mark and then a deep cut. Each extra pass is a `[sublayer LAYER/LABEL]` section, where `LAYER` is a layer that has its own `[layer LAYER]` section and `LABEL` is 1 to 40 letters, figures, `-` or `_` that tell the passes of the layer apart (a layer that is itself named with a slash is split at its last slash). A sub-layer works the same shapes as its layer, starts from the settings of the layer (wherever in the file the layer is written) and changes what it says, with the same keys and ranges as the table above. A layer has at most 8 sub-layers and a file at most 512. Every sub-layer is held by the limits of `[machine]` like a layer is (a sub-layer that is `skip` is not), and an error names the sub-layer, as `sublayer red/outline`. The program writes each pass under a comment `; layer red/outline: ...`. A layer that is `skip` is skipped with all its sub-layers, and a drawing sent to the server as a vector job carries the layers only (a note says the sub-layers are not sent).

## Not tried

Nothing here has cut or engraved anything on a machine: the program is tested on a computer with drawings written by hand, and the G-code has to be checked by a person before it is run.

# The library of materials

What suits a material is found once with the test grid (`phoenix-laser calibrate`) and then used again and again. A library is a plain text file with one `[material NAME]` section for each material **and way of working** (the cut of 3 mm birch plywood and its engraving are two entries). `examples/materials.cfg` shows every key. The settings file of the drawings is described in [vector-settings.md](vector-settings.md). The file is chosen with `--materials FILE` or with the variable `PHOENIX_LASER_MATERIALS`, and one material of it with `--material NAME` (capitals and the spaces around the name do not matter) on `engrave`, `cut`, `vector` and `layers`; `phoenix-laser materials FILE` lists what a library holds.

Every key is optional, and a command takes from a material only what it says. The order is: the settings file, then the material (for a drawing, over `[defaults]` and every layer, like an option of the command line), then the options of the command line, which win. The limit of power (`max_power_percent`, `--max-power`) holds over all of them: a material whose power is over it is an error, never lowered.

| Key | Range | Used by | Meaning |
|---|---|---|---|
| `note` | up to 300 characters | all | a reminder shown in the list |
| `power` | 0.001 to 100 | all | percent of full power |
| `speed` | 1 to 30000 | all | mm/min (the feed of an engraving) |
| `passes` | 1 to 1000 | drawings | how many times |
| `focus_step` | -20 to 20 | drawings | mm the focus moves at each pass |
| `kerf` | 0 to 5 | drawings | the width of the beam, mm |
| `hatch_spacing`, `hatch_angle` | 0.01 to 5, -360 to 360 | drawings | the hatch of the shapes that are engraved |
| `hatch_cross` | `on` / `off` | drawings | fill the engraved shapes a second time at right angles to the first lines |
| `dot` | 0.01 to 5 | pictures | the size of a dot, mm |
| `overscan` | 0 to 50 | pictures | the run past the work at each end of a line, mm |
| `dither` | `fs`, `atkinson`, `threshold`, `bayer` | pictures | how the grey levels become dots |
| `threshold` | 0 to 255 | pictures | the level of `dither = threshold` (128 when it is not said) |
| `gamma`, `contrast`, `brightness` | 0.1 to 10, -100 to 100, -100 to 100 | pictures | the tone curve (see `phoenix-laser help`) |
| `invert`, `auto_level` | `on` / `off` | pictures | the tone curve |

The library is read as a whole, so a line that is not understood, a key that is not known, a value out of its range, a name used twice or a file larger than 1 MiB is an error that names the line. The values are the ones the person found for their own machine: the program knows no material.

In the window the material section edits a library without a text editor: a material can be edited, copied, made from the boxes of the window and deleted, and a new library can be created. One box for each key of the table holds its value (an empty box leaves it out), and saving rewrites the section of that material in its place, or adds it at the end when it is new, and changes nothing else in the file: the comments, the blank lines, the order, the line ends and the other materials stay as they were. A value out of the range of the table, a name that is empty, longer than 80 characters or already used, a note over 300 characters, a library that would pass 1 MiB or 500 materials, and a file that is not UTF-8 text or not a valid library are refused with the key named and the file untouched; nothing is cut down. Deleting needs a second press of the button, and a new library is never written over a file that is there.


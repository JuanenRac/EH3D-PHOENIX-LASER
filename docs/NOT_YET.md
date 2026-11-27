# EH3D-PHOENIX-LASER: not yet

Good ideas that are not in the current version. Writing them here keeps anybody from implementing them early or twice.

| Idea | Why not yet | Revisit when |
|---|---|---|
| A real laser as the first proof | Nothing has been burnt; the proof needs a person at a machine with the emergency stop in reach | the owner has a laser ready for the first run |
| Pictures of an LBRN2 project in a format other than PNG or PGM | The program has no decoder for a JPEG and a project of the real examples held only a PNG | a project that the owner needs holds a JPEG |
| The picture options of an LBRN2 layer (gamma, crop, the dither of the layer) | The picture is engraved with the options of `engrave`; the options of the layer were not studied | a project that the owner needs depends on them |
| The variable text of a shape of an LBRN2 project that names a column | Only the counter of the project is read; a series is made by `text` with its own places | the owner has a project that does it |
| The fill pattern of a hatch, and the text and dimensions of a DXF | The safe subset reads lines, arcs, circles, ellipses, splines, blocks and the outline of hatches and solids; the rest is skipped with a note | a drawing of the owner needs one of them |
| The power that follows the speed in the corners of a cut | The controllers have a dynamic power mode that does it, and the program writes for it | a controller without that mode is used |
| Reading the marks with a real camera | The finding of the shift and the turn is tried on simulated pictures; a camera needs the vision tool of the suite | the vision tool gives a picture of the bed |
| Rotary and other axes | The program writes for a flat bed | the owner has a rotary |
| Nesting of parts on a sheet | The cut works on the drawing as it is; nesting needs its own engine and limits | the owner asks for it |
| Material tests stored by the server | The library is a plain file next to the settings and the server stores jobs only | the server offers a place for them |
| An independent review of the checks | The checks have not been reviewed by anyone but their author | a reviewer is found |
| Sub-layers, linked presets and the live adjustment in the window | The model, the limits and the tests are done; the window has no table, box or slider for them and the links of the presets are not kept in the settings file | the window is looked at on a screen and the owner asks for them |
| Sending a live adjustment to a job that runs | Nothing here talks to a machine; the plugin of the server has no route for it and it needs a person at a laser | the server offers a route and a machine is ready for a first run |
| Reading the settings of the controller from its port, and writing them | The program opens no port: a person saves the answer to $$ in a file and nothing is ever written | a machine is ready for a first run and the owner asks for it |
| A switch that turns a different full power of the controller into a warning | A different value gives another power than the one asked, so it stays serious | a real machine shows a case where it is safe |

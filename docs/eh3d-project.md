# The `.eh3d` project and the branch `laser/`

An `.eh3d` file is the project of the EH3D Fabrication Suite: one ZIP file with a branch for each program (`pcb/`, `cad/`, `3dp/`, `laser/`, `cnc/`...). The format is defined in `docs/FORMAT_EH3D.md` of EH3D-COMMON and read and written by its crate `eh3d-project`. `phoenix-laser` writes only `laser/`; every other branch, known or not, keeps its bytes.

## The commands

| Command | What it does |
|---|---|
| `project list FILE.eh3d [--json]` | the branches, who wrote each one, their format, and what is out of date or wrong |
| `project save FILE.eh3d [--drawing D] [--config C] [--materials M] [--program P] [--name N] [--id ID]` | puts the files in `laser/`; the project is created when it is not there (the name and the identifier are only for a new one); what is not given stays |
| `project import FILE.eh3d DRAWING` | takes an LBRN2, LBRN, SVG or DXF file, keeps it as it is under `laser/origin/` with `report.json`, and makes it the drawing |
| `project take FILE.eh3d BRANCH NAME` | copies a file of another branch to `laser/taken/BRANCH/NAME` with a reference to the state of its source; a drawing becomes the drawing of the work, a mesh is only kept |
| `project open FILE.eh3d [--into DIR]` | reads and checks `laser/` and says what it holds; with `--into` writes its files |
| `project extract FILE.eh3d BRANCH DIR` | writes the files of any branch, byte for byte |

`--config` of `import`, `open` and `take` gives the `[read]` settings with which a drawing is read. A file that is there is never written over by `open` or `extract`. The project is replaced in one step (a temporary file, then a rename).

What every program does with its branch (open or create, save, import with a report, take, out-of-date notices, list and extract) is the module `appbranch` of the crate `eh3d-project`; this program says what its files are. `list` says when a branch is signed. Writing the branch removes a signature of the project, and a signature of the branch when there is one; the command says so. These commands neither sign nor check signatures: `eh3d-project` of EH3D-COMMON does.

## In the window

File > Suite project (.eh3d) does the same with the window: Open suite project, Save suite project, Save suite project as, Import a file and keep the original, Take from another branch, Extract a branch and Branches of the project. A project can also be dropped on the window or given on the command line.

- Open replaces the work of the window with what `laser/` holds: the drawing, the settings, the library of materials and, with no drawing, the first program. A project without `laser/` is taken as it is, with no work, and Save puts the work there. A project that cannot be read changes nothing.
- Save puts the drawing, the settings file and the library that were chosen in the window (or that the project held) and the program that was worked out and is up to date. A file the window read by itself when it started is not put, and neither is a settings file with the token of the server. What the project holds already is not written again, so a file taken from another branch keeps its reference. What is typed in the boxes is not kept, and a picture to engrave has no place in the branch: only its program goes.
- Save as to a new file copies the whole project first, so every branch goes with it. A file that is there keeps its own branches: only `laser/` is written, after a question.
- The window asks before writing a project it did not open, a `laser/` that changed since it was read, or one whose signature (of the project or of the branch) the write removes.
- A bar under the toolbar names the project and says what is OUT OF DATE and what is signed. It is read again when the window gets the keyboard back.
- The settings of a project are applied over those of the person, and never loosen them: the server, what is held back before a send, the interlocks and the machine profiles stay as the person has them, the limits of power and speed are the lower of the two and the forbidden zones of both hold.

## What is in the branch

```
laser/
  branch.json          written by the container: program, format 1.0, content hash, references
  laser.json           which file is the drawing, the settings, the materials, the programs
  drawing/NAME         a drawing given to `save`
  settings/laser.cfg
  settings/materials.cfg
  programs/NAME.nc
  origin/NAME          the original of the last import
  origin/report.json   what was read, not read and lost on export
  taken/BRANCH/NAME    copies of files of other branches
```

Fields of `laser.json` and of the record that this version does not know, and files it does not know, are kept. A branch of another format than 1.0 is not written (`EH3D-PKG-024`); a sealed branch or project is not written (`EH3D-PKG-020`).

## Out of date

A copy made with `take` keeps the content hash and the format of the branch it came from. When that branch changes, `list` and `open` say `OUT OF DATE`; take the file again to bring it up to date. Nothing is ever changed on its own.

## Limits

An import keeps one original at a time: a second import replaces the first and says so. A LightBurn project cannot be written back: `convert` writes SVG or DXF, without power, speed, passes or pictures. Pictures of a project are not part of the vector drawing.

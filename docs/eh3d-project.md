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

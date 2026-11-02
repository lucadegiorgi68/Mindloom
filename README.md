# Mindloom

Organic mind mapping in Rust, for macOS and Windows. Personal project, clean
room: behaviour is specified in `docs/behaviour_spec.md` and all artwork is original.

The signature feature is the organic drawing: branches taper from thick to thin,
swell and drift slightly (deterministically, so a map always looks the same),
labels ride on underlines that continue the branch, and the central idea is a
hand drawn marker frame whose stroke overshoots its start.

## Links

Select a node and press `L` (or the Link button), then click another node to
draw a curved relation between them. Click a link to choose a solid or dashed
line, arrowheads (none, end, start, both) and a colour; `Delete` removes it.
Links follow their nodes, survive folding, and are exported to SVG and saved in
`.mindloom` files.

With a link selected, drag the dot on its curve to bend it (or drag the curve itself), and drag either end dot onto another node to reattach it. Each drag is one undo step.


## Fonts

Seven open licensed fonts are bundled in `ui/fonts` (SIL Open Font License, see
`ui/fonts/LICENSES.txt`) so maps look the same on every machine: Open Sans,
Montserrat, Lora, Caveat, Patrick Hand, Pacifico and Source Code Pro. The
catalogue lives in `crates/mindmap_core/src/fonts.rs`, which is the only place
to edit to add a font (drop the `.woff2` files in `ui/fonts`, add an entry, and
add the file names to `crates/mindmap_io/src/fontdata.rs`). Each node has its
own font, new branches inherit their parent's font, and exported SVGs embed
just the fonts and weights the map uses.

## Layout

| Path | Purpose |
| --- | --- |
| `crates/mindmap_core` | Document model, organic geometry, layout, undo and redo. No UI. |
| `crates/mindmap_io` | `.mindloom` files, SVG, Markdown and OPML export, OPML import. |
| `crates/mindmap_api` | Command layer (`Session::invoke`) shared by the app and the dev server. |
| `ui/` | Plain HTML, CSS and JS canvas. Draws the scene Rust returns, sends commands back. |
| `src-tauri/` | Desktop shell (Tauri 2). A thin wrapper around `mindmap_api`. |

## Develop

```
cargo test                                              # 55 tests
cargo run -p mindmap_api --example dev_server           # UI at http://127.0.0.1:5173
cargo run -p mindmap_io --example sample -- out.svg     # render a sample map
```

The dev server runs the real Rust logic behind the real UI in any browser, so
most work needs no desktop toolchain.

## Build installers

Push to GitHub. `.github/workflows/ci.yml` runs the tests on Ubuntu, macOS and
Windows, then builds a `.dmg` (macOS) and `.msi` / `.exe` (Windows) and uploads
them as workflow artifacts. Builds are unsigned (personal use): on macOS
right click the app and choose Open the first time; on Windows choose
"More info" then "Run anyway" in SmartScreen.

Local build, with Rust and Node installed:

```
npm install -g @tauri-apps/cli@2
tauri icon app-icon.png
tauri build
```

## Shortcuts

Tab add branch, Enter add sibling, C add a second central idea, F2 or double click edit, Delete remove, I add a picture as a node under the selected one, Shift+I place a free picture,
Space fold, Ctrl or Cmd Z / Y undo and redo, S save, O open, N new, 0 fit.
Drag a node to move it with its subtree. Shift plus drop on another node
re-parents it. Wheel zooms, dragging the background pans.

## Pictures

A picture can be a node of the tree (toolbar "Image node", key I, or paste, or drop a file on a node) or
sit on the canvas on its own (toolbar "Free image", key Shift+I, or drop a file on empty canvas). A node
can also take a picture on top of its text with the "Image" button of the format bar, and the text then
becomes the caption. Drag a corner dot, or use the Size slider, to resize; the proportions are kept.
PNG, JPEG, GIF, WebP and SVG are accepted. Big bitmaps are scaled down to 1600 px on the longest side.
Pictures are stored once inside the `.mindloom` file (identical ones are shared, unused ones are dropped
on save) and are embedded in the SVG export.

## Known limits

* Text width is estimated from a per font average (measured in a browser, plus
  headroom), so frames can be slightly loose or tight with unusual text such as
  all capitals. Measuring each label for real is a later layout task.
* The Tauri shell is written but has only been built in CI, not on the machine
  that authored it.

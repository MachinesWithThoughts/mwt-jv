# MWT JSON Viewer

Machines With Thoughts JSON Viewer (`mwt-jv`) is a Rust terminal JSON viewer built with [Ratatui](https://ratatui.rs/) and Crossterm. Browse highlighted JSON, fold nested containers, or inspect any node as a table.

The top-right corner shows the version, build timestamp in UTC, and relative build age (for example, `v0.1.0 · built 2026-10-15 19:30:00 UTC (5m ago)`). The age updates while the app is idle. Narrow terminals show a compact version and age. `--version` also prints build information. The timestamp is embedded at build time; unchanged builds retain their timestamp. Reproducible builds can set `SOURCE_DATE_EPOCH` to Unix seconds.

## Run

Requires Rust 1.88 or newer and a Unicode terminal.

Use the launcher to build and run the release version:

```sh
./run.sh examples/demo.json
./run.sh --demo
./run.sh --help
```

The script works from any directory, forwards all arguments, and resolves input paths relative to your current directory. Cargo rebuilds when needed.

Or run Cargo directly:

```sh
cargo run --release -- examples/demo.json
cargo run --release -- --demo
```

Build a standalone executable:

```sh
cargo build --release
./target/release/mwt-jv examples/demo.json
cat examples/demo.json | ./target/release/mwt-jv
./target/release/mwt-jv --help
```

The viewer reads a file, `-`, or piped standard input. Standard output must be a terminal. Input is read locally and never modified. Invalid input produces an error with its source line and column before entering the terminal UI.

## Tree mode

Strings, object keys, numbers, booleans, and null have distinct colors. Each row shows a **line number** and **nesting depth** in the left gutter. Line numbers refer to JSON formatted with two-space indentation, rather than the original file layout; they stay fixed when branches collapse. Root depth is zero. The status bar shows the selected node's line and depth, and the header shows its JSON Pointer.

Containers use **▶** when collapsed and **▼** when expanded. Click a triangle or use the keyboard to toggle it. The triangles use text presentation and a fixed-width slot so highlighting a row does not shift its text. Empty objects and arrays render inline. The first two levels start expanded; deeper containers start collapsed.

## Table mode

Select a node and press **t**. Press **t** again or **Esc** to return to the same tree selection.

- Arrays of objects use the union of their keys as columns, preserving encounter order. A missing key appears as `—`; JSON null appears as `null`.
- Objects use **Key / Value** columns.
- Other arrays use **Index / Value** columns, including mixed arrays and arrays of empty objects.
- Scalars use a single **Value** cell. Empty containers show an empty-state message.

Nested objects and arrays remain compact raw JSON in a single cell, with syntax highlighting. Values are not truncated: scroll horizontally to see wide cells and additional columns. Each table row retains its source line number and depth. **Enter** opens the selected row's node in the tree.

## Controls

| Key / mouse | Action |
| --- | --- |
| ↑ / ↓ or j / k | Move selection |
| → or l | Expand a container, then enter its first child |
| ← or h | Collapse a container, then move to its parent |
| Space / Enter | Toggle selected container in tree mode |
| Click triangle | Toggle container |
| Mouse wheel | Move selection |
| t | Toggle table for selected node |
| Enter in table | Inspect selected row in tree |
| ← / → or h / l in table | Scroll horizontally |
| Shift+← / Shift+→ | Scroll horizontally in either mode |
| 0 | Reset horizontal scroll |
| PageUp / PageDown or b / f | Move one page |
| Home / End or g / G | Select first / last row |
| e / c | Expand / collapse all in tree mode |
| Esc | Return to tree and reset horizontal scroll |
| ? / F1 | Open help; any key closes it |
| Ctrl+Q / q / Ctrl+C | Quit |

Mouse support requires a terminal that forwards mouse events. On Unix, piped-input navigation uses the controlling terminal through Crossterm; an interactive terminal is still required.

## Development

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

The tests cover stable numbering and JSON Pointer escaping, table conversion, nested JSON preservation, Unicode highlighting, keyboard navigation, mouse folding, and rendering at narrow terminal sizes. The document is parsed in memory using `serde_json` with insertion-order preservation and arbitrary-precision numbers. The UI restores the terminal on normal exit, errors, and Rust panics.

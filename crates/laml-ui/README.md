# LAML Editor

Native Rust/egui editor, developed in the LAML workspace as a separate binary.

## Development status

This is an initial development shell, not a finished editor. It accepts an optional
source filename, exposes editable source, and compiles GNE previews on a background
worker. Invalid source retains the last valid image with an out-of-date warning.
Stale worker results cannot replace a newer document's preview.

Click nodes in the preview to select them; Shift-click toggles multi-selection.
The last selected node is marked as the reference. Selection highlights the source
line, and placing the text cursor on a node declaration selects that node. Source
text has initial syntax highlighting.

The inspector shows rendered fill, border color and border width, including mixed
values. Set a value and choose **Apply locally**, or **Reset local override** to
remove direct node declarations in this file. Changes are compiled in the worker
before the source transaction is committed. Invalid geometry or a shadowed override
rejects the transaction without changing source. Typing during preflight cancels
the outdated visual command. Successful multi-node edits are one undo step.

## File operations

Use **New**, **Open…**, **Save**, and **Save as…** in the toolbar. The open/save-as
windows currently accept a file path. Shortcuts are Ctrl/Cmd+N, Ctrl/Cmd+O,
Ctrl/Cmd+S and Ctrl/Cmd+Shift+S. Opening another file, creating a new document or
closing the window asks before discarding local changes. Text can be saved even
when it has compiler errors.

The editor checks its source file for external changes once per second. Clean
files reload automatically; local edits or pending visual commands cause a conflict
notice instead. **Reload from disk…** discards local changes only after confirmation;
**Save a copy…** keeps them under another path. Existing save-as targets require
explicit replacement confirmation. Save-as does not copy imported style/font files;
relative imports resolve from the new location.

Writes use a flushed temporary file in the destination directory. Disk contents are
compared with the expected snapshot before writing and again before replacement;
new targets use no-clobber persistence. This detects observed conflicts, but is not
an atomic compare-and-swap or a cross-process lock against arbitrary concurrent
writers. Crash recovery of unsaved buffers remains a separate follow-up.

## Creating nodes and relations

Enter an optional label and choose **Add · automatic placement**. The editor creates
a collision-free node ID and lets LAML place it, without adding layout constraints.

Choose **Connect**, select `->`, `<->` or `--`, then drag from the source node to the
target node. The drag previews the connection; dropping on empty space or pressing
Escape cancels it. New relations receive unique IDs. Connection sides remain
automatic. Nodes themselves are not freely draggable.

Creation commands are compiled before they are committed and can be undone as one
step each.

## Relative placement

In **Select / arrange** mode, drag one node onto the left, right, top or bottom
edge zone of another node. Nodes remain stationary during the gesture: the editor
changes relative layout rules, not screen coordinates or semantic relationships.
The inspector lists the new rule and every removed rule before the drop.

- Yellow means the proposed layout and routing are being compiled.
- Green means releasing will commit the verified source patch as one undo step.
- Red means the drop is blocked; its explanation appears in the inspector.
- Escape, dropping outside a zone or releasing before validation cancels the move.

Only directly incident, conflicting rules on the edited axis are removed. Other-axis
rules, unrelated rules, comments and container membership remain unchanged. This
initial gesture supports nodes in the same container. When removing a placement
rule with a perpendicular `align` constraint, the editor preserves that constraint
as an independent `?` rule. For example, moving A to the right of C converts
`A < B : align=center` into `A ? B : align=center-y` and adds `A > C`.

When a move changes one axis of `A ? B : align=center`, the other center axis is
preserved. Alignment attached to a retained perpendicular placement can be changed
without deleting that placement or its gap. The inspector lists these preserved
rules alongside additions and removals.

## Alignment controls

Select multiple nodes, then use **Align** in the toolbar: Left, Right, Top, Bottom,
Center X, Center Y or **Center · overlap**. The last selected node is the reference.
The inspector previews the rules to remove, preserve and add. **Apply alignment**
compiles the candidate before committing the entire selection as one undo step.
Changing the selection or source invalidates an unconfirmed proposal.

The commands write `A ? B : align=...`. `center` intentionally aligns both centers
and permits their overlap; `center-x` and `center-y` each constrain only one axis.
No screen coordinates are stored. The untouched axis, unrelated rules and
container membership remain mandatory. If they make the request impossible, the
compiler rejects the change instead of silently deleting more constraints.

## Build and run

From the repository root:

```sh
cargo build --release -p laml-ui
cargo run -p laml-ui -- examples/architecture.laml
```

Replace the example path with an existing GNE file. Without a filename the editor
opens a small in-memory diagram. The binary is `target/release/laml-ui` (with `.exe`
on Windows).

The compiler remains independently buildable:

```sh
cargo build --release -p laml
```

A plain `cargo build` builds only the existing root package. GUI dependencies belong
to `laml-ui`, not to the compiler package. Both packages share the workspace lockfile.

## Agreed editing contract

- Source is the sole diagram authority; no hidden layout coordinates.
- Text and visual commands share one history. A compound visual command is atomic.
- Preserve comments, formatting and unrelated source; formatting is explicit.
- Relative drop zones create layout rules, never arbitrary coordinates.
- Preview removed/added rules before a drop. Only directly affected rules on the
  edited axis may be replaced. Preserve other-axis and unrelated-object rules.
- Reject a drop that cannot satisfy that boundary.
- Creating a relation does not create explicit placement constraints. Automatic
  connection sides remain the default. Fixed sides require an explicit action;
  language extensions must be agreed before implementation.
- Multi-selection edits common properties, shows mixed values, and disables
  inapplicable properties. Changes create local overrides; reset removes overrides.
- Alignment uses a visibly marked reference, the last selected node.
- Invalid source disables visual mutations, but retains preview navigation.
- Delete nodes together with incident relations and affected layout rules. For
  containers distinguish deleting contents from retaining them outside the container.
- Changing container membership is semantic. Reject invalid moves rather than
  silently deleting newly invalid relationships.
- Generate collision-free IDs separately from labels. Explicit renaming updates
  references atomically within the edited source.
- External file updates reload automatically only when the document is clean.
  Dirty documents require conflict resolution, never silent overwrite.
- Explicit saving plus crash recovery; begin with GNE, then other diagram types.

## Implementation order

1. Atomic edits, revision guards and shared history (initial foundation present).
2. Native source/preview shell (initial foundation present).
3. Compiler-backed element/source mapping, selection and editable properties
   (initial node selection, source-line mapping and three style properties present).
4. Relative placement, connections, alignment and multi-selection
   (automatic node creation, drag-to-connect, relative drop zones and seven alignment actions present).
5. Code-editor features, file lifecycle, recovery and export
   (explicit saving, opening, conflict checks and clean-file reload present).

Source ranges are UTF-8 byte ranges. The current line index maps compiler diagnostic
lines without normalizing LF/CRLF. It is not yet a token-level source map or a
lossless syntax tree. History currently uses full text snapshots; grouping typing
and bounding memory consumption remain follow-up work.

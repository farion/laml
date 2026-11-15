# LAML Editor — implementation specification and session handover

Date: 2026-11-14  
Repository: `/opt/repos/laml`  
Application: `laml-ui`  
Status: GNE functional implementation A–D completed locally; native multi-platform shipping acceptance remains open.

## 1. Purpose and next-session objective

Continue the existing Rust/egui editor implementation, rather than creating another
prototype or redesigning the agreed interaction model.

**Section 14 is the completed GNE baseline; sections 15–25 record subsequent
UI and routing changes. Sections 26–28 record the browser-backend foundation,
owned protocol, asynchronous compiler/file/export workflows and WASM library build,
and link the next implementation steps. Sections 11–13 and section 4 preserve earlier milestones,
not current feature limitations.** The delivered syntax/UI inventory is
`SPEC/laml-editor-gne-inventory.md`. Preserve the full dirty worktree. Remaining
shipping acceptance is native human testing and real macOS/Windows execution of
the prepared workspace CI/release pipeline, not more work on the old A–D gaps.

The editor presents editable LAML source, a rendered editing view and a properties
sidebar. Both input paths are equally important. The visual view edits source
semantics, layout rules or styles; it is not an independent drawing model.

## 2. Preserve the current worktree

At handover, Git HEAD is:

```text
28fe16d Require relation labels to be perceptually nearest to their own connection
```

The editor and independent-alignment implementation are **local, uncommitted work**.
The worktree includes modifications to the root Cargo files, compiler/parser,
geometry and documentation, plus untracked `crates/`, new alignment modules and
`tests/independent_alignment.rs`. This specification is another new file.

Do not reset, clean, discard or overwrite this work. Inspect the actual worktree
before continuing. HEAD alone does not contain the implementation described here.
No editor release or publication is established by this document.

`AGENTS.md` limits each Rust source file to **500 lines**. Keep new functionality
in focused modules instead of growing the existing UI files indefinitely.

## 3. Binding product decisions

### 3.1 Source and architecture

- Rust and egui are fixed technology choices.
- Same repository, separate `laml-ui` package and executable.
- CLI and UI must remain separately buildable. GUI dependencies belong to the UI.
- Reuse the existing `laml` library for parsing, validation, placement, routing and
  rendering. Do not build a second diagram renderer or a hidden UML model.
- The `.laml` source and its declared dependencies determine exported geometry.
  No editor-only coordinates, layout lockfiles or previous-render state may affect it.
- View state such as zoom, selection and panel dimensions may remain editor state.
- Language extensions require discussion with Frieder before implementation.
  The independent `?` alignment syntax in section 5 has already been approved.

### 3.2 Editing and history

- Visual changes are minimal source patches. Preserve comments, line endings,
  formatting and unrelated declarations. Whole-file formatting is a separate action.
- Text edits and visual edits share one undo/redo history.
- A compound action—multi-selection style change, rename, delete or relative move—
  is one undo transaction.
- Validate all patch ranges before applying any part of a transaction.
- Bind commands and build results to document revisions. Never apply a stale
  command or replace a newer preview with an older build result.
- An invalid visual edit leaves the source unchanged and explains the failure.
- Text input may temporarily be invalid. Keep the last valid picture, label it as
  out of date and disable visual mutations until a complete build succeeds.
- Navigation and selection remain available on an out-of-date preview; do not use
  its obsolete source positions to mutate the current document.

### 3.3 Layout gestures

- No arbitrary node coordinates and no freely draggable arrow bend points.
- Relative drop zones express left/right/above/below relationships.
- Hover shows added, removed and preserved rules before a drop.
- Replace only directly affected constraints on the edited axis. Preserve the
  other axis and constraints exclusively between unrelated objects.
- Preserve the perpendicular alignment of a removed directional placement as a
  standalone `?` rule when necessary.
- If the remaining constraints cannot satisfy the action, reject it rather than
  deleting more unrelated rules.
- Creating a semantic relation does not create an explicit placement rule.
- Automatic connection sides are the default. Fixing a side must be a deliberate
  operation, not an accidental consequence of where the mouse was released.
- Container membership changes are semantic operations, not ordinary layout moves.

### 3.4 Selection and properties

- Multi-selection applies to both contextual controls and the properties sidebar.
- Show mixed values explicitly. Disable properties not applicable to every selected
  element and explain why they are disabled.
- Display effective rendered values; edits create object-local overrides.
- Reset removes the relevant local override, exposing inherited values again.
  Selected objects may consequently return to different values.
- The last selected node is the alignment reference. This does not imply an
  editor-only fixed pixel position.
- Labels and IDs are separate. Renaming a label must not silently rename its ID.
- Generate valid, collision-free IDs. Explicit ID renaming updates all references
  in the edited source atomically; it must not rewrite unrelated text or labels.

### 3.5 Deletion, containers and files

- Deleting a node removes its incident relations and directly affected layout
  references, with a visible affected-elements preview and one undo step.
- For containers, distinguish deleting the container with its contents from
  removing the container while retaining its contents.
- Moving into or out of a container must not silently delete relations or rules
  made invalid by the membership change. Reject and explain such a move.
- Explicit save is the default. Unsaved work must eventually have crash recovery.
- Reload external source changes automatically only when the document is clean;
  otherwise present a conflict, not silent replacement.
- Begin with GNE. Other, more restrictive diagram types follow after the GNE editor.

## 4. Implemented baseline

The following exists in the worktree and must be retained:

| Area | Implemented scope |
|---|---|
| Application | Native `laml-ui` executable, separate Cargo package, existing compiler library dependency |
| Source view | Editable text, initial syntax highlighting, error messages, node/source-line selection linkage |
| Preview | Background full compilation, last-valid-image retention, stale-result rejection |
| History | Atomic text patches, revision guards, shared undo/redo; currently full-text snapshots |
| Node selection | Click and Shift-click multi-selection; last-selected reference |
| Inspector | Effective fill, border color and border width; local edits/reset; mixed values |
| Creation | Simple nodes with optional labels and generated IDs; automatic placement |
| Connections | Drag-to-create `->`, `<->` and `--`, with named relations and automatic sides |
| Placement | Validated relative drop zones for nodes in the same container; no free positioning |
| Alignment | Seven independent alignment actions; rule-change proposal and confirmation |
| Files | New, open, save, save-as, unsaved-change prompts, source-file polling and conflicts |
| Saving | Same-directory temporary writes, snapshot checks, no-clobber creation, explicit overwrite confirmation |
| Clipboard | Native clipboard integration is enabled in eframe features |

Baseline limitations (historical; superseded by sections 11–14):

- Existing arrows are not selectable/editable; only creation exists.
- Node labels and IDs are not editable in the inspector.
- No visual delete or container-membership operations.
- Creation currently creates simple nodes, not a complete GNE element palette.
- The inspector exposes three style properties, not the whole language.
- Source mapping is primarily line-based, not a lossless token-level editing model.
- Preview automatically fits its panel; no zoom/pan interaction yet.
- Panels are currently three fixed columns rather than resizable editor panes.
- Open/save-as currently accept a typed path rather than presenting a file chooser.
- No UI export or crash recovery yet.
- Save snapshot checks are not an atomic compare-and-swap or a cross-process lock
  against arbitrary simultaneous external writers. Do not overstate that guarantee.
- Native visual acceptance across desktop platforms has not been established by
  the headless tests.

## 5. Approved independent alignment language

These rules belong in `layout`:

```laml
A ? B : align=left
A ? B : align=right
A ? B : align=top
A ? B : align=bottom
A ? B : align=center-x
A ? B : align=center-y
A ? B : align=center
```

Semantics:

- Edge values align their corresponding measured box edges.
- `center-x` constrains only X midpoint; `center-y` only Y midpoint.
- **`center` constrains both midpoints and explicitly requests overlap.**
- Independent alignment does not impose ordering or a gap.
- Directional `A < B : align=center` retains its existing Y-center meaning;
  `A above B : align=center` retains its X-center meaning.
- Shared-center overlap permission is confined to explicitly connected center
  groups, not all nodes. One-axis alignment alone does not disable collision checks.
- The current compiler also recognizes combined X/Y center constraints as a shared
  center. Shared-center groups are supported within the same container/root scope.
- Routing is still validated normally. Intentional node overlap is not a waiver
  for impossible or misleading arrow routes.

Example of a relative move preserving the untouched axis:

```laml
// Before
A < B : align=center
B < C

// After moving A to the right of C
A ? B : align=center-y
B < C
A > C
```

## 6. Next work package: existing relation editing

### Required behavior

1. Click an existing relation to select it, including parallel relations and loops.
2. Link that selection to its actual source declaration and display its properties.
3. Edit its label, supported relationship operator/direction and applicable styles.
4. Drag an endpoint to another valid node to reconnect it.
5. Explicitly set either endpoint to `auto`, `left`, `right`, `top` or `bottom`.
6. Keep automatic sides for ordinary creation/reconnection unless the user explicitly
   chooses a fixed side.
7. Resetting a fixed side removes only that side constraint, not unrelated end-label
   settings on the same source rule.
8. Validate the entire candidate and commit atomically; preserve source on failure.

### Existing syntax to reuse

Named relation ends already support layout constraints such as:

```laml
relation[request].source : side=right
relation[request].target : side=left
```

Inspect `src/parser/layout.rs` and the router's side-rule handling before designing
anything new. Do not introduce a second side-selection syntax.

If a relation must become named to address a property, assign a valid unique name
as part of that one transaction. Respect the source's configured naming convention.
Do not rename existing named relations gratuitously.

### Implementation constraints

- Carry source relation identity through the compiler/render metadata where needed.
- Do not identify a relation solely by its endpoints: parallel edges make that
  ambiguous. Do not assume route-vector order equals source order without a tested
  contract, especially with filtering, attachments and specialized render paths.
- Hit testing must use the displayed route and the same viewport transformation
  used for nodes; include sensible pointer tolerance at different zoom scales.
- Preserve differences between semantic direction and endpoint layout constraints.
  A fixed entry side does not reverse the relation.
- Prefer targeted source-token ranges over whole-file reserialization or global
  string replacement.

### Acceptance criteria

- Two parallel A–B relations can be selected and edited independently.
- Self-loops and named/unnamed relations have unambiguous selection behavior.
- Changing one relation leaves other relations, comments and layout rules intact.
- Fix/reset one endpoint without losing another end property.
- Reconnection updates only the chosen endpoint and any necessarily affected
  references; invalid reconnections are rejected without a partial edit.
- Selection still works on the last valid picture while source is invalid, but
  mutation remains blocked.
- One undo restores each relation operation completely.
- Stale preflight results cannot overwrite newer source or a different opened file.

## 7. Subsequent work packages

### A. Node editing and deletion

- Edit labels independently of IDs.
- Rename IDs with exact reference updates across semantics, layout and local styles.
- Delete nodes/relations with a preview of affected references and one undo step.
- Handle containers with the two explicit deletion modes.
- Test scoped IDs, explicit global IDs, names embedded in longer identifiers,
  comments and quoted labels. Renaming must not accidentally rewrite those strings.

### B. Full GNE element and property coverage

- Creation/editing for supported shapes, notes, containers and ports.
- Insert nodes through relative zones as well as automatic placement.
- Explicit container membership moves with validation and a source-change preview.
- Expand inspector coverage from three style properties to the applicable GNE
  syntax: text/font, size/padding, shapes, icons, line styles, relation labels and
  other delivered language properties.
- Expose effective values and reset behavior consistently for multi-selection.
- Inventory the current language and parser first; do not invent unsupported fields.

### C. Code-editor completion

- Line numbers and diagnostic highlighting at actual source ranges.
- Search/replace, context-sensitive completion, bracket matching, comment commands
  and suitable indentation.
- Explicit format-source action, distinct from ordinary visual edits.
- Source linkage for relations, layout rules and properties, not just node lines.
- Group normal typing into useful undo units and bound history memory consumption.
- Handle UTF-8, CRLF, incomplete input and comments throughout source patching.

### D. View and workflow completion

- Zoom/pan, fit/reset view and resizable panes.
- Selection of obscured/overlapping objects, particularly after `align=center`.
- Contextual tools and a consistent selection experience; do not conflate drawing
  coordinates with persistent layout intent.
- File chooser UI, SVG/PNG/PDF export using the compiler's actual export pipeline.
- Crash recovery for unsaved buffers, with clear restoration/discard choices.
- Consider dependency-change handling for imported styles/fonts; current polling
  is for the opened source file, not a complete dependency watcher.

### E. Shipping

- Native manual/visual testing on Linux, macOS and Windows.
- Extend CI to test/build the UI explicitly or with `--workspace`. Current CI and
  release scripts use root-default commands; the root package remains the default.
- Separate UI release artifacts/install path while retaining standalone CLI builds.
- Document actual supported usage and validate installation/startup before calling
  the editor released. Headless rendering tests are not desktop visual acceptance.

## 8. Architecture map

### Editor package: `crates/laml-ui/`

| Path under `src/` | Responsibility |
|---|---|
| `document.rs` | Source, revision guards, atomic edits, undo/redo, saved state |
| `source.rs`, `source_patch.rs` | Line mapping and source-preserving section/line patches |
| `preview.rs` | Real compiler pipeline, diagram/node metadata and SVG preview |
| `selection.rs` | Ordered node selection and node hit testing |
| `files.rs` | File baseline, polling, explicit writes and conflict checks |
| `semantic_edit.rs` | Node/relation creation patches |
| `style_edit.rs` | Local node style updates/reset |
| `placement_edit.rs` | Relative-placement rule transformations |
| `alignment_edit.rs` | Independent-alignment proposals and axis preservation |
| `app.rs` | UI coordination and publishing accepted previews |
| `app/worker.rs` | Revision-bound background builds and visual postconditions |
| `app/canvas.rs`, `app/creation.rs` | Selection, connection gesture and creation tools |
| `app/placement*.rs` | Relative drag zones, hover validation and commit |
| `app/alignment.rs` | Alignment toolbar, proposal and confirmation |
| `app/inspector.rs`, `app/source_editor.rs` | Properties and source UI |
| `app/file_ui.rs` | File actions, prompts and conflict UI |

Visual commands generally create a candidate source, compile it on the worker and
commit their edits only after successful validation. Relative drop hover has its
own revision-bound preflight and commits its validated candidate on release.
Keep these concurrency guarantees when adding new interactions.

### Compiler additions relevant to alignment

- `src/syntax/alignment.rs`: independent alignment model and anchor meanings.
- `src/parser/alignment.rs`: `?` grammar and reference checks.
- `src/geometry/constraints/alignment.rs`: equalities and center-overlap permission.
- `src/geometry/containers/alignment.rs`: scoped projection and final validation.
- `src/svg.rs`: shared canvas bounds for editor coordinate mapping.
- `tests/independent_alignment.rs`: compiler regression coverage.

Language documentation is in `docs/language-reference.md`,
`docs/language-definition.md` and `docs/diagrams/gne.md`. The MCP GNE guide includes
that GNE document, so keep it accurate when extending the language.

## 9. Validation and working procedure

1. Read `AGENTS.md`, inspect `git status` and this specification.
2. Start from the current implementation and extend its tests alongside each feature.
3. Use synthetic/local test fixtures. Do not modify unrelated client diagram sources
   to make a compiler or editor test pass.
4. Run focused tests while developing; run the complete workspace suite before
   reporting a major milestone as regression-tested.
5. Build the actual release binary so restarting `target/release/laml-ui` uses the
   new code. A test binary alone is insufficient.
6. Report what works, what remains and what was actually tested. Do not equate a
   passing test count with completion of the product contract.

Useful commands, from `/opt/repos/laml`:

```bash
cargo fmt --all --check
cargo test -p laml-ui
cargo test -p laml --test independent_alignment
cargo clippy --workspace --all-targets -- -D warnings
LAML_NO_UPDATE_CHECK=1 cargo test --workspace --release
cargo build --release -p laml
cargo build --release -p laml-ui
./target/release/laml-ui
./target/release/laml-ui examples/architecture.laml
```

The last completed implementation round reported **543 passing local workspace
tests in release mode**, strict workspace Clippy, and both release builds. Its test
log was `/tmp/laml-alignment-workspace-tests.log`. This is historical validation of
that worktree, not a substitute for running tests after new changes. Tests were
not rerun merely to write this specification.

## 10. GNE completion gate

Call the GNE editor feature-complete only when users can create, select, edit,
connect, reconnect, style, arrange, align, rename and delete the supported GNE
elements through the visual interface; make equivalent changes in source; save,
reopen and export; and recover from invalid input and file conflicts without silent
loss or hidden layout state. The code-editor and recovery requirements above are
part of that scope, not replaced by the existing basic text widget.

No new language decision is needed to start relation editing. Ask only when a
concrete required operation cannot be represented faithfully with the delivered
syntax, or when a genuinely unresolved product choice affects implementation.

## 11. Continuation implementation — 2026-11-13

This section supersedes the baseline status in sections 4 and 6. Product decisions
and the completion gate remain unchanged. No new language syntax was introduced.

### Implemented in this continuation

- Compiler `Route.source_line` retains the original declaration identity through
  visibility filtering, alternative route order and attachments. The UI does not
  infer identity from endpoint pairs or route-vector indices.
- Existing arrows, parallel relations, self-loops and their labels are selectable.
  Source selection also links to relation declarations. Shift-selection includes
  arrows; mixed node/arrow property controls are disabled with an explanation.
- Picking and selection highlights use flattened geometry from the compiler's SVG
  path builder, including modern rounded bends, bridge humps and underpass gaps.
  Label picking uses measured, rotated rendered extents. The normal image and
  interaction geometry share the same viewport transform.
- Relation labels, all delivered GNE operators, color, width and font size can be
  edited. Relation-only multi-selection supports common label/operator/style and
  endpoint-side edits, showing mixed values explicitly.
- Source and target handles reconnect an individual semantic end to a node.
  Automatic sides remain automatic; deliberate fixed sides survive reconnection.
  Explicit `auto|left|right|top|bottom` controls reuse existing end-layout rules.
  Reset removes only `side=`, retaining other end properties and trailing comments.
- Addressed changes assign unnamed relations unique names, respecting strict
  lowercase-relation naming when configured. Multi-edits reserve distinct names
  and insert a single section rather than duplicate blocks.
- Token-targeted changes preserve unrelated text, quoted labels, comments, UTF-8
  and LF/CRLF. Declaration selection remaps through patches when styling/layout
  sections precede semantics. New builds from ordinary text edits clear relation
  selection when its declaration identity changed, rather than retargeting by line.
- Node labels are editable separately from IDs, including multi-selection and
  inline/block notes. Node ID renaming assigns or updates a global `@ID`, keeps the
  visible name, and updates resolved source references, default descendant/port
  addresses, layout and applicable local selectors in one transaction.
- ID collisions, global-reference shadowing and shared relative selectors whose
  target set would change are rejected. Comments, labels, class names and merely
  similarly spelled IDs are not globally replaced.
- Deletion has a visible affected-elements proposal and explicit confirmation.
  Nodes, incident relations, dependent edge attachments, affected layout rules and
  direct local styles are handled in one undo transaction. Mixed selection works.
- Container deletion distinguishes deleting contents from retaining them. Keeping
  contents rebases default addresses and references; collisions are rejected.
  Container-local contextual style rules currently block keeping contents, with an
  explanation, rather than being silently dropped or reinterpreted.
- The properties pane scrolls so the expanded controls remain accessible.

### Modules added / extended

- `crates/laml-ui/src/relation_edit.rs`, `relation_edit/{sides,mapping,tests}.rs`
- `crates/laml-ui/src/relation_selection.rs`
- `crates/laml-ui/src/node_edit.rs`, `node_edit/{rename,style_refs,tests}.rs`
- `crates/laml-ui/src/delete_edit.rs`, `delete_edit/tests.rs`
- `crates/laml-ui/src/app/{relations,relation_canvas,objects,deletion}.rs`
- Headless interaction tests in `app/{relation_tests,object_tests}.rs`
- `src/svg/picking.rs`, publicly exposed through `laml::svg::route_pick_paths`
- Actual usage guide: `docs/editor.md`

Preview metadata, selection, the revision-bound worker, source linkage and file
view reset were extended to support these operations. Node styling now shares its
source-preserving local-property helper with relation styling. CLI relation
creation also respects configured lowercase-relation naming in the UI helper.

### Validation and working-tree state

- `cargo fmt --all --check`: passed.
- `cargo clippy --workspace --all-targets -- -D warnings`: passed.
- `LAML_NO_UPDATE_CHECK=1 cargo test --workspace --release`: **576 tests passed**.
  Full log: `/tmp/laml-editor-continuation-tests.log`.
- Both `cargo build --release -p laml` and `-p laml-ui`: passed.
- `/opt/repos/laml/target/release/laml-ui --version`: `laml-ui 0.1.0`.
- Native startup smoke: the release UI stayed running for five seconds with no
  stdout/stderr before the test timeout stopped it (exit 124). Log:
  `/tmp/laml-editor-native-smoke.log`. This is **not** a manual visual acceptance
  test or a Linux/macOS/Windows portability result.
- `git diff --check`: passed. No Rust file exceeds 500 lines.
- Changes remain local and uncommitted on top of `28fe16d`; the earlier editor and
  alignment work was preserved. No push, tag, release or installed-binary update.

### Remaining scope

The GNE completion gate is still open. In particular, element palette and complete
property coverage, port editing, membership moves, richer source editing, zoom/pan,
resizable panes, file chooser, export, recovery and UI CI/release artifacts remain.
Shared relative selector edge cases are handled conservatively by rejection, not
by speculative rewriting. Imports and external styles are never rewritten by a
local rename/delete operation. Native visual acceptance is still required.

## 12. GNE palette, property coverage, ports and membership — 2026-11-13

This is the current implementation state, superseding the remaining-scope list in
section 11 where it overlaps. Existing uncommitted work remains intact. No language
extension was introduced; see `SPEC/laml-editor-gne-inventory.md` for the actual
syntax/property inventory and remaining semantic gaps.

### Implemented

- Palette: rectangle, ellipse, diamond, semantic note and package creation; Root or
  package destination; automatic or left/right/above/below insertion relative to a
  selected node in the same container. Shapes are explicit local LSS intent.
- Node inspector: 17 delivered properties covering text/fill/border/base colors,
  sizes/minimum sizes, padding, rounding, shape, icon, symbol rendering/icon set and
  visibility. Icon browsing uses the compiler's bundled catalogue. Inapplicable
  structural fields/options are disabled with explanations.
- Property values come from actual compiler-scoped styles, rendered geometry and
  theme-adjusted paint. Imports/contextual specificity are retained. Compiler
  validation normalizes colors/numbers; no second property grammar was introduced.
  Applying a shadowed override is rejected instead of reporting a fictitious result.
- Elements list reaches nodes, ports and relations even when hidden/overlapped.
  Semantic selection survives `display: none`; properties can restore visibility.
  Mixed element-type property editing is disabled while mixed deletion is supported.
- Relation inspector additionally exposes base color, visibility and
  near/along/inline/default label placement, including multi-selection. A relation
  must have a label before the label-mode control is enabled.
- Ports: creation inside a body or conversion of a leaf to a body; source-linked
  pointer/list selection; node/port connection gestures and endpoint reconnection;
  automatic/fixed sides; fill/border/text/base color and visibility; name and global
  ID editing; incident-edge-aware deletion proposals and atomic undo.
- Existing default port names stay tied to their name-derived address until an
  explicit global ID is deliberately assigned. The UI explains this and does not
  silently change an ID when changing a name. New ports receive explicit IDs.
  Decorator classes and comments survive ID edits.
- Generated names reserve scoped default aliases to prevent accidental reference
  capture, not merely exact global-ID collisions. Creation also compares existing
  normalized relation endpoints before and after its source patch.
- Single-node membership moves have revision/selection-bound source-change previews
  and explicit confirmation. Declarations move with their comments and line
  endings; default descendant/port addresses and direct selectors rebase; explicit
  global IDs remain stable. Contextual selectors stay in their original scopes so
  inherited styles follow the new membership.
- Membership never drops a relation, attachment or layout rule. Cycles, collisions,
  changed endpoint resolution or invalid geometry/routing reject the transaction.
  Reordered relation declarations are verified as normalized multisets rather than
  assuming relation-vector order stayed fixed.
- Style insertion now finds the existing style section correctly even when a note
  body contains a literal opening brace. File changes reset stale palette targets.

### New modules / current architecture

- `crates/laml-ui/src/element_edit.rs`, `element_edit/tests.rs`: palette transactions.
- `crates/laml-ui/src/port_edit.rs`, `port_edit/tests.rs`: port source edits.
- `crates/laml-ui/src/properties.rs`, `properties/tests.rs`: property inventory and
  effective node/port/relation snapshots.
- `crates/laml-ui/src/membership_edit.rs`, `membership_edit/tests.rs`: membership
  planning and normalized semantic-preservation checks.
- `app/{palette,property_rows,ports,element_tree,membership}.rs`: new UI controls.
- `app/{palette_tests,membership_tests}.rs`: native-toolkit headless interaction and
  worker/undo/concurrency tests, including real port pointer gestures.
- `relation_edit/label_modes.rs`: existing per-relation label layout syntax.
- `styles::property_value`, `StyleSheet::for_node`,
  `StyleSheet::with_node_ancestor`, `bundled_icon_names`: narrow compiler inspection
  APIs; rendering/validation behavior and source syntax are unchanged.
- `docs/editor.md`: updated usage for the implemented workflows.

### Verified final state

- `cargo fmt --all --check`: passed.
- `cargo clippy --workspace --all-targets -- -D warnings`: passed.
- `LAML_NO_UPDATE_CHECK=1 cargo test --workspace --release`: **606 tests passed**.
  Log: `/tmp/laml-editor-gne-palette-tests.log`.
- Both release binaries rebuilt successfully with `cargo build --release -p laml`
  and `cargo build --release -p laml-ui`.
- Native release UI startup remained running for five seconds with no stdout/stderr
  before the smoke-test timeout stopped it (exit 124). Log:
  `/tmp/laml-editor-gne-native-smoke.log`. No manual visual acceptance or platform
  matrix is claimed.
- `git diff --check` and the 500-line maximum for every Rust file passed.
- Everything is still local/uncommitted on top of `28fe16d`; no push, release or
  installed-binary update. Restart `/opt/repos/laml/target/release/laml-ui` for the
  rebuilt editor.

### Next work

Continue the source editor and workflow packages: actual-range diagnostics,
completion/search/formatting, typing undo grouping and memory bounds, zoom/pan,
resizable panes, file chooser, export and recovery. Member/compartment controls,
attachment creation and specialized structured headings remain inventory gaps;
this milestone is not a feature-complete GNE/editor claim. Native visual acceptance
and UI-specific CI/release artifacts remain shipping work.

## 13. Source-editor foundations — 2026-11-13

This continuation implements the first substantial part of C without changing the
language or replacing existing editor work. Read this section as the current state.

### Implemented and tested

- Shared undo now records UTF-8-safe minimal text differences rather than full
  before/after snapshots. Adjacent insertions or forward/backward deletions within
  750 ms coalesce, but every mutation still increments the revision. Paste, newline,
  explicit commands, caret/focus changes, save, undo/redo and visual edits separate
  groups. Branching clears redo without reusing a revision.
- History is capped at 256 retained changes and 8 MiB of allocated edit/string
  payload. Live/saved buffers and the small bounded vector overhead are separate.
  Old/oversized entries are evicted; the source pane displays usage and warns if
  no undo entry could be retained for a dirty edit. Egui's redundant full-buffer
  source undo snapshots are cleared after each frame; Document remains the owner.
- Synchronized line-number gutter follows the actual galley cursor rows, including
  Unicode/CRLF offsets and trailing empty lines.
- `preview::build_diagnostic` returns a structured diagnostic with the compiler's
  actual file and line. The existing String-returning `preview::build` API remains
  available to other callers. The UI underlines the whole reported active-source
  line and offers a jump. Imported-style errors keep their own file location;
  unlocated theme/font/render errors remain unlocated. No error-string scraping
  and no fabricated token/column spans.
- Diagnostics carry the source revision and are cleared on mutation. The layouter
  checks the exact diagnosed buffer before applying byte-range styling: inserting
  a multibyte character cannot reuse a stale byte boundary inside that character.
- Literal find/replace with per-character Unicode-lowercase matching, next/previous
  wrapping, F3/Shift+F3, and Ctrl/Cmd+F/H. Match replacement requires the current
  selected match and current revision. Replace-all patches original nonoverlapping
  matches atomically with one shared undo step; it is raw text work, not ID rename.
  Search-field native undo does not trigger document undo.
- Source Tab/Shift+Tab indentation, Ctrl/Cmd+/ whole-line comment toggling and
  LF/CRLF-aware Enter indentation. Enter between bracket characters adds an inner
  line and an outer-indented closer. Quoted/commented opening characters do not
  request an extra level. These explicit text actions can edit incomplete source.
- Programmatic caret installation happens before the text widget consumes input,
  so Enter followed by text in one frame writes at the new caret. Focus locking
  survives repeated indent/outdent commands. IME confirmation Enter is not handled
  as a source-newline command; native preedit/commit keeps its widget state.
- Source selections used for mutations are revision-bound. A stale source selection
  is rejected, and source mutation cancels pending visual proposals/preflights.
  File reset clears source search/selection/diagnostic/widget state. Accepted visual
  commands invalidate pending raw-byte jumps before using fresh compiler linkage.

### Files

- `document/history.rs`, `document/history_tests.rs`: bounded delta history and
  typing grouping. `document.rs` remains the shared public transaction API.
- `text_tools.rs`, `text_tools/tests.rs`: byte-accurate search and text-command plans.
- `diagnostic.rs`: structured diagnostic locations and source-range projection.
- `app/source_tools.rs`: search/action state, shortcuts, shared undo dispatch and
  source-mutation cancellation. `app/source_tests.rs`: real keyboard/IME regressions.
- `app/source_editor.rs`: gutter, diagnostic paint, caret/source linkage and widget
  history ownership. `app/worker.rs`, `app.rs`, `app/file_ui.rs`: error/state wiring.
- User documentation updated in `docs/editor.md`.

### Final validation

- Formatting and strict workspace/all-target Clippy passed.
- `LAML_NO_UPDATE_CHECK=1 cargo test --workspace --release`: **633 tests passed**;
  log `/tmp/laml-editor-source-tools-tests.log`.
- Both actual release binaries rebuilt separately: `cargo build --release -p laml`
  and `cargo build --release -p laml-ui`.
- Native release startup stayed running with no stdout/stderr for five seconds
  before timeout terminated it (exit 124); log
  `/tmp/laml-editor-source-native-smoke.log`. This is not visual acceptance.
- Rust file-size check (maximum 500 lines per source file) and `git diff --check`
  passed. All changes still local/uncommitted on top of `28fe16d`.
- No push, tag, published release or installed-binary update. Restart the rebuilt
  `/opt/repos/laml/target/release/laml-ui` to use it.

### Continue here

C is not complete. Next implement context-sensitive completion and grammar-aware
bracket matching/explicit formatting, preserving free-form note bodies, comments,
Unicode and CRLF. Expand source linkage to layout/property declarations. Do not
call Enter's simple last-opening-character indentation a grammar-aware formatter.
Then continue D: zoom/pan/fit/reset, resizable panes, native chooser, real compiler
SVG/PNG/PDF export and recovery, including imported style/font dependency handling.
Keep B's member/attachment/structured-heading gaps and E's native/platform/UI-release
work visible. No native visual acceptance has been recorded.


## 14. GNE functional completion and local shipping validation — 2026-11-14

This section supersedes the open A–D lists in earlier milestone sections. Preserve
all existing changes: HEAD remains `28fe16d`, with the compiler/alignment/editor
work still local and uncommitted. No push, tag, public release or installed system
binary update was performed.

### Implemented continuation

- Context-sensitive source completion with revision-bound acceptance; lexical
  delimiter matching protects strings/comments/free-form note bodies. Explicit
  format-source normalizes structural indentation only after compiler-model
  equivalence, retaining comments, Unicode, note contents and LF/CRLF. The actual
  toolkit shortcut test prevents Ctrl/Cmd+Shift+F being consumed as Find.
- Source linkage covers member, layout, contextual style/property and attachment
  lines. An attachment line selects the attachment, not its note plus target.
- Fit/100%, pointer-anchored zoom, center-anchored +/- buttons, panning, clipped
  picking and bucketed bounded SVG rasterization; resizable source/properties
  panels. View state never creates source coordinates. Real pointer tests verify
  selection after zoom/pan.
- Native rfd open/save dialogs retain explicit-path alternatives. Compiler export
  runs asynchronously for SVG/PNG/vector PDF, protects source/style/font targets,
  binds completion to the source revision/dependencies and checks external output
  changes before writing. Dead export/preview workers report an error rather than
  leaving a permanent busy state.
- Local import/font polling includes nested, empty and missing dependencies.
  Changes invalidate pending builds/exports. Recovery uses private atomic snapshots
  and per-session locks; restore/discard excludes live sessions and obeys dirty-file
  prompts. Restore uses the current disk baseline without silently overwriting it.
- Palette now includes class, interface, enum, datatype, object and junction.
  Previewed declarations and body edits expose delivered structured headings,
  member/slot/literal/text/separator snippets and style classes. A compiler-backed
  postcondition rejects body/declaration input that escapes into global sections,
  changes other elements or alters unrelated topology.
- Member IDs can be made explicit while updating local references/styles. Member
  inspectors expose effective color/font-size/display/symbol-rendering/icon-set;
  compartment display supports local overrides/reset. Worker checks reject shadowed
  requested values. Node/relation/port declaration inspectors expose existing
  class-list and end-annotation syntax without inventing a second grammar.
- Address-based connections and note-to-relation attachments have previewed actions.
  Attachments are selectable and independently deletable through the confirmed
  delete pipeline; note and target relation survive. Mixed attachment selections
  do not expose inapplicable ordinary-relation controls.
- Note label edits preserve heading/body/closing annotations; unsafe block text or
  empty labels can use the existing quoted note form without losing literal text.
- CI uses workspace tests/clippy and separate CLI/UI release builds. The independent
  `editor-release.yml` validates `ui-vVERSION`, packages four platforms, a macOS
  app bundle and checksums, and explicitly avoids the CLI latest channel. Updated
  README, editor guide, installation guide and GNE inventory describe usage.

### Validation actually performed

- `cargo fmt --all --check`; strict locked workspace/all-target Clippy passed.
- `LAML_NO_UPDATE_CHECK=1 cargo test --locked --workspace --release`:
  **663 passed**, zero failed. Log: `/tmp/laml-editor-complete-tests.log`.
- Both actual release binaries built separately. Current executable:
  `/opt/repos/laml/target/release/laml-ui`, version `0.1.0`.
- Rust sources in `src`, `crates` and `tests` remain at or below 500 lines.
  `git diff --check` passed.
- Both changed workflow YAML files parsed successfully with Ruby/Psych. Their
  remote jobs have not run; syntax parsing is not cross-platform CI acceptance.
- The rebuilt editor captured its own native Linux window successfully, exit 0:
  `/opt/repos/laml/target/editor-gne-20261114-040300.png` (2190×1301).
  The image was inspected: source, navigation controls, inspector and the complete
  12-node architecture drawing are visible. This is automated visual inspection,
  not human acceptance of every interaction.
- A local Linux archive with notices, usage guides and `SHA256SUMS` was created:
  `/opt/repos/laml/target/editor-dist/final-20261114-041500/laml-ui-v0.1.0-x86_64-unknown-linux-gnu.tar.gz`.
  Checksum verified, archive extracted, executable compared to the release binary,
  and native launch/capture succeeded from the extracted application directory:
  `target/editor-dist/final-20261114-041500/installed-editor.png`.
- Negative native capture checks returned exit 1 for existing output (bytes/hash
  unchanged) and invalid source (no image written). Logs:
  `/tmp/laml-editor-complete-{installed,no-clobber,invalid-capture}.log`.
- End-to-end toolkit tests create class/node/member/port/relation/note/attachment,
  edit member style, format, export all three formats and undo all nine transactions.
  Additional tests cover stale export cancellation, external output replacement,
  interrupted workers, recovery baseline/prompt behavior and unsafe body escapes.

### Remaining shipping gate

A–D implementation is complete for this GNE scope. E still needs human native
acceptance, actual macOS/Windows execution and remote CI/release-artifact validation.
Do not describe prepared workflows or the local Linux archive as a published editor
release. Restart the rebuilt application to use this implementation; other diagram
families remain a separate product phase.

## 15. Mockup-based editor appearance — 2026-11-14

Frieder requested that layout and existing widgets follow the attached
`Gemini_Generated_Image_9bsfyk9bsfyk9bsf.jpeg`, including dimensions, colors,
spacing and corner radii. The image was read again before implementation.

- `app/appearance.rs` centralizes dark surface/outline/accent colors, bundled
  Inter UI typography, 32-point controls, 5-point widget corners, panel headers,
  floating toolbar frames, vector action icons and the editor-only grid.
- `app/shell.rs` implements a centered application title, file/export header,
  source/preview/inspector panes (default 36.6% / remainder / 18.2%, with minimum
  widths), selection heading and status bar. Initial native viewport changes
  and window resizing reapply these proportions; dividers remain draggable.
- Zoom and direct horizontal/vertical alignment actions float above the canvas.
  At narrow widths the toolbars explicitly stack, with reserved canvas space.
  The toolbar overflow menu retains insertion, connection and other alignments.
- File/recovery/history commands and source search/completion/formatting live in
  their respective overflow menus. Their shortcuts and compiler-backed actions
  remain intact. Hidden-element navigation and advanced inspector actions remain
  accessible through Elements and More actions.
- Node/port properties are compact, grouped rows with color swatches, text or
  dropdown fields and vector apply/reset buttons. Relation inspection includes
  endpoint selectors and grouped docking/style controls. Selection outlines use
  the reference's violet accent. Diagram rendering and export retain the actual
  source theme; the UI does not recolor the document to resemble the mockup.
- Layout tests cover node/relation selection and viewport resizing at 1000,
  1568 and 2190 points, plus floating-toolbar bounds at 324–706 points. Rendering
  the UI leaves document source and revision unchanged.
- Native Linux window captures were inspected at
  `target/ui-mockup-preview-v2.png` and `target/ui-mockup-before-selection.png`.
  The latter includes the final narrow-window toolbar stacking. These are real
  application captures, not a pixel-perfect acceptance of the entire mockup.
- Final validation: **666 workspace release tests passed**, strict all-target
  Clippy, release UI build, formatting and whitespace checks passed. Final native
  Linux application capture succeeded at `target/ui-mockup-final.png`.
- Validation logs: `/tmp/laml-ui-mockup-workspace-release-tests.log`,
  `/tmp/laml-ui-mockup-clippy.log`, `/tmp/laml-ui-mockup-release.log`.
  Restart `target/release/laml-ui` to use the rebuilt UI. All work remains local
  and uncommitted; native human acceptance and the platform matrix remain open.

## 16. Dark preview appearance override — 2026-11-14

Frieder revised the preview appearance contract: in the dark editor, render the
preview as modern-dark with a transparent canvas, irrespective of the document's
own theme/background. This supersedes section 15's source-theme preview policy;
exports still use the document settings. Do not rewrite source settings or create
an undo entry for this display preference.

- `preview::build_dark` / `build_dark_diagnostic` share the real compiler pipeline
  with document builds and override only the parsed theme/background in memory.
  Source locations, custom fonts and explicit/imported LSS colors are retained.
- Regular worker/preflight jobs and the asynchronous placement-hover worker use
  the dark preview builders. Inspector snapshots come from that same preview.
  `output::render` continues using the document-theme builder.
- Preserve the transparent SVG root marker after outlining, which otherwise drops
  custom attributes. PNG transparency is tested using actual decoded corner alpha,
  not just metadata; opaque document export remains opaque.
- **182 editor release tests passed** (111 library, 71 application), including
  light document → dark transparent preview, unchanged document/export settings,
  imported colors, source diagnostic lines and placement preview/commit styling.
  Strict workspace/all-target Clippy, formatting, whitespace and release UI build
  passed. Logs: `/tmp/laml-ui-dark-preview-{tests,clippy,build}.log`.
- Native Linux capture without source theme settings succeeded and was inspected:
  `target/ui-dark-preview.png`. Dark nodes/text and the underlying grid are visible
  with no white image canvas. Restart `target/release/laml-ui` for the rebuilt UI.
  Changes remain local/uncommitted; no publication or platform-matrix claim.

## 17. Preview chrome and independent document tabs — 2026-11-14

This supersedes the older grid/floating zoom controls and single-document shell.

- The 40-point footer contains Fit, 100%, −, + and the current zoom percentage,
  left-aligned under the preview, with 8-point vertical padding. Fit is visibly
  selected until manual navigation or 100%. The source header has a bottom border.
- Remove the background grid. A screen-space dotted border marks exact compiler
  image bounds. Persistent node/port/relation selection overlays are painted even
  when navigation is blocked or the pointer leaves the preview. These overlays
  and the border remain editor-only.
- `app/workspace.rs` now implements the native application. It owns independent
  `Editor` instances (text/history/cursor, view, selection, workers, dialogs,
  previews, recovery). Preview image URIs include a unique document ID; egui
  document UI IDs are scoped to that document. Inactive documents continue
  processing compiler/export jobs, external changes and recovery checkpoints.
- Global title bar: New (+), Open on the left, Settings gear at the right.
  Settings opens an intentionally empty window. Scrollable tabs have dirty dots,
  active highlighting and per-tab close icons. Newly active tabs scroll into view.
- New/Open add tabs without replacing dirty buffers. Opening an already-open
  canonical path activates its tab. Save as cannot overwrite another open tab.
  Closing dirty tabs asks save/discard/cancel; application close checks all tabs
  sequentially, including inactive tabs. Closing the final tab creates a blank tab.
- Phosphor UI icons come from pinned `egui-phosphor 0.14.0`. Register its font bytes
  in a dedicated egui 0.37 font family: proportional fallback incorrectly resolves
  some private-use glyphs from Inter. Do not regress to font fallback.
- Validation: 197 editor tests (111 library, 86 application), strict workspace
  Clippy, formatting, whitespace and release editor build passed. A complete
  debug workspace test run reached the 600-second limit in the existing exhaustive
  example-build test; do not report that run as completed. Native Linux screenshot
  `target/ui-tabs-phosphor-20261114.png` was captured and visually inspected.
  All work remains local/uncommitted; no release or Windows/macOS acceptance.

## 18. Source header and responsive action bars — 2026-11-14

- There is one Source header. Filenames belong only to the tab bar. Actions, in
  order: Save icon, Undo icon, Redo icon, Save as…, Export…, separator, Find/replace
  icon, Format source icon. Old source/file menus and the second Source row are
  removed; existing comment/completion keyboard shortcuts remain available.
- General toolbar rule: when actions do not fit, move a trailing suffix into an
  ellipsis menu in the same order. Menu entries for icon actions show icon + text.
  `app/toolbar.rs` is the reusable implementation, currently used by the source
  header and footer zoom controls. Keep action enablement and behavior identical
  between inline and overflow forms; do not introduce clipping or dangling dividers.
- Source header implementation lives in `app/source_header.rs`. Save uses the
  existing conflict-safe file flow; undo/redo use the document's shared history;
  Export keeps its SVG/PNG/PDF submenu even inside overflow.
- Native title icons have normal hover feedback, balanced vertical spacing and no
  title/tab separator. The footer has no automatic panel separator (its explicit
  border is sufficient); the extra separator consumed one point and clipped its
  buttons. Zoom uses centered Phosphor plus/minus icons in 24-point square buttons.
- Validation: 204 editor tests (111 library, 93 application), strict workspace
  Clippy and release UI build passed. Tests cover source widths 280–1000 points,
  overflow labels/actions, save, undo/redo, search, formatting and export submenu;
  footer tests inspect full button bounds/clipping at multiple display scales.
  Native Linux capture `target/ui-source-header-20261114.png` was inspected.
  Work remains local/uncommitted; no release or platform-matrix claim.

## 19. Direct graphical editing contract — 2026-11-14

Frieder approved and commissioned this GNE interaction model:

- Diagram header replaces floating controls. It contains Add node (+), a dropdown
  for new arrow types (default `->`), and six alignment icons only for node-only
  multi-selection. It shares the ordered, labeled overflow rule with Source/footer.
- Add node opens a type/ID/label dialog, initially Rectangle and a collision-free
  ID/label. Type changes update untouched suggestions only. Nodes use auto-layout;
  no coordinates or hidden placement rules. Stale dialogs cannot overwrite source.
- Alignment checks and commits immediately as one undo step against the last
  selected node, replaces only conflicting rules on the edited axis and preserves
  the other axis. No inspector Apply/proposal. Graph-local failures stay in the
  graphical area. Hover rulers overhang the selection, actual common axes are
  subtle and a successful alignment flashes briefly.
- Hovering a node exposes four fixed-side start arrow grips and an automatic center
  grip. Drag shows four side targets on nodes and existing ports. Node-body drops
  are automatic; edge drops constrain that side; port drops reference the port.
  Source and target choices are independent. New arrow type is frozen at drag start.
  Self-loops/parallel arrows are supported subject to normal compiler validation.
- Either endpoint of a selected relation can be dragged. The old arrow is really
  removed from a compiler-rerendered base image, with the original viewBox and node
  geometry retained; a moving curve with its markers/labels follows the pointer
  and snaps to docks. Explicit sides have outward tangents. This curve is only a
  gesture overlay; the compiler still produces the final route. A node-body drop
  clears only the moved end's side. The other endpoint, labels/type/styles survive.
  Escape, empty/outside drop, stale revision, navigation or resize cancel safely.
- Right-click preserves selection and offers Add port…, Delete this node… and
  Delete selected nodes (n)… (enabled for at least two selected nodes). Port dialog
  captures ID/name/side. Graph-local deletion confirmation lists dependencies.
  Deleting one selected node retains other surviving selections.
- Key modules: `app/graph*.rs`, `connection_{geometry,gesture,paint}.rs`,
  `alignment_guides.rs`; library `graph_edit.rs` composes validated atomic source
  transactions. `preview::without_route` handles the moving-arrow base image.
  Old floating-control helpers remain test-only for earlier regression cases.
- Validation: 225 editor tests (117 library, 108 application), strict workspace
  Clippy, formatting, whitespace and release editor build passed; Rust files remain
  under 500 lines. Pointer tests cover all source sides, automatic/explicit targets,
  ports, self-loops, both endpoint moves, type capture, cancellation and stale state.
  A PNG pixel test proves the old arrow disappears without changing node fills or
  geometry. Native header capture `target/ui-graph-header-20261114.png` inspected.
  Full native interaction acceptance and Windows/macOS checks remain separate.
  All work remains local/uncommitted; no release was performed.

## 20. Header, modal, connection and inspector refinements — 2026-11-14

This supersedes the corresponding visual choices in sections 18–19.

- Source/Diagram titles remain left; action groups align right, preserving ordered
  overflow. Alignment reference strokes are cyan; the active canvas guide is drawn
  last, solid and 2.5pt thick with a dark halo, rather than another faint dashed line.
- Shared `app/dialogs.rs` centers modals with an opaque panel and dark backdrop.
  Modal input blocks background shortcuts and canvas navigation. Modal fade-in is
  disabled to avoid translucent startup captures. Add node/port, deletion, Settings,
  file confirmations, recovery and source proposals use this contract; nonmodal
  file status is centered. Contextual completion popups still follow the source.
- Connection grip radii are proportional to diagram node dimensions and canvas
  scale; offsets, glyphs and hit testing scale consistently. Selected arrow-end
  handles and port targets scale as well. The moving arrow no longer uses a manual
  Bézier approximation: orthogonal transient routes use the shared compiler SVG
  route/qualifier painter, theme corners, markers, colors, widths and labels/fonts.
  The final obstacle-aware route is validated on drop. Cache invalidation also
  covers source changes, cancellation and reused editor image URIs.
- Inspector tabs: Allgemein and Style, independent per document. General contains
  identity, endpoints/layout, declarations/members and deletion; Style contains
  effective local style controls and member/compartment styling. No Element title,
  Elements block, More actions nesting or Apply buttons remain in Properties.
  All categories are flat, subtle bars with upper/lower borders. Captions and tab
  labels align left; fields use explicit 10pt horizontal, 7pt vertical padding.
- Enum/picker/catalog selections apply directly; text commits on Enter/blur.
  Shift+Enter inserts a newline at the Unicode-safe cursor/selection in supported
  fields. Label fields are single-line until an explicit newline exists. Tab
  switches are deferred until the old inspector has processed blur. ID renames,
  style reset, contextual inheritance and atomic preflight/undo are preserved.
- Diagram-wide Elements and address/attachment controls moved to the Diagram
  actions menu and centered modals. Placement feedback now overlays the diagram,
  not Properties. No diagram-wide actions are nested under a selected object.
- Native capture gained an explicit `--capture-view general|style|add-node` option.
  Inspected Linux captures include `target/ui-inspector-general-20261114.png`,
  `target/ui-inspector-left-20261114.png` and `target/ui-modal-node-opaque-20261114.png`.
  Tests cover direct enum/Enter commits, invalid input, Unicode newline insertion,
  field padding, left captions, header alignment/overflow, scaled grip hit areas,
  centered opaque modals/backdrop blocking and exact shared SVG route painting.
- Latest complete editor validation: 235 tests (119 library, 116 application).
  Additional compiler validation: 85 library tests and 18 theme integration tests.
  Work remains local/uncommitted; no native Windows/macOS acceptance or release.

## 21. Mockup-based properties columns and tabs — 2026-11-14

- Re-read Frieder's original 1568×975 mockup. The inspector now uses a dark,
  header-colored tab strip/inactive tab; the active tab has panel fill, a fine
  outline and accent underline. No blank panel-colored gutter separates tabs
  from the pane header or the first category.
- Category bars span the full properties pane without horizontal inset. Attribute
  rows alone use 16pt horizontal padding. Shared `inspector_ui::row` lays out a
  94pt left-aligned key, 10pt gap and a consistently aligned value column.
- Applied to General and Style for nodes, ports and relations, including container,
  declaration/body, member styles and compartments. Color swatches follow their
  text fields so hex-value fields start at the same x-coordinate as other fields.
  Reset actions have a guaranteed 24pt width, preventing narrow-pane overflow.
- New layout tests exercise node/port/relation inspectors at 270, 370 and 600pt:
  value-column alignment, full-width category paint, field bounds and zero source
  mutations. Separate tests verify both active-tab states and dark tab-strip paint.
- Inspected native captures: `target/ui-properties-columns-general-20261114.png`
  and `target/ui-properties-columns-style-20261114.png`.
- Validation: 237 editor tests (119 library, 118 application), strict workspace
  Clippy, release UI build, formatting/whitespace and Rust-file 500-line limit
  passed. All changes remain local/uncommitted; no push or release.

## 22. Icon chooser, access-marker clarity and modal errors — 2026-11-14

- Replaced the inline `Catalog` name list and editable icon-name field with an
  **Icon → Choose icon…** button. Current icon/name are shown on the button;
  reset removes only the local override, preserving inherited styles.
- The centered modal shows real vendored Phosphor SVGs, not font approximations:
  complete-name search, All/Architecture/People/Files/Security/Arrows filters,
  virtualized scrollable tiles, selected-icon preview and explicit **Use icon**.
  Selection alone never edits source. Cancel/Escape leave source untouched;
  revision/selection checks reject stale submissions. Native screenshot inspected:
  `target/ui-icons-picker-complete-20261114.png`.
- Added read-only compiler `bundled_icon_svg` access and atomic `node_properties`
  composition. Each confirmed icon choice still uses checked compilation/shared undo.
- Former `Symbols` means visibility/access signs, NOT additional node icons.
  **Header mark** is shown only for nodes with a header visibility sign; individual
  members with signs have **Access marks**. Both offer Text (+ - # ~) / Icons.
  Header and member styles have distinct compiler scopes: setting a node property
  does not implicitly change its members. Preserve this distinction.
- Selecting Icons sets `symbol-rendering: icons` and `icon-set: phosphor` together,
  including per-member edits. The separate icon-set prerequisite/dropdown is gone.
  Compound node expectations verify every requested effective property.
- Removed command-error banners from BOTH Properties and Diagram pane tops. Failed
  visual edits use a centered modal; chooser validation stays inline in its modal.
  File/history shortcuts are guarded even before a newly opened modal paints.
- Fixed shared modal height budgeting: cached/default area height must not trap
  nested scroll views and hide confirmation buttons. Regression checks require
  the tile labels and confirmation controls to be inside their visible clip rects.
- Eight new tests cover actual pointer tile selection/confirmation, real SVG
  textures, whole-catalogue search, categories/empty results, keyboard search and
  Escape, blocked background shortcuts, cancel/stale targets, atomic header/member
  dependencies with real rendered glyphs, single undo and modal-only errors.
- Latest validation: **246 editor tests** (119 library, 127 application), **85 core
  library tests**, strict workspace Clippy, release UI build, formatting/whitespace
  and maximum 500 lines per Rust file passed. No full exhaustive workspace run,
  native Windows/macOS acceptance, commit, push or release claimed.

## 23. Explanation-first validation messages — 2026-11-14

- Frieder asked to keep technical diagnostics but put the actual cause, clearly
  worded, above them. Shared `command_errors::details` now renders reason, remedy,
  then **Technical details** with the original unmodified diagnostic.
- Hiding B while a visible arrow still uses it explains the dependency and suggests
  hiding the connected arrows as well or keeping B visible. Similar explanations
  cover layout/attachment dependencies, numeric values, shadowed styles, stale
  actions and stopped workers. Unknown diagnostics retain their actual cause;
  the formatter must not invent one or infer source highlight locations.
- Used by the common edit-error modal and inline node/port/icon chooser validation.
  This changes presentation only: no automatic hiding of dependent arrows, no
  source mutations, no weakened compiler validation.
- Three new regressions cover explanation/remedy content and visible ordering of
  reason → remedy → original technical diagnostic. Latest: **249 editor tests**
  (119 library, 130 application), strict workspace Clippy, release UI build,
  formatting/whitespace and Rust-file 500-line checks passed. Still local/uncommitted.

## 24. Properties cleanup, semantic fields and graphical deletion — 2026-11-14

- Follow-up polish: technical-diagnostic heading is **LAML-Error**, and the redundant
  `This edit was not applied.` sentence is gone. Modal X buttons reuse the exact
  titlebar helper/font/hover treatment; property reset buttons use Phosphor
  `ARROW_COUNTER_CLOCKWISE` in the existing 24pt column instead of hand-painted arcs.
- Removed **Actions / Delete selection** from Properties. Diagram context menus
  offer deletion of the clicked node/port/arrow and **Delete selection…** for mixed
  selections. **Remove container, keep contents…** remains a context action.
  Selection deletion now uses a centered confirmation of affected elements, with
  cancel/Escape and revision/selection snapshots. Confirmed edits share atomic undo.
- **Delete/Backspace** requests confirmation when not editing text; text-field
  focus, other modals, open popups, active gestures, stale previews and modifier or
  repeat key events are guarded. Canvas response identity is stored on View.
- Fixed right-click for arrows and ports: shared visible-route/label picking handles
  curves/bridges/gaps, rather than requiring a node hit. Right-click preserves the
  current selection. Context targets carry the source revision, preventing a stale
  line number from deleting another arrow after text changes.
- Removed raw **Declaration / Definition** fields for nodes, ports and arrows.
  Node Identity now contains **Type**, applicable access **Visibility**, **Abstract**
  and class **Templates** controls. Access visibility is distinct from Style/Display.
  Checked kind conversions cover Node/Class/Interface/Enumeration/Data type;
  structural note/package/object/junction kinds are displayed read-only. Remove
  template parameters before changing away from Class. Other declaration syntax
  (e.g. object classifier headings, style classes, end roles) stays editable in Source.
- Library `declaration_fields::change` patches only the requested header property,
  validating identity/topology and preserving comments, CRLF, body and references.
  Template input cannot escape the header; explicit IDs remain stable. Compiler
  preflight and shared undo still apply. Node Body/member controls are retained.
- Updated `docs/editor.md`. Native captures inspected:
  `target/ui-semantic-properties-node-20261114.png` and
  `target/ui-semantic-properties-class-20261114.png`. The latter uses a GNE fixture
  with a class (`target/ui-semantic-fields-class-source-20261114.laml`), not a UML
  diagram: opening `examples/class-basic.laml` correctly hits the existing GNE-only
  editor restriction, so no native UML-editor support is claimed.
- Latest validation: **259 editor tests** (121 library, 138 application), including
  pointer-driven arrow context deletion, label/port picking, keyboard confirmation,
  text focus, mixed selections, semantic fields, CRLF and one-step undo; strict
  workspace Clippy, release UI build, formatting/whitespace and max-500-line checks
  passed. Full existing worktree remains local/uncommitted; no push or release.

## 25. Routing resilience and automatic spacing — 2026-11-14

- Frieder's contract: a longer route, extra crossings or expanded automatic spacing
  is preferable to rejecting a connection. Mandatory authored layout, dimensions,
  alignment and endpoint sides must still be respected. No additional syntax needed.
- Reproduced two nodes / three arrows with fixed right/left attachment sides. Fixed
  the inflated arrowhead obstacle (including fixed-box routing that passes no node
  list to marker checks); open arrows now use their directional visible footprint.
- Shared `route::place_and_route` entry point is used by the CLI and editor preview/
  export. It first routes compact geometry; after failure it recomputes placement
  with increasing spacing, including nested container scopes. Hard constraints are
  solved/validated on every retry, not overridden. Source/styles are never rewritten.
  Public `route::route` still accepts fixed boxes and never mutates caller geometry.
- GNE/package routing has a last-resort obstacle-aware grid search without a fixed
  bend count, after fast candidate/order attempts. Resource guard: one million grid
  positions; spacing retries 32–1024 px. Search exhaustion is not mathematical proof
  of impossibility, and the diagnostic says so. Other specialized renderers retain
  their own notation/routing contracts.
- Crossings normally use bridges. When these need too much room, a last-resort compact
  underpass can cross near bends or at shallow angles. Cuts are clipped and merged;
  endpoints/markers and collinear overlaps remain protected. SVG path generation
  suppresses degenerate round-capped dots at bends cut on both arms. Picking uses
  the same visible paths. A full preferred parallel face now triggers alternatives,
  not an early failure; seven arrows between two default nodes are covered.
- Added fixed-side three-arrow pointer-drag + undo test in the application, preview/
  document-export/core parity test, arrow matrix, maze, close/bend crossing, nested
  reflow and fixed-frame constraint tests. Updated one affected original image and
  its two themed previews plus cache keys; diagram example sources are unchanged.
- Final full `cargo test --workspace --release`: **757 tests passed**. Strict workspace
  Clippy/all targets, both release builds, formatting/whitespace and max-500-line
  Rust-file checks passed. Logs: `target/routing-full-tests.log`.
- Binaries rebuilt: `target/release/laml` and `target/release/laml-ui`. Restart editor
  to load changes. All existing and new work remains local/uncommitted; no push,
  release or regular installation update. Native human/platform acceptance remains open.

## 26. Desktop/browser backend foundation — 2026-11-14

- Frieder approved the architecture: same egui frontend, files/compiler/exports on
  the host machine, authenticated browser access to a bounded workspace, preserved
  native desktop mode. Implementation direction and remaining gates are recorded
  in `SPEC/laml-editor-web.md`.
- Added injectable `backend::Backend` and `LocalBackend`. Compiler jobs, visual
  preflight, placement previews, gesture SVGs, source-line selection, FileSession
  operations, dependency watches and export use the selected service. Source and
  output write checks still delegate to their original implementations.
- FileSession retains backend identity across open/reload/Save as/clone. Workspace
  passes the same backend to new/opened tabs and to the replacement last tab.
  Canonical paths and overwrite snapshots are resolved in the backend, not by the
  frontend's filesystem. Editor/document themes stay separate.
- Native job scheduling moved to `backend/jobs.rs`: independent coalescing queues
  for compiler and placement, one-shot export rendering. In-flight results remain
  revision-bound. Dependency-service failure cancels export without writing.
- In-memory backend tests prove service injection for compiler/preflight/placement,
  file conflicts, reload, Save as, export bytes, new/opened/replacement tabs and
  shared undo. Additional service parity, dependency and queue lifecycle tests.
- Final full `cargo test --workspace --release`: **767 tests passed**, log
  `target/backend-refactor-tests.log`. Strict workspace Clippy/all targets,
  formatting/whitespace and the max-500-line rule passed. Both native release
  binaries built. Native startup/capture inspected at
  `target/backend-smoke-oWzswb/editor.png`.
- This is the implemented in-process boundary, **not a running web edition**.
  Synchronous native file/gesture/watch scheduling, host dialogs/recovery and
  startup remain. Next: owned wire models, asynchronous frontend operations,
  portable/WASM module split, then restricted/authenticated server and real browser
  acceptance. Never expose unrestricted LocalBackend directly to the network.
- Full pre-existing worktree remains local/uncommitted. No push/release/install
  update and no new macOS/Windows acceptance claimed.

## 27. Async protocol and WASM library — 2026-11-14

- Implemented owned protocol v1 (`backend/protocol*`): explicit session/document/
  request/revision correlation, full preview model serialization, separate compiler
  diagnostics, source/open/save/snapshot operations, export operations and owned
  dependency watches. Added serde support to compiler model/geometry/style/route
  data and editor property/preview data without changing source syntax or layout.
  JSON float round-tripping preserves geometry exactly.
- Implemented nonblocking submit/poll client (`backend/async_client*`), exact context
  and payload-kind checks, duplicate/obsolete reply filtering, disconnect/protocol
  failure completion and bounded in-flight requests. No implicit local fallback.
- The native transport uses the real JSON codec in both directions and bounded
  queues. Desktop compiler previews, visual preflight and placement previews now
  use this path. Preflight commands remain frontend-owned and verified before
  applying their source patches. Changed hover targets forget the old request,
  including changes within the same document revision. Queued obsolete read-only
  builds can be skipped; accepted writes are not cancellable by forgetting replies.
- Added WASM browser transport with same-origin asynchronous POST/credentials,
  configurable timeout, streaming 64 MiB response cap, bounded requests and
  abort/timer cleanup. It is build-verified, not runtime-tested against a server.
  No HTTP server, authentication implementation or runnable browser UI yet.
- Split native preview/export implementations into `preview/native.rs` and
  `output/native.rs`; public native APIs remain re-exported. Native file/recovery/
  executor adapters and PDF/update/network/MCP dependencies are excluded from WASM.
  Portable editing/models/protocol/browser transport successfully build as a WASM
  library. The egui shell remains in the native binary, not in that library build.
- Protocol tests cover exact preview/picking/property round-trips, members/ports,
  packages/parallel arrows/self-loops, diagnostics, version/input/size rejection,
  file/output conflict preservation and dependency watch ownership/replacement.
  Client/native tests cover out-of-order/wrong-context/wrong-kind replies, duplicate
  and forgotten results, disconnects, backpressure and queued read-vs-write handling.
- Final `cargo test --workspace --release`: **781 tests passed**, log
  `target/async-backend-full-tests.log`. Strict native workspace/all-target Clippy
  and WASM-library Clippy passed. Both native release binaries and the WASM-library
  release build passed (`target/editor-wasm-build.log`). Formatting, whitespace
  and max-500-line checks passed. Native startup/capture inspected:
  `target/async-backend-smoke-UgHq2o/editor.png`.
- Full validation initially hit a full disk. Removed only regenerable
  `target/debug/incremental` (about 31 GiB), preserving source, previews, logs and
  other build outputs; repeated the full suite/builds successfully afterwards.
- Next steps in `SPEC/laml-editor-web.md`: asynchronous file/confirmation/export/
  dependency/gesture UI states; portable egui shell and web entry point; scoped
  network addresses and authoritative document/session state; authenticated server
  and real browser acceptance. The native dispatcher accepts trusted host paths:
  never expose it or unrestricted LocalBackend directly as an HTTP handler.
- Everything remains local/uncommitted. No push, release or installation update;
  no browser session or new native Windows/macOS acceptance claimed.

## 28. Asynchronous file, picker, dependency and export workflows — 2026-11-14

Historical milestone; the completed browser host and current policy are in section 29.

- Replaced UI FileSession with portable `file_state::FileState`: resolved file
  identity and acknowledged baseline only, no filesystem access. Added
  `Document::mark_saved_snapshot` so a delayed save acknowledgement never marks
  newer typing clean or replaces it. Text/visual history is retained. Native
  FileSession compatibility APIs and tests remain separate.
- File open/poll/reload/recovery-baseline/Save/Save-as run through async protocol
  requests. Revision/path-bound replies cannot replace newer buffers. Save-as
  target inspection and explicit overwrite confirmation precede checked writing;
  stale confirmations expire and snapshots are checked again. Other-tab paths
  and Save-as reservations are respected.
- Workspace opening is asynchronous. Canonical duplicate-file activation preserves
  dirty existing tabs; failed opens preserve every tab. Startup path resolution
  moved to the native entry point, outside the portable file state.
- Native file pickers now use `rfd::AsyncFileDialog`, polled with a repaint waker
  in `app/host_dialogs.rs`. No blocking file picker remains in the egui frame.
  Host/native picker behavior beyond the local build still needs platform testing.
- File UI split into focused `file_ui/actions.rs`, `io.rs`, `reset.rs`, `windows.rs`.
  Transitions wait for accepted source/export writes. Close rechecks dirty state
  after acknowledgement; Save-and-close aborts on newer typing. Write failure
  stops a waiting close; accepted writes are not presented as cancellable. Read
  operations can be dismissed without replacing source. Deferred transition intent
  cannot be silently replaced by another shortcut while waiting.
- Dependency registration/checks now use async service messages, not metadata reads
  from egui. Stale responses are ignored; changes invalidate the document preview.
- Added backend-owned export tickets and Begin/Commit/Cancel operations. Preparation
  derives target/dependency protection, captures output/dependency snapshots and
  renders on the backend. Existing output gets an explicit post-render confirmation.
  Commit rechecks owner/revision, dependencies and the exact output snapshot.
  Tickets are one-use, replaced/cancelled/released per document, and storage-bounded.
  No rendered bytes or binary overwrite snapshot must be uploaded back to commit.
- Source changes before commit cancel the ticket. Changes after commit submission
  do not pretend to cancel an accepted write: status reports the submitted revision,
  source remains untouched and close waits/rechecks unsaved changes. An export write
  failure stops deferred close rather than hiding the failure.
- Added deterministic delayed-response tests for save/new typing, save-and-close,
  close during write, stale open/overwrite reads, disconnects, stale export prepare,
  accepted export writes/new typing and failed-write/close handling. Ticket tests
  cover ownership/revision, replay, replace/cancel/close, dependency/output changes.
  Native file/tab/header/export regressions now wait for actual async completion.
  Updated `docs/editor.md` with the delivered workflow behavior.
- Final full `cargo test --workspace --release`: **795 tests passed**, log
  `target/file-workflow-full-tests.log`. Native all-target workspace Clippy and
  WASM-library Clippy passed. Both native release binaries and the WASM-library
  release build passed (`target/file-workflow-wasm-build.log`). Formatting,
  whitespace and max-500-line checks passed. Native startup/capture inspected at
  `target/file-workflow-smoke-kzJkPz/editor.png`.
- Next: asynchronous source-line selection/gesture SVGs, portable host interfaces
  and moving the actual egui shell into the WASM library, then browser entry/assets,
  scoped/authorized server and real browser acceptance. Remaining details are in
  `SPEC/laml-editor-web.md`. No callable browser editor/server yet; never expose
  the trusted native dispatcher or unrestricted LocalBackend directly over HTTP.
- Full worktree remains local/uncommitted; no push, release or installation update.
  Updated native editor: `/opt/repos/laml/target/release/laml-ui`.


## 29. Shared browser editor complete; deployment policies opt-in — 2026-11-14

This section supersedes earlier browser/host status and policy statements.

- Shared Rust/egui desktop and WASM shell is runnable. Text/visual editing, tabs,
  undo/redo, inspector, checked files/exports and recovery use the shared editor.
- Removed the additional application sign-in, credential-file handling, provider-
  specific networking and browser identity/session expiry behavior. The host
  exposes ordinary network parameters and does not issue or interpret cookies.
- Default CLI behavior has **no configured size/resource limits, concurrency cap,
  HTTP checks or HTTPS requirement**. Optional flags are `--limits`,
  `--parallelism N`, `--http-checks`, `--require-https`, all independent.
  Generic `--bind IP:PORT` and optional `--origin URL` configure deployment.
- Size/resource checks in scoped/staged IO, protocol/client, export/gesture state,
  host recovery and subprocess deadlines/OS limits are gated by `--limits`.
  Native queues are unbounded; default codecs have no configured byte quota.
  Protocol correctness, revision/overwrite safety and workspace scoping remain.
- Page correlation namespaces retain compiler/watch/ticket state without login.
  Recovery uses shared `STATE_DIR/recovery` and live file leases. Lost-page leases
  are released without cancelling accepted work. Restore only changes a buffer.
- `docs/editor-host.md` is the current build/start/policy guide. The current
  browser harness is `scripts/test-editor-browser.mjs`; its default run and
  `--limits` run (also parallelism 1 and HTTP checks) both passed in real Chrome.
  Evidence: `target/browser-editor-WKST4S`, `target/browser-editor-Y8qx3E`.
- Complete release-profile workspace suite: **812 tests passed**. Native and WASM
  strict Clippy, native CLI/UI release builds and browser release build passed.
  Logs: `target/policy-refactor-tests.log`, `target/policy-refactor-clippy.log`,
  `target/policy-refactor-wasm-clippy.log`, `target/policy-refactor-build.log`,
  `target/policy-refactor-web-build.log`.
- Rebuilt local archive `target/laml-editor-host-linux-x86_64.tar.gz`, with matching
  `.sha256`. Extracted host startup, direct assets, policy defaults, actual example
  plus stylesheet, checked SVG export, no-clobber and packaged native capture
  passed: `target/editor-host-package-smoke.log`, `target/host-package-hba826hj`.
  Package verifier: `scripts/test-editor-package.py`.
- Live host starts the repository binary with generic `--bind` and **none** of the
  optional policies. Runtime PID/log are in the user's private
  `.local/state/laml-editor-host`; workspace is repository `examples`, assets are
  `target/editor-web`. A direct source-open request and policy response succeeded
  after the final restart. The listener is reachable by anyone with network access
  to its selected address; deployment access control is the operator's choice.
- Entire worktree is still local/uncommitted. No push/release/install update.
  Native Windows/macOS, other browsers, remote CI and another physical client
  remain separate acceptance scopes; do not infer them from Linux/Chrome tests.

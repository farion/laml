# Diagram-specific editor implementation

Date: 2026-11-15

## Agreed contract

- The source is the sole diagram-type authority. Choose the concrete type when creating a document; opening detects it. A valid textual type change changes editor tools. Invalid text retains the stale preview and disables visual mutations.
- Implement all compiler-supported diagram constructs, not additional notation. Preserve current compiler styling and notation validation.
- Hide tools that cannot apply to the diagram; disable temporarily unavailable actions with explanations.
- Try the default relationship first. If it is invalid for the selected endpoints, ask the user to choose an applicable alternative; never silently substitute.
- Gestures modify notation semantics/layout, never hidden pixel coordinates. Container membership changes are semantic operations.
- Preserve atomic, revision-bound preflight, shared undo/redo, comments, CRLF and deletion-impact confirmation.
- Native and WASM share implementation. Compile debug targets and deploy. No commit, push or release requested.

## Work packages

1. Type registry, New dialog, all-type compiler previews/exports, capability-driven controls.
2. Class, object, package and C4 code visual editing.
3. Component, deployment, composite, profile and C4 context/container/component editing.
4. Use case, state, activity, communication and interaction-overview editing.
5. BPMN elements, flows, pools, lanes, subprocesses, membership and styles.
6. Sequence participants/messages/activation/lifetime, then fragments and all remaining supported constructs.
7. Timing lifelines, changes, events, ticks and numeric layout.

## Completion criteria

For each compiler-supported construct, track display, creation, property edits,
structural edits and deletion. A source-only fallback is useful but does not count
as complete visual editing. Compilation or HTTP 200 alone does not demonstrate
behavioral or visual acceptance.

## Progress

The registry, shared graph paths and specialized source-authoring paths are
implemented locally. The construct inventory is maintained in
`docs/editor-notations.md`; it maps every diagram type to creation, property,
structure and deletion controls. Implementation remains in the existing dirty
worktree; no publication is implied.

### Implemented

- Twenty type templates and a family/type New dialog; source-driven switching,
  all-type compiler previews and exports, notation-filtered graph palettes and
  semantic relationship catalogs.
- Graph classifier/member/port/interface-contract fields, stereotype-aware
  relationships, C4 source adapters and notation-specific construct forms.
- Specialized source-linked geometry and inspectors for sequence, BPMN and timing;
  a structural navigator for nested semantics, layout and styling.
- Atomic profile stereotype/extension, interaction-overview flow/branch insertion,
  and BPMN root-process-to-pool operations, retaining mandatory compiler validation.
- BPMN inline-chain edits and deletion, task/boundary-event membership moves,
  subprocess subtree moves and qualified reference updates.
- Sequence fragment/operand grouping, local execution IDs, named messages,
  notes/continuations with required placements, occurrence annotations, ordering,
  coregions, gates and inline decompositions. Branch-aware shorthand-reply and
  activation deletion dependencies preserve unrelated visible labels.
- Timing numeric occurrence movement and whole-lifeline reordering. Document
  settings, local styles and stylesheet-rule creation remain source transactions.
- Contextual structural checks explain invalid operations; candidate builds remain
  the final geometry/routing authority. All visual changes share revision checks,
  undo/redo and deletion confirmation.

### Verification record

Targeted authoring and inspector tests cover template builds, semantic relation
kinds, source preservation/CRLF, grouped frames, qualified references, BPMN inline
and membership operations, strict overview topology, execution-local identity,
CSS/visible-label isolation, geometry metadata and stale source-type handling.
Native/WASM build and deployment results are recorded at the end of this session.
User visual acceptance and platform publication are separate from implementation;
no broad regression-suite or multi-platform release validation is asserted.

### Final local verification, 2026-11-15

- Native: `cargo build --locked -p laml-ui` succeeded without warnings.
- Browser: `cargo build --locked -p laml-ui --lib --target wasm32-unknown-unknown`
  succeeded; debug web assets regenerated with the installed wasm-bindgen tool.
- Targeted checks passed: 24 authoring tests, two type/template tests (all twenty
  templates build), two specialized inspector tests, two structural transaction
  tests, and the existing node (9), graph (5), relation (7) and element (4) edit
  tests. Logs are `/tmp/laml-notation-*-tests.log`.
- Real headless Chrome reached ready preview, created a BPMN document, switched
  source to the sequence notes example, selected a note and opened Add construct.
  No runtime exceptions were observed. Browser evidence:
  `/tmp/laml-notation-browser-inspector.log`,
  `/tmp/laml-notation-note-inspector.png`,
  `/tmp/laml-notation-construct-dialog.png`.
- Debug host restarted on port 7878, PID 3986112; HTTP 200 confirmed. Workspace
  `/opt/repos/laml/examples`, assets `/opt/repos/laml/target/editor-web`, original
  host state directory preserved. Host log `/tmp/laml-editor-notation-host.log`.
- All Rust files remain at most 500 lines. No release build, installation update,
  commit, push or release was performed in this work. Observed current HEAD is
  `4d18de9`; preserve the complete dirty worktree, including untracked modules.

# Sequence editor interaction rebuild

Date: 2026-11-17
Status: interaction layer replaced locally; visual acceptance pending

## Scope

Replace the sequence-specific interaction layer, not another incremental repair
of its existing controls. Remove the old controls, interaction strategies and
shared-canvas sequence gesture fallbacks. No parallel legacy interaction mode.

Preserve the compiler, renderer, source-linked geometry, source transaction
pipeline, revision checks, preflight and shared undo/redo. Preserve unrelated
local changes. The approved mixed message/activation height binding is the only
language extension in this rebuild. Release/publication are separate actions.

This document describes the target, not delivered behavior. Public usage guides
must describe the new controls only after they have been implemented.

## Common interaction rules

- `.laml` is the sole authority. Never persist editor-only coordinates.
- Icon buttons appear only while hovering their associated objects or controls.
  Relevant drop targets remain visible throughout an active drag.
- Clicking selects the actual object and updates Properties. Right-click targets
  the object under the pointer without requiring prior selection.
- Escape cancels the entire gesture without modifying source. Dropping outside
  valid targets does nothing.
- Every completed action is one undo step, including all required source edits.
- Hide invalid insertion actions and invalid relation-type changes rather than
  offering an action known to be impossible. Final preflight still validates.
- Source revisions invalidate gestures and drafts; stale previews disable visual
  mutation. Editor overlays never appear in exports.
- Use shared dialog frames and existing context-menu appearance conventions.
- Native and browser use the same interaction implementation.

## Interaction matrix

| Object / area | Hover | Click | Drag | Context menu |
| --- | --- | --- | --- | --- |
| Empty sequence | Visible Add participant button | Open participant dialog | None | Shared applicable document actions |
| Participant head | Plus on its left and right | Head selects; plus opens participant dialog directly | Reorder horizontally | Delete with impact confirmation; applicable object actions |
| Bare lifeline insertion position | Sensitive message-start area on line; separate plus to its right | Plus opens non-message insertion menu | Sensitive area starts message | Applicable object actions |
| Activation | Highlight exact hovered area; own plus / message targets where applicable | Select or insert using distinct targets | Top/bottom edge resizes; separate inset sensitive areas start messages | Delete while retaining messages where semantically possible |
| Message endpoint | Distinct endpoint grip on hover | Select | Reconnect that endpoint to a valid lifeline/activation dock | Valid relation-type changes and deletion |
| Message body | Hover feedback | Select | Move whole message in event order, retaining participants/direction | Valid relation-type changes and deletion |
| Message label / unlabelled arrow | Inline editing target | Double-click edits label | Must not conflict with body movement | Message actions |
| Undestroyed lifeline end | Ghost X | Insert standalone destruction | Message / insertion actions use distinct areas | Applicable lifetime actions |
| Actual destruction X | Selection / drag feedback | Select, never toggle-delete | Move to valid event heights or gaps | Delete destruction |
| Fragment / operand / other construct | Applicable controls only | Select / contextual insertion | Explicit valid structural destinations only | Applicable structural actions and deletion |

## Participants and lifetimes

- Left/right head plus inserts a participant on that side and opens the dialog
  directly, with no intermediary menu.
- Adding a participant normally places its head at the top. A late first message
  does not move its head down.
- An explicit creation message is different: the compiler's creation notation
  places the target head/lifeline start at the creation occurrence. Changing the
  message type must preserve existing events and only be offered when valid.
- With no participants, show one Add participant action.
- Destruction can be standalone or attached to a destroying message. Removing
  destruction retains the message if its type permits it.
- No insertion or message docks after destruction or before explicit creation.

## Insertion points and messages

- Insertion plus buttons are beside the line, not on its message-start area.
- Offer non-message constructs through plus; messages are created only by drag.
- Message-start sensitive areas also exist on bare lifelines, so no activation is
  required to draw the first message.
- Use the footer's selected default relation type. Never silently substitute a
  different type. If it cannot apply, offer only applicable alternatives.
- Endpoints can dock on activations, bare lifelines and valid intervening slots.
- Distinguish exact event height from insertion before/after an event.
- Between adjacent events, after A and before B are one target when their source
  effect is identical. Split a dock into upper/lower parts only when their
  meaning differs, e.g. inside versus outside an activation or operand.
- Preview communicates the actual semantic difference before release.
- Self-messages expose separate send and receive occurrences. Never choose an
  ambiguous occurrence based only on owner or source line.
- Reconnecting an endpoint does not silently change the other endpoint's owner.
- Moving a message preserves its bound activation starts/ends where applicable.

## Activations and nesting

- Use plus to insert an activation. On a bare lifeline it is outermost; within an
  activation it is nested. Show the intended nesting in the insertion preview.
- Initially extend to the next valid event position within the enclosing scope.
- Top and bottom horizontal edges resize both inward and outward. Highlight only
  the hovered edge. Show valid exact-height and intervening guides once dragging.
- Message-start areas are separate small targets inset vertically from the
  resize edges by a few screen pixels. The same pixels must not perform both
  actions. Short/nested bars need enough space for unambiguous targets.
- Each nested level has its own hit areas and visual feedback.
- Inner boundaries cannot cross their enclosing activation. Outer boundaries
  cannot shrink past contained activations. Do not silently move other bars.
- Docking at an activation edge writes an explicit occurrence-height binding,
  retaining its declaration and any existing message binding. Docking within an
  activation does not create another activation.
- Resizing a message-bound activation boundary detaches only that boundary and
  preserves message event order; use a standalone source boundary if necessary.
- Moving a message retains its semantic +/- markers. An explicit row move releases
  its layout-only occurrence bindings rather than moving a stationary reference.
- Preserve stack nesting, the other endpoint's independent marker, self-call
  send/receive distinction, and unambiguous reply associations.

## Fragments and remaining constructs

- Preserve visual access to all currently supported constructs, not only the
  common participant/message/activation path.
- Provide explicit insertion into operands, operand addition and grouping of
  existing events within a fragment. Do not silently change operand membership
  during a drag.
- Notes, continuations, annotations, gates, coregions, ordering and inline
  decompositions must retain source-backed editing paths.
- Qualified participants in inline decompositions must not be mistaken for root
  participants or silently excluded from applicable authoring controls.

## Deletion

- Use existing deletion confirmation and impact reporting.
- Participant deletion explicitly identifies dependent content.
- Activation deletion retains messages wherever possible.
- Message deletion accounts for bound activation boundaries.
- Removing a destruction must not silently remove its carrying message.
- Preserve unrelated content, labels, comments, newline style and layout intent.

## Replacement boundary found in the current tree

Replace the sequence gesture dispatcher and controls under:

- `crates/laml-ui/src/app/notation_ui/sequence.rs`
- `crates/laml-ui/src/app/notation_ui/sequence/`

Also remove sequence-specific legacy fallback strategies from:

- `crates/laml-ui/src/app/notation_ui/canvas.rs`
  (generic connection, participant ordering, annotation movement and event/frame
  movement must not provide a second sequence interaction system).

Review the shared inspector/structure, default-relation selection, inline text
editing and context-menu entry points. Keep common infrastructure; route sequence
behavior through the new explicit interaction contract.

Source-edit helpers under `crates/laml-ui/src/notation_edit/` and renderer geometry
under `src/sequence/` are reusable infrastructure, not automatically disposable
legacy UI. Verify their semantic behavior before using them from the new layer.

The working tree already contains uncommitted compiler, transaction and UI work.
Do not reset the repository or discard that work as a shortcut for the UI reset.

## Implementation and acceptance

1. Remove the old sequence interaction dispatcher and its shared-canvas fallback
   gestures; establish one new gesture owner with explicit hit targets.
2. Build participant insertion/reordering and line insertion/message-start
   controls, then activation resizing/nesting and message docks.
3. Add lifetime controls, context actions and fragment/operand operations.
4. Verify source effects for nested bars, self-messages, bound boundaries, replies,
   destruction, qualified participants, invalid drops and cancellation.
5. Compile native and WASM debug targets and deploy the local editor on port 7878.
   Record actual results; compilation/HTTP success is not visual acceptance.

Targeted behavior checks should cover distinct hit areas and atomic source
transactions. No broad CI/release task is part of this interaction rebuild.

## Local implementation record

The previous independent sequence gesture handlers and shared-canvas sequence
fallbacks have been removed. The replacement has one screen-space hit map
(`sequence/scene.rs`), one gesture owner (`sequence/gestures.rs`) and precomputed
legal source transactions (`sequence/proposals.rs`). Participant dialogs, line
plus/message areas, activation boundaries, message endpoint/body drags, lifetime
controls and contextual fragment operations use that path.

Compiler rendering, source transactions and shared undo remain in use. New source
adapters preserve message direction during type changes and retain messages when
removing activation/destruction boundaries. Inline decompositions are projected
for local source operations, then patched back into the same source; this is not
an editor model or stored layout state.

Initial native and WASM debug builds passed. The local browser host was replaced
on port 7878, with editor and WASM HTTP 200. Targeted source and synthetic egui
interaction tests are logged under `target/sequence-rebuild/`. These results do
not establish visual acceptance or publication.

### Message-target correction and remaining syntax decision

The sender side of a new message no longer consumes/rebinds an activation
boundary. Regression coverage confirms that starts from named/labelled
activations retain valid gap destinations without losing the sender's identity
or label. Target generation now includes both sides of activations, boundary
hit-testing covers the complete edge, and all legal drag markers use readable
contrast rather than faint hover styling. The ghost uses the checked destination
identity instead of guessing the nearest raw geometric dock.

Targeted tests passed (sequence filter: 46; sequence UI filter: 12, with overlap).
Native/WASM debug builds and local deployment on port 7878 passed again. Logs:
`target/sequence-rebuild/message-target-tests.log` and
`target/sequence-rebuild/target-coverage-tests.log`.

The user approved mixed occurrence/activation height bindings on November 17,
2026. They are now implemented in `src/sequence/occurrence_layout.rs` and the
shared sequence rendering pipeline. For example:

```text
request.receive ? Service.work+ : align=center-y; reference=second
```

Message docking uses these explicit source constraints without consuming named,
labelled or already message-bound activation declarations. Message endpoint
bindings can be replaced/released independently. Source adapters preserve scoped
layout rules and update references during rename/deletion; moving a message row
or resizing an activation releases the relevant layout-only bindings.

The backend checks candidate geometry and caches duplicate dock sides. Candidate
layout coordinates must not be painted directly over the unchanged current image:
participants may move/resize and attachment sides may change after the drop.
During a drag, the moving send/receive endpoint is anchored exactly at the visible
current target; compiler arrow mechanics and candidate message styling are reused.
Only after dropping is the final compiled layout applied. Bundled font selection
and the effective preview theme match the compiler; external font-file contexts
remain backend owned rather than being substituted in the canvas.

Two additional regressions cover every offered create/reconnect target (including
both activation sides, self-messages, both message endpoints, zoom and translated
canvas origins), plus free-pointer tracking when no target is hot. The sequence
UI filter passed 16 tests for this correction.

Lifeline/activation insertion plus controls now open a naturally sized native menu
anchored to the clicked icon, using the same menu styling as BPMN. The intermediate
centered Add construct dialog has been removed. Icons and 16-point item padding
are retained; Escape/outside click dismiss without edits. Choosing Note opens its
input dialog afterward. Two focused regressions cover anchor placement, selection
and dismissal; the sequence UI filter now passes 18 tests.

Verification after the syntax implementation: 71 compiler tests across six
focused sequence targets passed. The sequence source-test filter passed 53 and
the sequence UI filter passed 14 (overlapping filters, not additive totals).
Native and WASM debug builds passed, with all Rust files at most 500 lines.

A disposable headless Chrome profile exercised an actual drag from C's lifeline
to the top of named/labelled activation `B.work` on the local server. The drop
advanced the revision, retained the activation declaration and generated the
expected height rule, with zero browser exceptions. This is a focused smoke test,
not blanket visual acceptance of every sequence interaction.

Evidence: `target/sequence-rebuild/browser-target-smoke.log`,
`browser-target-ready.png`, `browser-target-drag.png`, `browser-target-after.png`;
test/build logs remain in the same directory. The corrected debug host is on
port 7878; no publication is established by this deployment.


### Immediate drag and activation hover controls

Message creation/reconnection no longer compiles all possible targets on the
frontend before its first frame. `SequenceTargets` uses the existing asynchronous
native/browser service, carries source/revision correlation, and returns validated
source proposals and compiler message metadata. Native font-file contexts remain
backend-owned. Browser mode retains its transient source-only/no-host-files policy.
Participant references in the request must resolve to existing scoped participants.

The ghost paints immediately in the current canvas while validation is pending.
A fast release is held at that exact position until the matching result arrives;
Escape, invalidation or revision change discards it. The normal command preflight
still owns source commit and undo. No fallback target is silently substituted.

Activation message origins now have 20-point-wide hit areas, centered horizontally
and separated from plus buttons and resize strips. Closely overlapping origins
are consolidated; short activations use a single center grip. Caption bounds guide
placement where space permits. Hovering the activation reveals all of its origins
and plus buttons, retaining the group while crossing to a plus button.

Verification: 20 focused sequence UI tests and the scoped target/protocol regression
passed; native/WASM debug builds passed. A browser test with 1,800 ms artificial
network latency captured the arrow while target validation was still pending and
successfully committed a drop released before that result. Evidence lives in
`target/sequence-rebuild/instant-drag-*.log` and the hover/pending screenshots.


### Keep insertion plus separate from message endpoint grips

Insertion plus placement uses final message paths and screen-space grip rectangles.
The existing row/right side is preferred, then the left side. If both are occupied,
nearby clear bands are derived from actual obstacle bounds. Message strokes,
endpoint/start controls, other plus controls and caption bounds remain separate.
Only the overlay rectangle changes; Dock/source insertion intent and existing
message grips do not move. The insertion menu anchors to that actual rectangle.

Regressions reproduce the previous self-call return/plus overlap, check incoming
arrows from either side under zoom, and cover both arrow sides sharing a height.


### Reduce target readiness latency, not just first-arrow latency

Native and browser target batches now reuse one backend-owned font/style context
and the full sequence geometry validator in System text mode. Target preflight no
longer embeds fonts, outlines glyphs or finishes an export SVG for each candidate.
The ordinary full preview/command preflight still runs for the actual drop.
Scoped hosting stages the original document/assets once per batch, keeping its
existing workspace boundary. Other backend implementations retain their checked
compile fallback by default.

Text measurement memoization belongs to the selected font context and is discarded
with it. Keys include exact text, font size and regular/italic face. Measurements
remain exact; no document history or stale font metrics survive a font-context
change. Shared batch context reuses unchanged labels across all target candidates.

Measured debug backend examples (same source/target sets): 12 messages/90 valid
targets improved from 8,586 ms to 117.5 ms; 36 messages/234 valid targets improved
from 48,097 ms to 716.2 ms. These are local benchmark cases, not a guarantee for
arbitrary documents/hosts. Manual performance tests remain ignored by default.
A browser drag returned its target batch in 16.8 ms and showed markers in the first
captured drag step; screenshot timing includes capture overhead, not just latency.
Evidence: `target/sequence-rebuild/targets-before.log`, `targets-after.log`,
`targets-browser.log`, and `targets-early.png`.

Parity tests compare full-preview and fast preflight valid sets, edits and endpoint
coordinates across creation/reconnection, self-calls, occupied/named bars, fragment
operands, theme overrides, scoped decomposition and local font files. Font tests
verify exact values and context isolation; paths outside the font root remain
rejected. The async reply/revision/cancellation contract is unchanged.


### One centered activation drop zone

The user selected one shared activation target instead of two visually different
left/right targets with identical source effects. Activation boundary and interior
docks, including exact-row reconnection docks, now have one centered marker per
dock/height. Hit testing spans the activation width. Circular interior markers and
square boundary markers retain their distinct insertion/binding meanings. Message
ghosts snap to the centered marker; source constraints and final compiler attachment
selection remain unchanged. Insertion plus placement still uses actual bar edges.

Existing target assertions were adapted to the single-zone contract. Native/WASM
debug compilation and local deployment verify integration; practical interaction
acceptance is performed by the user, without another broad test block.


### Straighten a bent message by reconnecting one endpoint

The reported response had its send occurrence bound to an activation end while
its receive occurrence stayed on the original later row. Reconnection now projects
the stationary endpoint's height onto valid lifelines, as an explicit straightening
target alongside ordinary gaps and activation docks. Self-calls are not collapsed.
The target intent is carried through the existing asynchronous preflight; normal
same-row retargeting remains unchanged.

The source adapter releases only the moved occurrence's previous layout binding.
If the stationary occurrence has an activation anchor (explicit height rule or
semantic +/- marker), the moved occurrence shares that same activation reference,
without changing endpoint owners or the stationary binding. With no such anchor,
releasing the moved rule restores the natural horizontal row. Candidate geometry
must actually be horizontal for this target to be offered. No language extension
or editor-only position is introduced. Only the active target is highlighted.

A focused regression reconstructs the reported screenshot and exercises both
endpoints at two zoom levels, including scoped decomposition, receive-side height
bindings and semantic activation markers. It checks real compiler coordinates,
stationary endpoint identity/height, retained activation count and the drag ghost.
Evidence: `target/sequence-rebuild/straighten-regression.log`. This is a targeted
regression, not a new broad interaction-acceptance block.


### Joint self-call routing, crossing mechanics and centered Phosphor plus

Self-message corridors are now assigned jointly from current occurrence intervals,
not independently at a fixed offset from each sender. Contained self-calls stay
inside enclosing calls; overlapping/interleaving calls use distinct corridors.
Label clearance, automatic participant spacing, fragment extents, canvas bounds
and inline decomposition widths follow the resulting geometry. Occurrence heights,
source bindings and participant ordering remain unchanged.

Sequence rendering reuses shared above/left crossing bridges and visible SVG path
fragments; compact underpasses handle crossings without enough room for a hump.
Underpasses preserve arrowheads and do not paint a background-colored mask, so
transparent and dark diagrams remain correct. Source-linked regions retain logical
endpoints and carry visible picking fragments for bridges/gaps. Scoped regions
translate both forms. The backend returns the selected self-loop extent to the
current-canvas drag preview; no independent UI routing model is added.

The sequence plus uses the bundled regular Phosphor SVG, not a font glyph.
Button and image rectangles share the same pixel-aligned center; their physical
pixel extents have matching parity, so image edge snapping cannot add a half-pixel
offset. The Phosphor icon and button/hit-area size are retained. Glyph mesh bounds
alone were insufficient: font baseline snapping still displaced the visible ink.

Focused verification: five compiler routing regressions cover contained and
interleaving calls, unavoidable bridges/gaps, caption/lane clearance, frame bounds
and inline decomposition. The existing 24 sequence-render tests and 25 sequence
UI tests pass, including the screenshot-based reply-straightening regression.
Evidence: `target/sequence-rebuild/loop-render-regression.log`,
`loop-routing-ui.log` and `plus-center-regression.log`. Native/WASM debug builds
and deployment are recorded separately; this is not full visual acceptance or
a release/publication action.


### Correct visible plus centering, including pixel snapping

A real browser screenshot measurement reproduced the remaining font-glyph offset:
approximately 0.48 physical pixels horizontally and 0.43 vertically at 1x, up to
0.61 vertically in the measured 1.5x activation button. The previous mesh-bound
unit assertion did not measure this visible result and was replaced.

The actual bundled Phosphor plus SVG is now tinted and centered on the button's
pixel-aligned rectangle. Matching odd/even physical image/button dimensions also
prevents Image::paint_at edge rounding from introducing its own half-pixel offset.
The local image has no external asset/font dependency. A unit check covers image
geometry at 1x/1.25x/1.5x/2x. Browser screenshots check actual blue-ink centers
against actual button-outline centers, for both participant-head pluses and an
activation insertion plus at 1x/1.5x/2x. All nine measured offsets are below 0.01
physical pixels, with zero browser exceptions.

Evidence: `target/sequence-rebuild/plus-pixel-check.mjs`, `plus-pixels-before.log`,
`plus-pixels-after.log`, `plus-after-*-crop.png`, `plus-svg-regression.log`,
`plus-svg-native.log`, and `plus-svg-wasm.log`. These are focused visual pixel
checks, not blanket interaction acceptance.

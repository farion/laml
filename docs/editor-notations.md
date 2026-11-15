# Diagram-specific editing

The editor uses the compiler's existing Generic node–edge (GNE), UML, BPMN and
C4 notation. Choose **New → Family → Diagram type** to create a document. Opening
a file detects its type. Changing a valid `settings.type` declaration in the source
changes the available tools. A document has one type; New does not convert an
existing document.

## Common workflow

1. Select the rendered element, or use **Diagram actions → Elements…** to select
   a source construct, including nested constructs, members, layout and styles.
2. Use **General** for identity and semantic fields and **Style** for presentation.
   Text fields apply on Enter or focus loss. Relationship endpoints and types
   retain their compiler meaning.
3. Use **Add node…** for the graph palette, or **Add construct…** for typed
   notation-specific forms. The reference picker beside a field lists existing
   compiler identities. Choose the destination container and insertion point.
4. Sequence elements move directly by vertical dragging, with an insertion guide.
   Drag an activation's upper or lower edge to move its start or end. Near another
   matching edge, the gesture snaps flush and writes an equal-height layout rule.
   Dragging back to an ordinary event slot removes that edge's alignment, retaining
   the opposite edge and the alignments of other group members.
   Drag participant heads horizontally to reorder lifelines; the insertion marker
   shows the new position, while source event order stays unchanged. A message
   attached to that boundary moves with it; an open lower end gains an explicit
   closing declaration. Message endpoints and activation pairings remain intact;
   invalid moves leave the source unchanged. A drop that would change nothing shows
   **Already at this position**, with no insertion preview and a blocked cursor.
   Escape cancels. Other notations use
   their floating **↑ / ↓** controls where applicable.
   The selection follows the moved construct. A
   destination or notation-specific drag changes authored membership.
   **Unpack…** removes a supported wrapper while retaining its contents. The
   confirmation lists the effects.
5. Delete through the existing impact confirmation. Candidate source is built
   before the change is committed. Undo/redo includes visual and text edits.

Property fields, optional attributes and structural controls use padded pane
content. Names and optional attributes apply on Enter or focus loss, like other
fields; naming does not require an Assign button. Generated source-line identities
are not displayed as element names. The inspector keeps property and structural
editing separate from the canvas order buttons.

The graph palette and relationship picker depend on the diagram type. Tools
that do not apply to the notation are hidden. Contextually invalid structural
operations are disabled with the compiler's explanation. If the selected default
relationship cannot connect the chosen endpoints, the editor asks for another
valid relationship instead of substituting one silently.

Invalid source leaves the last valid image visible and stale; visual mutation is
disabled until the source builds again. Each accepted operation is revision-bound.
Source formatting is explicit, not a side effect of visual editing. The same
compiler produces the preview and SVG/PNG/PDF exports.

## Construct inventory

The following table identifies the implemented authoring paths. Every row uses
compiler display/export, creation controls, editable source-backed properties,
structural source transactions and confirmed deletion. The inventory describes
LAML's checked notation subsets, not every construct in the corresponding UML,
BPMN or C4 standard.

| Diagram | Constructs and authoring controls |
| --- | --- |
| GNE | Existing shapes, notes, packages, classes/interfaces, members, ports, junctions, relations, relative layout, alignment, membership and local styling. |
| Class | Class/interface/data type/enumeration; visibility and abstract modifiers; members and operations; templates/bindings; association ends, n-ary associations and association classes; supported classifier relationships. Graph palette plus construct forms. |
| Object | Typed objects, literal slots, links, containment and source-backed properties. |
| Package | Packages, nested membership, dependencies, public/private imports and merges. |
| Component | Components, interfaces, ports, required/provided contracts, parts, assembly/delegation connectors and supported dependencies. |
| Deployment | Nodes, devices, execution environments, artifacts, containment, communication, deployment and manifestation. |
| Composite structure | Structures, classifiers, parts/roles, inherited ports, interface contracts, connectors and membership. |
| Profile | Profiles, stereotypes, properties, checked metaclasses, extensions, references and profile applications. Forms create mandatory stereotype/extension combinations atomically. |
| Use case | Actors, system boundary, use cases, generalizations and include/extend relationships. |
| State | States, regions/composite states, initial/final/choice vertices, transition trigger/guard/effect and entry/do/exit behavior. Forms bundle vertices with their mandatory transitions. |
| Activity | Actions, object nodes, input/output pins, control vertices, guarded control flow and object flow. |
| Communication | Typed object lifelines, links, numbered messages and interaction settings. |
| Interaction overview | Control vertices and interaction uses, guards, alternatives and parallel branches. Insertion forms extend an existing flow atomically, preserving its authored label and identity. |
| Sequence | Participants and representations; sync/async, reply, found/lost, creation/deletion messages; explicit activations and lifetimes; all supported fragment operators and operands; gates, references, inline decompositions, notes, continuations, occurrence annotations/ordering, coregions and explicit coverage/placement. |
| Timing | Lifelines and classifiers, initial states, timed state changes, events, ruler ticks, scale, lane gap, end time, interaction/time-unit settings and styles. |
| BPMN | Task variants, supported events/markers, gateways and splits, sequence/message/data/annotation flows, pools/lanes, expanded/collapsed subprocesses, boundary events, data objects/stores, annotations, memberships, attachments and flow-label orientation. |
| C4 context | People, external people/systems, scoped system, descriptions, relationships and styling. |
| C4 container | System boundary, people/external systems, containers, technology/appearance, relationships and styling. |
| C4 component | Enclosing system and scoped container boundaries, components and external collaborators, technology, relationships and styling. |
| C4 code | Component scope with supported UML class notation and its graph/construct controls. |

### Sequence structure

Contextual **+** handles appear when hovering or selecting a participant or an
execution bar, hovering a lifeline, or approaching its open lower end:

- Beside a participant head, **Add participant…** inserts a named participant
  immediately to its right.
- Along a lifeline or execution bar, the horizontal guide identifies the source
  event boundary. Start an activation (nested on a bar), add a note, or choose a
  synchronous call, asynchronous message or reply. Activations begin as an
  opening/closing pair; **End activation here** can move an explicit closing
  event earlier or close an open activation when the resulting source is valid.
- **Call with activation →** creates the call and the receiver's activation in
  one undo step (`A -> B+`). The call form also offers **Start target activation**.
  A new bar starts at the receiving arrowhead and includes an initial closing
  event that can be edited like other activation ends.
- After choosing a message, drag or click onto a marked lifeline or activation
  dock, then enter the label. The target dock snaps to an event boundary and
  highlights the actual innermost active bar. Returning to the source participant
  creates a self-call. The moving arrow uses the compiler's path and arrowhead
  geometry: filled synchronous heads, open asynchronous heads, dashed replies,
  and proper self-call return legs.
- At an open lifeline's bottom, **Destroy participant ×** inserts the destruction
  event. Destroyed ends do not offer another bottom handle.

Hovering or selecting an existing message reveals a grip at **both endpoints**.
Drag either grip to reconnect the sender or receiver. The **square dock on a
bar's top edge** makes an incoming call start that existing activation: the
standalone opening becomes the call's `+` marker, while the existing end,
interior messages and comments stay intact. This works both when creating a call
and reconnecting its receiving endpoint. An independently named/labeled opening
or an activation already started by another message is kept intact; use an
interior dock or a new activation for those cases.

Current-row docks retain
its event position; dropping at another source boundary moves the whole message
in event order. This is not a hidden independent pixel position for one end.
Names, labels, styles, endpoint lifetime markers and unrelated source remain
intact. Implicit replies become explicit when reconnecting requires endpoint
text; their activation close is retained. Changes that would retarget another
shorthand reply are rejected rather than silently changing it.

Menus disable invalid lifetime/reply operations with an explanation. Message
candidates validate their actual endpoints before submission, including checks
that existing shorthand replies retain their original endpoints. Escape cancels
without changing source; each accepted insertion is one undo step. The handles,
source-boundary guide and pending arrows are editor-only, not exported artwork.
**Add construct…** remains available for fragments and other sequence constructs.

Drag a standalone destruction **X** vertically toward another X to align their
lifeline ends. The snap guide previews the common later row; release writes
`A! ? B! : align=center-y` to `layout` in one undoable change. Escape or dropping
away from another X cancels. Existing alignments between other members are retained
when one member is aligned with a different group. Message-bound or ambiguous
destructions cannot be aligned this way; the editor explains the conflict.

Fragments retain their operand grouping when reordered or moved. Note and
continuation creation includes required layout placement in the same undo step.
Annotation forms reference named occurrences; the picker offers message send and
receive occurrences. Existing named execution bars retain participant-local IDs
when renamed. Notes, frames, participants, messages, execution bars and annotation
text have source-linked picking regions derived from compiler geometry.

Dragging participants changes their authored lane order. Dragging messages or
fragments changes event order or frame membership. Dragging an annotation note
onto a participant/message changes its layout association. Numeric coordinates
are not stored in an editor-only model.

### BPMN inline authoring

Hover or select an element, flow, pool or lane to reveal its contextual **+**:

- **Add successor…** creates an element and sequence flow together. Gateways offer
  **Add branch…** with an optional flow label where the compiler permits it.
- **Insert element…** splits a sequence flow. Its original label stays upstream;
  gateway attachment rules stay with their original endpoint. Other inline-chain
  declarations, flows and comments remain intact.
- Pools and lanes offer **Add element here…**; pools also offer **Add lane…**.
  Expanded subprocesses offer **Add element inside…**. The source container
  determines membership, not the button's pixel position.
- Activities offer **Attach boundary event** for supported timer, message and
  error events, including non-interrupting timer/message variants. The form
  creates the event and an outgoing target together and lets you choose the
  target type.
- **Connect to existing element →** starts a connection gesture. The configured
  default is tried first; an invalid combination asks for an alternative.

The element form lists types valid in that context. Hovering or selecting a flow
reveals grips at both ends. Drag a grip onto a highlighted node to reconnect that
endpoint while retaining the flow type, label and applicable attachment rules.
The attached source of a boundary event stays with its owning activity; its
outgoing target can be reconnected. Escape cancels. Each accepted change is one
undo step; handles and drag indicators are excluded from exports. BPMN does not
show the sequence diagram's source-order **↑ / ↓** toolbar.

### BPMN membership

**Add construct → Pool** places the existing root process inside the first pool
atomically, retaining its flows, layout references and styles. The dialog explains
this before adding it. Once pools exist, the same action adds another root-level
pool; use **Lane** to add lanes within a pool.
Moving a task moves its attached boundary-event constructs with it. Moving an
expanded subprocess preserves its subtree and updates qualified references.
Compiler rules for sequence/message flow scope remain authoritative; moving an
element never silently converts or deletes a flow. Inline-chain deletion keeps
unrelated nodes and flows on the same source line.

### Timing edits

Horizontal dragging of a timed occurrence changes its explicit numeric time.
Vertical lifeline dragging reorders whole lifeline groups while retaining their
state changes, events and numeric times. The property inspector and Elements
navigator also reach timeline layout records.

### Presentation

The document inspector exposes theme, background, font files and applicable
notation settings even when those settings have not been written yet. **Style
rule** creates a compiler stylesheet rule through separate selector/property/value
fields; the selector picker includes named nodes and supported sequence parts.
Existing rule values can be selected through Elements. A local reset removes the
authored override; editor appearance overrides never rewrite document styling.

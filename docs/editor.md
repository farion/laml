# LAML editor

The `laml-ui` application edits the same `.laml` source as the compiler CLI.
Desktop and browser modes share the Rust/egui editor. The default browser host
stores documents automatically in a browser-local library and offers source/image
downloads; compilation runs transiently on the host. Explicit `--workspace` mode
instead edits files on that host.
Source editing, the rendered preview and the properties panel share one undo/redo
history. A visual transaction is applied only after the compiler successfully
builds its candidate source, including layout and routing. The preview and CLI
share the same placement/routing pipeline: difficult GNE connections can use
longer paths and compact crossing underpasses, then larger automatic spacing
if necessary. Authored layout constraints and endpoint sides stay mandatory;
these automatic geometry changes do not rewrite the source. The rubber-band
line during a drag is transient; dropping runs the full compiler.

See [desktop installation](editor-installation.md) or
[browser hosting and startup](editor-host.md). Build and start the desktop editor
from the repository:

```bash
cargo build --release -p laml-ui
./target/release/laml-ui examples/architecture.laml
```

Build the standalone CLI with `cargo build --release -p laml`.

## First launch

A welcome dialog explains LAML's markup-first workflow and the editor's role as
an aid to writing source. It links to draw.io, yEd and OmniGraffle for users who
prefer freeform drawing. Close it or choose **Continue to LAML Editor** to proceed;
in desktop/workspace mode, any recovery chooser follows afterward. The dismissed state is saved with editor
preferences, per browser origin/profile or native user. Clearing those preferences
shows the introduction again.

## Browser-local documents

In default browser hosting, **Browser library** is a compact file manager for locally
stored documents. The sidebar switches between Documents and Trash; search filters
file names, and the scrollable list sorts them alphabetically and shows diagram
types. Click to select, double-click to open, or right-click for file actions:
open, rename, download and move to trash; trashed files offer restore and permanent
deletion. Icon actions below the list operate on the selected file. Import is in
the top toolbar; the bottom **…** menu contains whole-library deletion and empty trash.
Delete everything, empty trash and individual permanent deletion open a separate
modal confirmation; the library stays behind it. New files are added immediately,
even before their first edit. Edits are saved automatically, even while markup is invalid. The footer reports
browser-save status. **Download LAML** replaces Save/Save as; Export downloads an
SVG, PNG or PDF. Browser mode accepts self-contained sources, inline styles and
bundled fonts/icons, not external assets. Clearing site data deletes the library;
download important sources as backups. See [hosting modes](editor-host.md) for
session admission and storage details. The file-save and recovery controls below
apply to desktop and explicit server-workspace mode.

## Diagram types

**New** opens a dialog with an editable **File name**, prefilled with an unused
default name, and a family/type chooser for Generic node–edge (GNE), the fourteen
UML diagram types, BPMN and the four C4 levels. A name is required; `.laml` is
appended when omitted. Opening a file detects its source type.
Tools and properties adapt to that notation; changing a valid source `type:`
declaration switches them without converting the document.

Use **Add construct…** and **Elements…** for notation-specific structures,
including sequence fragments/annotations, BPMN memberships and timing records.
See [diagram-specific editing and construct inventory](editor-notations.md) for
creation forms, semantic dragging, structural actions and presentation controls.
The graph-editing controls below apply where the selected notation supports them.

## Workspace and controls

The editor uses a three-pane workspace: source on the left, interactive preview
in the center and selection properties on the right. Pane dividers are resizable.

The **Help** icon at the far right of the titlebar opens a compact explanation of
source editing, visual editing, layout and export, with a link to the GitHub
Getting Started documentation.

**Editor theme** controls the application itself. **Settings → Theming** offers
**Auto** (default), **Dark** and **Light**. Auto follows the browser or system
preference, including changes while the editor is open, and falls back to Dark
when that preference is unavailable. Editor settings are saved locally: in browser
storage for the browser editor, and in the user's LAML configuration directory for
the desktop editor.

The diagram **theme** belongs to the `.laml` file. New documents start with an
explicit settings block: `modern-dark` in a dark editor or `modern` in a light
editor, with `background: transparent`. Changing the editor theme does not rewrite
an existing document.

Previews normally use the file's theme and background, including placement and
connection previews. **Ignore LAML theme and fit to editor theme** is unchecked by
default. Enabling it forces `modern-dark` or `modern` with a transparent background
for previews only. Explicit local/imported style colors and custom fonts still
apply. Export always uses the theme and background authored in the file.

A dotted outline marks the rendered image bounds. This outline and selection
marks are editor-only and are not exported. Selection marks remain visible when
the pointer leaves the image or preview.

- The global title bar has Phosphor **New (+)** and **Open** icon buttons on the
  left and a **Settings** gear on the right. Settings opens a centered modal.
  Modal dialogs dim the background and block its pointer and keyboard actions;
  normal file-status windows are centered without blocking editing.
- The tab bar lists open files, marks unsaved changes with a dot and provides an
  **X** to close each tab. It scrolls horizontally when needed. **New** creates a
  separate named document after confirmation in the New file dialog; **Open** adds a tab or activates the file's existing
  tab without replacing any buffer. Each tab keeps its own source, undo/redo,
  text cursor, selection, view, preview, compiler worker and recovery snapshot.
- The source pane has one **Source** header: icon buttons for **Save**, **Undo** and
  **Redo**, then **Save as…**, **Export…**, a separator, and icon buttons for
  **Find / replace…** and **Format source**. Header titles stay left-aligned and
  action groups align right, including responsive overflow. The filename appears
  in its tab.
- Responsive action bars move trailing actions into **…** when space runs out.
  Overflow entries retain their order, enabled state and behavior; icon actions
  include both their icon and text label. This applies to the source and diagram
  headers and footer zoom controls.
- The footer contains **Fit**, **100%**, **−**, **+** and the current zoom percentage,
  left-aligned under the preview. **Fit** stays highlighted while automatic fitting
  is active; manual zooming, panning or **100%** exits that mode.
- The **Diagram** header contains notation-appropriate creation and navigation
  actions. The footer's **Default Relation Type** selects the default for new
  connections. With two or more graph nodes selected, six alignment
  buttons appear: left, horizontal center, right, top, vertical center and bottom.
  Alignment is checked and applied directly as one undo step. Hover rulers preview
  the axis; existing common alignments remain subtle and successful changes flash
  briefly. The active guide is solid, thicker and cyan; reference strokes in the
  alignment icons are cyan too. The last selected node is the alignment reference.
- Properties has **Allgemein** (general) and **Style** tabs. General contains
  identity, element type/access visibility, connection endpoints, layout and members.
  Style contains appearance, dimensions, typography/icons, visibility and applicable
  member/compartment styles. The tab strip and inactive tab use the editor header
  background; the active tab uses the properties-panel fill, a border and blue underline.
  Categories are flat, understated bars spanning the full pane without side gutters.
- Inspector keys form a left-aligned first column; all text fields and value-control
  groups start at the same second-column position, across nodes, ports, connections
  and both tabs. Attribute rows retain side padding; category bars do not.
  Fields have horizontal/vertical inner padding.
  Dropdown and color-picker choices apply directly. The icon chooser previews a
  selection until **Use icon** confirms it. Text fields confirm with
  **Enter** or when focus leaves; **Shift+Enter** inserts a newline where supported.
  Label fields start as one line and grow when an explicit newline is inserted.
  Every accepted change is compiler-checked and has its own undo step.
- The circular-arrow icon resets a local style override. **Diagram actions (…)
  → Elements…** opens a separate diagram navigator, including hidden elements;
  **Connect by address…** opens diagram-wide connection/attachment controls.

## Create elements

Select **+** in the Diagram header. For GNE, the dialog offers **Rectangle** (default),
**Ellipse**, **Diamond**, **Note**, **Package**, **Class**, **Interface**,
**Enumeration**, **Data type**, **Object** and **Junction**. ID and label are
prefilled and editable; automatic suggestions follow the type until you edit them.
Junctions are drawn without labels. **Add node** validates the ID and compiles the
candidate before committing one undo step. Nodes are inserted at the root and
placed by auto-layout. Use Container membership to move them into a package.

A note stores its visible text in semantics. Its shape, like the other palette
shapes, is an explicit local LSS override. Packages contain their child declarations.

Right-click a generic node or class and select **Add port…**. The dialog contains
an explicit ID, port name and side; the closest side to the context click is
prefilled. Port names and IDs follow the identifier grammar: letters, digits and
underscores, starting with a letter or underscore. ID and side are committed
atomically; port names can subsequently be edited independently of their IDs.

## Select and inspect

- Click a node, boundary port or arrow. Shift-click adds/removes individual elements.
  Clicking an arrow label also selects that arrow.
- **Diagram actions → Elements…** lists source elements, including overlapping or
  hidden nodes, ports and relations. Use it to reach an element obscured in the
  picture, or restore its `display` setting. An ancestor hidden by `display: none`
  must itself be restored for its descendants to appear.
- Selection links to the source declaration. Parallel arrows and self-loops retain
  their own identities.
- Properties show effective compiler values, including imported and contextual
  styles. Measured width/height are the actual rendered dimensions. Unspecified
  properties without a concrete rendered value show **automatic / unset**.
- Mixed values are explicit. A mixed node/port/relation selection can be deleted as
  one action; properties that differ by element type are disabled with an explanation.
- Ordinary node labels and inline/block note text can be edited independently of
  IDs, including node-only multi-selection. Classes expose template parameters
  separately from their derived heading text and IDs.
- **Allgemein → Identity → ID** assigns or updates a global `@ID`, preserving
  the visible name. Source references, default descendant/port addresses, layout
  and applicable local selectors are changed atomically. Collisions, reference
  capture and ambiguous shared relative selectors reject the operation.

## Local properties

The node inspector exposes fill, border/text/base color, border width, font size,
padding, fixed/minimum width and height, corner radius, shape, icon, access-marker
rendering and visibility. Structural restrictions are retained: a package has a
rectangular frame; additional icons apply to generic nodes; member compartments
cannot be placed in an ellipse or diamond.

**Icon → Choose icon…** opens a centered Phosphor chooser with actual SVG previews,
name search across the complete bundled catalogue, category filters and a selected-icon
preview. Click a tile, then **Use icon**. **Cancel**, Escape or closing the dialog
leaves the document unchanged. A confirmed choice is compiler-checked and applied
to the original selected nodes as one undo step; stale documents/selections are rejected.
The icon button shows the current icon and name; its reset action restores inherited styles.

**Header mark** changes a node heading's existing visibility marker; individual
members expose their own **Access marks** control. These controls appear where a
marker exists. **Text (+ - # ~)** keeps the written visibility signs; **Icons** uses
globe (public), lock (private), shield (protected) and package (package visibility).
This is separate from the freely chosen node icon. Selecting **Icons** sets the
required `icon-set: phosphor` together with the mode in the same transaction.
There is no separate icon-set prerequisite to configure in the inspector.

Port properties cover fill, border/text/base color and visibility. Relation
properties cover stroke/base color, width, font size and visibility. Colors accept
six-digit hex, `rgb(r,g,b)` and `none` where allowed by the compiler.

Confirming a style field writes an object-specific override. **Reset local override**
removes that property from direct rules in this file, exposing inherited values
again; selected objects may consequently regain different values. Imports and
shared rules remain unchanged. A shadowed or invalid override leaves the source
unchanged and reports the reason in a centered modal, never as a pane-top banner.
Errors start with the cause and a suggested remedy; the original compiler message
remains below under **LAML-Error**. For example, hiding a connected node
explains that its visible arrows must also be hidden or the node kept visible.
Chooser validation uses the same explanation-first presentation inside its dialog.
Fixed sizes and visibility must also satisfy
existing layout, ports and relations.

## Ports and arrows

Hover a node to show four edge-arrow grips and a center grip. Drag from the center
for an automatic source side, or from an edge grip to fix the source side. The
Diagram header's type selector applies only to new connections and is captured at
drag start. Grips, their hit areas, port targets and endpoint handles scale with
canvas zoom and object dimensions, not the UI font size. Self-connections and
parallel relationships are supported.

During dragging, node-side and port drop targets appear. Drop on a node's body for
an automatic target side, on an edge target for a fixed side, or on a port to
connect to that port rather than its owner. Ports can also be drag origins. These
gestures add endpoint-side rules only when a side was explicitly chosen, never
node-placement coordinates. Escape or dropping into empty space cancels.

The relation inspector edits labels, supported GNE operators and label placement
(`near`, `along`, `inline`, or the diagram default). Label placement requires an
actual label. These controls, local styles and endpoint sides support relation-only
multi-selection.

For one selected arrow, drag either blue endpoint handle to reconnect that semantic
end. The old arrow is removed from the preview image during the gesture; the moving
arrow and its markers/labels follow the pointer. Transient orthogonal routes use
the compiler's actual SVG painter, including theme corner treatment, line widths,
colors, dashes, markers, label typography and backgrounds. The checked drop still
computes the final obstacle-aware route. The same side/port targets apply.
Dropping on a node body resets only the moved end's side to automatic; the opposite
end and the relationship's type, label and style are retained. Escape or an empty
drop restores the original arrow without changing source.
Source/target side controls set `left`, `right`, `top` or `bottom`; `auto` removes
only that end's side override, retaining role/multiplicity placement on the same rule.
An unnamed relation receives a unique name only when an addressed property needs
one; generated names respect the configured naming convention.

Select a port to edit its explicit name/ID, local properties or fixed side. Port
side `auto` removes its `port[id] : side=...` rule and exposes the compiler's routing
choice. For a default-addressed port from existing source, first assign an explicit
global ID if you want to change its name without also changing its name-derived ID.

## Element properties, members and attachments

For a single selected node, **Allgemein → Identity** provides **Type** and applicable
**Visibility**, **Abstract** and **Templates** properties. Type offers checked
conversions between Node, Class, Interface, Enumeration and Data type; structural
notes, packages, objects and junctions show their existing kind. Visibility means
access (+ public, - private, # protected, ~ package), while **Style → Display**
controls whether an element is drawn. Enter class template parameters without angle
brackets. Remove template parameters before changing a class to another kind.
These edits preserve IDs, labels, body text, comments and endpoint references;
compiler preflight commits each accepted change as one undo step.

**Members / compartments** edits the selected node's body. Insert applicable field,
method, enum-literal, object-slot, text or separator snippets, then edit their text
and confirm with Enter or by leaving the field. Shift+Enter inserts a newline.
Existing port declarations remain part of this body.
Referenced addresses must remain valid: assign a stable member ID before renaming
a name-derived member that participates in a connection. The ID action updates
local references and member styles atomically. Member controls expose effective
text color/font size, visibility, symbol rendering and icon set; compartment visibility has
direct-edit/reset controls. Imported overrides that still win reject an ineffective
requested change rather than reporting success.

Ports have dedicated ID, name and side fields; arrows have endpoint, label, type,
label-placement and connection-side controls. Use the Source pane for declaration
syntax such as style classes, object classifier headings or relation-end roles and
multiplicities.

**Diagram actions → Connect by address…** creates connections involving explicit member
addresses and attaches a note to a selected source relation. An unnamed target
relation receives a collision-free name in the same transaction. Attachments can
be selected in the Elements list. **Delete selection…** on an attachment
removes that attachment only, retaining its note and target relation. The address
panel also offers previewed attachment removal.

## Containers and deletion

Right-click a node, port, arrow or arrow label to open its context menu. It offers
**Delete this node…**, **Delete this port…** or **Delete this arrow…**, respectively,
as well as **Delete selection…** for the current node/port/arrow selection.
Right-clicking preserves the existing selection. Arrow picking uses the same visible
curves, labels, bridges and gaps as left-click selection. A centered confirmation
lists affected elements and connections before committing; stale targets cannot
overwrite newer source.

For one selected node, use **Allgemein → Layout → Container** and choose a package
or **Root**. The change is checked and applied directly. The transaction moves the
source declaration, rebases default descendant addresses and updates references.
Explicit global IDs stay stable. Contextual styles follow the new ancestry.

Membership changes preserve all relationships, attachments and layout rules. Cycles,
address collisions, reference capture or incompatible constraints reject the whole
operation; the editor does not delete additional rules to make a move fit.

**Delete selection…** in the diagram context menu, or **Delete/Backspace** while
not editing text, previews affected nodes, ports, arrows, attachments, local rules
and source references in a confirmation dialog. Cancel or Escape leaves the source
unchanged. The shortcuts are inactive in text fields, other dialogs and active gestures. Deleting a node/port includes its
incident relations and affected layout rules. One undo restores the whole action.

For a container, choose deletion with contents or **Remove container, keep contents…**.
Keeping contents rebases their default addresses. Collisions or container-local
contextual styles that cannot be preserved cause rejection rather than silent loss.

## Source editor

The source pane has a synchronized line-number gutter. Compiler diagnostics mark
and underline the reported source line; **Go to reported line** selects and scrolls
to it. Imported-file errors retain their own path and line rather than highlighting
an unrelated line in the diagram. Diagnostics are tied to the source revision and
are cleared immediately when that buffer changes. The compiler currently reports
whole lines, so the editor does not guess narrower token spans.

Use **Find / replace…**, `Ctrl/Cmd+F` or `Ctrl/Cmd+H` to open literal search.
`F3` finds the next match; `Shift+F3` finds the previous one, wrapping at EOF/start.
Ignoring case compares per-character Unicode lowercase forms. **Replace match**
requires the selected match; **Replace all** replaces the original non-overlapping
matches in one undo step. These are source-text operations, so replacing a name
here is distinct from the semantic ID-renaming command.

With the source field focused:

- `Tab` inserts spaces to the next four-column stop, or indents selected lines.
- `Shift+Tab` removes up to four leading spaces or one leading tab.
- `Ctrl/Cmd+/` toggles whole-line comments, retaining existing indentation.
- `Enter` carries indentation forward and adds one level after an opening bracket.
  Between a matching bracket pair, it puts the closer on its own outer-indented
  line. Newlines follow the document's existing LF/CRLF convention.

Normal adjacent typing/deletion within 750 ms forms one shared undo step. Cursor
movement, idle gaps, paste, line breaks, source commands, saving, undo and visual
commands separate groups. Each text mutation still receives a new revision so
coalescing cannot make an old compiler result current again.

History retains up to 256 changes and 8 MiB of allocated edit strings, using text
deltas rather than entire-document snapshots. Old or oversized entries are evicted;
the source pane shows retained usage and warns when an edit could not retain an
undo entry. Live and saved buffers are separate from that history budget. Undo in
the search fields edits their drafts instead of undoing the diagram.

`Ctrl+Space` or **Complete…** suggests tokens for the active source context, including
settings, semantic declarations, layout addresses and style properties. Use arrows
and Enter to accept, or Escape to dismiss. Completion is revision-bound and one
shared undo action. Matching delimiter highlighting ignores strings, comments and
free-form note text.

**Format source**, or `Ctrl/Cmd+Shift+F` with the source focused, explicitly normalizes
structural indentation. It preserves note bodies, comments, Unicode and LF/CRLF,
and checks that the parsed compiler model remains equivalent before applying the
patch. It requires parseable GNE source; ordinary editing never formats implicitly.
Source selection also links layout/property declarations and member/attachment
lines to their compiler targets.

## View, arrange and save

Drag the pane separators to resize source and properties. **Fit** centers the
whole diagram; **100%** resets zoom/pan. Use **+**/**−**, Ctrl+wheel or pinch to zoom,
and middle-drag or Space+drag to pan. Zoom anchors at the pointer (viewport center
for buttons). View changes do not write layout coordinates into source.


Drag a node's body away from its connection grips onto relative placement zones.
The proposal shows changed and preserved constraints. Header alignment uses the
last selected node as its reference, replaces conflicting rules only on the edited
axis and preserves constraints on the other axis. Invalid layout leaves source
unchanged and reports the failure in the graphical area. These gestures write
layout intent, not hidden canvas coordinates.

Save explicitly through the file controls. File operations run asynchronously.
You can keep typing during a save: its acknowledgement marks only the submitted
text as saved, so newer edits remain dirty and undo history stays available.
External changes reload automatically only for a clean document; otherwise the
editor presents a conflict. If source is invalid, the last valid picture remains
selectable and is marked out of date. Visual mutations stay locked until the
current source successfully builds.


Use the global **Open** button and **Save as…** for native file dialogs.
In browser mode these controls accept workspace-relative host paths instead.
`Ctrl/Cmd+N` creates a new tab, `Ctrl/Cmd+O`
opens a file and `Ctrl/Cmd+W` closes the active tab. `Ctrl+Tab` and `Ctrl+Shift+Tab`
cycle between tabs. Save and Save as operate on the active tab; Save as refuses to
overwrite a file already open in another tab.

Closing a dirty tab offers keep editing, save or discard. Closing the application
checks all tabs, including inactive ones; cancelling leaves remaining tabs open.
Startup without an explicit file and closing the last tab show an empty workspace,
not an automatically created document. Its icon buttons **Create new file** and
**Open from library** (browser storage) or **Open file** (desktop/server workspace)
open the corresponding dialogs. The empty view disappears while any tab is open.
The New file dialog includes **Include example diagram**, enabled by default:
every diagram type starts with a small, editable example. Uncheck it to create
only the selected type's settings and an empty semantics block. New examples
follow the editor's light or dark theme and use a transparent background.
A close requested during
an accepted write waits for its acknowledgement. Changes typed during that wait
are preserved and checked again before closing. Recovery opens separate unsaved
tabs, leaving existing buffers untouched.

**Export…** builds SVG, PNG or vector PDF through the compiler. After rendering,
existing output requires confirmation. The editor checks the source revision before
submitting the write; the backend rechecks dependencies and the output snapshot.
Changes while rendering or awaiting confirmation cancel that export. If you edit
after the write has already been accepted, it completes for the submitted revision
and the status says that the document has newer changes. The source and its imported
stylesheet/font dependencies are protected against being used as export targets.
Imports (including nested or currently missing files) and local fonts are watched
asynchronously for changes and trigger a fresh build rather than silently retaining
an old picture.

Dirty buffers receive private atomic recovery snapshots. At startup, **Restore
unsaved work?** lists abandoned documents with all entries selected, excluding live
editor sessions. **Restore** / **Restore selected** opens each selected document in
its own tab marked **(recovered)** and unsaved, without replacing an open buffer or
writing to a saved file. Use **Save As** to save the recovered document.

**Not now**, closing the dialog or Escape keeps the drafts available through the
**Restore unsaved work** icon button in the titlebar. Its red badge counts available
recovery drafts; the entire button is hidden when none remain. Like the other
titlebar icons, its background and frame appear only on hover. Clicking opens the
chooser directly.
**Discard selected…** requires confirmation and
removes only the selected recovery drafts, never their saved files. In the browser,
removal waits for host acknowledgement; restoring retains the old draft until the
host confirms its replacement recovery snapshot. Desktop snapshots use the
[local recovery locations](editor-installation.md#files-and-recovery); browser
snapshots use the [host's private session state](editor-host.md#saves-and-recovery).
Browser navigation warns about dirty documents.

### Native UI captures

`laml-ui --capture NEW.png --capture-view general|style|add-node [FILE.laml]`
selects a node and inspector tab, or opens the centered node modal, for an explicit
application-framebuffer capture. It never captures the desktop or overwrites an
existing output file. Omit `--capture-view` for the normal startup view.

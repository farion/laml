# Shared desktop/browser editor — current implementation

Updated: 2026-11-14. This document supersedes earlier host-policy descriptions.

## Product boundary

The native application and browser use the same Rust/egui editor. Browser
HTML/JavaScript only loads the WASM application. Source files, compiler jobs,
dependency checks, exports and recovery are on the configured host.

The host has ordinary network configuration. Access control and encryption can
be provided outside the application by the deployment operator. The editor opens
directly. Protocol correlation/document IDs identify mutable service state; they
are not authentication or user accounts. Recovery uses one configured directory
and normal live-file leases, not browser identity storage.

## Explicit, opt-in policies

Default configuration:

- `limits = false`
- `parallelism = None`
- `http_checks = false`
- `require_https = false`

CLI switches:

- `--limits`: enable the previous size/resource profile (protocol, source,
  dependencies, node/relation counts, retained tickets/recovery, compiler
  watchdog/OS limits, browser timeout). Without the switch those restrictions
  are not applied. Wire-format numeric widths and OS/filesystem capabilities
  are inherent representation/platform constraints, not configurable quotas.
- `--parallelism N`: optional cap on active compiler requests. It is independent
  of `--limits`. Fixed connection/request/response-buffer caps were removed.
- `--http-checks`: optional Host/Origin validation and restrictive security headers.
- `--require-https`: optional HTTPS advertised-origin requirement; TLS is supplied
  by a reverse proxy rather than silently provided by the HTTP listener.
- `--bind IP:PORT`, optional `--origin URL`: generic network parameters. There is
  no provider-specific network integration.

`docs/editor-host.md` describes startup and policy behavior.

## Async shared application

`services::Services` builds native or browser async clients. File/placement jobs,
compiler preflight, transient gesture SVGs and exports keep revision-bound replies.
Source-line selection uses parsed data and needs no filesystem access. Native
transport and HTTP use owned protocol v1 and full preview serialization.

Browser clients in one page share a correlation namespace but retain independent
pending-request handling. The server retains mutable compiler/watch/export state
per page namespace and serializes operations sharing that state. Separate page
clients have independent workers. Abandoned-page recovery file leases are released
when its polling stops; accepted work still holding a process cannot be terminated
by that idle cleanup. Source data is not persisted in browser storage.

The frontend transports and default codec no longer impose fixed queue/message
caps. Explicit bounded codec/client paths support `--limits`; native transport uses
unbounded message channels rather than the former fixed eight-entry queues.

## Editing safety remains

- `.laml` is the truth, with one text/visual undo history.
- Invalid text retains the last valid picture and blocks visual mutations.
- Late/stale replies cannot replace a newer source revision or another file.
- Save acknowledges its submitted snapshot; newer typing stays dirty.
- Save-as/overwrite and export commits recheck target contents and dependencies.
- Accepted writes are not presented as cancellable after a connection is lost.
- Deferred close waits for writes and rechecks dirty state.
- The workspace capability boundary, typed file roles, staged authorized
  dependencies and private-origin translation remain in place.

Recovery lives in `STATE_DIR/recovery`. Restoration loads an unsaved buffer, never
automatically overwrites the original source file, and retains the old snapshot
until the replacement checkpoint is acknowledged. Recovery polling/leases keep
live buffers distinct from abandoned ones. Clearing browser state does not remove
the host's recovery files or select a separate recovery namespace.

## Tools and evidence

- `scripts/build-editor-web.sh`: locked WASM build and matching bindings.
- `scripts/package-editor-host.sh`: local executable/assets/example/guides/notices
  archive, SHA-256 and no-clobber publication.
- `scripts/test-editor-browser.mjs`: real Chrome + real host in default mode and
  with `--limits` (also opts into parallelism 1 and HTTP checks).
- `scripts/test-editor-package.py`: extracted-archive startup, assets, real
  example/stylesheet, checked export and optional native capture.

Browser tests verify policy defaults, source files above the optional size limit,
checked saves, export tickets, source edits, shared undo/redo, direct connections,
inspector edits, SVG/PNG/PDF and page reload/recovery. Full test/build/lint results
are recorded in section 29 of `SPEC/laml-editor-next-session.md`.

Everything remains local/uncommitted. No push, release or normal installation
update. Linux/Chrome/native acceptance does not establish native Windows/macOS,
other browsers or another physical client. Windows cross-compilation on this
machine was blocked by the missing MinGW C compiler.

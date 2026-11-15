# LAML Editor server - Linux x86-64

This release bundle contains prebuilt `laml` and `laml-ui` binaries, matching
browser WASM/JavaScript assets, a Dockerfile and Docker Compose configuration.
The Docker build assembles the runtime image; it does not compile Rust and does
not download another LAML version. Docker Engine with the Compose plugin is the
only server prerequisite. The base image/runtime packages require network access
on the first image build.

## Install

Download `laml-server-vVERSION-linux-amd64.tar.gz` and `SHA256SUMS` from the same
GitHub release. In the download directory (substitute the actual version):

```sh
version=1.17.0
archive="laml-server-v${version}-linux-amd64.tar.gz"
grep -F "  ${archive}" SHA256SUMS | sha256sum --check --strict -
tar -xzf "$archive"
cd "laml-server-v${version}-linux-amd64"
docker compose up -d --build
```

Open `http://YOUR-SERVER:7878`. The default port is published on all interfaces.
The application has no login; use your server's firewall or existing reverse
proxy to select who can access it. Use an HTTPS reverse proxy for a public
installation to protect document contents in transit.

No source checkout, Rust toolchain, image registry, database or data volume is
needed. Documents are stored in the visitor's browser (IndexedDB), not on the
server. Clearing site data loses that library. Keep `.laml` downloads as backups.
Rendering and visual edits require a connection to the server.

## Configure

All deployment values are directly in `compose.yaml`:

- `image`: fixed release version, for example `laml-editor:1.17.0`.
- `ports`: `0.0.0.0:7878:7878` publishes host port 7878 on all interfaces.
  For a host-based reverse proxy, use `127.0.0.1:7878:7878` instead.
- `command`: the value after `--max-users` sets the concurrent browser session
  limit (default 10). Keep the internal `--bind` at `0.0.0.0:7878`.

After changing the configuration, run `docker compose up -d`.
A browser profile shares one session slot across its tabs; a slot expires after
ten minutes without a heartbeat, but running requests retain their slot until
completion.

Keep the public hostname, scheme and port unchanged when updating: browser
storage belongs to that origin, not to a server release or container.

## Update or stop

Download and verify the new version's bundle. Stop the old deployment with
`docker compose down` in its directory, then start the new directory with
`docker compose up -d --build`. Carry over any port/bind/session configuration,
but keep the new bundle's image version. The Compose project name is stable.

```sh
docker compose logs --tail=100
docker compose down
```

The native binaries can also run outside Docker:

```sh
./bin/laml-ui serve --assets "$PWD/web" --bind 0.0.0.0:7878
./bin/laml --version
```

## Source and licenses

`SOURCE.txt` identifies the exact public source tag and build instructions.
The compiler is MIT-licensed; the editor is GPL-3.0-or-later. License notices are
in `licenses/` and are retained in the container image. The corresponding source
archive is supplied alongside the binaries in the same GitHub release.

#!/bin/sh
# Install the latest published LAML release for this machine, without root.
set -eu

for tool in curl tar mktemp; do
    command -v "$tool" >/dev/null 2>&1 || { echo "LAML installer requires $tool" >&2; exit 1; }
done

case "$(uname -s)" in
    Linux)  case "$(uname -m)" in x86_64|amd64) target=x86_64-unknown-linux-gnu ;; *) echo 'Unsupported Linux architecture' >&2; exit 1 ;; esac ;;
    Darwin) case "$(uname -m)" in arm64|aarch64) target=aarch64-apple-darwin ;; x86_64) target=x86_64-apple-darwin ;; *) echo 'Unsupported macOS architecture' >&2; exit 1 ;; esac ;;
    *) echo 'Use scripts/install.ps1 from PowerShell on Windows' >&2; exit 1 ;;
esac

repo=https://github.com/farion/laml
latest="$(curl -fsSL --retry 2 -o /dev/null -w '%{url_effective}' "$repo/releases/latest")"
tag="${latest##*/}"
printf '%s\n' "$tag" | grep -Eq '^v[0-9]+\.[0-9]+\.[0-9]+$' || { echo 'No published stable LAML release found' >&2; exit 1; }
name="laml-$tag-$target"
archive="$name.tar.gz"
base="$repo/releases/download/$tag"
work="$(mktemp -d)"
staged=
cleanup() { [ -z "$staged" ] || rm -f "$staged"; rm -rf "$work"; }
trap cleanup EXIT HUP INT TERM
curl -fsSL --retry 2 -o "$work/$archive" "$base/$archive"
curl -fsSL --retry 2 -o "$work/SHA256SUMS" "$base/SHA256SUMS"
expected="$(awk -v file="$archive" '$2 == file { print $1 }' "$work/SHA256SUMS")"
printf '%s\n' "$expected" | grep -Eq '^[0-9a-fA-F]{64}$' || { echo 'Missing or invalid archive checksum' >&2; exit 1; }
if command -v sha256sum >/dev/null 2>&1; then
    actual="$(sha256sum "$work/$archive" | awk '{print $1}')"
elif command -v shasum >/dev/null 2>&1; then
    actual="$(shasum -a 256 "$work/$archive" | awk '{print $1}')"
else
    echo 'A SHA-256 utility (sha256sum or shasum) is required' >&2; exit 1
fi
[ "$actual" = "$expected" ] || { echo 'Archive checksum mismatch; nothing installed' >&2; exit 1; }

install_dir="${LAML_INSTALL_DIR:-${HOME:?}/.local/bin}"
mkdir -p "$install_dir"
staged="$(mktemp "$install_dir/.laml.XXXXXX")"
tar -xOf "$work/$archive" "$name/laml" > "$staged"
[ -s "$staged" ] || { echo 'Archive contains no LAML binary' >&2; exit 1; }
chmod 755 "$staged"
mv -f "$staged" "$install_dir/laml"
staged=
echo "Installed LAML $tag to $install_dir/laml"
case ":$PATH:" in *":$install_dir:"*) ;; *) echo "Add $install_dir to your PATH to run laml" ;; esac

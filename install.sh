#!/usr/bin/env bash
set -euo pipefail

repo="owainlewis/cortex"
install_dir="${CORTEX_INSTALL_DIR:-$HOME/.local/bin}"
version=""
install_tmp=""

usage() {
  cat <<'USAGE'
Install or update Cortex from GitHub Releases.

Usage:
  install.sh [--version vX.Y.Z]

Environment:
  CORTEX_INSTALL_DIR  Install directory. Defaults to ~/.local/bin.
USAGE
}

error() {
  printf 'cortex install: %s\n' "$*" >&2
  exit 1
}

need() {
  command -v "$1" >/dev/null 2>&1 || error "missing required command: $1"
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --version)
      [ "$#" -ge 2 ] || error "--version requires a tag such as v0.2.0"
      version="$2"
      shift 2
      ;;
    -h | --help)
      usage
      exit 0
      ;;
    *)
      error "unknown argument: $1"
      ;;
  esac
done

need curl
need head
need install
need mkdir
need mktemp
need mv
need rm
need sed
need shasum
need tar
need uname

os="$(uname -s)"
arch="$(uname -m)"

if [ "$os" != "Darwin" ]; then
  error "unsupported platform: $os. Cortex currently ships macOS binaries only."
fi

case "$arch" in
  arm64)
    target_triple="aarch64-apple-darwin"
    ;;
  *)
    error "unsupported macOS architecture: $arch. Cortex currently ships arm64 binaries only."
    ;;
esac

latest_version() {
  response="$(curl -fsSL "https://api.github.com/repos/${repo}/releases/latest")" \
    || error "failed to fetch latest release metadata"
  tag="$(printf '%s\n' "$response" | sed -n 's/.*"tag_name"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' | head -n 1)"
  [ -n "$tag" ] || error "could not find a latest release tag"
  printf '%s\n' "$tag"
}

if [ -z "$version" ]; then
  version="$(latest_version)"
fi

archive="cortex-${version}-${target_triple}.tar.gz"
checksum="${archive}.sha256"
base_url="https://github.com/${repo}/releases/download/${version}"
tmp_dir="$(mktemp -d)"

cleanup() {
  rm -rf "$tmp_dir"
  if [ -n "$install_tmp" ]; then
    rm -f "$install_tmp"
  fi
}
trap cleanup EXIT

printf 'Installing Cortex %s for %s\n' "$version" "$target_triple"

curl -fsSL "${base_url}/${archive}" -o "${tmp_dir}/${archive}" \
  || error "failed to download ${archive}"
curl -fsSL "${base_url}/${checksum}" -o "${tmp_dir}/${checksum}" \
  || error "failed to download ${checksum}"

(
  cd "$tmp_dir"
  shasum -a 256 -c "$checksum" >/dev/null
) || error "checksum verification failed"

tar -xzf "${tmp_dir}/${archive}" -C "$tmp_dir" cortex \
  || error "failed to extract cortex binary"

mkdir -p "$install_dir"
install_tmp="$(mktemp "${install_dir}/.cortex.XXXXXX")" \
  || error "failed to create temporary install file in ${install_dir}"
install -m 0755 "${tmp_dir}/cortex" "$install_tmp" \
  || error "failed to stage cortex binary in ${install_dir}"
mv -f "$install_tmp" "${install_dir}/cortex" \
  || error "failed to replace ${install_dir}/cortex"
install_tmp=""

printf 'Installed cortex to %s/cortex\n' "$install_dir"

case ":$PATH:" in
  *":$install_dir:"*) ;;
  *)
    printf 'Add %s to PATH to run cortex from any shell.\n' "$install_dir"
    ;;
esac

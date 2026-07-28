#!/usr/bin/env bash
set -euo pipefail

error() {
  printf 'package macOS arm64: %s\n' "$*" >&2
  exit 1
}

need() {
  command -v "$1" >/dev/null 2>&1 || error "missing required command: $1"
}

if [ "$#" -ne 2 ]; then
  error "usage: $0 <arm64-binary> <archive.tar.gz>"
fi

binary="$1"
archive="$2"
checksum="${archive}.sha256"
work_dir=""
archive_tmp=""
checksum_tmp=""

cleanup() {
  if [ -n "$work_dir" ]; then
    rm -rf -- "$work_dir"
  fi
  if [ -n "$archive_tmp" ]; then
    rm -f -- "$archive_tmp"
  fi
  if [ -n "$checksum_tmp" ]; then
    rm -f -- "$checksum_tmp"
  fi
}
trap cleanup EXIT

need dirname
need gzip
need install
need lipo
need mkdir
need mktemp
need mv
need rm
need shasum
need tar
need touch
need uname

[ -f "$binary" ] || error "expected arm64 binary was not produced: $binary"
[ -x "$binary" ] || error "expected arm64 binary is not executable: $binary"

binary_arches="$(lipo -archs "$binary" 2>/dev/null)" \
  || error "could not inspect binary architecture: $binary"
[ "$binary_arches" = "arm64" ] \
  || error "expected a thin arm64 binary, found: $binary_arches"

if [ "$(uname -m)" = "arm64" ]; then
  "$binary" --version >/dev/null \
    || error "arm64 binary smoke check failed: $binary"
fi

archive_dir="$(dirname "$archive")"
mkdir -p "$archive_dir"
work_dir="$(mktemp -d "${TMPDIR:-/tmp}/cortex-package.XXXXXX")"
mkdir "${work_dir}/staging" "${work_dir}/verify"
archive_tmp="$(mktemp "${archive}.tmp.XXXXXX")"
checksum_tmp="$(mktemp "${checksum}.tmp.XXXXXX")"

install -m 0755 "$binary" "${work_dir}/staging/cortex"
TZ=UTC touch -t 197001010000 "${work_dir}/staging/cortex"

COPYFILE_DISABLE=1 tar \
  --format ustar \
  --uid 0 \
  --gid 0 \
  --uname root \
  --gname wheel \
  -cf - \
  -C "${work_dir}/staging" \
  cortex \
  | gzip -n -9 > "$archive_tmp"

archive_contents="$(tar -tzf "$archive_tmp")"
[ "$archive_contents" = "cortex" ] \
  || error "archive must contain only cortex, found: $archive_contents"

tar -xzf "$archive_tmp" -C "${work_dir}/verify"
[ -x "${work_dir}/verify/cortex" ] \
  || error "archived cortex binary is not executable"

archived_arches="$(lipo -archs "${work_dir}/verify/cortex" 2>/dev/null)" \
  || error "could not inspect archived binary architecture"
[ "$archived_arches" = "arm64" ] \
  || error "expected archived thin arm64 binary, found: $archived_arches"

mv -f "$archive_tmp" "$archive"
archive_tmp=""

shasum -a 256 "$archive" > "$checksum_tmp"
mv -f "$checksum_tmp" "$checksum"
checksum_tmp=""
shasum -a 256 -c "$checksum" >/dev/null \
  || error "checksum verification failed"

printf 'Packaged %s (%s) with checksum %s\n' \
  "$archive" "$archived_arches" "$checksum"

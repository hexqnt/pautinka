#!/usr/bin/env bash
set -euo pipefail

MODE="${1:-all}"
if [[ $# -gt 1 ]] || [[ "$MODE" != all && "$MODE" != --build-only && "$MODE" != --upload-only ]]; then
  echo "Usage: $0 [--build-only|--upload-only]" >&2
  exit 1
fi

PROJECT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
DIST_DIR="${DIST_DIR:-${PROJECT_DIR}/target/stress-dist}"
if [[ "$DIST_DIR" != /* ]]; then
  DIST_DIR="${PROJECT_DIR}/${DIST_DIR}"
fi
DIST_DIR="${DIST_DIR%/}"
S3_BUCKET="${S3_BUCKET:-pautinka.hexq.ru}"
S3_PREFIX="${S3_PREFIX:-}"
while [[ "$S3_PREFIX" == /* ]]; do S3_PREFIX="${S3_PREFIX#/}"; done
while [[ "$S3_PREFIX" == */ ]]; do S3_PREFIX="${S3_PREFIX%/}"; done

require_command() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "Error: required command '$1' is not available in PATH." >&2
    exit 1
  fi
}

content_type_for() {
  case "$1" in
    *.html) echo "text/html; charset=utf-8" ;;
    *.js | *.mjs) echo "text/javascript; charset=utf-8" ;;
    *.css) echo "text/css; charset=utf-8" ;;
    *.wasm) echo "application/wasm" ;;
    *.svg) echo "image/svg+xml" ;;
    *.png) echo "image/png" ;;
    *.jpg | *.jpeg) echo "image/jpeg" ;;
    *.webp) echo "image/webp" ;;
    *.json) echo "application/json" ;;
    *.txt) echo "text/plain; charset=utf-8" ;;
    *.ico) echo "image/x-icon" ;;
    *.woff2) echo "font/woff2" ;;
    *) echo "application/octet-stream" ;;
  esac
}

if [[ "$MODE" != --build-only ]]; then
  require_command yc
  require_command gzip
  require_command find
  require_command sort
  if [[ ! "$S3_BUCKET" =~ ^[a-z0-9][a-z0-9.-]+[a-z0-9]$ ]]; then
    echo "Error: S3_BUCKET must be a bucket name without a scheme or path." >&2
    exit 1
  fi
fi

if [[ "$MODE" != --upload-only ]]; then
  require_command trunk
  echo "Building the stress demo with trunk..."
  (
    cd -- "${PROJECT_DIR}/demos/stress"
    trunk build --release --locked --public-url ./ --dist "$DIST_DIR"
  )
fi

if [[ ! -s "$DIST_DIR/index.html" ]]; then
  echo "Error: '$DIST_DIR/index.html' is missing or empty." >&2
  exit 1
fi
if [[ "$MODE" == --build-only ]]; then
  echo "Build complete: $DIST_DIR"
  exit 0
fi

TARGET_BASE_URI="s3://${S3_BUCKET}"
if [[ -n "$S3_PREFIX" ]]; then
  TARGET_BASE_URI+="/${S3_PREFIX}"
fi
UPLOAD_TEMP="$(mktemp -d)"
trap 'rm -rf -- "$UPLOAD_TEMP"' EXIT

upload_file() {
  local src_path="$1"
  local rel_path="${src_path#"$DIST_DIR"/}"
  local options=(--content-type "$(content_type_for "$rel_path")" --cache-control no-cache)

  if [[ "$rel_path" == *.wasm ]]; then
    gzip -9 -n -c -- "$src_path" >"$UPLOAD_TEMP/asset.wasm.gz"
    src_path="$UPLOAD_TEMP/asset.wasm.gz"
    options+=(--content-encoding gzip)
  fi

  yc storage s3 cp \
    "$src_path" "${TARGET_BASE_URI}/${rel_path}" \
    "${options[@]}"
  echo "Uploaded: ${rel_path} -> ${TARGET_BASE_URI}/${rel_path}"
}

mapfile -d '' dist_files < <(find "$DIST_DIR" -type f -print0 | sort -z)
html_files=()
echo "Uploading static assets to ${TARGET_BASE_URI}..."
for src_path in "${dist_files[@]}"; do
  if [[ "$src_path" == *.html ]]; then
    html_files+=("$src_path")
  else
    upload_file "$src_path"
  fi
done

# HTML переключается после ресурсов, до удаления старых файлов.
for src_path in "${html_files[@]}"; do
  upload_file "$src_path"
done

cleanup_options=(--recursive)
for src_path in "${dist_files[@]}"; do
  cleanup_options+=(--exclude "${src_path#"$DIST_DIR"/}")
done
echo "Removing obsolete files from ${TARGET_BASE_URI}..."
yc storage s3 rm "${TARGET_BASE_URI}/" "${cleanup_options[@]}"

echo "Publish complete."

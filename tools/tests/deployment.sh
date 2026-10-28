#!/usr/bin/env bash
set -euo pipefail

PROJECT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"
TEST_DIR="$(mktemp -d)"
trap 'rm -rf -- "$TEST_DIR"' EXIT
mkdir -p "$TEST_DIR/bin" "$TEST_DIR/dist"
export PATH="$TEST_DIR/bin:$PATH"
export DIST_DIR="$TEST_DIR/dist"
export DEPLOY_TEST_LOG="$TEST_DIR/upload.log"
export DEPLOY_TEST_PROJECT="$PROJECT_DIR"
export S3_BUCKET="test.example.org"
export S3_PREFIX="//demo/stress//"

cat > "$TEST_DIR/bin/trunk" <<'MOCK'
#!/usr/bin/env bash
set -euo pipefail
[[ "$PWD" == "$DEPLOY_TEST_PROJECT/demos/stress" ]]
[[ "$*" == "build --release --locked --public-url ./ --dist $DIST_DIR" ]]
[[ "${DEPLOY_TEST_BUILD_FAIL:-0}" == 0 ]]
printf '<html></html>' > "$DIST_DIR/index.html"
printf '\0asm\1\0\0\0' > "$DIST_DIR/app.wasm"
mkdir -p "$DIST_DIR/snippets"
printf 'export {}' > "$DIST_DIR/snippets/a file.js"
MOCK
cat > "$TEST_DIR/bin/yc" <<'MOCK'
#!/usr/bin/env bash
set -euo pipefail
[[ "$1" == storage && "$2" == s3 && "$3" == cp ]]
source_path="$4"
destination="$5"
shift 5
[[ "$*" == *'--cache-control no-cache'* ]]
case "$destination" in
  *.wasm)
    [[ "$*" == *'--content-type application/wasm'* && "$*" == *'--content-encoding gzip'* ]]
    gzip -dc "$source_path" | cmp - "$DIST_DIR/app.wasm"
    [[ "${DEPLOY_TEST_UPLOAD_FAIL:-0}" == 0 ]]
    ;;
  *.html) [[ "$*" == *'--content-type text/html; charset=utf-8'* ]] ;;
  *.js) [[ "$*" == *'--content-type text/javascript; charset=utf-8'* ]] ;;
  *) exit 1 ;;
esac
printf '%s\n' "$destination" >> "$DEPLOY_TEST_LOG"
MOCK
chmod +x "$TEST_DIR/bin/"*

# Запуск снаружи репозитория проверяет привязку путей к скрипту.
cd -- "$TEST_DIR"
bash "$PROJECT_DIR/build_and_push.sh" --build-only
[[ ! -e "$DEPLOY_TEST_LOG" ]]
bash "$PROJECT_DIR/build_and_push.sh" --upload-only
mapfile -t uploads < "$DEPLOY_TEST_LOG"
[[ ${#uploads[@]} == 3 ]]
[[ "${uploads[0]}" == s3://test.example.org/demo/stress/app.wasm ]]
[[ "${uploads[1]}" == 's3://test.example.org/demo/stress/snippets/a file.js' ]]
[[ "${uploads[2]}" == s3://test.example.org/demo/stress/index.html ]]
[[ ! -e "$DIST_DIR/app.wasm.gz" ]]

rm -- "$DEPLOY_TEST_LOG"
S3_PREFIX='' bash "$PROJECT_DIR/build_and_push.sh"
[[ "$(tail -n 1 "$DEPLOY_TEST_LOG")" == s3://test.example.org/index.html ]]

rm -- "$DEPLOY_TEST_LOG"
if DEPLOY_TEST_UPLOAD_FAIL=1 bash "$PROJECT_DIR/build_and_push.sh" --upload-only; then
  echo "Error: a failed asset upload must stop deployment." >&2
  exit 1
fi
[[ ! -e "$DEPLOY_TEST_LOG" ]]
if DEPLOY_TEST_BUILD_FAIL=1 bash "$PROJECT_DIR/build_and_push.sh"; then
  echo "Error: a failed build must stop deployment." >&2
  exit 1
fi
[[ ! -e "$DEPLOY_TEST_LOG" ]]

rm -- "$DIST_DIR/index.html"
if bash "$PROJECT_DIR/build_and_push.sh" --upload-only; then
  echo "Error: a missing index must stop deployment." >&2
  exit 1
fi
[[ ! -e "$DEPLOY_TEST_LOG" ]]
if S3_BUCKET='s3://wrong/path' bash "$PROJECT_DIR/build_and_push.sh" --upload-only; then
  echo "Error: an invalid bucket must stop deployment." >&2
  exit 1
fi
if bash "$PROJECT_DIR/build_and_push.sh" --unknown; then
  echo "Error: an unknown option must be rejected." >&2
  exit 1
fi

echo "Deployment script checks passed."

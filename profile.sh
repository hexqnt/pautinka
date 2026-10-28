#!/usr/bin/env bash
set -euo pipefail

usage() {
    cat <<'EOF'
Usage: ./profile.sh [--config FILE] [OUTPUT]

Build and launch the native stress demo under Linux perf.
Load a TOML scenario with --config; its run limits can finish recording automatically.
Without run limits, close the demo window to finish recording.
OUTPUT defaults to a new directory under target/stress-profile/.
A copy of the scenario is saved alongside OUTPUT as OUTPUT.toml.
EOF
}

config=
output=
while (( $# > 0 )); do
    case $1 in
        --help|-h)
            usage
            exit 0
            ;;
        --config)
            if (( $# < 2 )) || [[ -z ${2:-} || -n $config ]]; then
                echo 'Error: --config requires one path and can only be specified once.' >&2
                exit 2
            fi
            config=$2
            shift 2
            ;;
        --)
            shift
            if (( $# != 1 )) || [[ -n $output || -z ${1:-} ]]; then
                usage >&2
                exit 2
            fi
            output=$1
            shift
            ;;
        -*)
            printf 'Error: unknown option: %s\n' "$1" >&2
            exit 2
            ;;
        *)
            if [[ -n $output || -z $1 ]]; then
                usage >&2
                exit 2
            fi
            output=$1
            shift
            ;;
    esac
done
if [[ -n $config && ! -f $config ]]; then
    printf 'Error: scenario file not found: %s\n' "$config" >&2
    exit 2
fi

if [[ $(uname -s) != Linux ]]; then
    echo 'Error: this script requires Linux perf.' >&2
    exit 1
fi
for tool in cargo rustc perf; do
    if ! command -v "$tool" > /dev/null; then
        printf 'Error: required command not found: %s\n' "$tool" >&2
        exit 1
    fi
done

repo_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
profile_dir="$repo_dir/target/stress-profile"
if [[ -n $output ]]; then
    mkdir -p -- "$(dirname -- "$output")"
    if [[ -e $output ]]; then
        printf 'Error: output already exists: %s\n' "$output" >&2
        exit 1
    fi
else
    mkdir -p -- "$profile_dir"
    output="$(mktemp -d "$profile_dir/recording-XXXXXX")/perf.data"
fi
if [[ -n $config && -e $output.toml ]]; then
    printf 'Error: scenario copy already exists: %s.toml\n' "$output" >&2
    exit 1
fi

# Отдельный каталог сохраняет сборку с символами; явная host-цель исключает WASM из Cargo-конфига.
host_target=$(rustc -vV | sed -n 's/^host: //p')
build_dir="$profile_dir/build"
CARGO_PROFILE_RELEASE_DEBUG=line-tables-only CARGO_PROFILE_RELEASE_STRIP=none \
    cargo build --manifest-path "$repo_dir/Cargo.toml" --locked --release \
    --target "$host_target" --target-dir "$build_dir" \
    -p pautinka-stress --bin pautinka-stress

demo_args=()
if [[ -n $config ]]; then
    cp -- "$config" "$output.toml"
    demo_args=(--config "$output.toml")
    printf 'Scenario saved to %s.toml\n' "$output"
fi
printf 'Recording to %s\nThe scenario can finish automatically; closing the demo also stops recording.\n' "$output"
printf 'Inspect the recording with: perf report -i %q\n' "$output"
perf record --call-graph dwarf -F 997 -e cycles:u -o "$output" \
    -- "$build_dir/$host_target/release/pautinka-stress" "${demo_args[@]}"

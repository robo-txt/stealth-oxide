#!/usr/bin/env bash
set -u

lab_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd -- "$lab_root/../.." && pwd)"
artifact_dir="$lab_root/artifacts"
mkdir -p -- "$artifact_dir"

run_case() {
    name="$1"
    shift
    output="$artifact_dir/creepjs-${name}.json"
    echo "Running CreepJS case: $name"
    (
        cd -- "$repo_root"
        env \
            STEALTH_OXIDE_DIAGNOSTIC_PROFILE=linux \
            STEALTH_OXIDE_DIAGNOSTIC_WAIT="${STEALTH_OXIDE_DIAGNOSTIC_WAIT:-5}" \
            "$@" \
            cargo run --quiet --example site_diagnostic -- \
            https://abrahamjuliot.github.io/creepjs/
    ) >"$output" 2>&1
    status=$?
    if [ "$status" -ne 0 ]; then
        echo "  failed with status $status; see $output" >&2
    else
        echo "  wrote $output"
    fi
}

run_case legacy-headless
run_case mesa-angle STEALTH_OXIDE_USE_MESA=1

echo "CreepJS artifacts are under $artifact_dir"
echo "Headful is intentionally not launched by this script."

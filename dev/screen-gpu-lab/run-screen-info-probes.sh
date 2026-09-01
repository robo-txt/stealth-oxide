#!/usr/bin/env bash
set -u

lab_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
artifact_dir="$lab_root/artifacts"
mkdir -p -- "$artifact_dir"

if [ -n "${CHROME_BIN-}" ]; then
    chrome="$CHROME_BIN"
elif command -v google-chrome >/dev/null 2>&1; then
    chrome="$(command -v google-chrome)"
elif command -v chromium >/dev/null 2>&1; then
    chrome="$(command -v chromium)"
elif command -v chromium-browser >/dev/null 2>&1; then
    chrome="$(command -v chromium-browser)"
else
    echo "No Chromium executable found; set CHROME_BIN." >&2
    exit 1
fi

probe_url="file://$lab_root/probe.html"

run_case() {
    name="$1"
    shift
    profile_dir="$(mktemp -d "${TMPDIR:-/tmp}/stealth-oxide-lab-${name}.XXXXXX")"
    output="$artifact_dir/chromium-${name}.html"
    echo "Running $name"
    "$chrome" \
        --headless \
        --disable-dev-shm-usage \
        --no-first-run \
        --no-default-browser-check \
        --user-data-dir="$profile_dir" \
        --dump-dom \
        "$@" \
        "$probe_url" >"$output" 2>&1
    status=$?
    rm -rf -- "$profile_dir"
    if [ "$status" -ne 0 ]; then
        echo "  failed with status $status; see $output" >&2
    else
        echo "  wrote $output"
    fi
}

run_case legacy-headless
run_case mesa-angle \
    --enable-gpu \
    --use-gl=angle \
    --use-angle=gl \
    --ignore-gpu-blocklist \
    --enable-gpu-rasterization
run_case screen-info \
    '--screen-info={0,0 1920x1080 workAreaBottom=40}'
run_case modern-screen-info \
    --headless=new \
    '--screen-info={0,0 1920x1080 workAreaBottom=40}'
run_case modern-screen-info-mesa \
    --headless=new \
    --enable-gpu \
    --use-gl=angle \
    --use-angle=gl \
    --ignore-gpu-blocklist \
    --enable-gpu-rasterization \
    '--screen-info={0,0 1920x1080 workAreaBottom=40}'

echo "Probe artifacts are under $artifact_dir"

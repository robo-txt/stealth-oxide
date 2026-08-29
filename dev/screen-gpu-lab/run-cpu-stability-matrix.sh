#!/usr/bin/env bash
set -u

lab_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
artifact_dir="$lab_root/artifacts/cpu-stability"
mkdir -p -- "$artifact_dir"

if [ -n "${CHROME_BIN-}" ]; then
    chrome="$CHROME_BIN"
elif command -v google-chrome >/dev/null 2>&1; then
    chrome="$(command -v google-chrome)"
elif command -v chromium >/dev/null 2>&1; then
    chrome="$(command -v chromium)"
else
    echo "No Chromium executable found; set CHROME_BIN." >&2
    exit 1
fi

probe_url="file://$lab_root/cpu-stability-probe.html"
summary="$artifact_dir/summary.tsv"
printf 'case\texit_status\twebgl1\twebgl2\trenderers\tlog\n' >"$summary"

run_case() {
    name="$1"
    shift
    profile_dir="$(mktemp -d "${TMPDIR:-/tmp}/stealth-oxide-cpu-${name}.XXXXXX")"
    output="$artifact_dir/$name.html"
    log="$artifact_dir/$name.log"
    echo "Running $name"
    env_args=()
    while [ "$#" -gt 0 ] && [[ "$1" == *=* ]]; do
        env_args+=("$1")
        shift
    done
    timeout --foreground -k 3s "${CASE_TIMEOUT:-20}s" env "${env_args[@]}" "$chrome" \
        --headless=new \
        --disable-dev-shm-usage \
        --no-first-run \
        --no-default-browser-check \
        --no-sandbox \
        --user-data-dir="$profile_dir" \
        --enable-logging=stderr \
        --dump-dom \
        "$@" \
        "$probe_url" >"$output" 2>"$log"
    status=$?
    webgl1="$(awk '/"name": "webgl"/ {getline; if ($0 ~ /"available":/) {gsub(/[^a-z]/, ""); print}}' "$output" | head -1 || true)"
    webgl2="$(awk '/"name": "webgl2"/ {getline; if ($0 ~ /"available":/) {gsub(/[^a-z]/, ""); print}}' "$output" | head -1 || true)"
    renderers="$(sed -n 's/.*"renderer": "\([^"]*\)".*/\1/p' "$output" | tr '\n' '|' || true)"
    printf '%s\t%s\t%s\t%s\t%s\t%s\n' "$name" "$status" "${webgl1:-unknown}" "${webgl2:-unknown}" "${renderers:-none}" "$log" >>"$summary"
    echo "  status=$status output=$output log=$log"
    rm -rf -- "$profile_dir"
}

run_case legacy-native
run_case modern-native
run_case modern-llvmpipe \
    LIBGL_ALWAYS_SOFTWARE=true MESA_LOADER_DRIVER_OVERRIDE=llvmpipe \
    --enable-gpu --use-gl=angle --use-angle=gl --ignore-gpu-blocklist --enable-gpu-rasterization
run_case modern-softpipe \
    LIBGL_ALWAYS_SOFTWARE=true MESA_LOADER_DRIVER_OVERRIDE=softpipe \
    --enable-gpu --use-gl=angle --use-angle=gl --ignore-gpu-blocklist --enable-gpu-rasterization
run_case modern-swiftshader \
    --enable-gpu --use-gl=angle --use-angle=swiftshader --enable-unsafe-swiftshader

if [ -n "${DISPLAY-}" ]; then
    run_case x11-llvmpipe \
        LIBGL_ALWAYS_SOFTWARE=true MESA_LOADER_DRIVER_OVERRIDE=llvmpipe \
        --ozone-platform=x11 --enable-gpu --use-gl=angle --use-angle=gl \
        --ignore-gpu-blocklist --enable-gpu-rasterization
fi

echo "Summary: $summary"

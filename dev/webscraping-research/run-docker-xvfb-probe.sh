#!/usr/bin/env bash
set -u

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"
image="${RESEARCH_IMAGE:-stealth-oxide/xvfb-research:latest}"
artifact_dir="$repo_root/dev/screen-gpu-lab/artifacts/docker-xvfb"
mkdir -p -- "$artifact_dir"

run_case() {
    name="$1"
    driver="$2"
    vendor="${3-}"
    renderer="${4-}"
    output="$artifact_dir/$name.html"
    log="$artifact_dir/$name.log"
    docker_env=(
        -e LIBGL_ALWAYS_SOFTWARE=true
        -e "MESA_LOADER_DRIVER_OVERRIDE=$driver"
    )
    if [ -n "$vendor" ]; then
        docker_env+=( -e "ANGLE_GL_VENDOR=$vendor" )
    fi
    if [ -n "$renderer" ]; then
        docker_env+=( -e "ANGLE_GL_RENDERER=$renderer" )
    fi
    echo "Running Docker Xvfb case: $name"
    timeout --foreground -k 3s "${CASE_TIMEOUT:-30}s" docker run --rm \
        --network none \
        --shm-size=2g \
        "${docker_env[@]}" \
        -v "$repo_root/dev/screen-gpu-lab:/research:ro" \
        --entrypoint /bin/sh \
        "$image" \
        -c 'Xvfb :99 -screen 0 1920x1080x24 -nolisten tcp >/tmp/xvfb.log 2>&1 &
            xvfb_pid=$!
            trap "kill $xvfb_pid 2>/dev/null || true" EXIT
            export DISPLAY=:99
            exec /usr/bin/chromium --headless=new --ozone-platform=x11 --no-sandbox \
                --disable-dev-shm-usage --no-first-run --no-default-browser-check \
                --user-data-dir=/tmp/chrome-profile --enable-gpu --use-gl=angle \
                --use-angle=gl --ignore-gpu-blocklist --enable-gpu-rasterization \
                --dump-dom file:///research/cpu-stability-probe.html' \
        /bin/sh >"$output" 2>"$log"
    status=$?
    echo "  status=$status output=$output log=$log"
}

run_case mesa-llvmpipe llvmpipe
run_case mesa-softpipe softpipe
run_case mesa-llvmpipe-angle-profile llvmpipe AMD "AMD Radeon HD 3200 Graphics"

echo "Artifacts: $artifact_dir"

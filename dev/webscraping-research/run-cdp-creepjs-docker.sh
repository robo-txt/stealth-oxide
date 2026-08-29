#!/usr/bin/env bash
set -u

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"
image="${RESEARCH_IMAGE:-stealth-oxide/xvfb-research:latest}"
artifact_dir="$repo_root/dev/screen-gpu-lab/artifacts/docker-xvfb"
mkdir -p -- "$artifact_dir"

container_id=""
cleanup() {
    if [ -n "$container_id" ]; then
        docker stop "$container_id" >/dev/null 2>&1 || true
    fi
}
trap cleanup EXIT

container_id="$(docker run -d --rm \
    --network host \
    --shm-size=2g \
    -e LIBGL_ALWAYS_SOFTWARE=true \
    -e MESA_LOADER_DRIVER_OVERRIDE=llvmpipe \
    -e ANGLE_GL_VENDOR=AMD \
    -e 'ANGLE_GL_RENDERER=AMD Radeon HD 3200 Graphics' \
    --entrypoint /bin/sh \
    "$image" \
    -c 'Xvfb :99 -screen 0 1920x1080x24 -nolisten tcp >/tmp/xvfb.log 2>&1 &
        xvfb_pid=$!
        trap "kill $xvfb_pid 2>/dev/null || true" EXIT
        export DISPLAY=:99
        exec /usr/bin/chromium --headless=new --ozone-platform=x11 --no-sandbox \
            --disable-dev-shm-usage --no-first-run --no-default-browser-check \
            --user-data-dir=/tmp/chrome-creepjs-profile --enable-gpu --use-gl=angle \
            --use-angle=gl --ignore-gpu-blocklist --enable-gpu-rasterization \
            --remote-debugging-address=0.0.0.0 --remote-debugging-port=9222 \
            about:blank')"

ready=false
for _ in $(seq 1 30); do
    if curl -fsS http://127.0.0.1:9222/json/version >/dev/null; then
        ready=true
        break
    fi
    sleep 1
done
if [ "$ready" != true ]; then
    echo "Chromium CDP endpoint did not become ready" >&2
    docker logs "$container_id" >&2 || true
    exit 1
fi

output="$artifact_dir/creepjs-docker-angle-profile.json"
log="$artifact_dir/creepjs-docker-angle-profile.log"
screenshot="$artifact_dir/creepjs-docker-angle-profile.png"
timeout --foreground -k 3s "${CASE_TIMEOUT:-120}s" \
    env CREEPJS_SCREENSHOT="$screenshot" CREEPJS_WAIT_SECONDS="${CREEPJS_WAIT_SECONDS:-15}" \
    cargo run --offline --quiet --manifest-path \
    "$repo_root/dev/webscraping-research/cdp-creepjs-probe/Cargo.toml" \
    >"$output" 2>"$log"
status=$?
echo "status=$status output=$output screenshot=$screenshot log=$log"

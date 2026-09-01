#!/usr/bin/env bash
set -u

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"
image="${RESEARCH_IMAGE:-stealth-oxide/xvfb-research:latest}"
artifact_dir="$repo_root/dev/screen-gpu-lab/artifacts/docker-xvfb"
mkdir -p -- "$artifact_dir"

output="$artifact_dir/worker-propagation.html"
log="$artifact_dir/worker-propagation.log"

timeout --foreground -k 3s "${CASE_TIMEOUT:-30}s" docker run --rm \
  --network host \
  --shm-size=2g \
  -e LIBGL_ALWAYS_SOFTWARE=true \
  -e MESA_LOADER_DRIVER_OVERRIDE=llvmpipe \
  -e ANGLE_GL_VENDOR=AMD \
  -e 'ANGLE_GL_RENDERER=AMD Radeon HD 3200 Graphics' \
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
        --virtual-time-budget=20000 --dump-dom \
        http://127.0.0.1:8765/worker-propagation.html' \
  /bin/sh >"$output" 2>"$log"
status=$?
echo "status=$status output=$output log=$log"

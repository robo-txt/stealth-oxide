#!/bin/sh
set -eu

# Refresh the per-user cache so fonts mounted by Kubernetes or Compose after
# image construction are visible to Chromium.
fc-cache -f >/dev/null

Xvfb "${DISPLAY}" -screen 0 1920x1080x24 -ac +extension GLX +render -noreset \
    >/tmp/stealth-oxide-xvfb.log 2>&1 &
xvfb_pid=$!

cleanup() {
    kill "${panel_pid:-}" "${window_manager_pid:-}" "${xvfb_pid}" 2>/dev/null || true
}
trap cleanup EXIT INT TERM

display_ready=false
attempt=0
while [ "${attempt}" -lt 50 ]; do
    if xdpyinfo -display "${DISPLAY}" >/dev/null 2>&1; then
        display_ready=true
        break
    fi
    attempt=$((attempt + 1))
    sleep 0.1
done

if [ "${display_ready}" != true ]; then
    echo "Xvfb failed to become ready; see /tmp/stealth-oxide-xvfb.log" >&2
    exit 1
fi

openbox >/tmp/stealth-oxide-openbox.log 2>&1 &
window_manager_pid=$!

window_manager_ready=false
attempt=0
while [ "${attempt}" -lt 50 ]; do
    if xprop -root _NET_SUPPORTED 2>/dev/null | grep -q _NET_SUPPORTED; then
        window_manager_ready=true
        break
    fi
    attempt=$((attempt + 1))
    sleep 0.1
done

if [ "${window_manager_ready}" != true ]; then
    echo "Openbox failed to become ready; see /tmp/stealth-oxide-openbox.log" >&2
    exit 1
fi

tint2 -c /etc/stealth-oxide/tint2rc >/tmp/stealth-oxide-tint2.log 2>&1 &
panel_pid=$!

work_area_ready=false
attempt=0
while [ "${attempt}" -lt 50 ]; do
    if xprop -root _NET_WORKAREA 2>/dev/null | grep -q "1920, 1040"; then
        work_area_ready=true
        break
    fi
    attempt=$((attempt + 1))
    sleep 0.1
done

if [ "${work_area_ready}" != true ]; then
    echo "Tint2 failed to reserve the 1920x1040 work area; see /tmp/stealth-oxide-tint2.log" >&2
    exit 1
fi

exec "$@"

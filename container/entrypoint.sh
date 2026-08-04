#!/bin/sh
set -eu

# Refresh the per-user cache so fonts mounted by Kubernetes or Compose after
# image construction are visible to Chromium. Isolation is opt-in and refuses
# to run without actual font binaries, avoiding a glyph-less browser profile.
if [ "${STEALTH_OXIDE_FONT_ISOLATION:-0}" = "1" ] || [ "${STEALTH_OXIDE_FONT_ISOLATION:-0}" = "true" ]; then
    font_count=$(find /usr/local/share/fonts/windows -maxdepth 1 -type f \( \
        -name '*.ttf' -o -name '*.ttc' -o -name '*.otf' \
    \) | wc -l)
    if [ "${font_count}" -eq 0 ]; then
        echo "Font isolation requested, but no Windows font binaries are mounted" >&2
        exit 1
    fi
    export FONTCONFIG_FILE=/etc/stealth-oxide/windows-fonts.conf
fi
fc-cache -f >/dev/null

# Chromium dynamically discovers native Linux voices through Speech Dispatcher.
# Keep this experimental surface opt-in: eSpeak's large catalog is itself a
# strong Linux fingerprint and is not coherent with the Windows profile.
if [ "${STEALTH_OXIDE_SPEECH_DISPATCHER:-0}" = "1" ] || [ "${STEALTH_OXIDE_SPEECH_DISPATCHER:-0}" = "true" ]; then
    mkdir -p /tmp/stealth-runtime
    chmod 0700 /tmp/stealth-runtime
    export XDG_RUNTIME_DIR=/tmp/stealth-runtime
    speech-dispatcher -d >/tmp/stealth-oxide-speech-dispatcher.log 2>&1 || true
fi

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

#!/usr/bin/env bash
set -u

lab_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
artifact_dir="$lab_root/artifacts"
mkdir -p -- "$artifact_dir"
output="$artifact_dir/host-facts.txt"

{
    echo "timestamp=$(date --iso-8601=seconds)"
    echo "cwd=$(pwd)"
    echo
    echo "== session =="
    printf 'XDG_SESSION_TYPE=%s\n' "${XDG_SESSION_TYPE-}"
    printf 'DISPLAY=%s\n' "${DISPLAY-}"
    printf 'WAYLAND_DISPLAY=%s\n' "${WAYLAND_DISPLAY-}"
    printf 'XDG_CURRENT_DESKTOP=%s\n' "${XDG_CURRENT_DESKTOP-}"
    printf 'XDG_SESSION_DESKTOP=%s\n' "${XDG_SESSION_DESKTOP-}"

    echo
    echo "== compositor/monitors =="
    if command -v hyprctl >/dev/null 2>&1; then
        hyprctl monitors -j
    else
        echo "hyprctl: unavailable"
    fi

    echo
    echo "== DRM devices =="
    ls -l /dev/dri 2>&1 || true

    echo
    echo "== OpenGL =="
    if command -v glxinfo >/dev/null 2>&1; then
        glxinfo -B
    else
        echo "glxinfo: unavailable"
    fi

    echo
    echo "== Vulkan =="
    if command -v vulkaninfo >/dev/null 2>&1; then
        vulkaninfo --summary
    else
        echo "vulkaninfo: unavailable"
    fi

    echo
    echo "== PCI display devices =="
    if command -v lspci >/dev/null 2>&1; then
        lspci -nnk | rg -A3 'VGA|3D|Display' || true
    else
        echo "lspci: unavailable"
    fi

    echo
    echo "== Chromium =="
    for candidate in "${CHROME_BIN-}" google-chrome chromium chromium-browser; do
        if [ -n "$candidate" ] && command -v "$candidate" >/dev/null 2>&1; then
            printf '%s: ' "$candidate"
            "$candidate" --version 2>&1 || true
        fi
    done

    echo
    echo "== Docker =="
    if command -v docker >/dev/null 2>&1; then
        docker info 2>&1 | sed -n '1,80p'
    else
        echo "docker: unavailable"
    fi
} >"$output" 2>&1

echo "Wrote $output"

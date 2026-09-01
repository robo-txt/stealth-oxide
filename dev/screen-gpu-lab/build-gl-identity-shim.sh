#!/usr/bin/env bash
set -euo pipefail

lab_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
cc="${CC:-cc}"

"$cc" -shared -fPIC -O2 -Wall -Wextra -Werror \
    -o "$lab_root/artifacts/libstealth_oxide_gl_identity.so" \
    "$lab_root/gl-identity-shim.c" -ldl

echo "Built $lab_root/artifacts/libstealth_oxide_gl_identity.so"
